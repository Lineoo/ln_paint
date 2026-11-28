use std::time::Instant;

use glam::{DVec2, I64Vec2, UVec2};
use lnworld::{Element, Handle, HandleAny, HandleGeneric, World};
use winit::{
    cursor::{Cursor, CursorIcon},
    event::{ButtonSource, ElementState, MouseButton, PointerKind, PointerSource, WindowEvent},
    keyboard::KeyCode,
};

use crate::{
    input::{
        collider::{ToolControl, ToolControlUpdate, ToolHitResult},
        drag::Drag,
        hold::Hold,
        pointer::{Pointer, PointerEvent, PointerReacquire, PointerTool, PointerTransfer},
    },
    lnwin::{Lnwindow, RootCamera},
    paint::{
        brush::{Dab, param::BrushParamKey},
        wrapper::LayerPage,
    },
    render::RenderPrepareControl,
    ui::{
        theme::{Theme, ThemeScheme},
        widget::area::{Area, AreaMode},
    },
    unit::{FI64Ext, IRect},
    val::{Val, ValChanged},
};

/// drag layer, always owns the control group
pub struct LayerDrag {
    pub velocity: DVec2,
    pub zoom_velocity: f64,
    pub damping: f32,
    pub zoom_damping: f32,
    pub touch_draw: bool,
    pub space: bool,
    pub ctrl: bool,
    pub shift: bool,
    pub blocking: bool,
}

#[derive(Debug)]
enum LayerDragState {
    None,
    Grab {
        start_position: DVec2,
        start_pinch: Option<f64>,
        last_center: I64Vec2,
        last_zoom: i64,
        last_instant: Instant,
        velocity: DVec2,
        zoom_velocity: f64,
        /// control group press, waiting to decide between drag and object
        pending: Option<HandleAny>,
    },
    Scale {
        start_position: DVec2,
    },
}

/// draw layer, always owns the draw group
pub struct LayerDraw {
    pub mode: LayerInputMode,
    pub hold_pick: bool,
}

#[derive(Default, Debug, Clone, Copy)]
pub enum LayerInputMode {
    #[default]
    Normal,
    Pick,
    Area,
}

#[derive(Debug)]
enum LayerDrawState {
    None,
    Paint,
    PaintNoErase,
    PaintErase,
    PickColor,
    CreateArea {
        start_position: I64Vec2,
        area: Handle<Area>,
    },
}

impl Default for LayerDrag {
    fn default() -> Self {
        Self {
            velocity: DVec2::ZERO,
            zoom_velocity: 0.0,
            damping: 0.002,
            zoom_damping: 0.0001,
            touch_draw: false,
            space: false,
            ctrl: false,
            shift: false,
            blocking: false,
        }
    }
}

impl Default for LayerDraw {
    fn default() -> Self {
        Self {
            mode: LayerInputMode::Normal,
            hold_pick: false,
        }
    }
}

