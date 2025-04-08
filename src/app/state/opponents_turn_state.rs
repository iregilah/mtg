use crate::app::bot::Bot;
use crate::app::state::State;
use std::thread::sleep;
use std::time::Duration;
use crate::app::state::first_main_phase_state::FirstMainPhaseState;

pub struct OpponentsTurnState {}

impl OpponentsTurnState {
    pub fn new() -> Self {
        Self {}
    }
}

impl State for OpponentsTurnState {
    fn update(&mut self, bot: &mut Bot) {
        tracing::info!("OpponentsTurnState: handling opponent's turn.");
        // Ellenfél körének kezelése
        bot.handle_opponents_turn();
        sleep(Duration::from_secs(1));
    }

    fn next(&mut self) -> Box<dyn State> {
        tracing::info!("OpponentsTurnState: transitioning to FirstMainPhaseState.");
        // Ha vége az ellenfél körének, visszalépünk az első main phase-be
        Box::new(FirstMainPhaseState::new())
    }
}
