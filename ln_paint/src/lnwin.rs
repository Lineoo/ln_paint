use std::{
    sync::Arc,
    time::{Duration, Instant},
};

use glam::{DVec2, I64Vec2, IVec2, UVec2};
use hashbrown::HashMap;
use ln_world::{Element, Handle, World};
#[cfg(target_os = "android")]
use winit::platform::android::activity::AndroidApp;
use winit::{
    application::ApplicationHandler,
    dpi::{PhysicalPosition, PhysicalSize},
    event::{StartCause, WindowEvent},
    event_loop::{ActiveEventLoop, ControlFlow},
    window::{Window, WindowAttributes, WindowId},
};

use crate::{
    input::{mouse::MouseTool, pointer::PointerTool},
    render::{
        Render,
        camera::{Camera, CameraDescriptor, CameraPipeline},
        nest::Nest,
    },
    save::{AutosaveScheduler, Save},
    ui::{
        common::WidgetCommon,
        renderer::{
            canvas::CanvasPipeline, quad::QuadMeshPipeline, rrect::RRectMaterial,
            text::TextPipeline, vtor::VtorPipeline, wireframe::WireframePipeline,
        },
        section::{desktop_ui::DesktopUi, mobile_ui::MobileUi},
        theme::{Theme, ThemeScheme},
        widget::palette::{
            hsl::{HslRingMaterial, HslSquareMaterial},
            oklab::{OklabBarMaterial, OklabPolarMaterial},
        },
    },
    unit::{FI64Ext, IRect, URect},
    val::{Val, ValChanged},
};

type Payload = Arc<dyn Fn(&World, WidgetCommon, Handle<Val<IRect>>) + Send + Sync>;

#[derive(Default)]
pub struct Lnwin {
    pub world: World,
    pub windows: HashMap<WindowId, Handle<Lnwindow>>,
    pub payload: Option<Payload>,
}

impl ApplicationHandler for Lnwin {
    fn can_create_surfaces(&mut self, event_loop: &dyn ActiveEventLoop) {
        if self.windows.is_empty() {
            let mut win_attr = WindowAttributes::default()
                .with_transparent(true)
                .with_title("LnPaint");

            if let Some(value) = std::env::var_os("LNPAINT_WINDOW_SIZE") {
                let value = value.to_string_lossy();
                let (width, height) = value.split_once('x').unwrap();
                let width: u32 = width.parse().unwrap();
                let height: u32 = height.parse().unwrap();
                let size = PhysicalSize::new(width, height);
                log::warn!("surface size overridden to {size:?} by LNPAINT_WINDOW_SIZE");
                win_attr = win_attr.with_surface_size(size);
            }

            let window = event_loop.create_window(win_attr).unwrap();
            let window = Arc::from(window);

            let payload = self.payload.clone();
            let scale_factor_override = std::env::var("LNPAINT_SCALE_FACTOR").ok().map(|value| {
                let scale = value.parse::<f64>().unwrap();
                assert!(
                    scale.is_finite() && scale > 0.0,
                    "invalid scale factor: {value}"
                );
                log::warn!("scale factor overridden to {scale} by LNPAINT_SCALE_FACTOR");
                scale
            });
            let lnwindow = Lnwindow {
                window,
                payload,
                scale_factor_override,
            };

            let window_id = lnwindow.window.id();
            let lnwindow = self.world.insert(lnwindow);
            self.windows.insert(window_id, lnwindow);
        } else {
            let mut render = self.world.single_fetch_mut::<Render>();
            let lnwindow = self.world.single_fetch::<Lnwindow>();
            render.target_recreate(&lnwindow);
        }

        self.world.flush();
    }

    fn new_events(&mut self, event_loop: &dyn ActiveEventLoop, _cause: StartCause) {
        let now = Instant::now();
        let mut earliest = None;
        self.world.foreach_fetch::<Task>(|task| {
            let earliest = earliest.get_or_insert(task.instant);
            if task.instant < *earliest {
                *earliest = task.instant;
            }
            if task.instant <= now {
                let task = task.retrieve();
                self.world.queue(move |world| {
                    world.remove_with(task, |task, world| {
                        log::debug!("Oi!");
                        (task.content)(world)
                    });
                });
            }
        });
        if let Some(earliest) = earliest {
            log::debug!("Lets wait!");
            event_loop.set_control_flow(ControlFlow::WaitUntil(earliest));
        } else {
            event_loop.set_control_flow(ControlFlow::Wait);
        }

        self.world.flush();
    }

