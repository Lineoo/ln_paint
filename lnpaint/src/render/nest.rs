use glam::{DVec2, I64Vec2, UVec2};
use lnworld:{Descriptor, Element, Handle, HandleGeneric, World};

use crate::{
    input::collider::{ToolControl, ToolHitResult},
    lnwin::Lnwindow,
    render::{
        Render, RenderDrawControl, RenderExtra,
        camera::{Camera, CameraDescriptor},
    },
    ui::common::WidgetCommon,
    unit::{FI64Ext, IRect},
    val::{Val, ValBind, ValChanged},
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

/// inner area when scroll is zero for container
pub struct NestRect {
    pub source: Handle<Nest>,
}

/// setup standard pointer tool
pub struct NestTool {
    pub source: Handle<Nest>,
}

/// Full builder, Nest + (sub) Camera + NestRect
pub struct NestBuilder {
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

impl Descriptor for NestBuilder {
    type Target = (Handle<Nest>, Handle<Val<IRect>>, WidgetCommon);

    fn when_build(self, world: &World) -> Self::Target {
        let camera = world.insert_with(move |world| {
            let parent = world.fetch(self.common.camera);
            let rect = world.fetch(self.rect).val;
            world.build(self.attr.calc_camera(rect, &parent))
        });

        let nest = world.insert(Nest {
            common: self.common,
            order: 100,
            child: camera,
            rect: self.rect,
            attr: self.attr,
        });

        let rect = world.build(NestRect { source: nest });
        world.build(NestTool { source: nest });

        let common = WidgetCommon {
            camera,
            ..self.common
        };

        (nest, rect, common)
    }
}

impl Descriptor for NestTool {
    type Target = Handle<ToolControl>;

    fn when_build(self, world: &World) -> Self::Target {
        world.insert_with(move |world| {
            let source = self.source;
            let nest = world.fetch(source);
            ToolControl {
                common: nest.common,
                order: nest.order,
                hit: Box::new(move |world, pointer| {
                    let source = world.fetch(source);
                    let parent = world.fetch(source.common.camera);
                    let position = parent.screen_to_world(pointer.screen);
                    let rect = world.fetch(source.rect).val;
                    if rect.contains(position.q32_floor()) {
                        if let Some(hit) = ToolControl::intersect(world, pointer, source.child) {
                            Some(hit)
                        } else {
                            Some(ToolHitResult {
                                camera: source.child,
                                handle: source.handle().untyped(),
                            })
                        }
                    } else {
                        None
                    }
                }),
            }
        })
    }
}

impl Element for Nest {
    fn when_insert(&mut self, world: &World, this: Handle<Self>) {
        world.scope("container");
        world.set_autodeps(Some(this));

        world.dependency(this, self.rect);
        world.dependency(this, self.common.visible);
        world.dependency(self.child, this);
        world.dependency(this, self.common.camera);

        let mut camera = world.fetch_mut(self.common.camera);
        camera.nests.push(this);

        world.insert(RenderDrawControl {
            common: self.common,
            order: self.order,
            draw: Box::new(move |world, rpass, extra| {
                let lnwindow = world.single_fetch::<Lnwindow>();
                let this = world.fetch(this);
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

        world.observer(self.rect, move |ValChanged, world| {
            world.queue(move |world| Nest::refresh_camera(world, this));
        });
    }

    fn when_remove(&mut self, world: &World, this: Handle<Self>) {
        let mut camera = world.fetch_mut(self.common.camera);
        camera.nests.retain(|&x| x != this);
    }
}

impl Nest {
    /// Recompute the child camera from parent and rect, then refresh nests nested in it.
    pub fn refresh_camera(world: &World, this: Handle<Self>) {
        let nest = world.fetch(this);
        let parent = world.fetch(nest.common.camera);
        let rect = world.fetch(nest.rect).val;
        let lnwindow = world.single_fetch::<Lnwindow>();
        let mut camera = world.fetch_mut(nest.child);

        let desc = nest.attr.calc_camera(rect, &parent);
        camera.zoom = desc.zoom;
        camera.center = desc.center;
        let size = lnwindow.window.surface_size();
        camera.surface_size = UVec2::new(size.width, size.height);
        camera.upload();
        lnwindow.window.request_redraw();

        for &other in &camera.nests {
            world.queue(move |world| Nest::refresh_camera(world, other));
        }
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

impl Descriptor for NestRect {
    type Target = Handle<Val<IRect>>;

    fn when_build(self, world: &World) -> Self::Target {
        let container = self.source;
        let inner = ValBind::new(world, move |world| {
            let container = world.fetch(container);
            let parent = world.fetch(container.common.camera);
            let child = world.fetch(container.child);
            let rect = world.fetch(container.rect).val;
            let zoom = child.zoom - parent.zoom;
            container.inner_rect(rect, I64Vec2::ZERO, zoom)
        });

        let bind = inner.pack();
        world.queue(move |world| {
            world.scope("container_echo");
            world.set_autodeps(Some(container));

            let container_rect = world.fetch(container).rect;
            bind(world).listen(container_rect);
        });

        inner.finish()
    }
}
