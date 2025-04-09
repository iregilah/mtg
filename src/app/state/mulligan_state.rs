use crate::app::bot::Bot;
use crate::app::state::State;
use std::thread::sleep;
use std::time::Duration;
use crate::app::state::first_main_phase_state::FirstMainPhaseState;
use crate::app::{ocr, ui};

pub struct MulliganState {}

impl MulliganState {
    pub fn new() -> Self {
        Self {}
    }
}

impl State for MulliganState {
    fn update(&mut self, bot: &mut Bot) {
        tracing::info!("MulliganState: starting mulligan phase.");
        bot.loading();
        if bot.card_count == 8 {
            tracing::info!("Opponent started; waiting for 'Next' before hovering...");
            loop {
                let is_red = ui::check_button_color(&bot.cords) == "red";
                let main_text = ocr::check_main_region_text(bot.screen_width as u32, bot.screen_height as u32, is_red);
                if main_text.contains("Next") {
                    tracing::info!("'Next' detected; proceeding to hoovering.");
                    break;
                }
                std::thread::sleep(Duration::from_secs(2));
            }
        }
        bot.examine_cards();
        {
            let center_x = bot.screen_width / 2;
            let center_y = bot.screen_height / 2;
            ui::set_cursor_pos(center_x, center_y);
            tracing::info!("Cursor moved to screen center: ({}, {})", center_x, center_y);
        }
        std::thread::sleep(Duration::from_secs(1));
    }

    fn next(&mut self) -> Box<dyn State> {
        tracing::info!("MulliganState: transitioning to FirstMainPhaseState.");
        // Átmegyünk az első main phase-re
        Box::new(FirstMainPhaseState::new())
    }
}
