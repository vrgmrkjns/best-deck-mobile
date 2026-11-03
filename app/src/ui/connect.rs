//! Choosing a host and, the first time, typing its pairing code.

use egui::{Align2, Color32, Rect, RichText, Sense, Stroke, Ui, pos2, vec2};

use super::theme::*;
use super::{App, Key, filled_button, keypad, text_button};
use crate::connection::Phase;

const CODE_LENGTH: usize = 6;

impl App {
    pub(super) fn connect_screen(&mut self, ui: &mut Ui) {
        let (phase, error, host) = {
            let model = self.connection.model();
            (model.phase, model.error.clone(), model.host.clone())
        };
        let pairing = phase == Phase::Pairing;
        egui::ScrollArea::vertical().auto_shrink(false).show(ui, |ui| {
            egui::Frame::new().inner_margin(egui::Margin { left: 20, right: 20, top: 32, bottom: 24 }).show(ui, |ui| {
                ui.set_width(ui.available_width());
                ui.spacing_mut().item_spacing.y = 0.0;
                let title = if pairing { "Párosítás" } else { "Melyik géphez?" };
                ui.label(RichText::new(title).font(strong_font(30.0)).color(INK));
                ui.add_space(8.0);
                let lead = if pairing {
                    "Írd be a hat számjegyet, amit a gépen a BestDeck szerkesztő Eszközök füle mutat. Egy gépet csak egyszer kell párosítani."
                } else {
                    "Indítsd el a BestDecket a gépen. A telefon és a gép legyen ugyanazon a Wi-Fi-n, így a telefon magától megtalálja."
                };
                ui.label(RichText::new(lead).size(15.0).color(MUTE));
                ui.add_space(28.0);

                if pairing {
                    self.pairing(ui, host.as_deref().unwrap_or(""));
                } else {
                    self.choosing(ui, phase, host.as_deref(), error.is_some());
                }
                if let Some(error) = error {
                    ui.add_space(16.0);
                    ui.label(RichText::new(error).size(14.0).color(ALERT_INK));
                }
                ui.add_space(40.0);
                ui.label(RichText::new("Nincs fiók és nincs felhő. A telefon csak a helyi hálózaton beszél a géppel.").size(13.0).color(MUTE));
            });
        });
    }

    fn choosing(&mut self, ui: &mut Ui, phase: Phase, host: Option<&str>, failed: bool) {
        let connecting = phase == Phase::Connecting && !failed;
        let found = self.finder.found();
        if !self.typing {
            label(ui, "A hálózaton");
            ui.add_space(8.0);
            if found.is_empty() {
                searching(ui);
            }
            for pc in &found {
                let busy = connecting && host == Some(pc.host.as_str());
                if pc_row(ui, &pc.name, &pc.host, busy).clicked() && !busy {
                    self.address = pc.host.clone();
                    self.connection.connect(&pc.host);
                }
                ui.add_space(8.0);
            }
            ui.add_space(8.0);
            if text_button(ui, "Cím megadása kézzel", BODY).clicked() {
                self.typing = true;
            }
            return;
        }

        label(ui, "A gép címe");
        ui.add_space(8.0);
        field(ui, &self.address, "192.168.0.110:8191", true);
        ui.add_space(12.0);
        match keypad(ui, [Some('.'), Some(':')]) {
            Some(Key::Char(ch)) if self.address.len() < 40 => self.address.push(ch),
            Some(Key::Back) => {
                self.address.pop();
            }
            _ => {}
        }
        ui.add_space(12.0);
        let text = if connecting { "Csatlakozás folyamatban" } else { "Csatlakozás" };
        if filled_button(ui, text, !connecting && !self.address.trim().is_empty()).clicked() {
            let address = self.address.clone();
            self.connection.connect(&address);
        }
        ui.add_space(4.0);
        if text_button(ui, "Vissza a talált gépekhez", BODY).clicked() {
            self.typing = false;
        }
    }

    fn pairing(&mut self, ui: &mut Ui, host: &str) {
        label(ui, "Párosító kód");
        ui.add_space(8.0);
        code_boxes(ui, &self.code);
        ui.add_space(12.0);
        let mut submit = false;
        match keypad(ui, [None, None]) {
            Some(Key::Char(ch)) if self.code.len() < CODE_LENGTH => {
                self.code.push(ch);
                // The sixth digit sends it; nothing else is left to type.
                submit = self.code.len() == CODE_LENGTH;
            }
            Some(Key::Back) => {
                self.code.pop();
            }
            _ => {}
        }
        ui.add_space(12.0);
        if filled_button(ui, "Párosítás", self.code.len() == CODE_LENGTH).clicked() {
            submit = true;
        }
        if submit {
            self.connection.pair(&self.code);
        }
        ui.add_space(4.0);
        if text_button(ui, &format!("Másik gép: {host}"), BODY).clicked() {
            self.code.clear();
            self.connection.forget();
        }
    }
}

