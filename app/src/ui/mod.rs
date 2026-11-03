//! The app's two screens: choosing and pairing a PC, and the deck itself.

mod connect;
mod deck;
pub mod theme;

use std::collections::HashMap;
use std::time::Instant;

use egui::{Align2, Color32, CornerRadius, Rect, Sense, Stroke, Ui, Vec2, vec2};

use crate::connection::{Connection, Phase};
use crate::discovery::Finder;
use crate::platform::Platform;
use crate::store::Store;
use crate::updater::Updater;
use theme::*;

/// A finger held on a slider; while it is down it wins over what the host reports.
struct Drag {
    value: u8,
    sent: Instant,
}

pub struct App {
    platform: Platform,
    connection: Connection,
    finder: Finder,
    updater: Updater,
    ctx: egui::Context,
    /// [`crate::connection::Model::links`] when updates were last looked for.
    checked_links: u64,
    screen_kept_on: bool,
    folder: usize,
    /// The sheet about the connected PC is open.
    sheet: bool,
    /// The address being typed on the keypad.
    address: String,
    /// The keypad for typing an address is open.
    typing: bool,
    code: String,
    sliders: HashMap<String, Drag>,
    /// Development aid, see [`App::screenshot_step`].
    screenshot: Option<(Instant, bool)>,
}

impl App {
    pub fn new(cc: &eframe::CreationContext<'_>, platform: Platform) -> Self {
        theme::setup(&cc.egui_ctx);
        egui_extras::install_image_loaders(&cc.egui_ctx);
        let ctx = cc.egui_ctx.clone();
        let waker = ctx.clone();
        let store = Store::open(&platform.data_dir());
        let mut connection = Connection::new(store, platform.device_name(), move || waker.request_repaint());
        connection.restore();
        let address = connection.model().host.clone().unwrap_or_default();
        App {
            platform,
            connection,
            finder: Finder::default(),
            updater: Updater::new(),
            ctx,
            checked_links: 0,
            screen_kept_on: false,
            folder: 0,
            sheet: false,
            address,
            typing: false,
            code: String::new(),
            sliders: HashMap::new(),
            screenshot: None,
        }
    }

    fn waker(&self) -> impl Fn() + Send + Sync + 'static {
        let ctx = self.ctx.clone();
        move || ctx.request_repaint()
    }

    /// Keeps the background helpers in step with the screen on show.
    fn housekeeping(&mut self) {
        let (phase, online, links, host) = {
            let model = self.connection.model();
            (model.phase, model.online, model.links, model.host.clone())
        };
        let ready = phase == Phase::Ready;
        if ready {
            self.finder.stop();
        } else if !self.finder.is_running() {
            self.finder.start(self.waker());
        }
        // A deck that dims after thirty seconds is useless next to a keyboard; once the app is in
        // the background the screen may sleep again.
        let shown = self.ctx.input(|i| i.viewport().focused).unwrap_or(true);
        let keep = ready && shown;
        if keep != self.screen_kept_on {
            self.platform.keep_screen_on(keep);
            self.screen_kept_on = keep;
        }
        // Ask the host for a newer app each time the link comes up.
        if ready && online && links != self.checked_links {
            self.checked_links = links;
            if let Some(host) = host {
                self.updater.check(&host, self.waker());
            }
        }
    }

    /// Development aid: with `BEST_DECK_SCREENSHOT=<file.png>` the app saves a picture of itself
    /// after `BEST_DECK_SCREENSHOT_AFTER` milliseconds (default 2500) and closes.
    fn screenshot_step(&mut self, ctx: &egui::Context) {
        let Some(path) = std::env::var_os("BEST_DECK_SCREENSHOT") else { return };
        let (started, requested) = *self.screenshot.get_or_insert((Instant::now(), false));
        let after = std::env::var("BEST_DECK_SCREENSHOT_AFTER").ok().and_then(|ms| ms.parse().ok()).unwrap_or(2500);
        if !requested && started.elapsed().as_millis() >= after {
            self.screenshot = Some((started, true));
            let waker = ctx.clone();
            ctx.request_screenshot(move |image| {
                let pixels: Vec<u8> = image.pixels.iter().flat_map(|c| c.to_array()).collect();
                if let Some(picture) = image::RgbaImage::from_raw(image.size[0] as u32, image.size[1] as u32, pixels) {
                    let _ = picture.save(std::path::Path::new(&path));
                }
                waker.send_viewport_cmd(egui::ViewportCommand::Close);
            });
        }
        ctx.request_repaint_after(std::time::Duration::from_millis(100));
    }
}

impl eframe::App for App {
    fn ui(&mut self, ui: &mut Ui, _frame: &mut eframe::Frame) {
        self.housekeeping();
        let (top, bottom) = self.platform.insets();
        let scale = ui.ctx().pixels_per_point();
        let frame = egui::Frame::new().fill(CANVAS).inner_margin(egui::Margin {
            left: 0,
            right: 0,
            top: (top / scale) as i8,
            bottom: (bottom / scale) as i8,
        });
        egui::CentralPanel::no_frame().frame(frame).show(ui, |ui| {
            if self.connection.model().phase == Phase::Ready {
                self.deck_screen(ui);
            } else {
                self.connect_screen(ui);
            }
        });
        self.screenshot_step(&ui.ctx().clone());
    }
}

