use std::time::Instant;

use glam::{DVec2, I8Vec2, I64Vec2, Vec2};
use hashbrown::HashMap;
use ln_world::{Element, Handle, HandleAny, World};
use winit::event::{
    ButtonSource, ElementState, MouseButton, MouseScrollDelta, PointerKind, PointerSource,
    WindowEvent,
};

use crate::{
    input::collider::{ToolControl, ToolControlUpdate},
    lnwin::Lnwindow,
    render::camera::Camera,
};

/// Capture variants carry data from `Press`; hover variants only describe the current pointer.
#[derive(Debug, Clone)]
pub enum PointerEvent {
    Enter(Pointer),
    Leave(Pointer),
    Hover(Pointer),
    Press(PointerCapture),
    Move(PointerCapture),
    Release(PointerCapture),
    Scroll { pointer: Pointer, delta: DVec2 },
}

#[derive(Debug, Clone, Copy)]
pub struct Pointer {
    pub screen: DVec2,
    pub kind: PointerKind,
    pub tilt: Vec2,
    pub force: Option<f32>,
    pub position: I64Vec2,
}

#[derive(Debug, Clone)]
pub struct PointerCapture {
    pub pointer: Pointer,
    pub start: Pointer,
    pub start_instant: Instant,
    pub members: Vec<Pointer>,
}

/// trigger on [PointerTool], release the records of `target` and acquire them again
pub struct PointerReacquire {
    pub target: HandleAny,
}

impl PointerEvent {
    pub fn pointer(&self) -> Pointer {
        match self {
            Self::Enter(pointer) | Self::Leave(pointer) | Self::Hover(pointer) => *pointer,
            Self::Scroll { pointer, .. } => *pointer,
            Self::Press(capture) | Self::Move(capture) | Self::Release(capture) => capture.pointer,
        }
    }

    pub fn capture(&self) -> Option<&PointerCapture> {
        match self {
            Self::Press(capture) | Self::Move(capture) | Self::Release(capture) => Some(capture),
            _ => None,
        }
    }
}

#[derive(Default)]
pub struct PointerTool {
    pointers: HashMap<PointerKind, PointerState>,
    groups: HashMap<HandleAny, Vec<PointerKind>>,
}

struct PointerState {
    pointer: Pointer,
    hover: Option<HandleAny>,
    capture: Option<Capture>,
}

#[derive(Clone, Copy)]
struct Capture {
    target: HandleAny,
    camera: Handle<Camera>,
    start: Pointer,
    start_instant: Instant,
}

