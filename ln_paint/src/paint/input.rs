use std::time::{Duration, Instant};

use glam::{DVec2, I64Vec2, Mat3, UVec2, Vec4};
use ln_world::{Element, Handle, HandleGeneric, World};
use winit::{
    cursor::{Cursor, CursorIcon},
    event::{ButtonSource, ElementState, MouseButton, PointerKind, PointerSource, WindowEvent},
    keyboard::KeyCode,
};

use crate::{
    input::{
        collider::ToolRectCollider,
        pointer::{PointerHover, PointerHoverStatus, PointerScroll},
        touch::{MultiTouchGroup, MultiTouchStatus},
    },
    layout::val::{Val, ValChanged},
    lnwin::Lnwindow,
    paint::{
        brush::{Dab, param::BrushParamKey},
        wrapper::{BrushConfigurationChanged, LayerPage},
    },
    render::{RenderPrepareControl, camera::UICamera},
    save::Save,
    ui::{
        theme::Theme,
        widget::area::{Area, AreaMode},
    },
    unit::{FI64Ext, IRect},
};

const DRAG_DISTANCE: f64 = 0.005;
const ERASE_TIMER: f64 = 0.4;

pub struct LayerInput {
    pub velocity: DVec2,
    pub zoom_velocity: f64,
    pub damping: f32,
    pub zoom_damping: f32,
    pub touch_draw: bool,
    pub space: bool,
    pub ctrl: bool,
    pub shift: bool,
    pub mode: LayerInputMode,
    pub hold_pick: bool,
    pub input_offset_main: I64Vec2,
}

#[derive(Default, Debug, Clone, Copy)]
pub enum LayerInputMode {
    #[default]
    Normal,
    Pick,
    Area,
}

#[derive(Debug)]
enum LayerInputState {
    None,
    Paint {
        start_position: DVec2,
        start_instant: Instant,
    },
    PaintNoErase,
    PaintErase,
    Grab {
        start_position: DVec2,
        start_pinch: Option<f64>,
        last_center: I64Vec2,
        last_zoom: i64,
        last_instant: Instant,
        velocity: DVec2,
        zoom_velocity: f64,
    },
    Scale {
        start_position: DVec2,
    },
    PickColor,
    CreateArea {
        start_position: I64Vec2,
        area: Handle<Area>,
    },
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

impl Default for LayerInput {
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
            mode: LayerInputMode::Normal,
            hold_pick: false,
            input_offset_main: I64Vec2::ZERO,
        }
    }
}

