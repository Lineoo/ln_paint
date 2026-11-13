use glam::{DVec2, I64Vec2, UVec2};
use ln_world::{Descriptor, Element, Handle, HandleGeneric, World};

use crate::{
    input::collider::{ToolControl, ToolHitResult},
    lnwin::Lnwindow,
    render::{
        Render, RenderDrawControl, RenderExtra,
        camera::{Camera, CameraDescriptor, CameraOutdated, SetCameraOutdated},
    },
    ui::common::WidgetCommon,
    unit::{FI64Ext, IRect},
    val::{Val, ValChanged},
};

/// separated coordination
pub struct Nest {
    pub common: WidgetCommon,
    pub order: isize,
    pub child: Handle<Camera>,
    /// outer rectangle
    pub rect: Handle<Val<IRect>>,
    pub attr: NestAttr,
}

#[derive(Clone, Copy)]
pub struct NestAttr {
    /// where is the zero point on outer rectangle
    pub anchor: DVec2,
    /// whether scissor rect should be set
    pub clip: bool,
    /// whether to ignore parent camera's linear transform
    pub top: bool,
    pub center: I64Vec2,
    pub zoom: i64,
}

pub struct NestedCamera {
    pub common: WidgetCommon,
    /// outer rectangle
    pub rect: Handle<Val<IRect>>,
    pub attr: NestAttr,
}

impl Default for NestAttr {
    fn default() -> Self {
        Self {
            anchor: DVec2::new(0.0, 1.0),
            clip: true,
            top: false,
            center: I64Vec2::ZERO,
            zoom: 0,
        }
    }
}

impl Descriptor for NestedCamera {
    type Target = (Handle<Nest>, WidgetCommon);

    fn when_build(self, world: &World) -> Self::Target {
        let camera = world.insert_with(move |world| {
            let parent = world.fetch(self.common.camera);
            let rect = world.fetch(self.rect).val;
            world.build(self.attr.calc_camera(rect, &parent))
        });

        let container = world.insert(Nest {
            common: self.common,
            order: 100,
            child: camera,
            rect: self.rect,
            attr: self.attr,
        });

        let common = WidgetCommon {
            camera,
            visible: self.common.visible,
        };

        (container, common)
    }
}

impl Element for Nest {
    fn when_insert(&mut self, world: &World, handle: Handle<Self>) {
        world.scope("container");
        world.set_autodeps(Some(handle));

        world.dependency(handle, self.rect);
        world.dependency(handle, self.common.visible);
        world.dependency(self.child, handle);
        world.dependency(handle, self.common.camera);

        world.insert(RenderDrawControl {
            common: self.common,
            order: self.order,
            draw: Box::new(move |world, rpass, extra| {
                let lnwindow = world.single_fetch::<Lnwindow>();
                let this = world.fetch(handle);
                let child = world.fetch(this.child);

                if this.attr.clip {
                    let camera = extra.camera;
                    let panel_rect = world.fetch(this.rect).val;
                    let clip = lnwindow.screen_scissor(panel_rect, camera);

                    let clipped = match extra.scissor.intersect(clip) {
                        Some(result) => result,
                        // nothing to draw here, skip
                        None => return,
                    };

                    rpass.set_scissor_rect(
                        clipped.origin.x,
                        clipped.origin.y,
                        clipped.extend.x,
                        clipped.extend.y,
                    );
                    Render::draw(
                        &child,
                        world,
                        rpass,
                        RenderExtra {
                            scissor: clipped,
                            ..extra
                        },
                    );

                    // revert
                    rpass.set_scissor_rect(
                        extra.scissor.origin.x,
                        extra.scissor.origin.y,
                        extra.scissor.extend.x,
                        extra.scissor.extend.y,
                    );
                } else {
                    Render::draw(&child, world, rpass, extra);
                }
            }),
        });

        world.insert(ToolControl {
            common: self.common,
            order: self.order,
            hit: Box::new(move |world, screen| {
                let this = world.fetch(handle);
                let parent = world.fetch(this.common.camera);
                let position = parent.screen_to_world(screen);
                let rect = world.fetch(this.rect).val;
                if rect.contains(position.q32_floor()) {
                    if let Some(hit) = ToolControl::intersect(world, screen, this.child) {
                        Some(hit)
                    } else {
                        Some(ToolHitResult {
                            camera: this.child,
                            handle: handle.untyped(),
                        })
                    }
                } else {
                    None
                }
            }),
        });

        world.observer(self.common.camera, move |CameraOutdated, world| {
            let this = world.fetch(handle);
            world.queue_trigger(this.child, SetCameraOutdated);
        });

        world.observer(self.child, move |SetCameraOutdated, world| {
            let this = world.fetch(handle);
            let lnwindow = world.single_fetch::<Lnwindow>();
            let parent_camera = world.fetch(this.common.camera);
            let mut child_camera = world.fetch_mut(this.child);
            let rect = world.fetch(this.rect).val;
            let size = lnwindow.window.surface_size();

            this.update_camera(rect, &parent_camera, &mut child_camera);
            child_camera.surface_size = UVec2::new(size.width, size.height);
            child_camera.upload();
            lnwindow.window.request_redraw();

            world.queue_trigger(this.child, CameraOutdated);
        });

        world.observer(self.rect, move |ValChanged, world| {
            let this = world.fetch(handle);
            world.queue_trigger(this.child, SetCameraOutdated);
        });
    }
}

