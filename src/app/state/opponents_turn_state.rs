// app/state/opponents_turn_state.rs

use std::{thread::sleep, time::Duration};
use tracing::{info};

use crate::app::{
    bot::Bot,
    state::{State, first_main_phase_state::FirstMainPhaseState},
    ui::press_key,
    ocr::check_main_region_text,
};

pub struct OpponentsTurnState {}

impl OpponentsTurnState {
    pub fn new() -> Self {
        Self {}
    }
}

impl State for OpponentsTurnState {
    fn update(&mut self, bot: &mut Bot) {
        bot.land_played_this_turn = false;
        info!("OpponentsTurnState: handling opponent's turn.");
        Self::process_opponents_turn(self, bot);

        // Mark that opponent turn has finished, so next draw should occur
        bot.last_opponent_turn = true;

        info!("Opponent turn complete, will draw next turn.");
        sleep(Duration::from_secs(1));
    }

    fn next(&mut self) -> Box<dyn State> {
        info!("OpponentsTurnState: transitioning to FirstMainPhaseState.");
        Box::new(FirstMainPhaseState::new())
    }
}

impl OpponentsTurnState {
    fn process_opponents_turn(&self, bot: &mut Bot) {
        loop {
            // 1) Read red-mode text (e.g., "My Turn", "Resolve", "Next", "Pass")
            let main_text_red = check_main_region_text(
                bot.screen_width as u32,
                bot.screen_height as u32,
                true,
            );
            info!("(Opponent turn - red) Main region text: {}", main_text_red);

            // Handle "My Turn" and "Resolve" when in red mode
            if main_text_red.contains("My Turn") {
                info!("Detected 'My Turn' in red mode. Pressing space.");
                press_key(winapi::um::winuser::VK_SPACE as u16);
            }
            if main_text_red.contains("Resolve") {
                info!("Detected 'Resolve' in red mode. Pressing space.");
                press_key(winapi::um::winuser::VK_SPACE as u16);
            }

            // Handle "Pass"
            if main_text_red.contains("Pass") {
                info!("Detected 'Pass' in non-red mode. Pressing space.");
                press_key(winapi::um::winuser::VK_SPACE as u16);
            }

            // If we see "Next", end opponent turn immediately
            if main_text_red.contains("Next") {
                info!("Detected 'Next' in red mode. Ending opponent's turn phase.");
                bot.draw_card();
                break;
            }

            /*
            // 2) Read non-red text (e.g., later)
            let main_text_non_red = check_main_region_text(
                bot.screen_width as u32,
                bot.screen_height as u32,
                false,
            );
            info!("(Opponent turn - non-red) Main region text: {}", main_text_non_red);
            */

            sleep(Duration::from_secs(2));
        }
    }
}