impl Element for LayerDrag {
    fn when_insert(&mut self, world: &World, this: Handle<Self>) {
        world.scope("layer_drag");
        world.set_autodeps(Some(this));

        let (common, rect, child) = {
            let page = world.single_fetch::<LayerPage>();
            let nest = world.fetch(page.nest);
            (nest.common, nest.rect, nest.child)
        };
        world.insert(ToolControl {
            common,
            order: -100,
            // page entry: the draw group delegates to the page camera, the control group stays here
            hit: Box::new(move |world, pointer| {
                let parent = world.fetch(common.camera);
                let position = parent.screen_to_world(pointer.screen);
                if !world.fetch(rect).val.contains(position.q32_floor()) {
                    return None;
                }

                if draw_group(pointer, &world.fetch(this)) {
                    ToolControl::intersect(world, pointer, child)
                } else {
                    Some(ToolHitResult {
                        camera: child,
                        handle: this.untyped(),
                    })
                }
            }),
        });

        let mut timer = None;
        world.insert(RenderPrepareControl {
            prepare: Box::new(move |world| {
                let this = &mut *world.fetch_mut(this);
                let page = world.single_fetch::<LayerPage>();
                if let Some(last_instant) = timer.replace(Instant::now()) {
                    let dt = last_instant.elapsed().as_secs_f64();
                    if this.velocity.length_squared() > 1e-3 || this.zoom_velocity.abs() > 1e-3 {
                        log::trace!(
                            "drag inertia dt={dt:.4} velocity={:?} zoom_velocity={}",
                            this.velocity,
                            this.zoom_velocity,
                        );
                        let mut utils = world.fetch_mut(page.camera_utils);
                        utils.read_camera(world, page.nest);
                        this.velocity *= (this.damping as f64).powf(dt).min(1.0);
                        this.zoom_velocity *= (this.zoom_damping as f64).powf(dt).min(1.0);
                        let zoom = utils.camera_zoom() + i64::q32_from_f64(this.zoom_velocity * dt);
                        let center =
                            utils.camera_center() + I64Vec2::q32_from_f64(this.velocity * dt);
                        utils.camera_center_by_camera_cursor(center);
                        utils.camera_zoom_by_camera_distance_center(zoom);
                        utils.write_camera(world, page.nest);
                        let lnwindow = world.single_fetch::<Lnwindow>();
                        lnwindow.window.request_redraw();
                    }
                }
            }),
        });

        let lnwindow = world.single::<Lnwindow>();
        world.observer(lnwindow, move |event: &WindowEvent, world| {
            let WindowEvent::KeyboardInput { event, .. } = event else {
                return;
            };

            if event.repeat {
                return;
            }

            let mut this = world.fetch_mut(this);
            let lnwindow = world.fetch(lnwindow);
            let ctrl = this.ctrl;
            let shift = this.shift;
            let press = event.state == ElementState::Pressed;

            match KeyCode::from(event.physical_key) {
                KeyCode::KeyZ if press && ctrl => {
                    let mut page = world.single_fetch_mut::<LayerPage>();

                    if !shift {
                        page.undo();
                    } else {
                        page.redo();
                    }

                    lnwindow.window.request_redraw();
                }
                KeyCode::KeyE if press => {
                    let mut page = world.single_fetch_mut::<LayerPage>();
                    let active = page.active_mut();
                    let is_erase = active.toggle(BrushParamKey::Erase).unwrap_or_default();
                    active.set_toggle(BrushParamKey::Erase, !is_erase);
                    world.queue_trigger(page.handle(), ValChanged);
                }
                KeyCode::Space => {
                    this.space = press;
                    world.queue_trigger(world.single::<Lnwindow>(), ToolControlUpdate);
                }
                KeyCode::ControlLeft => {
                    this.ctrl = press;
                }
                KeyCode::ShiftLeft => {
                    this.shift = press;
                }
                KeyCode::F8 if press => {
                    log::debug!("{}", world.stat_deps());
                }
                KeyCode::F5 if press => {
                    let theme = world.single_fetch::<Theme>();
                    let mut scheme = world.fetch_mut(theme.scheme);
                    scheme.val = match scheme.val {
                        ThemeScheme::Dark => ThemeScheme::DarkVariant,
                        ThemeScheme::DarkVariant => ThemeScheme::Light,
                        ThemeScheme::Light => ThemeScheme::LightVariant,
                        ThemeScheme::LightVariant => ThemeScheme::Neutral,
                        ThemeScheme::Neutral => ThemeScheme::NeutralVariant,
                        ThemeScheme::NeutralVariant => ThemeScheme::Dark,
                    };
                    world.queue_trigger(scheme.handle(), ValChanged);
                }
                _ => (),
            }

            let mode = world.single_fetch::<LayerDraw>().mode;
            update_drag_icon(&this, &LayerDragState::None, mode, &lnwindow);
        });

        let mut middle = false;
        world.observer(lnwindow, move |event: &WindowEvent, world| match event {
            WindowEvent::PointerButton {
                position,
                state: ElementState::Pressed,
                button: ButtonSource::Mouse(MouseButton::Middle),
                ..
            } => {
                let mut this = world.fetch_mut(this);
                let main = world.single_fetch::<LayerPage>().nest;
                let utils = world.single_fetch::<LayerPage>().camera_utils;
                let lnwindow = world.single_fetch::<Lnwindow>();
                let cursor = lnwindow.cursor_to_screen(*position);
                middle = true;

                this.velocity = DVec2::ZERO;
                this.zoom_velocity = 0.0;
                let mut camera_utils = world.fetch_mut(utils);
                camera_utils.read_camera(world, main);
                camera_utils.anchor_cursor_by_anchor_center(DVec2::ZERO);
                camera_utils.camera_cursor_by_anchor_center(cursor);
            }

            WindowEvent::PointerMoved {
                position,
                source: PointerSource::Mouse,
                ..
            } => {
                let mut this = world.fetch_mut(this);
                let main = world.single_fetch::<LayerPage>().nest;
                let utils = world.single_fetch::<LayerPage>().camera_utils;
                let lnwindow = world.single_fetch::<Lnwindow>();
                let cursor = lnwindow.cursor_to_screen(*position);
                drop(lnwindow);

                if middle {
                    this.velocity = DVec2::ZERO;
                    this.zoom_velocity = 0.0;
                    let mut camera_utils = world.fetch_mut(utils);
                    camera_utils.read_camera(world, main);
                    camera_utils.camera_cursor_by_camera_center(cursor);
                    camera_utils.write_camera(world, main);
                }
            }

            WindowEvent::PointerButton {
                state: ElementState::Released,
                button: ButtonSource::Mouse(MouseButton::Middle),
                ..
            } => {
                middle = false;
            }

            _ => {}
        });

        let mut drag = Drag::default();
        let hold = Hold::default();
        let mut state = LayerDragState::None;
        world.observer(this, move |event: &PointerEvent, world| {
            if matches!(event, PointerEvent::Menu(_)) {
                return;
            }

            let mut this = world.fetch_mut(this);
            let page = &mut *world.single_fetch_mut::<LayerPage>();
            let container = page.nest;
            let child = world.fetch(page.nest).child;
            let lnwindow = world.fetch(lnwindow);
            let utils = &mut *world.fetch_mut(page.camera_utils);

            if let PointerEvent::Scroll { pointer, delta } = event {
                let zoom_delta = -delta.y;
                utils.read_camera(world, container);
                utils.anchor_cursor_by_anchor_center(DVec2::ZERO);
                utils.camera_cursor_by_anchor_center(pointer.screen);
                utils.anchor_distance_by_anchor_zoom(200.0);
                if zoom_delta > 0.0 {
                    utils.camera_distance_by_anchor_zoom_cursor(200.0);
                    utils.camera_distance_by_camera_zoom_center(200.0 + zoom_delta);
                } else {
                    utils.camera_distance_by_anchor_zoom_cursor(200.0 - zoom_delta);
                    utils.camera_distance_by_camera_zoom_center(200.0);
                }
                utils.write_camera(world, container);
                return;
            }

            match event {
                PointerEvent::Enter(_) | PointerEvent::Leave(_) | PointerEvent::Hover(_) => {
                    let mode = world.single_fetch::<LayerDraw>().mode;
                    update_drag_icon(&this, &state, mode, &lnwindow);
                    return;
                }
                _ => {}
            }

            let center = group_center(event);
            let pinch = group_pinch(event, center);

            let dragged = drag.update(event).is_some();
            let held = hold.reached(event);

            let prev = std::mem::replace(&mut state, LayerDragState::None);
            state = match (prev, event) {
                (LayerDragState::None, PointerEvent::Press(capture)) => {
                    world.insert(hold.task());
                    this.velocity = DVec2::ZERO;
                    this.zoom_velocity = 0.0;
                    let draw = world.single::<LayerDraw>().untyped();
                    // the draw layer is the draw group's fallback, not an object target
                    let pending = ToolControl::intersect(world, &capture.pointer, child)
                        .filter(|hit| hit.handle != draw)
                        .map(|hit| hit.handle);
                    this.velocity = DVec2::ZERO;
                    utils.read_camera(world, container);
                    utils.anchor_cursor_by_anchor_center(DVec2::ZERO);
                    utils.camera_cursor_by_anchor_center(center);
                    if let Some(distance) = pinch {
                        utils.camera_distance_by_anchor_zoom_cursor(distance);
                    }

                    if this.ctrl {
                        utils.camera_distance_by_anchor_zoom_cursor(1.0);
                        utils.write_camera(world, container);
                        LayerDragState::Scale {
                            start_position: center,
                        }
                    } else {
                        utils.write_camera(world, container);
                        LayerDragState::Grab {
                            start_position: center,
                            start_pinch: pinch,
                            last_center: utils.camera_center(),
                            last_zoom: utils.camera_zoom(),
                            last_instant: Instant::now(),
                            velocity: DVec2::ZERO,
                            zoom_velocity: 0.0,
                            pending,
                        }
                    }
                }
                (LayerDragState::None, PointerEvent::Move(_)) => LayerDragState::None,
                (LayerDragState::None, PointerEvent::Release(_)) => LayerDragState::None,

                // Drag
                (
                    LayerDragState::Grab {
                        last_center,
                        last_zoom,
                        last_instant,
                        velocity,
                        zoom_velocity,
                        pending,
                        ..
                    },
                    PointerEvent::Press(capture) | PointerEvent::Release(capture),
                ) => {
                    let is_release = matches!(event, PointerEvent::Release(_));
                    let term_velocity;
                    let term_zoom_velocity;
                    // see evaluation below
                    let dt = last_instant.elapsed().as_secs_f64();
                    let prev_center = utils.camera_center();
                    let prev_zoom = utils.camera_zoom();
                    if dt > 1e-4 {
                        let new_velocity = (prev_center - last_center).q32_as_f64() / dt;
                        let new_zoom_velocity = (prev_zoom - last_zoom).q32_as_f64() / dt;
                        let factor = 1.0 - 0.01f64.powf(dt * 10.0);
                        term_velocity = velocity * (1. - factor) + new_velocity * factor;
                        term_zoom_velocity =
                            zoom_velocity * (1. - factor) + new_zoom_velocity * factor;
                    } else {
                        term_velocity = velocity;
                        term_zoom_velocity = zoom_velocity;
                    }

                    log::trace!(
                        "drag grab term {:?} release={is_release} \
                        members={} dt={dt:.5} \
                        center_delta={:?} term_velocity={term_velocity:?} \
                        term_zoom_velocity={term_zoom_velocity}",
                        capture.pointer.kind,
                        capture.members.len(),
                        (prev_center - last_center).q32_as_f64(),
                    );

                    if !capture.members.is_empty() {
                        this.velocity = DVec2::ZERO;
                        this.zoom_velocity = match (is_release, capture.members.len()) {
                            (true, 1) => term_zoom_velocity,
                            _ => 0.0,
                        };

                        utils.read_camera(world, container);
                        utils.anchor_cursor_by_anchor_center(DVec2::ZERO);
                        utils.camera_cursor_by_anchor_center(center);
                        if let Some(distance) = pinch {
                            utils.camera_distance_by_anchor_zoom_cursor(distance);
                        }
                        utils.write_camera(world, container);

                        LayerDragState::Grab {
                            start_position: center,
                            start_pinch: pinch,
                            last_center: prev_center,
                            last_zoom: prev_zoom,
                            last_instant: Instant::now(),
                            velocity,
                            zoom_velocity,
                            pending: None,
                        }
                    } else {
                        this.velocity = term_velocity;
                        if !dragged && let Some(pending) = pending {
                            world.queue_trigger(pending, PointerEvent::Press(capture.clone()));
                            world.queue_trigger(pending, PointerEvent::Release(capture.clone()));
                        }
                        lnwindow.window.request_redraw();
                        LayerDragState::None
                    }
                }
                (
                    LayerDragState::Grab {
                        start_position,
                        start_pinch,
                        last_center,
                        last_zoom,
                        last_instant,
                        velocity,
                        zoom_velocity,
                        pending,
                    },
                    PointerEvent::Move(capture),
                ) => {
                    this.velocity = DVec2::ZERO;
                    if !capture.members.is_empty() {
                        this.zoom_velocity = 0.0;
                    }

                    let prev_center = utils.camera_center();
                    let prev_zoom = utils.camera_zoom();

                    utils.read_camera(world, container);
                    utils.camera_cursor_by_camera_center(center);
                    if let Some(distance) = pinch {
                        utils.camera_distance_by_camera_zoom_center(distance);
                    }

                    if held && let Some(pending) = pending {
                        world.queue_trigger(
                            world.single::<PointerTool>(),
                            PointerTransfer {
                                kind: capture.pointer.kind,
                                target: pending,
                            },
                        );
                        LayerDragState::None
                    } else if this.ctrl {
                        utils.camera_cursor_by_anchor_center(center);
                        utils.camera_distance_by_anchor_zoom_cursor(1.0);
                        utils.write_camera(world, container);
                        LayerDragState::Scale {
                            start_position: center,
                        }
                    } else {
                        utils.write_camera(world, container);
                        let dt = last_instant.elapsed().as_secs_f64();

                        log::trace!(
                            "drag grab move {:?} dt={dt:.5} \
                            center_delta={:?} velocity={velocity:?}",
                            capture.pointer.kind,
                            (prev_center - last_center).q32_as_f64(),
                        );

                        let pending = match dragged {
                            true => pending,
                            false => None,
                        };

                        // only the latest point in a pointer event batch is valid
                        // .. > (.. .. last) > (.. .. prev) > (now .. ..)
                        // what we evaluate here is (prev - last)
                        if dt > 1e-4 {
                            let new_velocity = (prev_center - last_center).q32_as_f64() / dt;
                            let new_zoom_velocity = (prev_zoom - last_zoom).q32_as_f64() / dt;
                            let factor = 1.0 - 0.01f64.powf(dt * 10.0);
                            LayerDragState::Grab {
                                start_position,
                                start_pinch,
                                last_center: prev_center,
                                last_zoom: prev_zoom,
                                last_instant: Instant::now(),
                                velocity: velocity * (1. - factor) + new_velocity * factor,
                                zoom_velocity: zoom_velocity * (1. - factor)
                                    + new_zoom_velocity * factor,
                                pending,
                            }
                        } else {
                            LayerDragState::Grab {
                                start_position,
                                start_pinch,
                                last_center,
                                last_zoom,
                                last_instant,
                                velocity,
                                zoom_velocity,
                                pending,
                            }
                        }
                    }
                }

                // Scale
                (
                    LayerDragState::Scale { .. },
                    PointerEvent::Press(capture) | PointerEvent::Release(capture),
                ) => {
                    if !capture.members.is_empty() {
                        utils.read_camera(world, container);
                        utils.camera_cursor_by_anchor_center(center);
                        if let Some(distance) = pinch {
                            utils.camera_distance_by_anchor_zoom_cursor(distance);
                        } else {
                            utils.camera_distance_by_anchor_zoom_cursor(1.0);
                        }
                        utils.write_camera(world, container);

                        LayerDragState::Scale {
                            start_position: center,
                        }
                    } else {
                        LayerDragState::None
                    }
                }
                (LayerDragState::Scale { start_position }, PointerEvent::Move(_)) => {
                    utils.read_camera(world, container);
                    if let Some(distance) = pinch {
                        utils.camera_distance_by_camera_zoom_center(distance);
                    } else {
                        utils.camera_distance_by_camera_zoom_center(
                            (center - start_position).element_sum().exp2(),
                        );
                    }

                    if this.ctrl {
                        utils.write_camera(world, container);
                        LayerDragState::Scale { start_position }
                    } else {
                        utils.camera_cursor_by_anchor_center(center);
                        if let Some(distance) = pinch {
                            utils.camera_distance_by_anchor_zoom_cursor(distance);
                        }
                        utils.write_camera(world, container);
                        LayerDragState::Grab {
                            start_position: center,
                            start_pinch: pinch,
                            last_center: utils.camera_center(),
                            last_zoom: utils.camera_zoom(),
                            last_instant: Instant::now(),
                            velocity: DVec2::ZERO,
                            zoom_velocity: 0.0,
                            pending: None,
                        }
                    }
                }

                (_, PointerEvent::Enter(_)) => LayerDragState::None,
                (_, PointerEvent::Leave(_)) => LayerDragState::None,
                (_, PointerEvent::Hover(_)) => LayerDragState::None,
                (_, PointerEvent::Scroll { .. }) => LayerDragState::None,
                (_, PointerEvent::Menu(_)) => LayerDragState::None,
            };

            if matches!(event, PointerEvent::Release(_)) {
                log::trace!(
                    "drag release applied {:?} velocity={:?} zoom_velocity={}",
                    event.pointer().kind,
                    this.velocity,
                    this.zoom_velocity,
                );
            }

            if this.blocking
                && matches!(event, PointerEvent::Release(_))
                && matches!(state, LayerDragState::None)
            {
                // blocking ends when the handed-over gesture finishes
                this.blocking = false;
            }

            let mode = world.single_fetch::<LayerDraw>().mode;
            update_drag_icon(&this, &state, mode, &lnwindow);
        });
    }
}

