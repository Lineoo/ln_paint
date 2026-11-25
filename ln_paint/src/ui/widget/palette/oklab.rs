use glam::{I64Vec2, IVec2, UVec2, Vec2, Vec4};
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

pub struct ColorOklab(pub Oklab);
pub struct SetColorOklab(pub Oklab);

fn thumb_color(oklab: Oklab) -> SrgbLinear {
    oklab.to_srgb_linear()
}

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

impl OklabPolar {
    fn init(&mut self, world: &World, this: Handle<Self>) {
        world.scope("oklab_polar");
        world.set_autodeps(Some(this));

        let thumb_rect = ValBind::new(world, move |world| {
            let this = world.fetch(this);
            let rect = world.fetch(this.rect).val;
            polar_thumb(rect, this.oklab)
        })
        .listen(self.rect)
        .finish();
        let thumb_light_rect = ValBind::transfer(world, thumb_rect, |rect| rect.expand(1));
        let thumb_shadow_rect = ValBind::transfer(world, thumb_rect, |rect| rect.expand(2));

        let quad = world.insert(QuadMesh {
            common: self.common,
            rect: self.rect,
            order: 60,
            material: OklabPolarMaterial { oklab: self.oklab },
        });

        let chroma_indicator = world.insert(Vtor {
            common: self.common,
            rect: self.rect,
            order: 70,
            color: SrgbLinear(Vec4::new(0.2, 0.2, 0.2, 0.4)),
            path: chroma_to_path(Vec2::new(self.oklab.0.y, self.oklab.0.z).length()),
            outdated: false,
        });

        let thumb_color_val = world.insert(Val::new(thumb_color(self.oklab)));
        world.insert(RRect {
            common: self.common,
            rect: thumb_rect,
            order: 150,
            color: thumb_color_val,
            radius: THUMB_RADIUS,
            width: 0.0,
        });

        world.insert(RRect {
            common: self.common,
            rect: thumb_light_rect,
            order: 140,
            color: world.insert(Val::new(SrgbLinear(Vec4::ONE))),
            radius: THUMB_RADIUS + 1.0,
            width: 0.0,
        });

        world.insert(RRect {
            common: self.common,
            rect: thumb_shadow_rect,
            order: 130,
            color: world.insert(Val::new(SrgbLinear(Vec4::W))),
            radius: THUMB_RADIUS + 2.0,
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
            world.queue_trigger(this.handle(), ColorOklab(this.oklab));
        });

        world.observer(this, move |&SetColorOklab(oklab), world| {
            let mut this = world.fetch_mut(this);
            let mut chroma_indicator = world.fetch_mut(chroma_indicator);
            let rect = world.fetch(this.rect).val;
            let mut thumb_rect = world.fetch_mut(thumb_rect);
            let mut thumb_color_val = world.fetch_mut(thumb_color_val);
            this.oklab = oklab;
            chroma_indicator.path = chroma_to_path(Vec2::new(oklab.0.y, oklab.0.z).length());
            chroma_indicator.outdated = true;
            thumb_rect.val = polar_thumb(rect, this.oklab);
            thumb_color_val.val = thumb_color(oklab);
            world.queue_trigger(thumb_rect.handle(), ValChanged);
            world.queue_trigger(thumb_color_val.handle(), ValChanged);
            world.queue_trigger(quad, SetQuadMaterial(this.mat()));
        });
    }

    fn mat(&self) -> OklabPolarMaterial {
        OklabPolarMaterial { oklab: self.oklab }
    }
}

impl OklabBar {
    fn init(&mut self, world: &World, this: Handle<Self>) {
        world.scope("oklab_bar");
        world.set_autodeps(Some(this));
        world.dependency(this, self.rect);
        world.dependency(this, self.common.visible);

        let thumb_rect = ValBind::new(world, move |world| {
            let this = world.fetch(this);
            let rect = world.fetch(this.rect).val;
            bar_thumb(rect, this.oklab)
        })
        .listen(self.rect)
        .finish();
        let thumb_light_rect = ValBind::transfer(world, thumb_rect, |rect| rect.expand(1));
        let thumb_shadow_rect = ValBind::transfer(world, thumb_rect, |rect| rect.expand(2));

        let quad = world.insert(QuadMesh {
            common: self.common,
            rect: self.rect,
            order: 60,
            material: OklabBarMaterial { oklab: self.oklab },
        });

        let thumb_color_val = world.insert(Val::new(thumb_color(self.oklab)));
        world.insert(RRect {
            common: self.common,
            rect: thumb_rect,
            order: 150,
            color: thumb_color_val,
            radius: THUMB_RADIUS,
            width: 0.0,
        });

        world.insert(RRect {
            common: self.common,
            rect: thumb_light_rect,
            order: 140,
            color: world.insert(Val::new(SrgbLinear(Vec4::ONE))),
            radius: THUMB_RADIUS + 1.0,
            width: 0.0,
        });

        world.insert(RRect {
            common: self.common,
            rect: thumb_shadow_rect,
            order: 130,
            color: world.insert(Val::new(SrgbLinear(Vec4::W))),
            radius: THUMB_RADIUS + 2.0,
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
            world.queue_trigger(this.handle(), ColorOklab(this.oklab));
        });

        world.observer(this, move |&SetColorOklab(oklab), world| {
            let mut this = world.fetch_mut(this);
            let rect = world.fetch(this.rect).val;
            let mut thumb_rect = world.fetch_mut(thumb_rect);
            let mut thumb_color_val = world.fetch_mut(thumb_color_val);
            this.oklab = oklab;
            thumb_rect.val = bar_thumb(rect, this.oklab);
            thumb_color_val.val = thumb_color(oklab);
            world.queue_trigger(thumb_rect.handle(), ValChanged);
            world.queue_trigger(thumb_color_val.handle(), ValChanged);
            world.queue_trigger(quad, SetQuadMaterial(this.mat()));
        });
    }

    fn mat(&self) -> OklabBarMaterial {
        OklabBarMaterial { oklab: self.oklab }
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

impl Element for OklabPolar {
    fn when_insert(&mut self, world: &World, this: Handle<Self>) {
        self.init(world, this);
    }
}

impl Element for OklabBar {
    fn when_insert(&mut self, world: &World, this: Handle<Self>) {
        self.init(world, this);
    }
}
