use glam::I64Vec2;
use ln_world::{Element, Handle, World};

use crate::{
    input::pointer::PointerEvent,
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
        let nest = world.fetch(self.nest);

        world.insert(RRect {
            common: nest.common,
            rect: nest.rect,
            order: 10,
            color: theme_val.gray1,
            radius: theme.roundness,
            width: 0.0,
        });

        let mut last = None;
        world.observer(self.nest, move |event: &PointerEvent, world| match event {
            PointerEvent::Press(capture) => last = Some(capture.pointer.screen),
            PointerEvent::Move(capture) | PointerEvent::Release(capture) => {
                let position = last.unwrap();
                let this = world.fetch(this);
                let mut nest = world.fetch_mut(this.nest);
                let rect = world.fetch(nest.rect).val;
                let restriction = world.fetch(this.restriction).val;
                let parent_camera = world.fetch(nest.common.camera);
                let delta =
                    parent_camera.screen_to_world_relative(capture.pointer.screen - position);

                nest.attr.center -= delta;
                nest.clamp_scroll(rect, restriction);
                let nest = nest.handle();
                world.queue(move |world| Nest::refresh_camera(world, nest));

                last = match event {
                    PointerEvent::Release(_) => None,
                    _ => Some(capture.pointer.screen),
                };
            }
            PointerEvent::Scroll { delta, .. } => {
                let this = world.fetch(this);
                let mut nest = world.fetch_mut(this.nest);
                let rect = world.fetch(nest.rect).val;
                let restriction = world.fetch(this.restriction).val;

                let delta = I64Vec2::q32_from_f64(*delta);
                nest.attr.center -= delta;
                nest.clamp_scroll(rect, restriction);
                let nest = nest.handle();
                world.queue(move |world| Nest::refresh_camera(world, nest));
            }
            _ => {}
        });

        world.observer(nest.rect, move |ValChanged, world| {
            let this = world.fetch(this);
            let mut nest = world.fetch_mut(this.nest);
            let rect = world.fetch(nest.rect).val;
            let restriction = world.fetch(this.restriction).val;

            nest.clamp_scroll(rect, restriction);
            let nest = nest.handle();
            world.queue(move |world| Nest::refresh_camera(world, nest));
        });

        // Re-clamp when content shrinks, otherwise the camera stays below the new bounds.
        world.observer(self.restriction, move |ValChanged, world| {
            let this = world.fetch(this);
            let mut nest = world.fetch_mut(this.nest);
            let rect = world.fetch(nest.rect).val;
            let restriction = world.fetch(this.restriction).val;

            nest.clamp_scroll(rect, restriction);
            let nest = nest.handle();
            world.queue(move |world| Nest::refresh_camera(world, nest));
        });
    }
}
