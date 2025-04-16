use crate::app::bot::Bot;
use crate::app::state::State;
use std::thread::sleep;
use std::time::Duration;
use crate::app::ocr::check_main_region_text;
use crate::app::ui::press_key;
use crate::app::state::start_state::StartState;
use crate::app::card_library::{CardType, CREATURE_NAMES, LAND_NAMES};
use crate::app::card_library::CardType::Creature;
use crate::app::state::opponents_turn_state::OpponentsTurnState;


pub struct SecondMainPhaseState {}


impl State for SecondMainPhaseState {
    fn update(&mut self, bot: &mut Bot) {
        tracing::info!("SecondMainPhaseState: handling second main phase.");

        // 1. Első ellenőrzés: normál feldolgozás
        if !self.initial_check(bot) {
            return;
        }

        // 2. Creature castolási akciók végrehajtása
        bot.cast_creatures();

        // 3. Új ellenőrzés: ha most már "Opponent's Turn" szerepel a szövegben, kilépünk
        if !self.post_cast_check(bot) {
            return;
        }

        // 4. End Turn folyamat: red button feldolgozással
        self.process_end_turn(bot);

        // 5. Állapot reset
        self.reset_state(bot);
    }


    fn next(&mut self) -> Box<dyn State> {
        tracing::info!("SecondMainPhaseState: transitioning to new round (StartState).");
        Box::new(OpponentsTurnState::new())
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
        tracing::info!("(Initial check) Main region text (normal): {}", main_text);
        if main_text.contains("Opponent's Turn") {
            tracing::info!("Detected 'Opponent's Turn' during initial check.");
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
        tracing::info!(
            "(Post-cast check) Main region text (normal): {}",
            main_text_after
        );
        if main_text_after.contains("Opponent's Turn") {
            tracing::info!("Detected 'Opponent's Turn' after casting.");
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
            tracing::info!("(Red processing) Main region text: {}", main_text_red);
            if main_text_red.contains("Next") {
                tracing::info!("Detected 'Next' in red mode. Clicking...");
                press_key(winapi::um::winuser::VK_SPACE as u16);
                sleep(Duration::from_secs(1));
            } else if main_text_red.contains("End Turn") {
                tracing::info!("Detected 'End Turn' in red mode. Clicking to end turn.");
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
        for card in &mut bot.battlefield_creatures {
            if let Creature(ref mut creature) = card.card_type {
                creature.summoning_sickness = false;
            }
        }
    }
}