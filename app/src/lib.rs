//! The phone side of best-deck: a grid of keys that run actions on your PC, over the local network.

pub mod connection;
pub mod discovery;
pub mod icons;
pub mod platform;
pub mod protocol;
pub mod store;
pub mod ui;
pub mod updater;

/// Android starts here, on its own thread, through NativeActivity.
#[cfg(target_os = "android")]
#[unsafe(no_mangle)]
fn android_main(app: winit::platform::android::activity::AndroidApp) {
    android_logger::init_once(android_logger::Config::default().with_max_level(log::LevelFilter::Info).with_tag("best-deck"));
    let platform = platform::Platform::new(app.clone());
    let options = eframe::NativeOptions { android_app: Some(app), ..Default::default() };
    if let Err(error) = eframe::run_native("BestDeck", options, Box::new(move |cc| Ok(Box::new(ui::App::new(cc, platform))))) {
        log::error!("BestDeck stopped: {error}");
    }
}
