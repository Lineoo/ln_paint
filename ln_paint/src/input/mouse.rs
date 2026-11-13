use glam::IVec2;
use ln_world::{Element, Handle, World};
use winit::event::{ButtonSource, ElementState, MouseButton, WindowEvent};

use crate::{input::collider::ToolControl, lnwin::Lnwindow, unit::FI64Ext};

/// Mouse-specific operations like right-click and middle-click.
#[derive(Default)]
pub struct MouseTool;

/// Right-click events.
#[derive(Clone, Copy)]
pub struct MouseMenu(pub IVec2);

impl Element for MouseTool {
    fn when_insert(&mut self, world: &World, _this: Handle<Self>) {
        let lnwindow = world.single::<Lnwindow>();

        world.observer(lnwindow, move |event: &WindowEvent, world| match event {
            // right-click //
            WindowEvent::PointerButton {
                position,
                button: ButtonSource::Mouse(MouseButton::Right),
                state: ElementState::Pressed,
                ..
            } => {
                let lnwindow = world.single_fetch::<Lnwindow>();
                let screen = lnwindow.cursor_to_screen(*position);
                drop(lnwindow);

                let Some(hit) = ToolControl::intersect_main(world, screen) else {
                    return;
                };

                let camera = world.fetch(hit.camera);
                let position = camera.screen_to_world(screen);

                world.queue_trigger(hit.handle, MouseMenu(position.q32_floor()));
            }

            _ => {}
        });
    }
}
