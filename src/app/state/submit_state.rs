// app/state/submit_state.rs
//later state of the project it will be used for submit cards

use crate::app::error::AppError;
use crate::app::game_state::GamePhase;
use std::{thread::sleep, time::Duration};
use tracing::{info, warn};

use crate::app::{
    bot::Bot,
    state::{State, first_main_phase_state::FirstMainPhaseState},
    cards_positions::get_card_positions,
    ui::{left_click, set_cursor_pos},
};
pub struct SubmitState {}

impl SubmitState {
    pub fn new() -> Self {
        Self {}
    }
}

impl State<AppError> for SubmitState {
    fn update(&mut self, bot: &mut Bot) -> Result<(), AppError> {
        info!("SubmitState: handling submit phase (Submit 0).");
        let positions = get_card_positions(bot.card_count, bot.screen_width as u32);
        if bot.card_count >= 4 {
            let pos = &positions[3];
            let card_y = ((bot.screen_height as f64) * 0.97).ceil() as i32;
            set_cursor_pos(pos.hover_x as i32, card_y);
            left_click();
            info!("Clicked 4th card for 'Submit 0'.");
        } else {
            warn!("Not enough cards for Submit 0 action.");
        }
        sleep(Duration::from_secs(1));
        Ok(())
    }

    fn next(&mut self) -> Box<dyn State<AppError>> {
        info!("SubmitState: transitioning to FirstMainPhaseState.");

        Box::new(FirstMainPhaseState::new())
    }
    fn phase(&self) -> GamePhase {
        GamePhase::PreCombatMain
    }
}
