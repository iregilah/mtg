// src/app/state/start_state.rs

use enigo::Key;
use crate::app::error::AppError;
use crate::app::game_state::GamePhase;
use crate::app::bot::Bot;
use crate::app::state::State;
use tracing::info;
use std::{thread::sleep, time::Duration};
use crate::multiplatform::{move_cursor, click_left, send_key};


pub struct StartState {}

impl StartState {
    pub fn new() -> Self {
        Self {}
    }
}

impl State<AppError> for StartState {
    fn update(&mut self, bot: &mut Bot) -> Result<(), AppError> {
        info!("StartState: initiating game start.");
        // Give the launcher time to settle
        sleep(Duration::from_secs(5));

        // Click the “Home” button in the launcher UI
        let (hx, hy) = bot.cords.home_button;
        info!("Clicking Home button at ({}, {})", hx, hy);
        move_cursor(hx, hy)
            .map_err(|e| AppError::Other(format!("[StartState] move_cursor(Home) failed: {}", e)))?;
        let _ = click_left();
        sleep(Duration::from_secs(1));

        // Click “Play” to launch the game
        let (px, py) = bot.cords.play_button;
        info!("Clicking Play button at ({}, {})", px, py);
        move_cursor(px, py)
            .map_err(|e| AppError::Other(format!("[StartState] move_cursor(Play) failed: {}", e)))?;
        sleep(Duration::from_millis(500));
        let _ = click_left();

        // Confirm any startup dialogs by pressing Space twice
        for i in 1..=2 {
            sleep(Duration::from_millis(500));
            info!("Pressing Space to confirm dialog ({}/2)", i);
            send_key(Key::Space)
                .map_err(|e| AppError::Other(format!("[StartState] send_key(SPACE) failed: {}", e)))?;
        }

        // Final clicks to enter the game
        for i in 1..=2 {
            sleep(Duration::from_millis(500));
            info!("Final click to advance startup flow ({}/2)", i);
            let _ = click_left();
        }

        info!("StartState: start phase completed.");
        Ok(())
    }

    fn next(&mut self) -> Box<dyn State<AppError>> {
        info!("StartState: transitioning to MulliganState.");
        Box::new(crate::app::state::mulligan_state::MulliganState::new())
    }

    fn phase(&self) -> GamePhase {
        GamePhase::Beginning
    }
}