impl Element for LayerDraw {
    fn when_insert(&mut self, world: &World, this: Handle<Self>) {
        world.scope("layer_draw");
        world.set_autodeps(Some(this));

        let (common, child) = {
            let page = world.single_fetch::<LayerPage>();
            (page.common, world.fetch(page.nest).child)
        };
        world.insert(ToolControl {
            common,
            order: -1000,
            // bottom of the page camera stack, only reached through the drag layer
            hit: Box::new(move |_world, _pointer| {
                Some(ToolHitResult {
                    camera: child,
                    handle: this.untyped(),
                })
            }),
        });

        let lnwindow = world.single::<Lnwindow>();
        let mut drag = Drag::new(2.0);
        let hold = Hold::default();
        let mut state = LayerDrawState::None;
        world.observer(this, move |event: &PointerEvent, world| {
            if let PointerEvent::Scroll { .. } = event {
                world.queue_trigger(world.single::<LayerDrag>(), event.clone());
                return;
            }
            if matches!(event, PointerEvent::Menu(_)) {
                return;
            }

            let dragged = drag.update(event).is_some();
            let held = hold.reached(event);

            let mut this = world.fetch_mut(this);
            let pointer = event.pointer();
            let page = &mut *world.single_fetch_mut::<LayerPage>();
            let container = page.nest;
            let lnwindow = world.fetch(lnwindow);
            let ctrl = world.single_fetch::<LayerDrag>().ctrl;

            if let PointerEvent::Press(capture) = event
                && !capture.members.is_empty()
            {
                // multiple draw pointers: hand the whole gesture over to the drag layer
                page.discard();
                state = LayerDrawState::None;
                world.single_fetch_mut::<LayerDrag>().blocking = true;
                world.queue_trigger(
                    world.single::<PointerTool>(),
                    PointerReacquire {
                        target: this.handle().untyped(),
                    },
                );
                update_draw_icon(&this, ctrl, &state, &lnwindow);
                return;
            }

            let root_camera = world.single_fetch::<RootCamera>().0;
            let camera = world.fetch(root_camera);

            let mut cursor_rect = world.fetch_mut(page.cursor_rect);
            cursor_rect.val = IRect::new_half(
                camera.screen_to_world(pointer.screen).q32_round(),
                UVec2::new(1, 1),
            );
            world.queue_trigger(cursor_rect.handle(), ValChanged);

            let mut cursor_shadow_rect = world.fetch_mut(page.cursor_shadow_rect);
            cursor_shadow_rect.val = cursor_rect.val + (pointer.tilt * 48.0).as_ivec2();
            world.queue_trigger(cursor_shadow_rect.handle(), ValChanged);

            if !matches!(pointer.kind, PointerKind::Touch(_)) {
                let visible = !matches!(event, PointerEvent::Leave(_));
                let mut cursor_visible = world.fetch_mut(page.cursor_visible);
                if cursor_visible.val != visible {
                    cursor_visible.val = visible;
                    world.queue_trigger(cursor_visible.handle(), ValChanged);
                }
            }

            if matches!(
                event,
                PointerEvent::Enter(_) | PointerEvent::Leave(_) | PointerEvent::Hover(_)
            ) {
                update_draw_icon(&this, ctrl, &state, &lnwindow);
                return;
            }

            let center = group_center(event);

            let prev = std::mem::replace(&mut state, LayerDrawState::None);
            state = match (prev, event) {
                (LayerDrawState::None, PointerEvent::Press(capture)) => {
                    world.insert(hold.task());
                    match (this.mode, ctrl) {
                        (LayerInputMode::Normal, true) | (LayerInputMode::Pick, _) => {
                            page.begin_pick(world);
                            page.pick_color(capture.pointer.position, world);
                            LayerDrawState::PickColor
                        }
                        (LayerInputMode::Normal, false) => {
                            let draw = Dab {
                                position: capture.pointer.position,
                                force: capture.pointer.force.unwrap_or(1.0),
                            };

                            page.draw_active(draw);
                            lnwindow.window.request_redraw();

                            LayerDrawState::Paint
                        }
                        (LayerInputMode::Area, _) => {
                            let container = world.fetch(container);
                            let camera = world.fetch(container.child);
                            // TODO Theme is not singleton
                            let theme = world.single_fetch::<Theme>();
                            let start_position = camera.screen_to_world(center);
                            let rect = world.insert(Val::new(IRect::new_half(
                                start_position.q32_round(),
                                UVec2::ZERO,
                            )));
                            let mode = world.insert(Val::new(AreaMode::None));
                            let area = world.insert(Area {
                                common: page.common,
                                rect,
                                color: theme.color.primary,
                                mode,
                            });
                            LayerDrawState::CreateArea {
                                start_position,
                                area,
                            }
                        }
                    }
                }

                // Draw
                (
                    LayerDrawState::Paint
                    | LayerDrawState::PaintErase
                    | LayerDrawState::PaintNoErase,
                    PointerEvent::Press(_),
                ) => {
                    page.discard();
                    LayerDrawState::None
                }
                (LayerDrawState::Paint, PointerEvent::Move(capture)) => {
                    let draw = Dab {
                        position: capture.pointer.position,
                        force: capture.pointer.force.unwrap_or(1.0),
                    };

                    page.draw_active(draw);
                    lnwindow.window.request_redraw();

                    if dragged {
                        LayerDrawState::PaintNoErase
                    } else if held {
                        page.discard();
                        if this.hold_pick {
                            page.begin_pick(world);
                            LayerDrawState::PickColor
                        } else {
                            LayerDrawState::PaintErase
                        }
                    } else {
                        LayerDrawState::Paint
                    }
                }
                (LayerDrawState::PaintNoErase, PointerEvent::Move(capture)) => {
                    let draw = Dab {
                        position: capture.pointer.position,
                        force: capture.pointer.force.unwrap_or(1.0),
                    };

                    page.draw_active(draw);
                    lnwindow.window.request_redraw();

                    LayerDrawState::PaintNoErase
                }
                (LayerDrawState::PaintErase, PointerEvent::Move(capture)) => {
                    let draw = Dab {
                        position: capture.pointer.position,
                        force: capture.pointer.force.unwrap_or(1.0),
                    };

                    page.draw_erase(draw);
                    lnwindow.window.request_redraw();

                    LayerDrawState::PaintErase
                }
                (
                    p @ (LayerDrawState::Paint
                    | LayerDrawState::PaintErase
                    | LayerDrawState::PaintNoErase),
                    PointerEvent::Release(capture),
                ) => {
                    if capture.members.is_empty() {
                        page.submit();
                        LayerDrawState::None
                    } else {
                        p
                    }
                }

                (LayerDrawState::PickColor, PointerEvent::Press(capture)) => {
                    page.pick_color(capture.pointer.position, world);
                    LayerDrawState::PickColor
                }
                (LayerDrawState::PickColor, PointerEvent::Move(capture)) => {
                    page.pick_color(capture.pointer.position, world);
                    LayerDrawState::PickColor
                }
                (LayerDrawState::PickColor, PointerEvent::Release(capture)) => {
                    page.pick_color(capture.pointer.position, world);
                    if capture.members.is_empty() {
                        page.end_pick(world);
                        this.mode = LayerInputMode::Normal;
                        world.queue_trigger(this.handle(), ValChanged);
                        world.queue_trigger(world.single::<LayerPage>(), ValChanged);
                        LayerDrawState::None
                    } else {
                        LayerDrawState::PickColor
                    }
                }

                // Area
                (
                    LayerDrawState::CreateArea {
                        start_position,
                        area,
                    },
                    PointerEvent::Move(_),
                ) => {
                    let container = world.fetch(container);
                    let camera = world.fetch(container.child);
                    let end_position = camera.screen_to_world(center);
                    let area = world.fetch(area);
                    let mut rect = world.fetch_mut(area.rect);
                    rect.val =
                        IRect::new_minmax(start_position.q32_round(), end_position.q32_round());
                    world.queue_trigger(rect.handle(), ValChanged);
                    LayerDrawState::CreateArea {
                        start_position,
                        area: area.retrieve(),
                    }
                }
                (
                    LayerDrawState::CreateArea {
                        start_position,
                        area,
                    },
                    PointerEvent::Release(capture),
                ) => {
                    let container = world.fetch(container);
                    let camera = world.fetch(container.child);
                    let end_position = camera.screen_to_world(center);
                    let area = world.fetch(area);
                    let mut rect = world.fetch_mut(area.rect);
                    rect.val =
                        IRect::new_minmax(start_position.q32_round(), end_position.q32_round());
                    world.queue_trigger(rect.handle(), ValChanged);

                    if capture.members.is_empty() {
                        let mut mode = world.fetch_mut(area.mode);
                        mode.val = AreaMode::Four;
                        world.queue_trigger(mode.handle(), ValChanged);
                        this.mode = LayerInputMode::Normal;
                        world.queue_trigger(this.handle(), ValChanged);
                        LayerDrawState::None
                    } else {
                        LayerDrawState::CreateArea {
                            start_position,
                            area: area.retrieve(),
                        }
                    }
                }

                (it @ LayerDrawState::CreateArea { .. }, PointerEvent::Press(_)) => it,
                (LayerDrawState::None, _) => LayerDrawState::None,
                _ => unreachable!(),
            };

            update_draw_icon(&this, ctrl, &state, &lnwindow);
        });
    }
}

