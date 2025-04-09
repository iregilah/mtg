// state/mulligan_state.rs

use crate::app::bot::Bot;
use crate::app::state::State;
use std::thread::sleep;
use std::time::Duration;
use crate::app::ui::{set_cursor_pos, check_button_color};
use crate::app::ocr::{check_main_region_text, check_start_order_text, sanitize_ocr_text, preprocess_image};
use crate::app::state::first_main_phase_state::FirstMainPhaseState;
use crate::app::ui;


pub struct MulliganState {}

impl MulliganState {
    pub fn new() -> Self {
        Self {}
    }
}

impl State for MulliganState {
    fn update(&mut self, bot: &mut Bot) {
        tracing::info!("MulliganState: starting mulligan phase.");
        bot.time_waiting_started = std::time::Instant::now();
        tracing::info!("Waiting for Mulligan state... (Mulligan logic)");
        loop {
            let start_order_text = check_start_order_text(bot.screen_width as u32, bot.screen_height as u32);
            tracing::info!("Start order region text: {}", start_order_text);
            if start_order_text == "You Go First" || start_order_text == "Opponent Goes First" {
                if start_order_text == "Opponent Goes First" {
                    bot.card_count = 8;
                    tracing::info!("Opponent started. Card count set to 8.");
                } else {
                    bot.card_count = 7;
                    tracing::info!("We started. Card count remains 7.");
                }
                ui::press_key(winapi::um::winuser::VK_SPACE as u16);
                break;
            }
            if std::time::Instant::now().duration_since(bot.time_waiting_started) > bot.time_waiting_threshold {
                tracing::warn!("Mulligan waiting time passed. Exiting mulligan loop...");
                break;
            }
            sleep(Duration::from_secs(2));
        }
        tracing::info!("Mulligan state completed. Ready.");
        sleep(Duration::from_secs(1));

        if bot.card_count == 8 {
            tracing::info!("Opponent started; waiting for 'Next' before hovering...");
            loop {
                let is_red = check_button_color(&bot.cords) == "red";
                let main_text = check_main_region_text(bot.screen_width as u32, bot.screen_height as u32, is_red);
                if main_text.contains("Next") {
                    tracing::info!("'Next' detected; proceeding to hoovering.");
                    break;
                }
                sleep(Duration::from_secs(2));
            }
        }
        bot.examine_cards();
        let center_x = bot.screen_width / 2;
        let center_y = bot.screen_height / 2;
        set_cursor_pos(center_x, center_y);
        tracing::info!("Cursor moved to screen center: ({}, {})", center_x, center_y);
        sleep(Duration::from_secs(1));
    }

    fn next(&mut self) -> Box<dyn State> {
        tracing::info!("MulliganState: transitioning to FirstMainPhaseState.");
        Box::new(FirstMainPhaseState::new())
    }
}
