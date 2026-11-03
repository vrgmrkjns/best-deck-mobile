//! The deck: folder tabs, keys and volume sliders laid out on the folder's grid.

use std::time::{Duration, Instant};

use egui::text::{LayoutJob, TextFormat, TextWrapping};
use egui::{Align2, Color32, CornerRadius, Rect, RichText, Sense, Stroke, Ui, pos2, vec2};

use super::theme::*;
use super::{App, Drag, notice, outlined_button};
use crate::protocol::{DeckFolder, DeckKey, KeyState};
use crate::updater::Update;

const GAP: f32 = 10.0;
/// About twenty slider updates a second feels live without flooding the host.
const SLIDE_EVERY: Duration = Duration::from_millis(50);

impl App {
    pub(super) fn deck_screen(&mut self, ui: &mut Ui) {
        let (profile, online, folders, error) = {
            let model = self.connection.model();
            (model.profile.clone(), model.online, model.folders.clone(), model.error.clone())
        };
        let index = self.folder.min(folders.len().saturating_sub(1));
        self.folder = index;
        egui::Frame::new().inner_margin(egui::Margin::symmetric(16, 12)).show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.spacing_mut().item_spacing.y = 0.0;
            // The folder on show is the title; folders are opened with folder keys.
            let title = folders.get(index).map_or(profile.as_str(), |f| f.name.as_str());
            self.header(ui, title, online);
            ui.add_space(12.0);
            self.update_banner(ui);
            let text = if !online { Some("Nincs kapcsolat a géppel. Újrapróbálom.".to_owned()) } else { error };
            if let Some(text) = text {
                notice(ui, &text);
                ui.add_space(12.0);
            }
            match folders.get(index) {
                Some(folder) => self.grid(ui, folder, online),
                None => {
                    ui.add_space(ui.available_height() / 3.0);
                    ui.vertical_centered(|ui| {
                        ui.label(RichText::new("Ez a deck még üres. Adj hozzá gombokat a gépen, a szerkesztőben.").color(MUTE));
                    });
                }
            }
        });
        if self.sheet {
            self.host_sheet(ui, online);
        }
    }

    fn header(&mut self, ui: &mut Ui, title: &str, online: bool) {
        let (rect, _) = ui.allocate_exact_size(vec2(ui.available_width(), 44.0), Sense::hover());
        let status = if online { "Csatlakozva a géphez" } else { "Nincs kapcsolat" };
        let chip = ui.painter().layout_no_wrap(status.to_owned(), font(13.0), BODY);
        let chip_rect = Rect::from_min_max(pos2(rect.right() - chip.size().x - 34.0, rect.top()), rect.right_bottom());
        let response = ui.interact(chip_rect, ui.make_persistent_id("host-chip"), Sense::click());
        if response.is_pointer_button_down_on() {
            ui.painter().rect_filled(chip_rect, 8.0, KEY_TOP);
        }
        ui.painter().circle_filled(pos2(chip_rect.left() + 14.0, rect.center().y), 4.0, if online { ONLINE } else { ALERT_INK });
        ui.painter().galley(pos2(chip_rect.left() + 26.0, rect.center().y - chip.size().y / 2.0), chip, BODY);
        if response.clicked() {
            self.sheet = true;
        }
        let title = one_line(ui, title, strong_font(24.0), INK, chip_rect.left() - rect.left() - 12.0);
        ui.painter().galley(pos2(rect.left(), rect.center().y - title.size().y / 2.0), title, INK);
    }

    fn update_banner(&mut self, ui: &mut Ui) {
        let state = self.updater.state();
        let (release, progress, error) = match &state {
            Update::None => return,
            Update::Available(release) => (release, None, None),
            Update::Downloading(release, progress) => (release, Some(*progress), None),
            Update::Installing(release) => (release, Some(1.0), None),
            Update::Failed(release, error) => (release, None, Some(error.clone())),
        };
        let mut start = false;
        egui::Frame::new()
            .fill(SURFACE)
            .stroke(Stroke::new(1.0, LINE))
            .corner_radius(CornerRadius::same(12))
            .inner_margin(egui::Margin { left: 14, right: 8, top: 2, bottom: 2 })
            .show(ui, |ui| {
                ui.set_width(ui.available_width());
                // One 44-tall row: the message on the left, the action on the right, both centred.
                let (row, _) = ui.allocate_exact_size(vec2(ui.available_width(), 44.0), Sense::hover());
                let text = match &state {
                    Update::Downloading(_, p) => format!("Letöltés: {}%", (p * 100.0).round()),
                    Update::Installing(_) => "A telepítő megnyílt".to_owned(),
                    _ => format!("Új verzió a gépen: {}", release.version_name),
                };
                let enabled = progress.is_none() || matches!(state, Update::Installing(_));
                let label = if matches!(state, Update::Installing(_)) { "Újra" } else { "Frissítés" };
                let action = ui.painter().layout_no_wrap(label.to_owned(), strong_font(14.0), INK);
                let button = Rect::from_min_max(pos2(row.right() - action.size().x - 24.0, row.top()), row.right_bottom());
                let response = ui.interact(button, ui.make_persistent_id("update-action"), if enabled { Sense::click() } else { Sense::hover() });
                ui.painter().galley_with_override_text_color(button.center() - action.size() / 2.0, action, if enabled { INK } else { MUTE });
                let message = one_line(ui, &text, strong_font(14.0), INK, button.left() - row.left() - 8.0);
                ui.painter().galley(pos2(row.left(), row.center().y - message.size().y / 2.0), message, INK);
                if enabled && response.clicked() {
                    start = true;
                }
                if let Some(progress) = progress.filter(|_| matches!(state, Update::Downloading(..))) {
                    let (rect, _) = ui.allocate_exact_size(vec2(ui.available_width() - 6.0, 3.0), Sense::hover());
                    ui.painter().rect_filled(rect, 1.5, LINE);
                    ui.painter().rect_filled(Rect::from_min_size(rect.min, vec2(rect.width() * progress, 3.0)), 1.5, INK);
                    ui.add_space(4.0);
                }
                if let Some(error) = error {
                    ui.label(RichText::new(error).size(13.0).color(ALERT_INK));
                }
            });
        ui.add_space(12.0);
        if start {
            self.updater.start(self.platform.clone(), self.waker());
        }
    }

    /// Lays the keys out on the folder's grid, sized so every row fits without scrolling when it can.
    fn grid(&mut self, ui: &mut Ui, folder: &DeckFolder, online: bool) {
        let columns = f32::from(folder.columns.max(1));
        let rows = f32::from(folder.rows.max(1));
        let width = ui.available_width();
        let cell_width = (width - GAP * (columns - 1.0)) / columns;
        let fitted = (ui.available_height() - GAP * (rows - 1.0)) / rows;
        // Never squeeze a key below what its icon and label need; scroll instead.
        let cell_height = fitted.min(cell_width * 1.15).max(72.0);
        let height = cell_height * rows + GAP * (rows - 1.0);
        let states = self.connection.model().states.clone();
        let levels = self.connection.model().levels.clone();

        egui::ScrollArea::vertical().auto_shrink(false).show(ui, |ui| {
            let (area, _) = ui.allocate_exact_size(vec2(width, height), Sense::hover());
            for key in &folder.keys {
                let span = f32::from(key.span());
                let rect = Rect::from_min_size(
                    area.min + vec2(f32::from(key.col) * (cell_width + GAP), f32::from(key.row) * (cell_height + GAP)),
                    vec2(cell_width * span + GAP * (span - 1.0), cell_height),
                );
                if key.is_slider() {
                    self.slider(ui, rect, key, levels.get(&key.id).copied(), online);
                } else {
                    let state = key.lit_when.as_ref().and_then(|name| states.get(name));
                    self.key(ui, rect, key, state, online);
                }
            }
        });
    }

    /// One key of the deck. Sinks the moment a finger lands; fires when it lifts.
    fn key(&mut self, ui: &mut Ui, rect: Rect, key: &DeckKey, state: Option<&KeyState>, online: bool) {
        // Moving between folders happens here on the phone, so it works offline too.
        let navigates = key.open_folder.is_some() || key.back;
        let usable = (online && key.available) || navigates;
        let id = ui.make_persistent_id(("key", &key.id));
        let response = ui.interact(rect, id, if usable { Sense::click() } else { Sense::hover() });
        let down = usable && response.is_pointer_button_down_on();
        let scale = ui.ctx().animate_bool_with_time(id.with("down"), down, 0.12);
        let rect = Rect::from_center_size(rect.center(), rect.size() * (1.0 - 0.05 * scale));

        // A key that opens the folder on show lights, like a tab.
        let lit = state.is_some_and(|s| s.lit) || key.open_folder == Some(self.folder);
        let color = group(&key.group);
        let fade = |c: Color32| if usable { c } else { c.gamma_multiply(0.42) };
        let (fill, border) = if lit { (mix(color, 0.40, KEY_TOP), color) } else { (KEY_FACE, LINE) };
        let painter = ui.painter();
        painter.rect(rect, 14.0, fade(fill), Stroke::new(1.0, fade(border)), egui::StrokeKind::Inside);
        // The slight top light of a physical key.
        painter.line_segment([rect.left_top() + vec2(14.0, 1.0), rect.right_top() + vec2(-14.0, 1.0)], Stroke::new(1.0, Color32::from_white_alpha(16)));

        let label = match (lit, state.and_then(|s| s.label.clone()), &key.lit_label) {
            (true, Some(label), _) => label,
            (true, None, Some(label)) => label.clone(),
            _ => key.label.clone(),
        };
        let icon_name = if lit { key.lit_icon.as_deref().unwrap_or(&key.icon) } else { &key.icon };
        let ink = if lit { Color32::WHITE } else { BODY };
        let font_id = if lit { strong_font(12.0) } else { font(12.0) };
        let galley = one_line(ui, &label, font_id, fade(ink), rect.width() - 12.0);
        let content = 28.0 + 8.0 + galley.size().y;
        let top = rect.center().y - content / 2.0;
        icon(icon_name, fade(if lit { Color32::WHITE } else { color }), 28.0)
            .paint_at(ui, Rect::from_center_size(pos2(rect.center().x, top + 14.0), vec2(28.0, 28.0)));
        ui.painter().galley(pos2(rect.center().x - galley.size().x / 2.0, top + 36.0), galley, fade(ink));

        if response.clicked() {
            self.platform.tick();
            self.connection.press(&key.id);
            if let Some(target) = key.open_folder.filter(|&target| target != self.folder) {
                self.history.push(self.folder);
                self.folder = target;
            } else if key.back {
                self.folder = self.history.pop().unwrap_or(0);
            }
        }
    }

    /// A volume tile that is the slider itself: the level follows the finger 1:1.
    fn slider(&mut self, ui: &mut Ui, rect: Rect, key: &DeckKey, level: Option<u8>, online: bool) {
        let id = ui.make_persistent_id(("slider", &key.id));
        let response = ui.interact(rect, id, if online { Sense::click_and_drag() } else { Sense::hover() });
        let at_pointer = |pos: egui::Pos2| (((pos.x - rect.left()) / rect.width()) * 100.0).round().clamp(0.0, 100.0) as u8;

        if let Some(pos) = response.interact_pointer_pos().filter(|_| response.dragged() || response.drag_started()) {
            let value = at_pointer(pos);
            let drag = self.sliders.entry(key.id.clone()).or_insert(Drag { value, sent: Instant::now() - SLIDE_EVERY });
            if value != drag.value && value % 10 == 0 {
                self.platform.tick();
            }
            drag.value = value;
            if drag.sent.elapsed() >= SLIDE_EVERY {
                drag.sent = Instant::now();
                self.connection.slide(&key.id, value);
            }
        }
        if response.drag_stopped() {
            if let Some(drag) = self.sliders.remove(&key.id) {
                self.connection.slide(&key.id, drag.value);
            }
        }
        if response.clicked() {
            if let Some(pos) = response.interact_pointer_pos() {
                self.connection.slide(&key.id, at_pointer(pos));
            }
        }

        let dragging = self.sliders.get(&key.id).map(|d| d.value);
        let shown = dragging.or(level);
        let value = f32::from(shown.unwrap_or(0));
        let color = group(if key.group.is_empty() { "media" } else { &key.group });
        let fade = |c: Color32| if online { c } else { c.gamma_multiply(0.42) };
        let painter = ui.painter();
        painter.rect_filled(rect, 14.0, fade(KEY_BOTTOM));
        let filled = rect.width() * value / 100.0;
        if filled > 0.5 {
            // Round the far end only once the fill reaches the tile's own corner.
            let far = if rect.width() - filled < 14.0 { 14 } else { 0 };
            let radius = CornerRadius { nw: 14, sw: 14, ne: far, se: far };
            painter.rect_filled(Rect::from_min_size(rect.min, vec2(filled, rect.height())), radius, fade(mix(color, 0.28, KEY_BOTTOM)));
        }
        painter.rect_stroke(rect, 14.0, Stroke::new(1.0, fade(if dragging.is_some() { color } else { LINE })), egui::StrokeKind::Inside);
        let thumb_x = (rect.left() + filled).clamp(rect.left() + 8.0, rect.right() - 8.0);
        painter.rect_filled(Rect::from_center_size(pos2(thumb_x, rect.center().y), vec2(4.0, 40.0_f32.min(rect.height() - 16.0))), 2.0, fade(INK));

        let inner = rect.shrink2(vec2(14.0, 10.0));
        let label = one_line(ui, &key.label, font(12.0), fade(BODY), inner.width());
        ui.painter().galley(inner.left_top(), label, fade(BODY));
        let number = shown.map_or("-".to_owned(), |v| v.to_string());
        ui.painter().text(inner.left_bottom(), Align2::LEFT_BOTTOM, number, figures(22.0), fade(INK));
    }

    fn host_sheet(&mut self, ui: &mut Ui, online: bool) {
        let ctx = ui.ctx().clone();
        let screen = ctx.content_rect();
        let mut close = false;
        egui::Area::new(egui::Id::unique("best-deck host scrim")).order(egui::Order::Foreground).fixed_pos(screen.min).show(&ctx, |ui| {
            let (rect, response) = ui.allocate_exact_size(screen.size(), Sense::click());
            ui.painter().rect_filled(rect, 0.0, Color32::from_black_alpha(150));
            if response.clicked() {
                close = true;
            }
        });
        let (_, bottom) = self.platform.insets();
        let bottom = bottom / ctx.pixels_per_point();
        let height = 190.0 + bottom;
        let sheet = Rect::from_min_max(pos2(screen.left(), screen.bottom() - height), screen.right_bottom());
        egui::Area::new(egui::Id::unique("best-deck host sheet")).order(egui::Order::Tooltip).fixed_pos(sheet.min).show(&ctx, |ui| {
            ui.set_width(sheet.width());
            egui::Frame::new()
                .fill(SURFACE)
                .corner_radius(CornerRadius { nw: 20, ne: 20, sw: 0, se: 0 })
                .inner_margin(egui::Margin { left: 20, right: 20, top: 12, bottom: 20 + bottom as i8 })
                .show(ui, |ui| {
                    ui.set_width(sheet.width() - 40.0);
                    ui.spacing_mut().item_spacing.y = 0.0;
                    ui.vertical_centered(|ui| {
                        let (handle, _) = ui.allocate_exact_size(vec2(32.0, 4.0), Sense::hover());
                        ui.painter().rect_filled(handle, 2.0, LINE_STRONG);
                    });
                    ui.add_space(18.0);
                    let (heading, line) = if online {
                        ("Csatlakozva a géphez", "A gombok ezen a gépen futnak.")
                    } else {
                        ("Nincs kapcsolat a géppel", "Újrapróbálom; addig a gombok szürkék.")
                    };
                    ui.label(RichText::new(heading).font(strong_font(17.0)).color(INK));
                    ui.add_space(4.0);
                    ui.label(RichText::new(line).color(MUTE));
                    ui.add_space(20.0);
                    if outlined_button(ui, "Másik gép választása").clicked() {
                        self.sheet = false;
                        self.typing = false;
                        self.folder = 0;
                        self.history.clear();
                        self.connection.forget();
                    }
                });
        });
        if close {
            self.sheet = false;
        }
    }
}

/// One line of text cut with an ellipsis to `width`.
fn one_line(ui: &Ui, text: &str, font: egui::FontId, color: Color32, width: f32) -> std::sync::Arc<egui::Galley> {
    let mut job = LayoutJob::single_section(text.to_owned(), TextFormat::simple(font, color));
    job.wrap = TextWrapping { max_width: width.max(1.0), max_rows: 1, break_anywhere: true, overflow_character: Some('…') };
    ui.painter().layout_job(job)
}
