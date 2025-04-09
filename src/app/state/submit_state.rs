use crate::app::bot::Bot;
use crate::app::state::State;
use std::thread::sleep;
use std::time::Duration;
use crate::app::{cards_positions, ui};
use crate::app::cards_positions::get_card_positions;
use crate::app::state::first_main_phase_state::FirstMainPhaseState;
use crate::app::ui::{left_click, set_cursor_pos};

pub struct SubmitState {}

impl SubmitState {
    pub fn new() -> Self {
        Self {}
    }
}

impl State for SubmitState {
    fn update(&mut self, bot: &mut Bot) {
        tracing::info!("SubmitState: handling submit phase (Submit 0).");
        let positions = get_card_positions(bot.card_count, bot.screen_width as u32);
        if bot.card_count >= 4 {
            let pos = &positions[3];
            let card_y = ((bot.screen_height as f64) * 0.97).ceil() as i32;
            set_cursor_pos(pos.hover_x as i32, card_y);
            left_click();
            tracing::info!("Clicked 4th card for 'Submit 0'.");
        } else {
            tracing::warn!("Not enough cards for Submit 0 action.");
        }
        sleep(Duration::from_secs(1));
    }

    fn next(&mut self) -> Box<dyn State> {
        tracing::info!("SubmitState: transitioning to FirstMainPhaseState.");
        // Miután submitoltunk, visszalépünk az első main phase-be
        Box::new(FirstMainPhaseState::new())
    }
}
