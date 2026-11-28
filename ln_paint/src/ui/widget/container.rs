use glam::I64Vec2;
use ln_world::{Element, Handle, World};

use crate::{
    input::{drag::Drag, pointer::PointerEvent},
    lnwin::Lnwindow,
    render::nest::Nest,
    ui::{renderer::rrect::RRect, theme::Theme},
    unit::{FI64Ext, IRect},
    val::{Val, ValChanged},
};

/// - pointer scroll interaction
/// - add a non-shadow panel
/// - restrict is for scroll
pub struct Container {
    pub nest: Handle<Nest>,
    pub theme: Handle<Theme>,
    pub restriction: Handle<Val<IRect>>,
}

impl Element for Container {
    fn when_insert(&mut self, world: &World, this: Handle<Self>) {
        world.scope("container_deco");
        world.set_autodeps(Some(this));

        let theme = world.fetch(self.theme);
        let nest = world.fetch(self.nest);

        world.insert(RRect {
            common: nest.common,
            rect: nest.rect,
            order: 10,
            color: theme.color.surface,
            radius: world.fetch(theme.shape.corner_radius).val,
            width: 0.0,
        });

        let mut drag = Drag::default();
        world.observer(self.nest, move |event: &PointerEvent, world| {
            let delta = match event {
                PointerEvent::Scroll { delta, .. } => I64Vec2::q32_from_f64(*delta),
                _ => {
                    let Some(drag) = drag.update(event) else {
                        return;
                    };
                    let this = world.fetch(this);
                    let nest = world.fetch(this.nest);
                    let parent_camera = world.fetch(nest.common.camera);
                    let lnwindow = world.single_fetch::<Lnwindow>();
                    parent_camera
                        .screen_to_world_relative(lnwindow.cursor_to_screen_relative(drag.delta))
                }
            };

            let this = world.fetch(this);
            let mut nest = world.fetch_mut(this.nest);
            let rect = world.fetch(nest.rect).val;
            let restriction = world.fetch(this.restriction).val;

            nest.attr.center -= delta;
            nest.clamp_scroll(rect, restriction);
            let nest = nest.handle();
            world.queue(move |world| Nest::refresh_camera(world, nest));
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
