use glam::{I64Vec2, IVec2, UVec2, Vec2};
use ln_world::{Element, Handle, HandleGeneric, World};
use lyon::{
    math::Point,
    path::{Path, Winding},
};

use crate::{
    color::{oklab::Oklab, srgb::SrgbLinear},
    input::{collider::ToolRectCollider, pointer::PointerEvent},
    shaders::shader_compile,
    ui::{
        common::WidgetCommon,
        renderer::{
            quad::{QuadMaterial, QuadMesh, SetQuadMaterial},
            rrect::RRect,
            vtor::Vtor,
        },
    },
    unit::{FI64Ext, IRect},
    val::{Val, ValBind, ValChanged},
};

const THUMB_RADIUS: f32 = 5.0;

pub struct OklabPolar {
    pub common: WidgetCommon,
    pub rect: Handle<Val<IRect>>,
    pub oklab: Oklab,
}

pub struct OklabBar {
    pub common: WidgetCommon,
    pub rect: Handle<Val<IRect>>,
    pub oklab: Oklab,
}

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub struct OklabPolarMaterial {
    oklab: Oklab,
}

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub struct OklabBarMaterial {
    oklab: Oklab,
}

pub struct OklabPick(pub Oklab);

fn polar_thumb(rect: IRect, oklab: Oklab) -> IRect {
    IRect::new_half(
        rect.origin
            + (Vec2::new(oklab.0.y / 0.8 + 0.5, oklab.0.z / 0.8 + 0.5) * rect.extend.as_vec2())
                .as_ivec2(),
        UVec2::splat(THUMB_RADIUS as u32),
    )
}

fn bar_thumb(rect: IRect, oklab: Oklab) -> IRect {
    IRect::new_half(
        IVec2::new(
            rect.horizontal_center(),
            rect.origin.y + (oklab.0.x * rect.extend.y as f32) as i32,
        ),
        UVec2::new(rect.extend.x / 2, THUMB_RADIUS as u32),
    )
}

fn chroma_to_path(c: f32) -> Path {
    let mut builder = Path::builder();
    builder.add_circle(Point::new(0.5, 0.5), c / 0.8 + 0.005, Winding::Positive);
    builder.add_circle(Point::new(0.5, 0.5), c / 0.8 - 0.005, Winding::Negative);
    builder.build()
}

impl Element for OklabPolar {
    fn when_insert(&mut self, world: &World, this: Handle<Self>) {
        world.scope("oklab_polar");
        world.set_autodeps(Some(this));

        let theme = world.fetch(self.common.theme);

        let thumb_rect = ValBind::new(world, move |world| {
            let this = world.fetch(this);
            let rect = world.fetch(this.rect).val;
            polar_thumb(rect, this.oklab)
        })
        .listen(this)
        .listen(self.rect)
        .finish();
        let thumb_light_rect = ValBind::transfer(world, thumb_rect, |rect| rect.expand(1));
        let thumb_dark_rect = ValBind::transfer(world, thumb_rect, |rect| rect.expand(2));
        let thumb_color = ValBind::new(world, move |world| {
            let this = world.fetch(this);
            this.oklab.to_srgb_linear()
        })
        .listen(this)
        .finish();
        let chroma_indicator_color = ValBind::new(world, move |world| {
            let this = world.fetch(this);
            match this.oklab.0.x > 0.5 {
                false => SrgbLinear::WHITE * 0.4,
                true => SrgbLinear::BLACK * 0.4,
            }
        })
        .listen(this)
        .animate(theme.animation);

        world.insert(Vtor {
            common: self.common,
            rect: self.rect,
            color: theme.color.surface_container,
            order: 100,
            path: {
                let mut builder = Path::builder();
                builder.add_circle(Point::new(0.5, 0.5), 0.5, Winding::Positive);
                builder.build()
            },
            outdated: false,
        });

        let quad = world.insert(QuadMesh {
            common: self.common,
            rect: self.rect,
            order: 100,
            material: OklabPolarMaterial { oklab: self.oklab },
        });

        let chroma_indicator = world.insert(Vtor {
            common: self.common,
            rect: self.rect,
            order: 100,
            color: chroma_indicator_color,
            path: chroma_to_path(Vec2::new(self.oklab.0.y, self.oklab.0.z).length()),
            outdated: false,
        });

        world.insert(RRect {
            common: self.common,
            rect: thumb_dark_rect,
            order: 100,
            color: world.insert(Val::new(SrgbLinear::BLACK)),
            radius: THUMB_RADIUS + 2.0,
            width: 0.0,
        });

        world.insert(RRect {
            common: self.common,
            rect: thumb_light_rect,
            order: 100,
            color: world.insert(Val::new(SrgbLinear::WHITE)),
            radius: THUMB_RADIUS + 1.0,
            width: 0.0,
        });

        world.insert(RRect {
            common: self.common,
            rect: thumb_rect,
            order: 100,
            color: thumb_color,
            radius: THUMB_RADIUS,
            width: 0.0,
        });

        world.insert(ToolRectCollider {
            common: self.common,
            target: this.untyped(),
            rect: self.rect,
            order: 100,
        });

        world.observer(this, move |event: &PointerEvent, world| {
            let Some(capture) = event.capture() else {
                return;
            };

            let mut this = world.fetch_mut(this);
            let rect = world.fetch(this.rect).val;
            let delta = capture.pointer.position - I64Vec2::q32_from_i32(rect.origin);
            let uv = (delta.q32_as_f64() / rect.extend.as_dvec2()).as_vec2();

            this.oklab.0.y = (uv.x * 0.8 - 0.4).clamp(-0.4, 0.4);
            this.oklab.0.z = (uv.y * 0.8 - 0.4).clamp(-0.4, 0.4);
            this.oklab = this.oklab.clip_srgb();
            world.queue_trigger(this.handle(), OklabPick(this.oklab));
        });

        world.observer(this, move |ValChanged, world| {
            let this = world.fetch(this);
            let mut chroma_indicator = world.fetch_mut(chroma_indicator);
            chroma_indicator.path =
                chroma_to_path(Vec2::new(this.oklab.0.y, this.oklab.0.z).length());
            chroma_indicator.outdated = true;
            world.queue_trigger(
                quad,
                SetQuadMaterial(OklabPolarMaterial { oklab: this.oklab }),
            );
        });
    }
}

