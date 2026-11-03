//! The phone app in a phone-sized window on Windows, for development. It talks to a real host:
//!
//!     cargo run -p best-deck-mobile
//!
//! `BEST_DECK_PHONE_DATA` keeps its pairing apart; `BEST_DECK_SCREENSHOT` saves a picture and exits.

fn main() -> eframe::Result {
    let viewport = egui::ViewportBuilder::default().with_title("BestDeck (telefon)").with_inner_size([412.0, 892.0]);
    let options = eframe::NativeOptions { viewport, ..Default::default() };
    let platform = best_deck::platform::Platform::new();
    eframe::run_native("BestDeck phone", options, Box::new(move |cc| Ok(Box::new(best_deck::ui::App::new(cc, platform)))))
}
