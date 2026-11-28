use std::time::Instant;

use glam::{DVec2, I8Vec2, I64Vec2, Mat3, Vec2};
use hashbrown::HashMap;
use lnworld::{Element, Handle, HandleAny, World};
use winit::event::{
    ButtonSource, ElementState, MouseButton, MouseScrollDelta, PointerKind, PointerSource,
    WindowEvent,
};

use crate::{
    input::collider::{ToolControl, ToolControlUpdate},
    lnwin::Lnwindow,
    render::camera::Camera,
    save::Save,
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
    Menu(Pointer),
}

#[derive(Debug, Clone, Copy)]
pub struct Pointer {
    pub screen: DVec2,
    pub pixel: DVec2,
    pub kind: PointerKind,
    pub tilt: Vec2,
    pub force: Option<f32>,
    pub position: I64Vec2,
}

/// based on tilt data generate an actual used tilt data
#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct TiltCorrection {
    #[serde(default)]
    portrait: Mat3,
    #[serde(default)]
    landscape: Mat3,
}

/// based on tilt data generate an offset on position
#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct OffsetCorrection {
    #[serde(default)]
    portrait: Mat3,
    #[serde(default)]
    landscape: Mat3,
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

/// trigger on [PointerTool], resend [PointerEvent::Move] to every active capture
pub struct PointerNotify;

/// trigger on [PointerTool], move the capture of `kind` to `target`
pub struct PointerTransfer {
    pub kind: PointerKind,
    pub target: HandleAny,
}