    fn window_event(
        &mut self,
        event_loop: &dyn ActiveEventLoop,
        window_id: WindowId,
        event: WindowEvent,
    ) {
        if let Some(&lnwindow) = self.windows.get(&window_id) {
            if self.world.validate(lnwindow).is_err() {
                self.windows.remove(&window_id);
            } else {
                self.world.trigger(lnwindow, &event);
            }

            self.world.flush();
        }

        if self.windows.is_empty() {
            event_loop.exit()
        }
    }
}

/// The main window.
pub struct Lnwindow {
    pub window: Arc<dyn Window>,
    pub payload: Option<Payload>,
    pub scale_factor_override: Option<f64>,
}

impl Element for Lnwindow {
    fn when_insert(&mut self, world: &World, this: Handle<Self>) {
        world.scope("lnwindow");
        world.set_autodeps(Some(this));

        world.insert(pollster::block_on(Render::new(self)));
        world.insert_with(CameraPipeline::from_world);
        world.insert_with(CanvasPipeline::from_world);
        world.insert_with(QuadMeshPipeline::<HslSquareMaterial>::from_world);
        world.insert_with(QuadMeshPipeline::<HslRingMaterial>::from_world);
        world.insert_with(QuadMeshPipeline::<OklabPolarMaterial>::from_world);
        world.insert_with(QuadMeshPipeline::<OklabBarMaterial>::from_world);
        world.insert_with(QuadMeshPipeline::<RRectMaterial>::from_world);
        world.insert_with(WireframePipeline::from_world);
        world.insert_with(VtorPipeline::from_world);
        world.insert_with(TextPipeline::from_world);
        world.insert(MouseTool::default());
        world.insert(PointerTool::default());

        let rect = world.insert(Val::new(self.window_rect()));
        let camera = CameraDescriptor {
            center: self.surface_center(),
            zoom: i64::q32_from_f64(self.scale_factor().log2()),
        };
        let camera = world.insert_with(|world| world.build(camera));
        let visible = world.insert(Val::new(true));
        let theme_hue = world.insert(Val::new(0.67));
        let theme_scheme = world.insert(Val::new(ThemeScheme::Dark));
        let theme = world.insert(Theme::source(world, theme_hue, theme_scheme));
        world.insert(RootCamera(camera));
        let common = WidgetCommon {
            camera,
            visible,
            theme,
        };

        world.observer(this, move |event: &WindowEvent, world| match event {
            WindowEvent::CloseRequested => {
                world.queue(|world| {
                    world.scope("window-closed-cleanup");
                    world.clear();
                });
            }
            WindowEvent::SurfaceResized(_) => {
                let lnwindow = world.fetch(this);
                let mut rect = world.fetch_mut(rect);
                let mut camera = world.fetch_mut(camera);

                let scale = lnwindow.scale_factor();
                let next = lnwindow.window_rect();
                let size = lnwindow.window.surface_size();

                rect.val = next;
                world.queue_trigger(rect.handle(), ValChanged);

                camera.center = lnwindow.surface_center();
                camera.zoom = i64::q32_from_f64(scale.log2());
                camera.surface_size = UVec2::new(size.width, size.height);
                camera.upload();
                lnwindow.window.request_redraw();

                for &nest in &camera.nests {
                    world.queue(move |world| Nest::refresh_camera(world, nest));
                }
            }
            _ => (),
        });

        world.queue(move |world| {
            world.scope("lnwindow-payload");
            world.set_autodeps(Some(this));

            let this = world.fetch(this);
            if let Some(payload) = &this.payload {
                payload(world, common, rect);
            } else {
                // TODO async save initialization
                let save = Save::from_world(world);

                let use_desktop =
                    std::env::var_os("LNPAINT_DESKTOP_UI").is_some() || save.sn().desktop_mode;

                if let Some(scheme) = save.sn().theme.scheme {
                    let mut theme_scheme = world.fetch_mut(theme_scheme);
                    theme_scheme.val = scheme;
                    world.queue_trigger(theme_scheme.handle(), ValChanged);
                }
                if let Some(hue) = save.sn().theme.hue {
                    let mut theme_hue = world.fetch_mut(theme_hue);
                    theme_hue.val = hue;
                    world.queue_trigger(theme_hue.handle(), ValChanged);
                }

                world.insert(save);
                world.insert(AutosaveScheduler {
                    autosave_duration: Duration::from_secs(180),
                });

                if use_desktop {
                    log::warn!("using experimental desktop ui");
                    world.insert(DesktopUi { common, rect });
                } else {
                    world.insert(MobileUi { common, rect });
                }
            }
        });
    }
}

