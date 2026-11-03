//! The deck's palette: monochrome chrome, colour only on the keys.

use std::collections::HashSet;
use std::path::PathBuf;
use std::sync::{Mutex, OnceLock};

use egui::epaint::text::{FontInsert, FontPriority, InsertFontFamily};
use egui::{Color32, FontFamily, FontId, Image, Stroke, Vec2};

use crate::icons;

pub const CANVAS: Color32 = Color32::from_rgb(0x0A, 0x0B, 0x0D);
pub const SURFACE: Color32 = Color32::from_rgb(0x11, 0x12, 0x14);
pub const KEY_TOP: Color32 = Color32::from_rgb(0x1C, 0x1E, 0x21);
pub const KEY_BOTTOM: Color32 = Color32::from_rgb(0x14, 0x15, 0x18);
pub const KEY_FACE: Color32 = Color32::from_rgb(0x18, 0x1A, 0x1D);
pub const LINE: Color32 = Color32::from_rgb(0x26, 0x28, 0x2C);
pub const LINE_STRONG: Color32 = Color32::from_rgb(0x33, 0x36, 0x3B);
pub const INK: Color32 = Color32::from_rgb(0xF2, 0xF3, 0xF5);
pub const BODY: Color32 = Color32::from_rgb(0xC9, 0xCB, 0xD1);
pub const MUTE: Color32 = Color32::from_rgb(0x9A, 0x9C, 0xA3);
pub const HINT: Color32 = Color32::from_rgb(0x6E, 0x70, 0x78);
pub const ONLINE: Color32 = Color32::from_rgb(0x4C, 0xC3, 0x8A);
pub const ALERT_SURFACE: Color32 = Color32::from_rgb(0x2A, 0x15, 0x16);
pub const ALERT_INK: Color32 = Color32::from_rgb(0xFF, 0xB4, 0xB4);

pub fn group(name: &str) -> Color32 {
    match name {
        "voice" => Color32::from_rgb(0x6E, 0x9B, 0xFF),
        "capture" => Color32::from_rgb(0xFF, 0x6B, 0x6B),
        "media" => Color32::from_rgb(0x4C, 0xC3, 0x8A),
        "game" => Color32::from_rgb(0xF2, 0xB2, 0x3E),
        _ => MUTE,
    }
}

/// `amount` of `top` laid over `base`.
pub fn mix(top: Color32, amount: f32, base: Color32) -> Color32 {
    let channel = |a: u8, b: u8| (f32::from(a) * amount + f32::from(b) * (1.0 - amount)).round() as u8;
    Color32::from_rgb(channel(top.r(), base.r()), channel(top.g(), base.g()), channel(top.b(), base.b()))
}

/// Pictures added as icons on the PC: downloaded once into the app's files, then drawn from there.
struct Pictures {
    dir: PathBuf,
    /// `host:port` of the PC to fetch from.
    host: Mutex<Option<String>>,
    /// Files being fetched, or that failed a moment ago.
    asked: Mutex<HashSet<String>>,
    waker: Box<dyn Fn() + Send + Sync>,
}

static PICTURES: OnceLock<Pictures> = OnceLock::new();

/// Where downloaded pictures go, and how to ask for a repaint once one arrives.
pub fn init_pictures(dir: PathBuf, waker: impl Fn() + Send + Sync + 'static) {
    let _ = PICTURES.set(Pictures { dir, host: Mutex::new(None), asked: Mutex::default(), waker: Box::new(waker) });
}

/// The PC new pictures are fetched from.
pub fn set_picture_host(host: Option<String>) {
    if let Some(pictures) = PICTURES.get() {
        *pictures.host.lock().unwrap() = host;
    }
}

/// The local file of a custom icon, fetching it in the background when it is missing.
fn picture(file: &str) -> Option<PathBuf> {
    let pictures = PICTURES.get()?;
    let path = pictures.dir.join(file);
    if path.exists() {
        return Some(path);
    }
    let host = pictures.host.lock().unwrap().clone()?;
    if !pictures.asked.lock().unwrap().insert(file.to_owned()) {
        return None;
    }
    let file = file.to_owned();
    std::thread::spawn(move || {
        let fetched = ureq::get(format!("http://{host}/icons/{file}"))
            .call()
            .ok()
            .and_then(|mut response| response.body_mut().with_config().limit(6 * 1024 * 1024).read_to_vec().ok());
        let pictures = PICTURES.get().expect("set before any fetch");
        match fetched {
            Some(bytes) => {
                let _ = std::fs::create_dir_all(&pictures.dir);
                // Written aside first, so a half file is never drawn.
                let temp = pictures.dir.join(format!("{file}.part"));
                if std::fs::write(&temp, bytes).is_ok() {
                    let _ = std::fs::rename(&temp, pictures.dir.join(&file));
                }
                (pictures.waker)();
            }
            None => {
                // Try again a little later, not on every frame.
                std::thread::sleep(std::time::Duration::from_secs(15));
            }
        }
        pictures.asked.lock().unwrap().remove(&file);
    });
    None
}

