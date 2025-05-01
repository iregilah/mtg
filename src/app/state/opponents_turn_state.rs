// app/state/opponents_turn_state.rs

use crate::app::error::AppError;
use crate::app::game_state::GamePhase;
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

impl State<AppError> for OpponentsTurnState {
    fn update(&mut self, bot: &mut Bot) -> Result<(), AppError> {
        bot.on_turn_end();
        info!("OpponentsTurnState: handling opponent's turn.");
        Self::process_opponents_turn(self, bot);

        // Mark that opponent turn has finished, so next draw should occur
        bot.last_opponent_turn = true;

        info!("Opponent turn complete, will draw next turn.");
        sleep(Duration::from_secs(1));
        Ok(())
    }

    fn next(&mut self) -> Box<dyn State<AppError>> {
        info!("OpponentsTurnState: transitioning to FirstMainPhaseState.");
        Box::new(FirstMainPhaseState::new())
    }
    fn phase(&self) -> GamePhase {
        GamePhase::End
    }
}

impl OpponentsTurnState {
    fn process_opponents_turn(&self, bot: &mut Bot) {
        loop {
            // read the red‐mode “button” region
            let txt = check_main_region_text(
                bot.screen_width as u32,
                bot.screen_height as u32,
                true,
            );
            info!("(Opponent turn – red) Main region text: {}", txt);

            // whenever we see “My Turn”, “Resolve” or “Pass”, hit space
            if txt.contains("My Turn")
                || txt.contains("Resolve")
                || txt.contains("Pass")
            {
                info!("Detected prompt (‘My Turn’/‘Resolve’/‘Pass’). Pressing Space.");
                press_key(0x20);
            }

            // end the opponent’s turn when “Next” appears
            if txt.contains("Next") {
                info!("Detected ‘Next’. Ending opponent’s turn.");
                bot.draw_card();
                break;
            }

            sleep(Duration::from_secs(2));
        }
    }
}