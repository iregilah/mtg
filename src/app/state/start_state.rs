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


///creature read test
/*
/// Ez a függvény soha nem tér vissza (-> !), végtelen ciklusban olvassa a battlefieldet.
fn battlefield_debug_loop(bot: &mut Bot) -> ! {
    loop {
        // Frissítjük (OCR) a battlefieldet, és kiírjuk a tartalmát
        bot.refresh_battlefield();

        info!("===> Saját Battlefield:");
        for (name, card) in bot.battlefield_creatures.iter() {
            info!("   - [{}]: {:?}", name, card);
        }

        info!("===> Ellenfél Battlefield:");
        for (name, card) in bot.battlefield_opponent_creatures.iter() {
            info!("   - [{}]: {:?}", name, card);
        }

        // Várunk 3 másodpercet
        sleep(Duration::from_secs(3));
    }
}
*/

impl State<AppError> for StartState {
    fn update(&mut self, bot: &mut Bot) -> Result<(), AppError> {
        info!("StartState: initiating game start.");
        // 1) beállítjuk a bot.card_count mezőt, és beolvassuk a kezet
         sleep(Duration::from_secs(2));
        bot.land_number = 2;
        bot.land_count = 2;
        let initial_hand_count = 5;
        bot.card_count = initial_hand_count;
        bot.examine_cards();  // végig-hoovereli a hand kártyákat, OCR-el

        // 2) Egyszer battlefield-olvasás
        bot.refresh_battlefield();
        info!("Battlefield frissítés megtörtént.");

        // 3) Ha van instant a kezünkben, próbáljuk kijátszani és az 1. (index=0) creature-re targetelni
        bot.cast_instants_targeting_creature(0);

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