impl PointerEvent {
    pub fn pointer(&self) -> Pointer {
        match self {
            Self::Enter(pointer)
            | Self::Leave(pointer)
            | Self::Hover(pointer)
            | Self::Menu(pointer) => *pointer,
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

                    let state = this.state(kind);
                    state.pointer.screen = lnwindow.cursor_to_screen(*position);
                    state.pointer.pixel = DVec2::new(position.x, position.y);
                    state.pointer.tilt = source_tilt(source);
                    state.pointer.force = source_force(source);
                    correct(world, &mut state.pointer);

                    if this.pointers[&kind].capture.is_some() {
                        let pointer = this.pointers[&kind].pointer;
                        log::trace!(
                            "pointer move {kind:?} pixel={:?} screen={:?}",
                            pointer.pixel,
                            pointer.screen,
                        );
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
                    let (kind, menu) = match button {
                        ButtonSource::Mouse(MouseButton::Left | MouseButton::Right) => (
                            PointerKind::Mouse,
                            matches!(button, ButtonSource::Mouse(MouseButton::Right)),
                        ),
                        ButtonSource::Mouse(_) => return,
                        ButtonSource::Touch { finger_id, .. } => {
                            (PointerKind::Touch(*finger_id), false)
                        }
                        ButtonSource::TabletTool { kind, .. } => {
                            (PointerKind::TabletTool(*kind), false)
                        }
                        ButtonSource::Unknown(_) => (PointerKind::Unknown, false),
                    };
                    let pointer = this.state(kind);
                    pointer.pointer.screen = lnwindow.cursor_to_screen(*position);
                    pointer.pointer.pixel = DVec2::new(position.x, position.y);
                    pointer.pointer.tilt = button_tilt(button);
                    correct(world, &mut pointer.pointer);

                    match state {
                        ElementState::Pressed => {
                            pointer.pointer.force = button_force(button);
                            log::trace!(
                                "pointer press {kind:?} pixel={:?} screen={:?}",
                                pointer.pointer.pixel,
                                pointer.pointer.screen,
                            );
                            if menu {
                                this.menu(world, kind);
                            } else {
                                this.press(world, kind);
                            }
                        }
                        ElementState::Released => {
                            log::trace!(
                                "pointer release {kind:?} pixel={:?} screen={:?}",
                                pointer.pointer.pixel,
                                pointer.pointer.screen,
                            );
                            if !menu {
                                this.release(world, kind);
                            }
                        }
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

                    log::trace!("pointer scroll pixel={:?} delta={delta:?}", pointer.pixel);
                    world.queue_trigger(target, PointerEvent::Scroll { pointer, delta });
                }

                WindowEvent::PointerEntered { position, kind, .. } => {
                    let pointer = this.state(*kind);
                    pointer.pointer.screen = lnwindow.cursor_to_screen(*position);
                    pointer.pointer.pixel = DVec2::new(position.x, position.y);
                    correct(world, &mut pointer.pointer);
                    log::trace!(
                        "pointer entered {kind:?} pixel={:?} screen={:?}",
                        pointer.pointer.pixel,
                        pointer.pointer.screen,
                    );

                    this.update_hover(world, *kind, false);
                }

                WindowEvent::PointerLeft { position, kind, .. } => {
                    if let Some(position) = position
                        && let Some(state) = this.pointers.get_mut(kind)
                    {
                        state.pointer.screen = lnwindow.cursor_to_screen(*position);
                        state.pointer.pixel = DVec2::new(position.x, position.y);
                        correct(world, &mut state.pointer);
                    }

                    let Some(state) = this.pointers.get(kind) else {
                        return;
                    };
                    let pointer = state.pointer;
                    let hover = state.hover;
                    log::trace!(
                        "pointer left {kind:?} pixel={:?} screen={:?}",
                        pointer.pixel,
                        pointer.screen,
                    );

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

        world.observer(this, move |_: &PointerNotify, world| {
            let mut tool = world.fetch_mut(this);
            let kinds = tool
                .pointers
                .iter()
                .filter(|(_, state)| state.capture.is_some())
                .map(|(&kind, _)| kind)
                .collect::<Vec<_>>();
            for kind in kinds {
                tool.move_capture(world, kind);
            }
        });

        world.observer(this, move |PointerTransfer { kind, target }, world| {
            let mut tool = world.fetch_mut(this);
            tool.transfer(world, *kind, *target);
        });

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
                pixel: DVec2::ZERO,
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

    fn menu(&mut self, world: &World, kind: PointerKind) {
        let pointer = self.pointers[&kind].pointer;
        let Some(hit) = ToolControl::intersect_main(world, &pointer) else {
            return;
        };

        let position = world.fetch(hit.camera).screen_to_world(pointer.screen);
        world.queue_trigger(
            hit.handle,
            PointerEvent::Menu(Pointer {
                position,
                ..pointer
            }),
        );
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

    fn transfer(&mut self, world: &World, kind: PointerKind, target: HandleAny) {
        let source = self.pointers[&kind].capture.unwrap();
        if source.target == target {
            return;
        }

        let pointer = self.pointers[&kind].pointer;
        let position = world.fetch(source.camera).screen_to_world(pointer.screen);
        self.pointers.get_mut(&kind).unwrap().pointer.position = position;

        world.queue_trigger(source.target, PointerEvent::Release(self.capture(kind)));
        world.queue_trigger(source.target, PointerEvent::Leave(pointer));

        if let Some(group) = self.groups.get_mut(&source.target) {
            group.retain(|&other| other != kind);
            if group.is_empty() {
                self.groups.remove(&source.target);
            }
        }

        let capture = Capture {
            target,
            camera: source.camera,
            start: Pointer {
                position,
                ..pointer
            },
            start_instant: Instant::now(),
        };
        let state = self.pointers.get_mut(&kind).unwrap();
        state.hover = Some(target);
        state.capture = Some(capture);

        self.groups.entry(target).or_default().push(kind);

        world.queue_trigger(
            target,
            PointerEvent::Enter(Pointer {
                position,
                ..pointer
            }),
        );
        world.queue_trigger(target, PointerEvent::Press(self.capture(kind)));
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

fn correct(world: &World, pointer: &mut Pointer) {
    let save = world.single_fetch::<Save>();
    let sn = save.sn();
    let portrait = is_portrait(world);
    if let Some(correction) = &sn.tilt_correction {
        let matrix = match portrait {
            true => correction.portrait,
            false => correction.landscape,
        };
        let transformed = matrix * pointer.tilt.extend(1.);
        pointer.tilt = transformed.truncate() / transformed.z;
    }
    if let Some(correction) = &sn.offset_correction {
        let matrix = match portrait {
            true => correction.portrait,
            false => correction.landscape,
        };
        let transformed = matrix * pointer.tilt.extend(1.);
        pointer.screen = pointer.screen + (transformed.truncate() / transformed.z).as_dvec2();
    }
}

#[cfg(not(target_os = "android"))]
fn is_portrait(_world: &World) -> bool {
    false
}

#[cfg(target_os = "android")]
fn is_portrait(world: &World) -> bool {
    // Android NativeActivity due to some reason won't update configuration's
    // orientation attribute (keep init state), so we use surface size to tell
    // if it's portrait
    let app = world.single_fetch::<Lnwindow>();
    let size = app.window.surface_size();
    size.height > size.width
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