impl Lnwindow {
    pub fn scale_factor(&self) -> f64 {
        self.scale_factor_override
            .unwrap_or_else(|| self.window.scale_factor())
    }

    pub fn scaled_size(&self, size: UVec2) -> UVec2 {
        let scale = self.scale_factor();
        (size.as_dvec2() * scale).round().as_uvec2()
    }

    pub fn scaled_delta(&self, delta: IVec2) -> IVec2 {
        let scale = self.scale_factor();
        (delta.as_dvec2() * scale).round().as_ivec2()
    }

    pub fn screen_scissor(&self, rect: IRect, camera: &Camera) -> URect {
        let size = self.window.surface_size();
        let left_up =
            self.screen_to_cursor(camera.world_to_screen(I64Vec2::q32_from_i32(rect.left_up())));
        let right_down =
            self.screen_to_cursor(camera.world_to_screen(I64Vec2::q32_from_i32(rect.right_down())));

        // ceil/floor keeps the last pixel inside fractional-scaled panels.
        let width = size.width as f64;
        let height = size.height as f64;
        let x = left_up.x.clamp(0.0, width).floor() as u32;
        let y = left_up.y.clamp(0.0, height).floor() as u32;
        let right = right_down.x.clamp(0.0, width).ceil() as u32;
        let down = right_down.y.clamp(0.0, height).ceil() as u32;

        URect {
            origin: UVec2::new(x, y),
            extend: UVec2::new(right.saturating_sub(x), down.saturating_sub(y)),
        }
    }

    pub fn cursor_to_screen(&self, position: PhysicalPosition<f64>) -> DVec2 {
        let size = self.window.surface_size();
        let x = (position.x * 2.0) / size.width as f64 - 1.0;
        let y = 1.0 - (position.y * 2.0) / size.height as f64;
        DVec2::new(x, y)
    }

    pub fn screen_to_cursor(&self, ndc: DVec2) -> PhysicalPosition<f64> {
        let size = self.window.surface_size();
        let x = (ndc.x + 1.0) * size.width as f64 / 2.0;
        let y = (1.0 - ndc.y) * size.height as f64 / 2.0;
        PhysicalPosition::new(x, y)
    }

    fn window_rect(&self) -> IRect {
        let size = self.window.surface_size();
        let scale = self.scale_factor();
        let size = DVec2::new(size.width as f64, size.height as f64);
        let extend = (size / scale).ceil().as_uvec2();
        IRect {
            origin: IVec2::ZERO,
            extend,
        }
    }

    fn surface_center(&self) -> I64Vec2 {
        let size = self.window.surface_size();
        let scale = self.scale_factor();
        let size = DVec2::new(size.width as f64, size.height as f64);
        I64Vec2::q32_from_f64(size / scale / 2.0)
    }
}

pub struct RootCamera(pub Handle<Camera>);

impl Element for RootCamera {}

pub struct Task {
    pub instant: Instant,
    pub content: Box<dyn FnOnce(&World) + Send>,
}

impl Element for Task {}

#[cfg(target_os = "android")]
pub struct LnAndroid(pub AndroidApp);

#[cfg(target_os = "android")]
impl Element for LnAndroid {}