impl Element for LayerInput {
    fn when_insert(&mut self, world: &World, this: Handle<Self>) {
        world.scope("layer_input");
        world.set_autodeps(Some(this));

        let page = world.single_fetch::<LayerPage>();
        let collider_rect = world.insert(Val::new(IRect::FULLSCREEN));
        world.insert(ToolRectCollider {
            common: page.common,
            target: this.untyped(),
            rect: collider_rect,
            order: -100,
        });

        let mut timer = None;
        world.insert(RenderPrepareControl {
            prepare: Box::new(move |world| {
                let this = &mut *world.fetch_mut(this);
                let page = world.single_fetch::<LayerPage>();
                if let Some(last_instant) = timer.replace(Instant::now()) {
                    let dt = last_instant.elapsed().as_secs_f64();
                    if this.velocity.length_squared() > 1e-3 || this.zoom_velocity.abs() > 1e-3 {
                        let mut utils = world.fetch_mut(page.camera_utils);
                        utils.read_camera(world, page.container);
                        this.velocity *= (this.damping as f64).powf(dt).min(1.0);
                        this.zoom_velocity *= (this.zoom_damping as f64).powf(dt).min(1.0);
                        let zoom = utils.camera_zoom() + i64::q32_from_f64(this.zoom_velocity * dt);
                        let center =
                            utils.camera_center() + I64Vec2::q32_from_f64(this.velocity * dt);
                        utils.camera_center_by_camera_cursor(center);
                        utils.camera_zoom_by_camera_distance_center(zoom);
                        utils.write_camera(world, page.container);
                        let lnwindow = world.single_fetch::<Lnwindow>();
                        lnwindow.window.request_redraw();
                    }
                }
            }),
        });

        world.observer(this, move |event: &PointerHover, world| {
            if let PointerKind::Touch(_) = event.pointer.kind {
                return;
            }

            let mut this = world.fetch_mut(this);
            let page_camera = world.single_fetch::<LayerPage>().common.camera;
            let page_camera = world.fetch(page_camera);
            let ui_camera = world.single_fetch::<UICamera>().0;
            let camera = world.fetch(ui_camera);
            let wrapper = world.single_fetch::<LayerPage>();
            let save = world.single_fetch::<Save>();

            let mut offset = DVec2::ZERO;
            let mut screen = event.pointer.screen;
            let mut tilt = event.pointer.tilt;
            if let Some(tilt_correction) = &save.sn().tilt_correction {
                tilt = match is_portrait(world) {
                    true => {
                        let transformed = tilt_correction.portrait * tilt.extend(1.);
                        transformed.truncate() / transformed.z
                    }
                    false => {
                        let transformed = tilt_correction.landscape * tilt.extend(1.);
                        transformed.truncate() / transformed.z
                    }
                }
            }
            if let Some(offset_correction) = &save.sn().offset_correction {
                offset = match is_portrait(world) {
                    true => {
                        let transformed = offset_correction.portrait * tilt.extend(1.);
                        transformed.truncate() / transformed.z
                    }
                    false => {
                        let transformed = offset_correction.landscape * tilt.extend(1.);
                        transformed.truncate() / transformed.z
                    }
                }
                .as_dvec2()
            }
            screen += offset;
            this.input_offset_main = page_camera.screen_to_world_relative(offset);

            let mut cursor_rect = world.fetch_mut(wrapper.cursor_rect);
            cursor_rect.val =
                IRect::new_half(camera.screen_to_world(screen).q32_round(), UVec2::new(1, 1));
            world.queue_trigger(cursor_rect.handle(), ValChanged);

            let mut cursor_shadow_rect = world.fetch_mut(wrapper.cursor_shadow_rect);
            cursor_shadow_rect.val = cursor_rect.val + (tilt * 48.0).as_ivec2();
            world.queue_trigger(cursor_shadow_rect.handle(), ValChanged);

            match event.status {
                PointerHoverStatus::Enter => {
                    let mut visible = world.fetch_mut(wrapper.cursor_visible);
                    visible.val = true;
                    world.queue_trigger(visible.handle(), ValChanged);
                }
                PointerHoverStatus::Moving => {}
                PointerHoverStatus::Leave => {
                    let mut visible = world.fetch_mut(wrapper.cursor_visible);
                    visible.val = false;
                    world.queue_trigger(visible.handle(), ValChanged);
                }
            }
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
                    world.queue_trigger(page.handle(), BrushConfigurationChanged);
                }
                KeyCode::Space => {
                    this.space = press;
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
                    let mut theme = world.single_fetch_mut::<Theme>();
                    if theme.distinct.x == 1.0 {
                        *theme = Theme::light_mode();
                    } else {
                        *theme = Theme::dark_mode();
                    }
                    theme.sync(world);
                }
                _ => (),
            }

            update_icon(&this, &LayerInputState::None, &lnwindow);
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
                let main = world.single_fetch::<LayerPage>().container;
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
                let main = world.single_fetch::<LayerPage>().container;
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

        world.observer(this, move |event: &PointerScroll, world| {
            let main = world.single_fetch::<LayerPage>().container;
            let utils = world.single_fetch::<LayerPage>().camera_utils;
            let mut utils = world.fetch_mut(utils);

            let zoom_delta = -event.delta.y;
            utils.read_camera(world, main);
            utils.anchor_cursor_by_anchor_center(DVec2::ZERO);
            utils.camera_cursor_by_anchor_center(event.pointer.screen);
            utils.anchor_distance_by_anchor_zoom(200.0);
            if zoom_delta > 0.0 {
                utils.camera_distance_by_anchor_zoom_cursor(200.0);
                utils.camera_distance_by_camera_zoom_center(200.0 + zoom_delta);
            } else {
                utils.camera_distance_by_anchor_zoom_cursor(200.0 - zoom_delta);
                utils.camera_distance_by_camera_zoom_center(200.0);
            }
            utils.write_camera(world, main);
        });

        let mut state = LayerInputState::None;
        world.observer(this, move |event: &MultiTouchGroup, world| {
            let container = world.single_fetch::<LayerPage>().container;
            let utils = world.single_fetch::<LayerPage>().camera_utils;
            let mut this = world.fetch_mut(this);
            let lnwindow = world.fetch(lnwindow);
            let utils = &mut *world.fetch_mut(utils);
            let page = &mut *world.single_fetch_mut::<LayerPage>();

            let center = touch_center(event).unwrap_or(event.active.data.screen);
            let pinch = touch_pinch(event, center);

            let prev = std::mem::replace(&mut state, LayerInputState::None);
            state = match (prev, event.active.status) {
                (LayerInputState::None, MultiTouchStatus::Press)
                    if (!this.touch_draw
                        && matches!(event.active.data.kind, PointerKind::Touch(_)))
                        || this.space =>
                {
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
                        LayerInputState::Scale {
                            start_position: center,
                        }
                    } else {
                        utils.write_camera(world, container);
                        LayerInputState::Grab {
                            start_position: center,
                            start_pinch: pinch,
                            last_center: utils.camera_center(),
                            last_zoom: utils.camera_zoom(),
                            last_instant: Instant::now(),
                            velocity: DVec2::ZERO,
                            zoom_velocity: 0.0,
                        }
                    }
                }
                (LayerInputState::None, MultiTouchStatus::Press) => match (this.mode, this.ctrl) {
                    (LayerInputMode::Normal, true) | (LayerInputMode::Pick, _) => {
                        page.pick_color(event.active.position + this.input_offset_main, world);
                        LayerInputState::PickColor
                    }
                    (LayerInputMode::Normal, false) => {
                        let draw = Dab {
                            position: event.active.position + this.input_offset_main,
                            force: event.active.data.force.unwrap_or(1.0),
                        };

                        page.draw_active(draw);
                        lnwindow.window.request_redraw();

                        LayerInputState::Paint {
                            start_position: event.active.data.screen,
                            start_instant: Instant::now(),
                        }
                    }
                    (LayerInputMode::Area, _) => {
                        let container = world.fetch(container);
                        let camera = world.fetch(container.child);
                        let start_position = camera.screen_to_world(center);
                        let rect = world.insert(Val::new(IRect::new_half(
                            start_position.q32_round(),
                            UVec2::ZERO,
                        )));
                        let color = world.insert(Val::new(Vec4::new(0.3, 0.3, 0.3, 1.0)));
                        let mode = world.insert(Val::new(AreaMode::None));
                        let area = world.insert(Area {
                            common: page.common,
                            rect,
                            color,
                            mode,
                        });
                        LayerInputState::CreateArea {
                            start_position,
                            area,
                        }
                    }
                },

                // Grab
                (
                    LayerInputState::Grab {
                        last_center,
                        last_zoom,
                        last_instant,
                        velocity,
                        zoom_velocity,
                        ..
                    },
                    MultiTouchStatus::Press | MultiTouchStatus::Release,
                ) => {
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

                    if event.members.len() > 0 {
                        this.velocity = DVec2::ZERO;
                        if event.members.len() > 1 {
                            this.zoom_velocity = 0.0;
                        } else {
                            this.zoom_velocity = term_zoom_velocity;
                        }

                        utils.read_camera(world, container);
                        utils.anchor_cursor_by_anchor_center(DVec2::ZERO);
                        utils.camera_cursor_by_anchor_center(center);
                        if let Some(distance) = pinch {
                            utils.camera_distance_by_anchor_zoom_cursor(distance);
                        }
                        utils.write_camera(world, container);

                        LayerInputState::Grab {
                            start_position: center,
                            start_pinch: pinch,
                            last_center: prev_center,
                            last_zoom: prev_zoom,
                            last_instant: Instant::now(),
                            velocity,
                            zoom_velocity,
                        }
                    } else {
                        this.velocity = term_velocity;
                        LayerInputState::None
                    }
                }
                (
                    LayerInputState::Grab {
                        start_position,
                        start_pinch,
                        last_center,
                        last_zoom,
                        last_instant,
                        velocity,
                        zoom_velocity,
                    },
                    MultiTouchStatus::Holding,
                ) => {
                    this.velocity = DVec2::ZERO;
                    if event.members.len() > 1 {
                        this.zoom_velocity = 0.0;
                    }

                    let prev_center = utils.camera_center();
                    let prev_zoom = utils.camera_zoom();

                    utils.read_camera(world, container);
                    utils.camera_cursor_by_camera_center(center);
                    if let Some(distance) = pinch {
                        utils.camera_distance_by_camera_zoom_center(distance);
                    }

                    if this.ctrl {
                        utils.camera_cursor_by_anchor_center(center);
                        utils.camera_distance_by_anchor_zoom_cursor(1.0);
                        utils.write_camera(world, container);
                        LayerInputState::Scale {
                            start_position: center,
                        }
                    } else {
                        utils.write_camera(world, container);
                        let dt = last_instant.elapsed().as_secs_f64();

                        // only the latest point in a MultiTouch event batch is valid
                        // .. > (.. .. last) > (.. .. prev) > (now .. ..)
                        // what we evaluate here is (prev - last)
                        if dt > 1e-4 {
                            let new_velocity = (prev_center - last_center).q32_as_f64() / dt;
                            let new_zoom_velocity = (prev_zoom - last_zoom).q32_as_f64() / dt;
                            let factor = 1.0 - 0.01f64.powf(dt * 10.0);
                            LayerInputState::Grab {
                                start_position,
                                start_pinch,
                                last_center: prev_center,
                                last_zoom: prev_zoom,
                                last_instant: Instant::now(),
                                velocity: velocity * (1. - factor) + new_velocity * factor,
                                zoom_velocity: zoom_velocity * (1. - factor)
                                    + new_zoom_velocity * factor,
                            }
                        } else {
                            LayerInputState::Grab {
                                start_position,
                                start_pinch,
                                last_center,
                                last_zoom,
                                last_instant,
                                velocity,
                                zoom_velocity,
                            }
                        }
                    }
                }

                // Scale
                (
                    LayerInputState::Scale { .. },
                    MultiTouchStatus::Press | MultiTouchStatus::Release,
                ) => {
                    if event.members.len() > 0 {
                        utils.read_camera(world, container);
                        utils.camera_cursor_by_anchor_center(center);
                        if let Some(distance) = pinch {
                            utils.camera_distance_by_anchor_zoom_cursor(distance);
                        } else {
                            utils.camera_distance_by_anchor_zoom_cursor(1.0);
                        }
                        utils.write_camera(world, container);

                        LayerInputState::Scale {
                            start_position: center,
                        }
                    } else {
                        LayerInputState::None
                    }
                }
                (LayerInputState::Scale { start_position }, MultiTouchStatus::Holding) => {
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
                        LayerInputState::Scale { start_position }
                    } else {
                        utils.camera_cursor_by_anchor_center(center);
                        if let Some(distance) = pinch {
                            utils.camera_distance_by_anchor_zoom_cursor(distance);
                        }
                        utils.write_camera(world, container);
                        LayerInputState::Grab {
                            start_position: center,
                            start_pinch: pinch,
                            last_center: utils.camera_center(),
                            last_zoom: utils.camera_zoom(),
                            last_instant: Instant::now(),
                            velocity: DVec2::ZERO,
                            zoom_velocity: 0.0,
                        }
                    }
                }

                // Paint
                (
                    LayerInputState::Paint { .. }
                    | LayerInputState::PaintErase
                    | LayerInputState::PaintNoErase,
                    MultiTouchStatus::Press,
                ) => {
                    page.discard();

                    utils.read_camera(world, container);
                    utils.camera_cursor_by_anchor_center(center);
                    if let Some(distance) = pinch {
                        utils.camera_distance_by_anchor_zoom_cursor(distance);
                    }
                    utils.write_camera(world, container);

                    LayerInputState::Grab {
                        start_position: center,
                        start_pinch: pinch,
                        last_center: utils.camera_center(),
                        last_zoom: utils.camera_zoom(),
                        last_instant: Instant::now(),
                        velocity: DVec2::ZERO,
                        zoom_velocity: 0.0,
                    }
                }
                (
                    LayerInputState::Paint {
                        start_position,
                        start_instant,
                    },
                    MultiTouchStatus::Holding,
                ) => {
                    let draw = Dab {
                        position: event.active.position + this.input_offset_main,
                        force: event.active.data.force.unwrap_or(1.0),
                    };

                    page.draw_active(draw);
                    lnwindow.window.request_redraw();

                    if event.active.data.screen.distance(start_position) > DRAG_DISTANCE {
                        LayerInputState::PaintNoErase
                    } else if start_instant.elapsed() > Duration::from_secs_f64(ERASE_TIMER) {
                        page.discard();
                        if this.hold_pick {
                            LayerInputState::PickColor
                        } else {
                            LayerInputState::PaintErase
                        }
                    } else {
                        LayerInputState::Paint {
                            start_position,
                            start_instant,
                        }
                    }
                }
                (LayerInputState::PaintNoErase, MultiTouchStatus::Holding) => {
                    let draw = Dab {
                        position: event.active.position + this.input_offset_main,
                        force: event.active.data.force.unwrap_or(1.0),
                    };

                    page.draw_active(draw);
                    lnwindow.window.request_redraw();

                    LayerInputState::PaintNoErase
                }
                (LayerInputState::PaintErase, MultiTouchStatus::Holding) => {
                    let draw = Dab {
                        position: event.active.position + this.input_offset_main,
                        force: event.active.data.force.unwrap_or(1.0),
                    };

                    page.draw_erase(draw);
                    lnwindow.window.request_redraw();

                    LayerInputState::PaintErase
                }
                (
                    p @ (LayerInputState::Paint { .. }
                    | LayerInputState::PaintErase
                    | LayerInputState::PaintNoErase),
                    MultiTouchStatus::Release,
                ) => {
                    if event.members.len() > 1 {
                        p
                    } else {
                        page.submit();
                        LayerInputState::None
                    }
                }

                (
                    LayerInputState::PickColor,
                    MultiTouchStatus::Press | MultiTouchStatus::Holding,
                ) => {
                    page.pick_color(event.active.position + this.input_offset_main, world);
                    LayerInputState::PickColor
                }
                (LayerInputState::PickColor, MultiTouchStatus::Release) => {
                    page.pick_color(event.active.position + this.input_offset_main, world);
                    this.mode = LayerInputMode::Normal;
                    world.queue_trigger(world.single::<LayerPage>(), BrushConfigurationChanged);
                    LayerInputState::None
                }

                // Area
                (
                    LayerInputState::CreateArea {
                        start_position,
                        area,
                    },
                    MultiTouchStatus::Holding,
                ) => {
                    let container = world.fetch(container);
                    let camera = world.fetch(container.child);
                    let end_position = camera.screen_to_world(center);
                    let area = world.fetch(area);
                    let mut rect = world.fetch_mut(area.rect);
                    rect.val =
                        IRect::new_minmax(start_position.q32_round(), end_position.q32_round());
                    world.queue_trigger(rect.handle(), ValChanged);
                    LayerInputState::CreateArea {
                        start_position,
                        area: area.retrieve(),
                    }
                }
                (
                    LayerInputState::CreateArea {
                        start_position,
                        area,
                    },
                    MultiTouchStatus::Release,
                ) => {
                    let container = world.fetch(container);
                    let camera = world.fetch(container.child);
                    let end_position = camera.screen_to_world(center);
                    let area = world.fetch(area);
                    let mut rect = world.fetch_mut(area.rect);
                    let mut mode = world.fetch_mut(area.mode);
                    rect.val =
                        IRect::new_minmax(start_position.q32_round(), end_position.q32_round());
                    mode.val = AreaMode::Four;
                    world.queue_trigger(rect.handle(), ValChanged);
                    world.queue_trigger(mode.handle(), ValChanged);
                    this.mode = LayerInputMode::Normal;
                    LayerInputState::None
                }

                // Edge cases: Ignored
                (LayerInputState::None, MultiTouchStatus::Holding) => LayerInputState::None,
                (LayerInputState::None, MultiTouchStatus::Release) => LayerInputState::None,
                (it @ LayerInputState::CreateArea { .. }, MultiTouchStatus::Press) => it,
            };

            update_icon(&this, &state, &lnwindow);
        });
    }
}

