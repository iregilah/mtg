use crate::app::bot::Bot;
use crate::app::state::State;
use std::thread::sleep;
use std::time::Duration;
use crate::app::state::attack_phase_state::AttackPhaseState;

pub struct FirstMainPhaseState {}

impl FirstMainPhaseState {
    pub fn new() -> Self {
        Self {}
    }
}

impl State for FirstMainPhaseState {
    fn update(&mut self, bot: &mut Bot) {
        tracing::info!("FirstMainPhaseState: handling first main phase.");
        // Az első main phase logikája: land kijátszás és creature castolás
        bot.handle_first_main_phase();
        sleep(Duration::from_secs(1));
    }

    fn next(&mut self) -> Box<dyn State> {
        tracing::info!("FirstMainPhaseState: transitioning to AttackPhaseState.");
        // Átmegyünk a támadási fázisra
        Box::new(AttackPhaseState::new())
    }
}
