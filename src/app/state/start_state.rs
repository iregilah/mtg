use crate::app::bot::Bot;
use crate::app::state::State;
use std::thread::sleep;
use std::time::Duration;
use crate::app::state::mulligan_state::MulliganState;

pub struct StartState {}

impl StartState {
    pub fn new() -> Self {
        Self {}
    }
}

impl State for StartState {
    fn update(&mut self, bot: &mut Bot) {
        tracing::info!("StartState: initiating game start.");
        // Végrehajtjuk az ablakok fókuszálását és a start gombok műveletét
        bot.start_game();
        sleep(Duration::from_secs(1));
    }

    fn next(&mut self) -> Box<dyn State> {
        tracing::info!("StartState: transitioning to MulliganState.");
        // Átmegyünk a mulligan fázisra
        Box::new(MulliganState::new())
    }
}