impl Element for PointerTool {
    fn when_insert(&mut self, world: &World, this: Handle<Self>) {
        let lnwindow = world.single::<Lnwindow>();
        world.observer(lnwindow, move |event: &WindowEvent, world| {
            let mut this = world.fetch_mut(this);
            let lnwindow = world.single_fetch::<Lnwindow>();

            match event {
                WindowEvent::PointerMoved {
                    position, source, ..
                } => {
                    let kind = PointerKind::from(source.clone());
                    let screen = lnwindow.cursor_to_screen(*position);

                    let state = this.state(kind);
                    state.pointer.screen = screen;
                    state.pointer.tilt = source_tilt(source);
                    state.pointer.force = source_force(source);

                    if this.pointers[&kind].capture.is_some() {
                        this.move_capture(world, kind);
                    } else {
                        this.update_hover(world, kind, true);
                    }
                }

                WindowEvent::PointerButton {
                    position,
                    button,
                    state,
                    ..
                } => {
                    let kind = match button {
                        ButtonSource::Mouse(MouseButton::Left) => PointerKind::Mouse,
                        ButtonSource::Mouse(_) => return,
                        ButtonSource::Touch { finger_id, .. } => PointerKind::Touch(*finger_id),
                        ButtonSource::TabletTool { kind, .. } => PointerKind::TabletTool(*kind),
                        ButtonSource::Unknown(_) => PointerKind::Unknown,
                    };
                    let screen = lnwindow.cursor_to_screen(*position);

                    let pointer = this.state(kind);
                    pointer.pointer.screen = screen;
                    pointer.pointer.tilt = button_tilt(button);

                    match state {
                        ElementState::Pressed => {
                            pointer.pointer.force = button_force(button);
                            this.press(world, kind);
                        }
                        ElementState::Released => this.release(world, kind),
                    }
                }

                WindowEvent::MouseWheel { delta, .. } => {
                    let Some((pointer, target)) = this.scroll_target() else {
                        return;
                    };

                    let delta = match delta {
                        MouseScrollDelta::LineDelta(rows, lines) => {
                            DVec2::new(*rows as f64, -*lines as f64) * 20.0
                        }
                        MouseScrollDelta::PixelDelta(delta) => DVec2::new(delta.x, -delta.y),
                    };

                    world.queue_trigger(target, PointerEvent::Scroll { pointer, delta });
                }

                WindowEvent::PointerEntered { position, kind, .. } => {
                    let screen = lnwindow.cursor_to_screen(*position);

                    let pointer = this.state(*kind);
                    pointer.pointer.screen = screen;

                    this.update_hover(world, *kind, false);
                }

                WindowEvent::PointerLeft { position, kind, .. } => {
                    if let Some(position) = position {
                        let screen = lnwindow.cursor_to_screen(*position);
                        if let Some(state) = this.pointers.get_mut(kind) {
                            state.pointer.screen = screen;
                        }
                    }

                    let Some(state) = this.pointers.get(kind) else {
                        return;
                    };
                    let pointer = state.pointer;
                    let hover = state.hover;

                    if let Some((target, event)) = this.finish(world, *kind) {
                        world.queue_trigger(target, event);
                    }
                    if let Some(target) = hover {
                        world.queue_trigger(target, PointerEvent::Leave(pointer));
                    }

                    this.pointers.remove(kind);
                }

                _ => {}
            }
        });

        let ob = world.observer(lnwindow, move |ToolControlUpdate, world| {
            let mut tool = world.fetch_mut(this);
            let kinds = tool.pointers.keys().copied().collect::<Vec<_>>();
            for kind in kinds {
                tool.update_hover(world, kind, false);
            }
        });

        world.dependency(ob, this);

        world.observer(this, move |PointerReacquire { target }, world| {
            let mut tool = world.fetch_mut(this);
            let kinds = tool
                .pointers
                .iter()
                .filter(|(_, state)| {
                    state.capture.map(|capture| capture.target) == Some(*target)
                        || state.hover == Some(*target)
                })
                .map(|(&kind, _)| kind)
                .collect::<Vec<_>>();
            for kind in kinds {
                match tool.pointers[&kind].capture {
                    Some(_) => tool.press(world, kind),
                    None => tool.update_hover(world, kind, false),
                }
            }
        });
    }
}

impl PointerTool {
    fn state(&mut self, kind: PointerKind) -> &mut PointerState {
        self.pointers.entry(kind).or_insert_with(|| PointerState {
            pointer: Pointer {
                screen: DVec2::ZERO,
                kind,
                tilt: Vec2::ZERO,
                force: None,
                position: I64Vec2::ZERO,
            },
            hover: None,
            capture: None,
        })
    }

    fn members(&self, target: HandleAny, active: PointerKind) -> Vec<Pointer> {
        let Some(group) = self.groups.get(&target) else {
            return Vec::new();
        };

        group
            .iter()
            .filter(|&&kind| kind != active)
            .filter_map(|kind| self.pointers.get(kind))
            .map(|state| state.pointer)
            .collect()
    }

    fn capture(&self, kind: PointerKind) -> PointerCapture {
        let state = &self.pointers[&kind];
        let capture = state.capture.unwrap();

        PointerCapture {
            pointer: state.pointer,
            start: capture.start,
            start_instant: capture.start_instant,
            members: self.members(capture.target, kind),
        }
    }

    fn scroll_target(&self) -> Option<(Pointer, HandleAny)> {
        let state = self
            .pointers
            .get(&PointerKind::Mouse)
            .filter(|state| state.hover.is_some())
            .or_else(|| self.pointers.values().find(|state| state.hover.is_some()))?;

        state.hover.map(|target| (state.pointer, target))
    }

    fn press(&mut self, world: &World, kind: PointerKind) {
        if self.pointers[&kind].capture.is_some() {
            self.release(world, kind);
        }

        let pointer = self.pointers[&kind].pointer;
        let Some(hit) = ToolControl::intersect_main(world, &pointer) else {
            return;
        };

        let position = world.fetch(hit.camera).screen_to_world(pointer.screen);
        let start = Pointer {
            position,
            ..pointer
        };
        let capture = Capture {
            target: hit.handle,
            camera: hit.camera,
            start,
            start_instant: Instant::now(),
        };

        let previous = self.pointers[&kind].hover;
        if previous != Some(hit.handle)
            && let Some(previous) = previous
        {
            world.queue_trigger(previous, PointerEvent::Leave(pointer));
        }

        let state = self.pointers.get_mut(&kind).unwrap();
        state.pointer.position = position;
        state.hover = Some(hit.handle);
        state.capture = Some(capture);

        self.groups.entry(hit.handle).or_default().push(kind);

        let capture = self.capture(kind);
        world.queue_trigger(hit.handle, PointerEvent::Press(capture));
    }

