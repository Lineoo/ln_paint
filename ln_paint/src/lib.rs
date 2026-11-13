pub mod color;
pub mod i18n;
pub mod input;
pub mod layout;
pub mod lnwin;
pub mod paint;
pub mod render;
pub mod save;
pub mod ui;
pub mod unit;

pub fn desktop_main() {
    use winit::event_loop::EventLoop;

    env_logger::init();

    log::info!("This is LnPaint. Welcome!");

    let lnwin = lnwin::Lnwin::default();
    let event_loop = EventLoop::builder().build().unwrap();
    event_loop.run_app(lnwin).unwrap();
}

#[cfg(target_os = "android")]
#[unsafe(no_mangle)]
pub fn android_main(app: winit::platform::android::activity::AndroidApp) {
    use android_logger::{Config, FilterBuilder};
    use ln_world::World;
    use winit::{
        event_loop::EventLoop,
        platform::android::{EventLoopBuilderExtAndroid, activity::WindowManagerFlags},
    };

    android_logger::init_once(
        Config::default()
            .with_max_level(log::LevelFilter::Debug)
            .with_filter(
                FilterBuilder::new()
                    .filter(None, log::LevelFilter::Debug)
                    .filter(Some("naga"), log::LevelFilter::Warn)
                    .filter(Some("wgpu"), log::LevelFilter::Warn)
                    .build(),
            )
            .with_tag("ln_paint"),
    );

    log::info!("This is LnPaint Mobile. Welcome!");

    app.set_window_flags(WindowManagerFlags::FULLSCREEN, WindowManagerFlags::empty());

    let mut world = World::default();
    world.insert(lnwin::LnAndroid(app.clone()));
    world.flush();

    let lnwin = lnwin::Lnwin {
        world,
        ..Default::default()
    };
    let event_loop = EventLoop::builder().with_android_app(app).build().unwrap();
    event_loop.run_app(lnwin).unwrap();
}
