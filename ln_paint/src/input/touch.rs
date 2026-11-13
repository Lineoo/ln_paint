use glam::{DVec2, I64Vec2};
use hashbrown::HashMap;
use ln_world::{Element, Handle, HandleAny, World};
use winit::event::{
    ButtonSource, ElementState, MouseButton, PointerKind, PointerSource, WindowEvent,
};

use crate::{input::collider::ToolControl, lnwin::Lnwindow, render::camera::Camera};

/// Multi touch actions that allow inputs with more points than [`PointerTool`] but no hovering
#[derive(Default)]
pub struct MultiTouchTool {
    touches: HashMap<PointerKind, Touch>,
    groups: HashMap<HandleAny, Vec<MultiTouch>>,
}

#[derive(Debug, Clone, Copy)]
pub struct MultiTouch {
    pub position: I64Vec2,
    pub status: MultiTouchStatus,
    pub data: MultiTouchData,
}

#[derive(Debug)]
pub struct MultiTouchGroup {
    pub active: MultiTouch,
    pub members: Vec<MultiTouch>,
}

#[derive(Debug, Clone, Copy)]
pub enum MultiTouchStatus {
    Press,
    Holding,
    Release,
}

#[derive(Debug, Clone, Copy)]
pub struct MultiTouchData {
    pub screen: DVec2,
    pub kind: PointerKind,
    pub force: Option<f32>,
}

struct Touch {
    camera: Handle<Camera>,
    handle: HandleAny,
}

struct BranchData {
    screen: Option<DVec2>,
    // TODO device: Option<DeviceId>,
    kind: Option<PointerKind>,
    force: Option<f32>,
    event: BranchKind,
}

enum BranchKind {
    Enter,
    Press,
    Move,
    Release,
    Exit,
}