impl Nest {
    pub fn update_camera(&self, rect: IRect, parent: &Camera, child: &mut Camera) {
        let desc = self.attr.calc_camera(rect, parent);
        child.zoom = desc.zoom;
        child.center = desc.center;
    }

    pub fn inner_rect(&self, rect: IRect, center: I64Vec2, zoom: i64) -> IRect {
        let anchor = rect.q32_anchor(self.attr.anchor);
        let scale = zoom.q32_as_f64().exp2();
        let center = center.q32_as_f64();
        let min = (I64Vec2::q32_from_i32(rect.left_down()) - anchor).q32_as_f64();
        let max = (I64Vec2::q32_from_i32(rect.right_up()) - anchor).q32_as_f64();
        let min = (min / scale + center).round();
        let max = (max / scale + center).round();
        IRect::new_minmax(min.as_ivec2(), max.as_ivec2())
    }

    pub fn clamp_scroll(&mut self, rect: IRect, restrict: IRect) {
        let min = I64Vec2::q32_from_i32(rect.left_down());
        let max = I64Vec2::q32_from_i32(rect.right_up());
        let center = rect.q32_anchor(self.attr.anchor);
        let min = min - center;
        let max = max - center;
        let x = match rect.width() < restrict.width() {
            true => {
                let min = i64::q32_from_i32(restrict.left()) - min.x;
                let max = i64::q32_from_i32(restrict.right()) - max.x;
                self.attr.center.x.clamp(min, max)
            }
            false => 0,
        };
        let y = match rect.height() < restrict.height() {
            true => {
                let min = i64::q32_from_i32(restrict.down()) - min.y;
                let max = i64::q32_from_i32(restrict.up()) - max.y;
                self.attr.center.y.clamp(min, max)
            }
            false => 0,
        };

        self.attr.center = I64Vec2::new(x, y);
    }
}

impl NestAttr {
    pub fn calc_camera(&self, rect: IRect, parent: &Camera) -> CameraDescriptor {
        let zoom = match !self.top {
            true => parent.zoom + self.zoom,
            false => self.zoom,
        };
        let scale = (zoom - parent.zoom).q32_as_f64().exp2();
        let offset = (parent.center - rect.q32_anchor(self.anchor)).q32_as_f64();
        let center = I64Vec2::q32_from_f64(offset / scale) + self.center;
        CameraDescriptor { center, zoom }
    }
}

/// - inner area when scroll is zero for container
pub struct NestBind {
    pub container: Handle<Nest>,
}

impl Descriptor for NestBind {
    type Target = Handle<Val<IRect>>;

    fn when_build(self, world: &World) -> Self::Target {
        let container = self.container;
        let inner = world.insert(Val::new(IRect::default()));

        world.queue(move |world| {
            world.scope("container_echo");
            world.set_autodeps(Some(container));

            let container_rect = world.fetch(container).rect;
            world.observer(container_rect, move |ValChanged, world| {
                let container = world.fetch(container);
                let parent = world.fetch(container.common.camera);
                let child = world.fetch(container.child);
                let rect = world.fetch(container.rect).val;
                let zoom = child.zoom - parent.zoom;
                let mut inner = world.fetch_mut(inner);

                inner.val = container.inner_rect(rect, I64Vec2::ZERO, zoom);
                world.queue_trigger(inner.handle(), ValChanged);
            });
        });

        inner
    }
}