fn update_drag_icon(
    this: &LayerDrag,
    state: &LayerDragState,
    mode: LayerInputMode,
    lnwindow: &Lnwindow,
) {
    match (this.space, this.ctrl, mode, state) {
        (true, true, _, LayerDragState::None) => {
            lnwindow.window.set_cursor(Cursor::Icon(CursorIcon::ZoomIn));
        }
        (true, false, _, LayerDragState::None) => {
            lnwindow.window.set_cursor(Cursor::Icon(CursorIcon::Grab));
        }
        (false, true, _, LayerDragState::None) => {
            lnwindow
                .window
                .set_cursor(Cursor::Icon(CursorIcon::Crosshair));
        }
        (false, _, LayerInputMode::Pick, LayerDragState::None) => {
            lnwindow
                .window
                .set_cursor(Cursor::Icon(CursorIcon::Crosshair));
        }
        (.., LayerDragState::Grab { .. }) => {
            lnwindow
                .window
                .set_cursor(Cursor::Icon(CursorIcon::Grabbing));
        }
        (.., LayerDragState::Scale { .. }) => {
            lnwindow.window.set_cursor(Cursor::Icon(CursorIcon::ZoomIn));
        }
        _ => {
            lnwindow
                .window
                .set_cursor(Cursor::Icon(CursorIcon::Default));
        }
    }
}

