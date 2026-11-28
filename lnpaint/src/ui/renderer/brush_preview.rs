use glam::{DVec2, I64Vec2, UVec2};
use lnworld::{Descriptor, Element, Handle, HandleGeneric, World};
use wgpu::{
    CommandEncoderDescriptor, ComputePassDescriptor, Extent3d, TexelCopyBufferLayout,
    TexelCopyTextureInfo, TextureAspect,
};

use crate::{
    color::srgb::{Srgb, SrgbLinear, SrgbLinearPremultipliedEncoded},
    input::collider::ToolRectCollider,
    paint::{
        ChunkRenderer, PaintPipeline, Standalone,
        brush::{BrushParams, Dab, DabBatch, Stroke},
        wrapper::LayerPage,
    },
    render::{
        RenderDrawControl, RenderPrepareControl,
        nest::{Nest, NestAttr, NestBuilder},
    },
    ui::common::WidgetCommon,
    unit::{FI64Ext, IRect},
    val::{Val, ValBind, ValChanged},
};

pub struct BrushPreviewDescriptor {
    pub common: WidgetCommon,
    pub rect: Handle<Val<IRect>>,
    pub bg: Handle<Val<SrgbLinear>>,
    pub bg_color1: Handle<Val<SrgbLinear>>,
    pub bg_color2: Handle<Val<SrgbLinear>>,
    pub brush_color: Handle<Val<SrgbLinear>>,
    pub brush: Box<dyn BrushParams>,
}

/// A widget that displays an engine-rendered brush preview.
pub struct BrushPreview {
    pub common: WidgetCommon,
    pub rect: Handle<Val<IRect>>,
    pub bg: Handle<Val<Srgb>>,
    pub bg_color1: Handle<Val<Srgb>>,
    pub bg_color2: Handle<Val<Srgb>>,
    pub brush_color: Handle<Val<Srgb>>,
    pub brush: Box<dyn BrushParams>,
    pub canvas: Handle<Val<Standalone>>,
    pub renderer: ChunkRenderer,
    pub outdated: bool,
}

impl Descriptor for BrushPreviewDescriptor {
    type Target = BrushPreview;

    fn when_build(self, world: &World) -> Self::Target {
        let page = world.single_fetch::<LayerPage>();
        let renderer = page.lp.create_render(world.fetch(self.bg).val.to_encoded());
        let rect = self.rect;
        let canvas = world.insert_with(move |world| {
            let page = world.single_fetch::<LayerPage>();
            let rect = world.fetch(rect).val;
            let canvas = page.lp.create_standalone(rect);
            Val::new(canvas)
        });
        let bg = ValBind::transfer(world, self.bg, |&c| c.to_srgb());
        let bg_color1 = ValBind::transfer(world, self.bg_color1, |&c| c.to_srgb());
        let bg_color2 = ValBind::transfer(world, self.bg_color2, |&c| c.to_srgb());
        let brush_color = ValBind::transfer(world, self.brush_color, |&c| c.to_srgb());
        BrushPreview {
            common: self.common,
            rect: self.rect,
            bg,
            bg_color1,
            bg_color2,
            brush_color,
            brush: self.brush,
            canvas,
            renderer,
            outdated: false,
        }
    }
}

impl Element for BrushPreview {
    fn when_insert(&mut self, world: &World, this: Handle<Self>) {
        world.scope("brush_preview");
        world.set_autodeps(Some(this));
        world.dependency(this, self.rect);
        world.dependency(this, self.common.visible);

        let (nest, inner, common) = world.build(NestBuilder {
            common: self.common,
            rect: self.rect,
            attr: NestAttr {
                anchor: DVec2::new(0.0, 0.0),
                ..Default::default()
            },
        });
        world.queue(move |world| Nest::refresh_camera(world, nest));

        world.insert(ToolRectCollider {
            common,
            target: this.untyped(),
            rect: inner,
            order: 0,
        });

        world.insert(RenderPrepareControl {
            prepare: Box::new(move |world| {
                let mut this = world.fetch_mut(this);
                let visible = world.fetch(this.common.visible).val;
                if this.outdated && visible {
                    let page = world.single_fetch::<LayerPage>();
                    let bg_color1 = world.fetch(this.bg_color1).val.to_linear().to_encoded();
                    let bg_color2 = world.fetch(this.bg_color2).val.to_linear().to_encoded();
                    let brush_color = world.fetch(this.brush_color).val;
                    let canvas = world.fetch(this.canvas);
                    Self::generate_preview(
                        &page.lp,
                        &canvas.val,
                        this.brush.as_mut(),
                        bg_color1,
                        bg_color2,
                        brush_color,
                    );
                    this.outdated = false;
                }
            }),
        });

        world.insert(RenderDrawControl {
            common,
            order: 0,
            draw: Box::new(move |world, rpass, extra| {
                let this = world.fetch(this);
                let canvas = world.fetch(this.canvas);
                this.renderer
                    .render_chunk(rpass, extra.camera, &canvas.val.chunk);
            }),
        });

        world.observer(self.rect, move |ValChanged, world| {
            let mut this = world.fetch_mut(this);
            let rect = world.fetch(this.rect).val;
            let mut canvas = world.fetch_mut(this.canvas);

            let size = UVec2::new(rect.width(), rect.height());
            if size != canvas.val.rect.extend {
                let page = world.single_fetch::<LayerPage>();
                canvas.val = page
                    .lp
                    .create_standalone(IRect::new_extend(0, 0, size.x, size.y));
                this.outdated = true;
            }

            world.queue(move |world| Nest::refresh_camera(world, nest));
        });

        world.observer(self.bg, move |ValChanged, world| {
            let this = world.fetch(this);
            let background_color = world.fetch(this.bg).val;
            this.renderer
                .write_background(background_color.to_linear().to_encoded());
        });

        world.observer(self.bg_color1, move |ValChanged, world| {
            world.fetch_mut(this).outdated = true;
        });

        world.observer(self.bg_color2, move |ValChanged, world| {
            world.fetch_mut(this).outdated = true;
        });

        world.observer(self.brush_color, move |ValChanged, world| {
            world.fetch_mut(this).outdated = true;
        });
    }
}

