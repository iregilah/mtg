// src/app/state/start_state.rs

use enigo::Key;
use crate::app::error::AppError;
use crate::app::game_state::GamePhase;
use crate::app::bot::Bot;
use crate::app::state::State;
use tracing::info;
use std::{thread::sleep, time::Duration};
use crate::multiplatform::{move_cursor, click_left};

pub struct StartState;

impl StartState {
    pub fn new() -> Self {
        Self
    }
}

impl State<AppError> for StartState {
    fn update(&mut self, bot: &mut Bot) -> Result<(), AppError> {
        info!("StartState: initiating game start.");
        // Allow launcher to settle
        sleep(Duration::from_secs(5));

        // Click the “Home” button in the launcher UI
        let (hx, hy) = bot.cords.home_button;
        info!("StartState: clicking Home button at ({}, {})", hx, hy);
        move_cursor(hx, hy)
            .map_err(|e| AppError::Other(format!("[StartState] move_cursor(Home) failed: {}", e)))?;
        click_left()
            .map_err(|e| AppError::Other(format!("[StartState] click_left(Home) failed: {}", e)))?;
        sleep(Duration::from_secs(1));

        // Click 'Play' button twice to enter game
        let (px, py) = bot.cords.play_button;
        for i in 0..=2 {
            info!("StartState: clicking Play button ({}/2) at ({}, {})", i, px, py);
            move_cursor(px, py)
                .map_err(|e| AppError::Other(format!("[StartState] move_cursor(Play) failed: {}", e)))?;
            click_left()
                .map_err(|e| AppError::Other(format!("[StartState] click_left(Play) failed: {}", e)))?;
            sleep(Duration::from_millis(500));
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