// app/state/mulligan_state.rs

use crate::app::error::AppError;
use crate::app::game_state::GamePhase;
use std::{thread::sleep, time::Duration};
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
        bot.time_waiting_started = std::time::Instant::now();
        info!("Waiting for Mulligan state... (Mulligan logic)");
        Self::wait_for_start_order(self, bot);
        info!("Mulligan state completed. Ready.");
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
    fn wait_for_start_order(&self, bot: &mut Bot) {
        loop {
            let start_order_text = check_start_order_text(bot.screen_width as u32, bot.screen_height as u32);
            info!("Start order region text: {}", start_order_text);
            if start_order_text == "You Go First" || start_order_text == "Opponent Goes First" {
                if start_order_text == "Opponent Goes First" {
                    bot.card_count = 8;
                    info!("Opponent started. Card count set to 8.");
                } else {
                    bot.card_count = 7;
                    info!("We started. Card count remains 7.");
                }
                press_key(winapi::um::winuser::VK_SPACE as u16);
                break;
            }
            if std::time::Instant::now().duration_since(bot.time_waiting_started) > bot.time_waiting_threshold {
                warn!("Mulligan waiting time passed. Exiting mulligan loop...");
                break;
            }
            sleep(Duration::from_secs(2));
        }
    }

    fn wait_for_next_for_hover(&self, bot: &mut Bot) {
        if bot.card_count == 8 {
            info!("Opponent started; waiting for 'Next' before hovering...");
            loop {
                let is_red = check_button_color(&bot.cords) == "red";
                let main_text = check_main_region_text(bot.screen_width as u32, bot.screen_height as u32, is_red);
                if main_text.contains("Next") {
                    info!("'Next' detected; proceeding to hoovering.");
                    break;
                }
                sleep(Duration::from_secs(2));
            }
        }
    }

    fn move_cursor_and_examine_cards(&self, bot: &mut Bot) {
        bot.examine_cards();
        let center_x = bot.screen_width / 2;
        let center_y = bot.screen_height / 2;
        set_cursor_pos(center_x, center_y);
        info!("Cursor moved to screen center: ({}, {})", center_x, center_y);
        sleep(Duration::from_secs(1));
    }
}
