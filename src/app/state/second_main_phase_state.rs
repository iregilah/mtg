// app/state/second_main_phase_state.rs

use crate::app::error::AppError;
use crate::app::game_state::GamePhase;
use std::{thread::sleep, time::Duration};
use tracing::{info};

use crate::app::{
    bot::Bot,
    state::{State, opponents_turn_state::OpponentsTurnState},
    ui::press_key,
    ocr::check_main_region_text,
    card_library::CardType::Creature,
};

pub struct SecondMainPhaseState {}


impl State<AppError> for SecondMainPhaseState {
    fn update(&mut self, bot: &mut Bot) -> Result<(), AppError> {
        info!("SecondMainPhaseState: handling second main phase.");

        // 1. Első ellenőrzés: normál feldolgozás
        if !self.initial_check(bot) {
            return Ok(());
        }

        // 2. Creature castolási akciók végrehajtása
        bot.cast_creatures();

        // 3. Új ellenőrzés: ha most már "Opponent's Turn" szerepel a szövegben, kilépünk
        if !self.post_cast_check(bot) {
            return Ok(());
        }

        // 4. End Turn folyamat: red button feldolgozással
        self.process_end_turn(bot);

        // 5. Állapot reset
        self.reset_state(bot);
        Ok(())
    }


    fn next(&mut self) -> Box<dyn State<AppError>> {
        info!("SecondMainPhaseState: transitioning to new round (StartState).");
        Box::new(OpponentsTurnState::new())
    }
    fn phase(&self) -> GamePhase {
        GamePhase::PostCombatMain
    }
}



impl SecondMainPhaseState {
    pub fn new() -> Self {
        Self {}
    }

    /// 1) Read the non-red “main” region once before any casts.
    ///    If we already see “Opponent's Turn”, bail out early.
    fn initial_check(&self, bot: &mut Bot) -> bool {
        let txt = check_main_region_text(
            bot.screen_width as u32,
            bot.screen_height as u32,
            false,
        );
        info!("(Initial check) Main region text: {}", txt);
        if txt.contains("Opponent's Turn") {
            info!("Detected 'Opponent's Turn' on entry; skipping second main.");
            return false;
        }
        true
    }

    /// 3) After casting creatures, check again for “Opponent's Turn”.
    fn post_cast_check(&self, bot: &mut Bot) -> bool {
        let txt = check_main_region_text(
            bot.screen_width as u32,
            bot.screen_height as u32,
            false,
        );
        info!("(Post-cast check) Main region text: {}", txt);
        if txt.contains("Opponent's Turn") {
            info!("Detected 'Opponent's Turn' after casting; exiting.");
            return false;
        }
        true
    }

    /// 4) Loop in red-mode until we see “End Turn”, tapping space on “Next” or “End Turn”.
    fn process_end_turn(&self, bot: &mut Bot) {
        loop {
            let txt = check_main_region_text(
                bot.screen_width as u32,
                bot.screen_height as u32,
                true,
            );
            info!("(End-turn red) Main region text: {}", txt);

            if txt.contains("Next") {
                info!("Detected 'Next'; pressing Space to advance.");
                press_key(0x20);
                sleep(Duration::from_secs(1));
                continue;
            }

            if txt.contains("End Turn") {
                info!("Detected 'End Turn'; pressing Space to finish.");
                press_key(0x20);
                break;
            }

            // otherwise wait and retry
            sleep(Duration::from_secs(2));
        }
    }


    /// 5) Reset per-turn flags & clear summoning sickness.
    fn reset_state(&self, bot: &mut Bot) {
        bot.land_played_this_turn = false;
        for card in bot.battlefield_creatures.values_mut() {
            if let Creature(ref mut cr) = card.card_type {
                cr.summoning_sickness = false;
            }
        }
    }
}