impl Element for MultiTouchTool {
    fn when_insert(&mut self, world: &World, this: Handle<Self>) {
        world.scope("multi_touch_tool");
        world.set_autodeps(Some(this));

        let lnwindow = world.single::<Lnwindow>();
        world.observer(lnwindow, |event, world| {
            let lnwindow = world.single_fetch::<Lnwindow>();
            let tool = &mut *world.single_fetch_mut::<MultiTouchTool>();

            let case = match event {
                WindowEvent::PointerButton {
                    state,
                    position,
                    button,
                    ..
                } => {
                    let screen = lnwindow.cursor_to_screen(*position);
                    let kind = MultiTouchTool::button_to_kind(button);
                    BranchData {
                        screen: Some(screen),
                        event: match state {
                            ElementState::Pressed => BranchKind::Press,
                            ElementState::Released => BranchKind::Release,
                        },
                        kind,
                        force: MultiTouchTool::button_to_force(button),
                    }
                }

                WindowEvent::PointerMoved {
                    position, source, ..
                } => {
                    let screen = lnwindow.cursor_to_screen(*position);
                    BranchData {
                        screen: Some(screen),
                        kind: Some(PointerKind::from(source.clone())),
                        force: MultiTouchTool::pointer_to_force(source),
                        event: BranchKind::Move,
                    }
                }

                WindowEvent::PointerEntered { position, kind, .. } => {
                    let screen = lnwindow.cursor_to_screen(*position);
                    BranchData {
                        screen: Some(screen),
                        kind: Some(*kind),
                        force: None,
                        event: BranchKind::Enter,
                    }
                }

                WindowEvent::PointerLeft { position, kind, .. } => {
                    let screen = position.map(|x| lnwindow.cursor_to_screen(x));
                    BranchData {
                        screen,
                        kind: Some(*kind),
                        force: None,
                        event: BranchKind::Exit,
                    }
                }

                _ => return,
            };

            // return if it is right-click or something not for us.
            let Some(kind) = case.kind else { return };

            match (tool.touches.get(&kind), case.event, case.screen) {
                // not related
                (None, BranchKind::Enter, _) => (),
                // good, register the press
                (None, BranchKind::Press, Some(screen)) => {
                    let Some(hit) = ToolControl::intersect_main(world, screen) else {
                        return;
                    };

                    let camera = world.fetch(hit.camera);
                    let position = camera.screen_to_world(screen);
                    drop(camera);

                    let event = MultiTouch {
                        position,
                        status: MultiTouchStatus::Press,
                        data: MultiTouchData {
                            screen,
                            kind,
                            force: case.force,
                        },
                    };

                    let replaced = tool.touches.insert(
                        kind,
                        Touch {
                            camera: hit.camera,
                            handle: hit.handle,
                        },
                    );
                    assert!(replaced.is_none());

                    let list = tool.groups.entry(hit.handle).or_default();
                    list.push(event);

                    let mut group = MultiTouchGroup {
                        active: event,
                        members: std::mem::take(list),
                    };

                    world.trigger(hit.handle, &group.active);
                    world.trigger(hit.handle, &group);

                    std::mem::swap(list, &mut group.members);
                }
                // we don't know where the pointer is!
                (None, BranchKind::Press, None) => unreachable!("winit"),
                // not related
                (None, BranchKind::Move, _) => (),
                // somehow we don't know it's pressed
                (None, BranchKind::Release, _) => (),
                // not related
                (None, BranchKind::Exit, _) => (),
                // im not sure about this
                (Some(_), BranchKind::Enter, _) => {
                    log::warn!("pointer enter while we think it's still touching");
                }
                // im not sure about this
                (Some(_), BranchKind::Press, _) => {
                    log::warn!("pointer touch while we think it's still touching");
                }
                // good, let's update data
                (Some(target), BranchKind::Move, Some(screen)) => {
                    let list = tool.groups.get_mut(&target.handle).unwrap();
                    let touch = list.iter_mut().find(|x| x.data.kind == kind).unwrap();

                    let camera = world.fetch(target.camera);
                    let position = camera.screen_to_world(screen);
                    drop(camera);

                    *touch = MultiTouch {
                        position,
                        status: MultiTouchStatus::Holding,
                        data: MultiTouchData {
                            screen,
                            kind,
                            force: case.force,
                        },
                    };

                    let mut group = MultiTouchGroup {
                        active: *touch,
                        members: std::mem::take(list),
                    };

                    world.trigger(target.handle, &group.active);
                    world.trigger(target.handle, &group);

                    std::mem::swap(list, &mut group.members);
                }
                // we don't know where the pointer is, it's meaningless, ignore
                (Some(_), BranchKind::Move, None) => unreachable!("winit"),
                // good, release
                (Some(_), BranchKind::Release, Some(screen)) => {
                    let target = tool.touches.remove(&kind).unwrap();
                    let list = tool.groups.get_mut(&target.handle).unwrap();
                    list.retain(|touch| touch.data.kind != kind);

                    let camera = world.fetch(target.camera);
                    let position = camera.screen_to_world(screen);
                    drop(camera);

                    let active = MultiTouch {
                        position,
                        status: MultiTouchStatus::Release,
                        data: MultiTouchData {
                            screen,
                            kind,
                            force: case.force,
                        },
                    };

                    let mut group = MultiTouchGroup {
                        active,
                        members: std::mem::take(list),
                    };

                    world.trigger(target.handle, &group.active);
                    world.trigger(target.handle, &group);

                    std::mem::swap(list, &mut group.members);
                }
                // we don't know where the pointer is, released though so we use old data
                (Some(_), BranchKind::Release, None) => unreachable!("winit"),
                // oops, we may need to handle this
                (Some(_), BranchKind::Exit, _) => {
                    log::warn!("potential cancelled touch");
                }
            }
        });
    }
}

impl MultiTouchTool {
    fn button_to_kind(button: &ButtonSource) -> Option<PointerKind> {
        match button {
            ButtonSource::Mouse(MouseButton::Left) => Some(PointerKind::Mouse),
            ButtonSource::Mouse(_) => None,
            ButtonSource::Touch { finger_id, .. } => Some(PointerKind::Touch(*finger_id)),
            ButtonSource::TabletTool { kind, .. } => Some(PointerKind::TabletTool(*kind)),
            ButtonSource::Unknown(_) => Some(PointerKind::Unknown),
        }
    }

    fn button_to_force(button: &ButtonSource) -> Option<f32> {
        match button {
            ButtonSource::Mouse(_) => Some(1.0),
            ButtonSource::Touch { force, .. } => force.map(|x| x.normalized(None) as f32),
            ButtonSource::TabletTool { data, .. } => data.force.map(|x| x.normalized(None) as f32),
            ButtonSource::Unknown(_) => None,
        }
    }

    fn pointer_to_force(source: &PointerSource) -> Option<f32> {
        match source {
            PointerSource::Mouse => Some(1.0),
            PointerSource::Touch { force, .. } => force.map(|x| x.normalized(None) as f32),
            PointerSource::TabletTool { data, .. } => data.force.map(|x| x.normalized(None) as f32),
            PointerSource::Unknown => None,
        }
    }
}