fn update_draw_icon(this: &LayerDraw, ctrl: bool, state: &LayerDrawState, lnwindow: &Lnwindow) {
    match (ctrl, this.mode, state) {
        (_, LayerInputMode::Pick, LayerDrawState::None) => {
            lnwindow
                .window
                .set_cursor(Cursor::Icon(CursorIcon::Crosshair));
        }
        (true, _, LayerDrawState::None) => {
            lnwindow
                .window
                .set_cursor(Cursor::Icon(CursorIcon::Crosshair));
        }
        (.., LayerDrawState::PickColor) => {
            lnwindow
                .window
                .set_cursor(Cursor::Icon(CursorIcon::Crosshair));
        }
        _ => {
            lnwindow
                .window
                .set_cursor(Cursor::Icon(CursorIcon::Default));
        }
    }
}

fn draw_group(pointer: &Pointer, input: &LayerDrag) -> bool {
    if input.blocking || input.space {
        return false;
    }
    match pointer.kind {
        PointerKind::Mouse | PointerKind::TabletTool(_) => true,
        PointerKind::Touch(_) => input.touch_draw,
        PointerKind::Unknown => false,
    }
}

fn group_center(event: &PointerEvent) -> DVec2 {
    let (capture, active) = match event {
        PointerEvent::Press(capture) | PointerEvent::Move(capture) => (capture, true),
        PointerEvent::Release(capture) => (capture, false),
        _ => unreachable!(),
    };

    let mut sum = DVec2::ZERO;
    let mut cnt = 0;
    if active {
        sum += capture.pointer.screen;
        cnt += 1;
    }
    for member in &capture.members {
        sum += member.screen;
        cnt += 1;
    }

    if cnt > 0 {
        sum / cnt as f64
    } else {
        capture.pointer.screen
    }
}

fn group_pinch(event: &PointerEvent, center: DVec2) -> Option<f64> {
    let (capture, active) = match event {
        PointerEvent::Press(capture) | PointerEvent::Move(capture) => (capture, true),
        PointerEvent::Release(capture) => (capture, false),
        _ => unreachable!(),
    };

    let mut sum = 0.0;
    let mut cnt = 0;
    if active {
        sum += DVec2::distance(capture.pointer.screen, center);
        cnt += 1;
    }
    for member in &capture.members {
        sum += DVec2::distance(member.screen, center);
        cnt += 1;
    }

    if cnt >= 2 {
        Some(sum / cnt as f64)
    } else {
        None
    }
}