fn update_icon(this: &LayerInput, state: &LayerInputState, lnwindow: &Lnwindow) {
    match (this.space, this.ctrl, this.mode, state) {
        (true, true, _, LayerInputState::None) => {
            lnwindow.window.set_cursor(Cursor::Icon(CursorIcon::ZoomIn));
        }
        (true, false, _, LayerInputState::None) => {
            lnwindow.window.set_cursor(Cursor::Icon(CursorIcon::Grab));
        }
        (false, true, _, LayerInputState::None) => {
            lnwindow
                .window
                .set_cursor(Cursor::Icon(CursorIcon::Crosshair));
        }
        (false, _, LayerInputMode::Pick, LayerInputState::None) => {
            lnwindow
                .window
                .set_cursor(Cursor::Icon(CursorIcon::Crosshair));
        }
        (.., LayerInputState::PickColor) => {
            lnwindow
                .window
                .set_cursor(Cursor::Icon(CursorIcon::Crosshair));
        }
        (.., LayerInputState::Grab { .. }) => {
            lnwindow
                .window
                .set_cursor(Cursor::Icon(CursorIcon::Grabbing));
        }
        (.., LayerInputState::Scale { .. }) => {
            lnwindow.window.set_cursor(Cursor::Icon(CursorIcon::ZoomIn));
        }
        _ => {
            lnwindow
                .window
                .set_cursor(Cursor::Icon(CursorIcon::Default));
        }
    }
}

fn touch_center(event: &MultiTouchGroup) -> Option<DVec2> {
    let mut sum = DVec2::ZERO;
    let mut cnt = 0;
    for member in &event.members {
        sum += member.data.screen;
        cnt += 1;
    }

    if cnt > 0 {
        Some(sum / cnt as f64)
    } else {
        None
    }
}

fn touch_pinch(event: &MultiTouchGroup, center: DVec2) -> Option<f64> {
    let mut sum = 0.0;
    let mut cnt = 0;
    for member in &event.members {
        sum += DVec2::distance(member.data.screen, center);
        cnt += 1;
    }

    if cnt >= 2 {
        Some(sum / cnt as f64)
    } else {
        None
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