    fn move_capture(&mut self, world: &World, kind: PointerKind) {
        let capture = self.pointers[&kind].capture.unwrap();
        let screen = self.pointers[&kind].pointer.screen;
        let position = world.fetch(capture.camera).screen_to_world(screen);
        self.pointers.get_mut(&kind).unwrap().pointer.position = position;

        let event = PointerEvent::Move(self.capture(kind));
        world.queue_trigger(capture.target, event);
    }

    fn release(&mut self, world: &World, kind: PointerKind) {
        if let Some((target, event)) = self.finish(world, kind) {
            world.queue_trigger(target, event);
        }

        if !matches!(kind, PointerKind::Touch(_)) {
            self.update_hover(world, kind, false);
        }
    }

    fn finish(&mut self, world: &World, kind: PointerKind) -> Option<(HandleAny, PointerEvent)> {
        let capture = self.pointers.get(&kind)?.capture?;
        let screen = self.pointers[&kind].pointer.screen;
        let position = world.fetch(capture.camera).screen_to_world(screen);
        self.pointers.get_mut(&kind).unwrap().pointer.position = position;

        let event = PointerEvent::Release(self.capture(kind));

        if let Some(group) = self.groups.get_mut(&capture.target) {
            group.retain(|&other| other != kind);
            if group.is_empty() {
                self.groups.remove(&capture.target);
            }
        }

        self.pointers.get_mut(&kind).unwrap().capture = None;

        Some((capture.target, event))
    }

    fn update_hover(&mut self, world: &World, kind: PointerKind, moving: bool) {
        if self.pointers[&kind].capture.is_some() {
            return;
        }

        let pointer = self.pointers[&kind].pointer;
        let previous = self.pointers[&kind].hover;
        let hit = ToolControl::intersect_main(world, &pointer).map(|hit| {
            let position = world.fetch(hit.camera).screen_to_world(pointer.screen);
            (hit.handle, position)
        });

        if previous != hit.map(|hit| hit.0) {
            if let Some(previous) = previous {
                world.queue_trigger(previous, PointerEvent::Leave(pointer));
            }
            if let Some((target, position)) = hit {
                world.queue_trigger(
                    target,
                    PointerEvent::Enter(Pointer {
                        position,
                        ..pointer
                    }),
                );
            }
        } else if moving && let Some((target, position)) = hit {
            world.queue_trigger(
                target,
                PointerEvent::Hover(Pointer {
                    position,
                    ..pointer
                }),
            );
        }

        let state = self.pointers.get_mut(&kind).unwrap();
        if let Some((_, position)) = hit {
            state.pointer.position = position;
        }
        state.hover = hit.map(|hit| hit.0);
    }
}

fn source_tilt(source: &PointerSource) -> Vec2 {
    match source {
        PointerSource::Mouse => Vec2::ZERO,
        PointerSource::Touch { .. } => Vec2::ZERO,
        PointerSource::TabletTool { data, .. } => match data.tilt {
            Some(tilt) => I8Vec2::new(tilt.x, tilt.y).as_vec2() * Vec2::new(1.0, -1.0) / 128.0,
            None => Vec2::ZERO,
        },
        PointerSource::Unknown => Vec2::ZERO,
    }
}

fn button_tilt(button: &ButtonSource) -> Vec2 {
    match button {
        ButtonSource::Mouse(_) => Vec2::ZERO,
        ButtonSource::Touch { .. } => Vec2::ZERO,
        ButtonSource::TabletTool { data, .. } => match data.tilt {
            Some(tilt) => I8Vec2::new(tilt.x, tilt.y).as_vec2() / 128.0,
            None => Vec2::ZERO,
        },
        ButtonSource::Unknown(_) => Vec2::ZERO,
    }
}

fn source_force(source: &PointerSource) -> Option<f32> {
    match source {
        PointerSource::Mouse => Some(1.0),
        PointerSource::Touch { force, .. } => force.map(|x| x.normalized(None) as f32),
        PointerSource::TabletTool { data, .. } => data.force.map(|x| x.normalized(None) as f32),
        PointerSource::Unknown => None,
    }
}

fn button_force(button: &ButtonSource) -> Option<f32> {
    match button {
        ButtonSource::Mouse(_) => Some(1.0),
        ButtonSource::Touch { force, .. } => force.map(|x| x.normalized(None) as f32),
        ButtonSource::TabletTool { data, .. } => data.force.map(|x| x.normalized(None) as f32),
        ButtonSource::Unknown(_) => None,
    }
}