fn label(ui: &mut Ui, text: &str) {
    ui.label(RichText::new(text).font(strong_font(15.0)).color(INK));
}

/// A field showing what the keypad typed, with a caret.
fn field(ui: &mut Ui, text: &str, hint: &str, focused: bool) {
    let (rect, _) = ui.allocate_exact_size(vec2(ui.available_width(), 52.0), Sense::hover());
    let border = if focused { INK } else { LINE };
    ui.painter().rect(rect, 12.0, SURFACE, Stroke::new(1.0, border), egui::StrokeKind::Inside);
    let left = pos2(rect.left() + 14.0, rect.center().y);
    if text.is_empty() {
        ui.painter().text(left, Align2::LEFT_CENTER, hint, font(16.0), HINT);
        if focused {
            ui.painter().vline(left.x, (left.y - 10.0)..=(left.y + 10.0), Stroke::new(2.0, INK));
        }
    } else {
        let galley = ui.painter().layout_no_wrap(text.to_owned(), font(16.0), INK);
        let width = galley.size().x;
        ui.painter().galley(pos2(left.x, left.y - galley.size().y / 2.0), galley, INK);
        if focused {
            let x = left.x + width + 2.0;
            ui.painter().vline(x, (left.y - 10.0)..=(left.y + 10.0), Stroke::new(2.0, INK));
        }
    }
}

/// Six boxes for the pairing code; the next one to fill is outlined.
fn code_boxes(ui: &mut Ui, code: &str) {
    let gap = 8.0;
    let width = (ui.available_width() - gap * (CODE_LENGTH as f32 - 1.0)) / CODE_LENGTH as f32;
    let (area, _) = ui.allocate_exact_size(vec2(ui.available_width(), 60.0), Sense::hover());
    let digits: Vec<char> = code.chars().collect();
    for i in 0..CODE_LENGTH {
        let rect = Rect::from_min_size(area.min + vec2(i as f32 * (width + gap), 0.0), vec2(width, 60.0));
        let border = if i == digits.len() { INK } else { LINE };
        ui.painter().rect(rect, 12.0, SURFACE, Stroke::new(1.0, border), egui::StrokeKind::Inside);
        if let Some(digit) = digits.get(i) {
            ui.painter().text(rect.center(), Align2::CENTER_CENTER, digit.to_string(), figures(28.0), INK);
        }
    }
}

/// A PC found on the network.
fn pc_row(ui: &mut Ui, name: &str, host: &str, busy: bool) -> egui::Response {
    let (rect, response) = ui.allocate_exact_size(vec2(ui.available_width(), 64.0), Sense::click());
    let fill = if response.is_pointer_button_down_on() { KEY_TOP } else { SURFACE };
    ui.painter().rect(rect, 12.0, fill, Stroke::new(1.0, LINE), egui::StrokeKind::Inside);
    icon("monitor", BODY, 22.0).paint_at(ui, Rect::from_center_size(pos2(rect.left() + 28.0, rect.center().y), vec2(22.0, 22.0)));
    ui.painter().text(pos2(rect.left() + 52.0, rect.center().y - 9.0), Align2::LEFT_CENTER, name, strong_font(15.0), INK);
    ui.painter().text(pos2(rect.left() + 52.0, rect.center().y + 11.0), Align2::LEFT_CENTER, host, font(13.0), MUTE);
    let action = if busy { "Csatlakozás…" } else { "Csatlakozás" };
    ui.painter().text(pos2(rect.right() - 16.0, rect.center().y), Align2::RIGHT_CENTER, action, font(13.0), if busy { MUTE } else { BODY });
    response
}

/// Shown while no PC has answered yet.
fn searching(ui: &mut Ui) {
    let (rect, _) = ui.allocate_exact_size(vec2(ui.available_width(), 64.0), Sense::hover());
    ui.painter().rect(rect, 12.0, Color32::TRANSPARENT, Stroke::new(1.0, LINE), egui::StrokeKind::Inside);
    // Three dots taking turns, so it is clear the app is still listening.
    let time = ui.input(|i| i.time);
    for i in 0..3 {
        let phase = ((time * 2.0 - f64::from(i) * 0.35).rem_euclid(1.5) < 0.5) as u8;
        let color = if phase == 1 { BODY } else { LINE_STRONG };
        ui.painter().circle_filled(pos2(rect.left() + 22.0 + i as f32 * 10.0, rect.center().y), 3.0, color);
    }
    ui.painter().text(pos2(rect.left() + 58.0, rect.center().y), Align2::LEFT_CENTER, "Keresem a gépet a hálózaton", font(14.0), MUTE);
    ui.ctx().request_repaint_after(std::time::Duration::from_millis(120));
}
