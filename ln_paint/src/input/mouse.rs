use glam::{IVec2, Vec2};
use ln_world::{Element, Handle, World};
use winit::event::{ButtonSource, ElementState, MouseButton, PointerKind, WindowEvent};

use crate::{
    input::{collider::ToolControl, pointer::Pointer},
    lnwin::{Lnwindow, RootCamera},
    unit::FI64Ext,
};

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

                let root = world.single_fetch::<RootCamera>().0;
                let pointer = Pointer {
                    screen,
                    kind: PointerKind::Mouse,
                    tilt: Vec2::ZERO,
                    force: Some(1.0),
                    position: world.fetch(root).screen_to_world(screen),
                };

                let Some(hit) = ToolControl::intersect_main(world, &pointer) else {
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