/// The one filled button per screen: white with dark text, 48 tall.
fn filled_button(ui: &mut Ui, text: &str, enabled: bool) -> egui::Response {
    let width = ui.available_width();
    let (rect, response) = ui.allocate_exact_size(vec2(width, 48.0), if enabled { Sense::click() } else { Sense::hover() });
    let pressed = enabled && response.is_pointer_button_down_on();
    let fill = match (enabled, pressed) {
        (false, _) => Color32::from_rgb(0x3A, 0x3B, 0x3F),
        (true, true) => Color32::from_rgb(0xD9, 0xDA, 0xDD),
        (true, false) => Color32::WHITE,
    };
    ui.painter().rect_filled(rect, 10.0, fill);
    let ink = if enabled { CANVAS } else { MUTE };
    ui.painter().text(rect.center(), Align2::CENTER_CENTER, text, strong_font(15.0), ink);
    response
}

/// A secondary action: a 48-tall outline.
fn outlined_button(ui: &mut Ui, text: &str) -> egui::Response {
    let width = ui.available_width();
    let (rect, response) = ui.allocate_exact_size(vec2(width, 48.0), Sense::click());
    let fill = if response.is_pointer_button_down_on() { KEY_TOP } else { Color32::TRANSPARENT };
    ui.painter().rect(rect, 10.0, fill, Stroke::new(1.0, LINE_STRONG), egui::StrokeKind::Inside);
    ui.painter().text(rect.center(), Align2::CENTER_CENTER, text, strong_font(15.0), INK);
    response
}

/// A quiet text action, 44 tall so a thumb finds it.
fn text_button(ui: &mut Ui, text: &str, color: Color32) -> egui::Response {
    let galley = ui.painter().layout_no_wrap(text.to_owned(), font(14.0), color);
    let (rect, response) = ui.allocate_exact_size(vec2(galley.size().x + 24.0, 44.0), Sense::click());
    let color = if response.is_pointer_button_down_on() { INK } else { color };
    ui.painter().galley_with_override_text_color(rect.center() - galley.size() / 2.0, galley, color);
    response
}

/// A keypad for addresses and codes, so the app never needs the system keyboard.
/// `extra` are the keys beside 0 (such as `.` and `:`); returns what was tapped.
fn keypad(ui: &mut Ui, extra: [Option<char>; 2]) -> Option<Key> {
    let gap = 8.0;
    let width = ui.available_width();
    let cell = vec2((width - gap * 2.0) / 3.0, 52.0);
    let rows: [[Option<Key>; 3]; 4] = [
        [Some(Key::Char('1')), Some(Key::Char('2')), Some(Key::Char('3'))],
        [Some(Key::Char('4')), Some(Key::Char('5')), Some(Key::Char('6'))],
        [Some(Key::Char('7')), Some(Key::Char('8')), Some(Key::Char('9'))],
        [extra[0].map(Key::Char), Some(Key::Char('0')), Some(extra[1].map_or(Key::Back, Key::Char))],
    ];
    let (area, _) = ui.allocate_exact_size(vec2(width, cell.y * 4.0 + gap * 3.0), Sense::hover());
    let mut tapped = None;
    for (r, row) in rows.iter().enumerate() {
        for (c, key) in row.iter().enumerate() {
            let Some(key) = key else { continue };
            let rect = Rect::from_min_size(area.min + vec2(c as f32 * (cell.x + gap), r as f32 * (cell.y + gap)), cell);
            let response = ui.interact(rect, ui.make_persistent_id(("keypad", r, c)), Sense::click());
            let fill = if response.is_pointer_button_down_on() { KEY_TOP } else { SURFACE };
            ui.painter().rect(rect, 12.0, fill, Stroke::new(1.0, LINE), egui::StrokeKind::Inside);
            match key {
                Key::Char(ch) => {
                    ui.painter().text(rect.center(), Align2::CENTER_CENTER, ch.to_string(), font(22.0), INK);
                }
                Key::Back => backspace(BODY, 22.0).paint_at(ui, Rect::from_center_size(rect.center(), Vec2::splat(22.0))),
            }
            if response.clicked() {
                tapped = Some(*key);
            }
        }
    }
    // With both extras taken, backspace gets its own wide key below.
    if extra[1].is_some() {
        ui.add_space(gap);
        let (rect, response) = ui.allocate_exact_size(vec2(width, 44.0), Sense::click());
        let fill = if response.is_pointer_button_down_on() { KEY_TOP } else { SURFACE };
        ui.painter().rect(rect, 12.0, fill, Stroke::new(1.0, LINE), egui::StrokeKind::Inside);
        backspace(BODY, 20.0).paint_at(ui, Rect::from_center_size(rect.center(), Vec2::splat(20.0)));
        if response.clicked() {
            tapped = Some(Key::Back);
        }
    }
    tapped
}

#[derive(Clone, Copy, Debug, PartialEq)]
enum Key {
    Char(char),
    Back,
}

/// A rounded notice in the alert colours.
fn notice(ui: &mut Ui, text: &str) {
    egui::Frame::new().fill(ALERT_SURFACE).corner_radius(CornerRadius::same(12)).inner_margin(egui::Margin::symmetric(14, 12)).show(ui, |ui| {
        ui.set_width(ui.available_width());
        ui.label(egui::RichText::new(text).size(13.0).color(ALERT_INK));
    });
}
