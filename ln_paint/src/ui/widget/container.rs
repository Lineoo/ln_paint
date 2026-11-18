use glam::I64Vec2;
use ln_world::{Element, Handle, World};

use crate::{
    input::pointer::{PointerHit, PointerHitStatus, PointerScroll},
    render::nest::Nest,
    ui::{
        renderer::rrect::RRect,
        theme::{Theme, ThemeVal},
    },
    unit::{FI64Ext, IRect},
    val::{Val, ValChanged},
};

/// - pointer scroll interaction
/// - add a non-shadow panel
/// - restrict is for scroll
pub struct Container {
    pub nest: Handle<Nest>,
    pub restriction: Handle<Val<IRect>>,
}

impl Element for Container {
    fn when_insert(&mut self, world: &World, this: Handle<Self>) {
        world.scope("container_deco");
        world.set_autodeps(Some(this));

        let theme = world.single_fetch::<Theme>();
        let theme_val = world.single_fetch::<ThemeVal>();
        let handle = self.nest;
        let container = world.fetch(handle);

        world.insert(RRect {
            common: container.common,
            rect: container.rect,
            order: 10,
            color: theme_val.gray1,
            radius: theme.roundness,
            width: 0.0,
        });

        let mut status = None;
        world.observer(handle, move |event: &PointerHit, world| {
            status = match (status, event.status) {
                (None, PointerHitStatus::Press | PointerHitStatus::Moving) => {
                    Some(event.pointer.screen)
                }
                (None, PointerHitStatus::Release) => None,
                (Some(_), PointerHitStatus::Press) => Some(event.pointer.screen),
                (Some(position), PointerHitStatus::Moving) => {
                    let this = world.fetch(this);
                    let mut container = world.fetch_mut(handle);
                    let rect = world.fetch(container.rect).val;
                    let restriction = world.fetch(this.restriction).val;
                    let parent_camera = world.fetch(container.common.camera);
                    let delta =
                        parent_camera.screen_to_world_relative(event.pointer.screen - position);

                    container.attr.center -= delta;
                    container.clamp_scroll(rect, restriction);
                    world.queue(move |world| Nest::refresh_camera(world, handle));
                    Some(event.pointer.screen)
                }
                (Some(position), PointerHitStatus::Release) => {
                    let this = world.fetch(this);
                    let mut container = world.fetch_mut(handle);
                    let rect = world.fetch(container.rect).val;
                    let restriction = world.fetch(this.restriction).val;
                    let parent_camera = world.fetch(container.common.camera);
                    let delta =
                        parent_camera.screen_to_world_relative(event.pointer.screen - position);

                    container.attr.center -= delta;
                    container.clamp_scroll(rect, restriction);
                    world.queue(move |world| Nest::refresh_camera(world, handle));
                    None
                }
            }
        });

        world.observer(handle, move |event: &PointerScroll, world| {
            let this = world.fetch(this);
            let mut container = world.fetch_mut(handle);
            let rect = world.fetch(container.rect).val;
            let restriction = world.fetch(this.restriction).val;

            let delta = I64Vec2::q32_from_f64(event.delta);
            container.attr.center -= delta;
            container.clamp_scroll(rect, restriction);
            world.queue(move |world| Nest::refresh_camera(world, handle));
        });

        world.observer(container.rect, move |ValChanged, world| {
            let this = world.fetch(this);
            let mut container = world.fetch_mut(handle);
            let rect = world.fetch(container.rect).val;
            let restriction = world.fetch(this.restriction).val;

            container.clamp_scroll(rect, restriction);
            world.queue(move |world| Nest::refresh_camera(world, handle));
        });

        // Re-clamp when content shrinks, otherwise the camera stays below the new bounds.
        world.observer(self.restriction, move |ValChanged, world| {
            let this = world.fetch(this);
            let mut container = world.fetch_mut(handle);
            let rect = world.fetch(container.rect).val;
            let restriction = world.fetch(this.restriction).val;

            container.clamp_scroll(rect, restriction);
            world.queue(move |world| Nest::refresh_camera(world, handle));
        });
    }
}
