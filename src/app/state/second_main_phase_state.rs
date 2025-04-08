use crate::app::bot::Bot;
use crate::app::state::State;
use std::thread::sleep;
use std::time::Duration;
use crate::app::state::start_state::StartState;

pub struct SecondMainPhaseState {}

impl SecondMainPhaseState {
    pub fn new() -> Self {
        Self {}
    }
}

impl State for SecondMainPhaseState {
    fn update(&mut self, bot: &mut Bot) {
        tracing::info!("SecondMainPhaseState: handling second main phase and end turn.");
        // Második main phase: End Turn kezelése
        bot.second_main_phase();
        sleep(Duration::from_secs(1));
        // Reseteljük a per-turn állapotokat
        bot.land_played_this_turn = false;
        for creature in &mut bot.battlefield_creatures {
            creature.summoning_sickness = false;
        }
    }

    fn next(&mut self) -> Box<dyn State> {
        tracing::info!("SecondMainPhaseState: transitioning to new round (StartState).");
        // Új kör kezdődik – visszalépünk a start state-re
        Box::new(StartState::new())
    }
}
