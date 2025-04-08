use crate::app::bot::Bot;
use crate::app::state::State;
use std::thread::sleep;
use std::time::Duration;
use crate::app::state::first_main_phase_state::FirstMainPhaseState;

pub struct SubmitState {}

impl SubmitState {
    pub fn new() -> Self {
        Self {}
    }
}

impl State for SubmitState {
    fn update(&mut self, bot: &mut Bot) {
        tracing::info!("SubmitState: handling submit phase (Submit 0).");
        // "Submit 0" fázis kezelése
        bot.handle_submit_phase();
        sleep(Duration::from_secs(1));
    }

    fn next(&mut self) -> Box<dyn State> {
        tracing::info!("SubmitState: transitioning to FirstMainPhaseState.");
        // Miután submitoltunk, visszalépünk az első main phase-be
        Box::new(FirstMainPhaseState::new())
    }
}
