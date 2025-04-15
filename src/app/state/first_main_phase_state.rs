use crate::app::bot::Bot;
use crate::app::state::State;
use std::thread::sleep;
use std::time::Duration;
use crate::app::card_library::{CardType, CREATURE_NAMES, LAND_NAMES};
use crate::app::cards_positions::get_card_positions;
use crate::app::ui::{set_cursor_pos, left_click, press_key};
use crate::app::state::attack_phase_state::AttackPhaseState;
use crate::app::ui;
use crate::app::creature_positions::{get_own_creature_positions, get_opponent_creature_positions};

pub struct FirstMainPhaseState {}

impl State for FirstMainPhaseState {
    fn update(&mut self, bot: &mut Bot) {
        tracing::info!("FirstMainPhaseState: handling first main phase.");
        // 1. Play land if not already played during this turn
        bot.play_land();
        tracing::info!("Available mana after playing land: {}", bot.land_number);

        // 2. Update battlefield creatures with the OCR-based method
        Bot::update_battlefield_creatures_from_ocr(bot);

        // 3. Decide what to cast
        if bot.battlefield_creatures.len() > 0 {
            tracing::info!("Creature(s) found on battlefield – casting an instant.");
            let mana_left = bot.cast_instants();
            tracing::info!("Main phase finished (instant cast). Remaining mana: {}.", mana_left);
        } else {
            tracing::info!("No valid creature on battlefield – casting a creature.");
            let mana_left = bot.cast_creatures();
            tracing::info!("Main phase finished (creature cast). Remaining mana: {}.", mana_left);
        }
    }


    fn next(&mut self) -> Box<dyn State> {
        tracing::info!("FirstMainPhaseState: transitioning to AttackPhaseState.");
        Box::new(AttackPhaseState::new())
    }
}

impl FirstMainPhaseState {
    pub fn new() -> Self {
        Self {}
    }
}