/// A key icon of `size` points in `color`. Built-in icons are rendered once in white and tinted;
/// a picture added on the PC keeps its own colours and only fades with a faded key.
pub fn icon(name: &str, color: Color32, size: f32) -> Image<'static> {
    if let Some(path) = icons::custom_file(name).and_then(picture) {
        let alpha = f32::from(color.a()) / 255.0;
        let uri = format!("file:///{}", path.to_string_lossy().replace('\\', "/").trim_start_matches('/'));
        return Image::new(uri).tint(Color32::WHITE.gamma_multiply(alpha)).fit_to_exact_size(Vec2::splat(size));
    }
    let name = if name.starts_with(icons::CUSTOM) { "dot" } else { name };
    Image::from_bytes(format!("bytes://icon/{name}.svg"), icons::svg(name, "#FFFFFF").into_bytes())
        .tint(color)
        .fit_to_exact_size(Vec2::splat(size))
}

/// The backspace glyph of the keypad, in the icons' stroke.
pub fn backspace(color: Color32, size: f32) -> Image<'static> {
    let svg = r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="#FFFFFF" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M21 5H9l-6 7 6 7h12z"/><path d="M17 9l-5 6M12 9l5 6"/></svg>"##;
    Image::from_bytes("bytes://glyph/backspace.svg", svg.as_bytes()).tint(color).fit_to_exact_size(Vec2::splat(size))
}

pub fn strong() -> FontFamily {
    FontFamily::Name("strong".into())
}

pub fn font(size: f32) -> FontId {
    FontId::new(size, FontFamily::Proportional)
}

pub fn strong_font(size: f32) -> FontId {
    FontId::new(size, strong())
}

/// Figures that line up, for codes and levels.
pub fn figures(size: f32) -> FontId {
    FontId::monospace(size)
}

fn system_font(names: &[&str]) -> Option<Vec<u8>> {
    let dirs: Vec<std::path::PathBuf> = if cfg!(target_os = "android") {
        vec!["/system/fonts".into()]
    } else {
        std::env::var_os("WINDIR").map(|dir| std::path::PathBuf::from(dir).join("Fonts")).into_iter().collect()
    };
    dirs.iter().flat_map(|dir| names.iter().map(move |name| dir.join(name))).find_map(|path| std::fs::read(path).ok())
}

pub fn setup(ctx: &egui::Context) {
    // The phone's own Roboto (Segoe UI on the development build); egui's fonts stay as the fallback.
    let add = |name: &str, data: Option<Vec<u8>>, family: FontFamily| {
        if let Some(data) = data {
            ctx.add_font(FontInsert::new(name, egui::FontData::from_owned(data), vec![InsertFontFamily { family, priority: FontPriority::Highest }]));
        }
    };
    add("regular", system_font(&["Roboto-Regular.ttf", "segoeui.ttf"]), FontFamily::Proportional);
    add("strong", system_font(&["Roboto-Medium.ttf", "Roboto-Regular.ttf", "seguisb.ttf"]), strong());
    add("figures", system_font(&["RobotoMono-Regular.ttf", "DroidSansMono.ttf", "consola.ttf"]), FontFamily::Monospace);

    ctx.set_theme(egui::Theme::Dark);
    ctx.all_styles_mut(|style| {
        style.spacing.item_spacing = egui::vec2(8.0, 8.0);
        style.spacing.scroll.floating = true;
        style.spacing.scroll.bar_width = 4.0;
        // A finger moves a little while tapping; don't take that for a drag.
        style.interaction.interact_radius = 0.0;
        let visuals = &mut style.visuals;
        *visuals = egui::Visuals::dark();
        visuals.override_text_color = Some(INK);
        visuals.panel_fill = CANVAS;
        visuals.window_fill = SURFACE;
        visuals.extreme_bg_color = SURFACE;
        visuals.selection.bg_fill = INK;
        visuals.selection.stroke = Stroke::new(1.0, CANVAS);
    });
}
