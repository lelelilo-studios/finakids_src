//! Finakids — a 3D life simulation about money, decisions and consequences.

#![allow(deprecated)]

pub mod app;
pub mod audio;
pub mod character;
pub mod game;
pub mod gfx;
pub mod input;
pub mod math;
pub mod platform;
pub mod sdf;
pub mod ui;
pub mod world;

pub use game::GameConfig;

/// Desktop / iOS entry point.
pub fn run() {
    run_config(GameConfig::default());
}

pub fn run_config(cfg: GameConfig) {
    platform::init_logger();
    let event_loop = winit::event_loop::EventLoop::<app::UserEvent>::with_user_event()
        .build()
        .expect("event loop");
    app::run_with(event_loop, cfg);
}

#[cfg(target_os = "android")]
#[no_mangle]
fn android_main(android: winit::platform::android::activity::AndroidApp) {
    use winit::platform::android::EventLoopBuilderExtAndroid;
    android_logger::init_once(android_logger::Config::default().with_max_level(log::LevelFilter::Info));
    let mut cfg = GameConfig::default();
    cfg.save_dir = android.internal_data_path().map(|p| p.to_string_lossy().to_string());
    let event_loop = winit::event_loop::EventLoop::<app::UserEvent>::with_user_event()
        .with_android_app(android)
        .build()
        .expect("event loop");
    app::run_with(event_loop, cfg);
}

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen::prelude::wasm_bindgen(start)]
pub fn wasm_main() {
    let mut cfg = GameConfig::default();
    cfg.apply_url_query();
    run_config(cfg);
}
