use crate::app::ui::press_key;
use crate::app::bot::Bot;
use crate::app::state::State;
use std::thread::sleep;
use std::time::Duration;
use crate::app::ocr::check_main_region_text;
use crate::app::state::first_main_phase_state::FirstMainPhaseState;
use crate::app::ui::check_button_color;

pub struct OpponentsTurnState {}

impl OpponentsTurnState {
    pub fn new() -> Self {
        Self {}
    }
}

impl State for OpponentsTurnState {
    fn update(&mut self, bot: &mut Bot) {
        bot.land_played_this_turn = false; // máshová kéne majd tenni később
        tracing::info!("OpponentsTurnState: handling opponent's turn.");
        Self::process_opponents_turn(self, bot);
        sleep(Duration::from_secs(1));
    }

    fn next(&mut self) -> Box<dyn State> {
        tracing::info!("OpponentsTurnState: transitioning to FirstMainPhaseState.");
        // Ha vége az ellenfél körének, visszalépünk az első main phase-be
        Box::new(FirstMainPhaseState::new())
    }
}

impl OpponentsTurnState {
    fn process_opponents_turn(&self, bot: &mut Bot) {
        loop {
            // Első olvasás: red mode
            let main_text_red = check_main_region_text(
                bot.screen_width as u32,
                bot.screen_height as u32,
                true
            );
            tracing::info!("(Opponent turn - red) Main region text: {}", main_text_red);
            if main_text_red.contains("My Turn") {
                tracing::info!("Detected 'My Turn' in red mode. Pressing space.");
                press_key(winapi::um::winuser::VK_SPACE as u16);
            }

            // Második olvasás: nem red mode
            let main_text_non_red = check_main_region_text(
                bot.screen_width as u32,
                bot.screen_height as u32,
                false
            );
            tracing::info!("(Opponent turn - non-red) Main region text: {}", main_text_non_red);
            if main_text_non_red.contains("Pass") {
                tracing::info!("Detected 'Pass' in non-red mode. Pressing space.");
                press_key(winapi::um::winuser::VK_SPACE as u16);
            }
            if main_text_non_red.contains("Next") {
                tracing::info!("Detected 'Next' in non-red mode. Opponent turn phase finished.");
                break;
            }
            sleep(Duration::from_secs(2));
        }
    }
}