use std::f32::consts::TAU;

use glam::{I64Vec2, UVec2, Vec2, Vec4};
use lnworld::{Element, Handle, HandleGeneric, World};

use crate::{
    color::{hsl::Hsl, srgb::SrgbLinear},
    input::{
        collider::{ToolRectCollider, ToolRingCollider},
        pointer::PointerEvent,
    },
    shaders::shader_compile,
    ui::{
        common::WidgetCommon,
        renderer::{
            quad::{QuadMaterial, QuadMesh, SetQuadMaterial},
            rrect::RRect,
        },
    },
    unit::{FI64Ext, IRect},
    val::{Val, ValBind, ValChanged},
};

pub const INNER_RADIUS: f32 = 0.4;
pub const OUTER_RADIUS: f32 = 0.5;

const THUMB_RADIUS: f32 = 5.0;

pub struct HslSquare {
    pub common: WidgetCommon,
    pub rect: Handle<Val<IRect>>,
    pub hsl: Hsl,
}

pub struct HslRing {
    pub common: WidgetCommon,
    pub rect: Handle<Val<IRect>>,
    pub hsl: Hsl,
    pub inner_radius: f32,
}

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub struct HslSquareMaterial {
    hsl: Hsl,
}

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub struct HslRingMaterial {
    hsl: Hsl,
    inner_radius: f32,
    _pad: [u32; 3],
}

pub struct HslPick(pub Hsl);

fn square_thumb(rect: IRect, hsl: Hsl) -> IRect {
    let center = rect.origin + (Vec2::new(hsl.0.y, hsl.0.z) * rect.extend.as_vec2()).as_ivec2();
    IRect::new_half(center, UVec2::splat(THUMB_RADIUS as u32))
}

fn ring_thumb(rect: IRect, hsl: Hsl, inner_radius: f32) -> IRect {
    let angle = hsl.0.x * TAU;
    let radius = (inner_radius + OUTER_RADIUS) * 0.5;
    let uv = Vec2::splat(0.5) + Vec2::new(angle.cos(), angle.sin()) * radius;
    let center = rect.origin + (uv * rect.extend.as_vec2()).as_ivec2();
    IRect::new_half(center, UVec2::splat(THUMB_RADIUS as u32))
}

impl Element for HslSquare {
    fn when_insert(&mut self, world: &World, this: Handle<Self>) {
        world.scope("hsl_square");
        world.set_autodeps(Some(this));

        let thumb_rect = ValBind::new(world, move |world| {
            let this = world.fetch(this);
            let rect = world.fetch(this.rect).val;
            square_thumb(rect, this.hsl)
        })
        .listen(this)
        .listen(self.rect)
        .finish();
        let thumb_light_rect = ValBind::transfer(world, thumb_rect, |rect| rect.expand(1));
        let thumb_dark_rect = ValBind::transfer(world, thumb_rect, |rect| rect.expand(2));
        let thumb_color = ValBind::new(world, move |world| {
            let this = world.fetch(this);
            this.hsl.to_srgb().to_linear()
        })
        .listen(this)
        .finish();

        let quad = world.insert(QuadMesh {
            common: self.common,
            rect: self.rect,
            order: 100,
            material: HslSquareMaterial { hsl: self.hsl },
        });

        world.insert(RRect {
            common: self.common,
            rect: thumb_dark_rect,
            order: 100,
            color: world.insert(Val::new(SrgbLinear(Vec4::W))),
            radius: THUMB_RADIUS + 2.0,
            width: 0.0,
        });

        world.insert(RRect {
            common: self.common,
            rect: thumb_light_rect,
            order: 100,
            color: world.insert(Val::new(SrgbLinear(Vec4::ONE))),
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

            this.hsl.0.y = uv.x.clamp(0.0, 1.0);
            this.hsl.0.z = uv.y.clamp(0.0, 1.0);
            world.queue_trigger(this.handle(), HslPick(this.hsl));
        });

        world.observer(this, move |ValChanged, world| {
            let this = world.fetch(this);
            world.queue_trigger(quad, SetQuadMaterial(HslSquareMaterial { hsl: this.hsl }));
        });
    }
}

impl Element for HslRing {
    fn when_insert(&mut self, world: &World, this: Handle<Self>) {
        world.scope("hsl_ring");
        world.set_autodeps(Some(this));
        world.dependency(this, self.rect);
        world.dependency(this, self.common.visible);

        let thumb_rect = ValBind::new(world, move |world| {
            let this = world.fetch(this);
            let rect = world.fetch(this.rect).val;
            ring_thumb(rect, this.hsl, this.inner_radius)
        })
        .listen(this)
        .listen(self.rect)
        .finish();
        let thumb_light_rect = ValBind::transfer(world, thumb_rect, |rect| rect.expand(1));
        let thumb_dark_rect = ValBind::transfer(world, thumb_rect, |rect| rect.expand(2));
        let thumb_color = ValBind::new(world, move |world| {
            let this = world.fetch(this);
            this.hsl.to_srgb().to_linear()
        })
        .listen(this)
        .finish();

        let quad = world.insert(QuadMesh {
            common: self.common,
            rect: self.rect,
            order: 100,
            material: HslRingMaterial {
                hsl: self.hsl,
                inner_radius: self.inner_radius,
                _pad: [0; 3],
            },
        });

        world.insert(RRect {
            common: self.common,
            rect: thumb_dark_rect,
            order: 100,
            color: world.insert(Val::new(SrgbLinear(Vec4::W))),
            radius: THUMB_RADIUS + 2.0,
            width: 0.0,
        });

        world.insert(RRect {
            common: self.common,
            rect: thumb_light_rect,
            order: 100,
            color: world.insert(Val::new(SrgbLinear(Vec4::ONE))),
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

        world.insert(ToolRingCollider {
            common: self.common,
            target: this.untyped(),
            rect: self.rect,
            inner_radius: self.inner_radius,
            outer_radius: OUTER_RADIUS,
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
            let delta = uv - Vec2::splat(0.5);

            this.hsl.0.x = (delta.y.atan2(delta.x) / TAU).rem_euclid(1.0);
            world.queue_trigger(this.handle(), HslPick(this.hsl));
        });

        world.observer(this, move |ValChanged, world| {
            let this = world.fetch(this);
            world.queue_trigger(
                quad,
                SetQuadMaterial(HslRingMaterial {
                    hsl: this.hsl,
                    inner_radius: this.inner_radius,
                    _pad: [0; 3],
                }),
            );
        });
    }
}

impl QuadMaterial for HslSquareMaterial {
    fn label() -> &'static str {
        "palette_hsl_square"
    }

    fn shader() -> wgpu::ShaderSource<'static> {
        shader_compile(include_str!("hsl_square.wgsl"), &[])
    }

    fn fragment() -> Option<&'static str> {
        Some("main")
    }
}

impl QuadMaterial for HslRingMaterial {
    fn label() -> &'static str {
        "palette_hsl_ring"
    }

    fn shader() -> wgpu::ShaderSource<'static> {
        shader_compile(include_str!("hsl_ring.wgsl"), &[]).into()
    }

    fn fragment() -> Option<&'static str> {
        Some("main")
    }

    fn edge(&self) -> UVec2 {
        UVec2::splat(2)
    }
}
