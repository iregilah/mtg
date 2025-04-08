use crate::app::bot::Bot;
use crate::app::state::State;
use std::thread::sleep;
use std::time::Duration;
use crate::app::state::second_main_phase_state::SecondMainPhaseState;

pub struct AttackPhaseState {}

impl AttackPhaseState {
    pub fn new() -> Self {
        Self {}
    }
}

impl State for AttackPhaseState {
    fn update(&mut self, bot: &mut Bot) {
        tracing::info!("AttackPhaseState: starting attack phase.");
        // Támadási fázis
        bot.handle_attack_phase();
        sleep(Duration::from_secs(1));
    }

    fn next(&mut self) -> Box<dyn State> {
        tracing::info!("AttackPhaseState: transitioning to SecondMainPhaseState.");
        // Átmegyünk a második main phase-re
        Box::new(SecondMainPhaseState::new())
    }
}