impl Element for OklabBar {
    fn when_insert(&mut self, world: &World, this: Handle<Self>) {
        world.scope("oklab_bar");
        world.set_autodeps(Some(this));
        world.dependency(this, self.rect);
        world.dependency(this, self.common.visible);

        let theme = world.fetch(self.common.theme);

        let thumb_rect = ValBind::new(world, move |world| {
            let this = world.fetch(this);
            let rect = world.fetch(this.rect).val;
            bar_thumb(rect, this.oklab)
        })
        .listen(this)
        .listen(self.rect)
        .finish();
        let thumb_light_rect = ValBind::transfer(world, thumb_rect, |rect| rect.expand(1));
        let thumb_dark_rect = ValBind::transfer(world, thumb_rect, |rect| rect.expand(2));
        let thumb_color = ValBind::new(world, move |world| {
            let this = world.fetch(this);
            this.oklab.to_srgb_linear()
        })
        .listen(this)
        .finish();

        world.insert(RRect {
            common: self.common,
            rect: self.rect,
            order: 100,
            color: theme.color.surface_container,
            radius: 0.0,
            width: 0.0,
        });

        let quad = world.insert(QuadMesh {
            common: self.common,
            rect: self.rect,
            order: 100,
            material: OklabBarMaterial { oklab: self.oklab },
        });

        world.insert(RRect {
            common: self.common,
            rect: thumb_dark_rect,
            order: 100,
            color: world.insert(Val::new(SrgbLinear::BLACK)),
            radius: THUMB_RADIUS + 2.0,
            width: 0.0,
        });

        world.insert(RRect {
            common: self.common,
            rect: thumb_light_rect,
            order: 100,
            color: world.insert(Val::new(SrgbLinear::WHITE)),
            radius: THUMB_RADIUS + 1.0,
            width: 0.0,
        });

        world.insert(RRect {
            common: self.common,
            rect: thumb_rect,
            order: 100,
            color: thumb_color,
            radius: THUMB_RADIUS,
            width: 0.0,
        });

        world.insert(ToolRectCollider {
            common: self.common,
            target: this.untyped(),
            rect: self.rect,
            order: 100,
        });

        world.observer(this, move |event: &PointerEvent, world| {
            let Some(capture) = event.capture() else {
                return;
            };

            let mut this = world.fetch_mut(this);
            let rect = world.fetch(this.rect).val;
            let delta = capture.pointer.position - I64Vec2::q32_from_i32(rect.origin);
            let uv = (delta.q32_as_f64() / rect.extend.as_dvec2()).as_vec2();

            this.oklab.0.x = uv.y.clamp(0.0, 1.0);
            this.oklab = this.oklab.clip_srgb();
            world.queue_trigger(this.handle(), OklabPick(this.oklab));
        });

        world.observer(this, move |ValChanged, world| {
            let this = world.fetch(this);
            world.queue_trigger(
                quad,
                SetQuadMaterial(OklabBarMaterial { oklab: this.oklab }),
            );
        });
    }
}

impl QuadMaterial for OklabPolarMaterial {
    fn label() -> &'static str {
        "palette_oklab_polar"
    }

    fn shader() -> wgpu::ShaderSource<'static> {
        shader_compile(include_str!("oklab_polar.wgsl"), &[])
    }

    fn fragment() -> Option<&'static str> {
        Some("main")
    }
}

impl QuadMaterial for OklabBarMaterial {
    fn label() -> &'static str {
        "palette_oklab_bar"
    }

    fn shader() -> wgpu::ShaderSource<'static> {
        shader_compile(include_str!("oklab_bar.wgsl"), &[])
    }

    fn fragment() -> Option<&'static str> {
        Some("main")
    }
}
