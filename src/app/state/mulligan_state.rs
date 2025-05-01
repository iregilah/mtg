// app/state/mulligan_state.rs

use crate::app::error::AppError;
use crate::app::game_state::GamePhase;
use std::{thread::sleep, time::Duration};
use std::time::Instant;
use tracing::{warn, info};

use crate::app::{
    bot::Bot,
    state::{State, first_main_phase_state::FirstMainPhaseState},
    ui::{set_cursor_pos, check_button_color, press_key},
    ocr::{check_main_region_text, check_start_order_text},
};

pub struct MulliganState {}

impl MulliganState {
    pub fn new() -> Self {
        Self {}
    }
}

impl State<AppError> for MulliganState {
    fn update(&mut self, bot: &mut Bot) -> Result<(), AppError> {
        info!("MulliganState: starting mulligan phase.");
        Self::wait_for_start_order(self, bot);
        info!("Mulligan selection done.");

        // short pause before moving on
        sleep(Duration::from_secs(1));

        Self::wait_for_next_for_hover(self, bot);
        Self::move_cursor_and_examine_cards(self, bot);

        Ok(())
    }

    fn next(&mut self) -> Box<dyn State<AppError>> {
        info!("MulliganState: transitioning to FirstMainPhaseState.");
        Box::new(FirstMainPhaseState::new())
    }
    fn phase(&self) -> GamePhase {
        GamePhase::Beginning
    }
}

impl MulliganState {
    /// Wait until "You Go First" / "Opponent Goes First" appears or timeout.
    fn wait_for_start_order(&self, bot: &mut Bot) {
        bot.time_waiting_started = Instant::now();
        loop {
            let txt = check_start_order_text(
                bot.screen_width as u32,
                bot.screen_height as u32,
            );
            info!("Start order region text: {}", txt);

            match txt.as_str() {
                "Opponent Goes First" => {
                    bot.card_count = 8;
                    info!("Opponent starts; setting card_count = 8.");
                    press_key(0x20);
                    break;
                }
                "You Go First" => {
                    bot.card_count = 7;
                    info!("We start; keeping card_count = 7.");
                    press_key(0x20);
                    break;
                }
                _ if Instant::now().duration_since(bot.time_waiting_started) > bot.time_waiting_threshold => {
                    warn!("Mulligan timeout; exiting start-order loop.");
                    break;
                }
                _ => {
                    sleep(Duration::from_secs(2));
                }
            }
        }
    }


    /// If opponent started (8 cards), wait for “Next” before we hover cards.
    fn wait_for_next_for_hover(&self, bot: &mut Bot) {
        if bot.card_count == 8 {
            info!("Opponent started; waiting for 'Next' to hover cards.");
            loop {

                let is_red = check_button_color(&bot.cords) == "red";
                let txt = check_main_region_text(
                    bot.screen_width as u32,
                    bot.screen_height as u32,
                    is_red,
                );

                if txt.contains("Next") {
                    info!("Detected 'Next'; proceeding to hover cards.");
                    break;
                }
                sleep(Duration::from_secs(2));
            }
        }
    }

    /// Examine all cards (OCR) then center the cursor.
    fn move_cursor_and_examine_cards(&self, bot: &mut Bot) {
        bot.examine_cards();
        let (cx, cy) = (bot.screen_width / 2, bot.screen_height / 2);
        set_cursor_pos(cx, cy);
        info!("Cursor centered at ({}, {})", cx, cy);
        sleep(Duration::from_secs(1));
    }
}
