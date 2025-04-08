use crate::app::bot::Bot;
use crate::app::state::State;
use std::thread::sleep;
use std::time::Duration;
use crate::app::state::first_main_phase_state::FirstMainPhaseState;

pub struct MulliganState {}

impl MulliganState {
    pub fn new() -> Self {
        Self {}
    }
}

impl State for MulliganState {
    fn update(&mut self, bot: &mut Bot) {
        tracing::info!("MulliganState: starting mulligan phase.");
        // Mulligan logika: várakozás a start order szövegre, majd a kártyák beolvasása
        bot.loading();
        bot.examine_cards();
        sleep(Duration::from_secs(1));
    }

    fn next(&mut self) -> Box<dyn State> {
        tracing::info!("MulliganState: transitioning to FirstMainPhaseState.");
        // Átmegyünk az első main phase-re
        Box::new(FirstMainPhaseState::new())
    }
}
