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

    /// Ellenőrzi a normál módú main region szöveget.
    /// Ha "Opponent's Turn" szerepel, visszaadja false értékkel, jelezve, hogy nem kell tovább menni.
    fn initial_check(&self, bot: &mut Bot) -> bool {
        let main_text = check_main_region_text(
            bot.screen_width as u32,
            bot.screen_height as u32,
            false,
        );
        info!("(Initial check) Main region text (normal): {}", main_text);
        if main_text.contains("Opponent's Turn") {
            info!("Detected 'Opponent's Turn' during initial check.");
            return false;
        }
        true
    }


    /// Újra ellenőrzi a normál módú main region szöveget, és ha "Opponent's Turn" szerepel,
    /// visszaadja false értékkel.
    fn post_cast_check(&self, bot: &mut Bot) -> bool {
        let main_text_after = check_main_region_text(
            bot.screen_width as u32,
            bot.screen_height as u32,
            false,
        );
        info!("(Post-cast check) Main region text (normal): {}", main_text_after);
        if main_text_after.contains("Opponent's Turn") {
            info!("Detected 'Opponent's Turn' after casting.");
            return false;
        }
        true
    }

    /// A red button (white_invert_image) módszert használva olvassa a main region szöveget,
    /// és kattint azokra a helyzetekre, amikor "Next" szerepel, míg "End Turn" nem jön.
    fn process_end_turn(&self, bot: &mut Bot) {
        loop {
            let main_text_red = check_main_region_text(
                bot.screen_width as u32,
                bot.screen_height as u32,
                true,
            );
            info!("(Red processing) Main region text: {}", main_text_red);
            if main_text_red.contains("Next") {
                info!("Detected 'Next' in red mode. Clicking...");
                press_key(winapi::um::winuser::VK_SPACE as u16);
                sleep(Duration::from_secs(1));
            } else if main_text_red.contains("End Turn") {
                info!("Detected 'End Turn' in red mode. Clicking to end turn.");
                press_key(winapi::um::winuser::VK_SPACE as u16);
                break;
            } else {
                sleep(Duration::from_secs(2));
            }
        }
    }

    /// Reseteli az állapotot: pl. a land_played_this_turn flag-et false-ra állítja, valamint kikapcsolja
    /// a battlefield creature-ök summoning_sickness tulajdonságát.
    fn reset_state(&self, bot: &mut Bot) {
        bot.land_played_this_turn = false;
        for card in bot.battlefield_creatures.values_mut() {
            if let Creature(ref mut creature) = card.card_type {
                creature.summoning_sickness = false;
            }
        }
    }
}