impl BrushPreview {
    /// Redraw `brush` over a fixed seed into `canvas`.
    pub fn generate_preview(
        lp: &PaintPipeline,
        canvas: &Standalone,
        brush: &mut dyn BrushParams,
        bg_color1: SrgbLinearPremultipliedEncoded,
        bg_color2: SrgbLinearPremultipliedEncoded,
        brush_color: Srgb,
    ) {
        let swap = (canvas.chunk.read_write.is_none()).then(|| lp.create_standalone(canvas.rect));

        let mut encoder = (lp.device).create_command_encoder(&CommandEncoderDescriptor {
            label: Some("preview_paint_clear"),
        });

        let mut cpass = encoder.begin_compute_pass(&ComputePassDescriptor {
            label: Some("preview_paint_clear"),
            timestamp_writes: None,
        });

        lp.clear_chunk_direct(&mut cpass, &canvas.chunk, canvas.rect);

        drop(cpass);
        lp.queue.submit([encoder.finish()]);

        if brush.replace_mode() {
            Self::draw_background(lp, canvas, bg_color1, bg_color2);
        }

        brush.set_color(brush_color);
        Self::draw_stroke(lp, canvas, &swap, brush);
    }

    fn submit_batch(
        lp: &PaintPipeline,
        canvas: &Standalone,
        swap: &Option<Standalone>,
        brush: &dyn BrushParams,
        batch: &DabBatch,
    ) {
        batch.upload_dispatch(lp);

        let mut encoder = (lp.device).create_command_encoder(&CommandEncoderDescriptor {
            label: Some("preview_draw"),
        });

        let mut cpass = encoder.begin_compute_pass(&ComputePassDescriptor {
            label: Some("preview_draw"),
            timestamp_writes: None,
        });

        batch.draw_into_standalone(lp, &mut cpass, canvas, swap, brush);

        drop(cpass);
        lp.queue.submit([encoder.finish()]);
    }

    /// Two differently colored halves, left and right, as the preview background.
    fn draw_background(
        lp: &PaintPipeline,
        canvas: &Standalone,
        c0: SrgbLinearPremultipliedEncoded,
        c1: SrgbLinearPremultipliedEncoded,
    ) {
        let size = canvas.rect.extend;
        if size.x == 0 || size.y == 0 {
            return;
        }

        let left = c0.to_rgba_bytes();
        let right = c1.to_rgba_bytes();
        let split = size.x / 2;

        let mut row = Vec::with_capacity((size.x * 4) as usize);
        for x in 0..size.x {
            row.extend_from_slice(if x < split { &left } else { &right });
        }

        let mut data = Vec::with_capacity(row.len() * size.y as usize);
        for _ in 0..size.y {
            data.extend_from_slice(&row);
        }

        lp.queue.write_texture(
            TexelCopyTextureInfo {
                texture: &canvas.chunk.texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: TextureAspect::All,
            },
            &data,
            TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(size.x * 4),
                rows_per_image: Some(size.y),
            },
            Extent3d {
                width: size.x,
                height: size.y,
                depth_or_array_layers: 1,
            },
        );
    }

    /// A gentle S-curve with a pressure ramp, drawn with the previewed brush.
    fn draw_stroke(
        lp: &PaintPipeline,
        canvas: &Standalone,
        swap: &Option<Standalone>,
        brush: &dyn BrushParams,
    ) {
        const STEPS: i32 = 24;

        let mut stroke = Stroke::new(brush);
        let mut batches = Vec::new();

        for i in 0..=STEPS {
            let t = i as f32 / STEPS as f32;
            let x = canvas.rect.width() as f32 * 0.5 - 125. + 250. * t;
            let y = canvas.rect.height() as f32 * 0.5 + 10. * (t * std::f32::consts::TAU).sin();
            let f = 4. * t * (1. - t);
            let draw = Dab {
                position: I64Vec2::q32_from_f64(DVec2::new(x as f64, y as f64)),
                force: 0.25 + 0.75 * f,
            };

            let Some(batch) = stroke.batch(brush, draw) else {
                break;
            };

            stroke.confirm_batch(&batch);
            batches.push(batch);
        }

        let mut merged = Vec::new();

        for batch in batches {
            let Some(prev) = merged.pop() else {
                merged.push(batch);
                continue;
            };

            let (prev, rest) = prev.merge(batch);
            merged.push(prev);
            if let Some(rest) = rest {
                merged.push(rest);
            }
        }

        for batch in &merged {
            Self::submit_batch(lp, canvas, swap, brush, batch);
        }
    }
}
