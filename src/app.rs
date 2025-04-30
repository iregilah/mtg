// app.rs

pub mod bot;
pub mod card_library;
pub mod cards_positions;
pub mod ocr;
pub mod ui;
pub mod state;
pub mod creature_positions;
pub mod card_attribute;
pub mod gre;
pub mod game_state;

pub mod error;
pub mod game_state_updater;

use crate::app::error::AppError;
use crate::app::game_state::Player;
use crate::app::game_state::GameEvent;
use tracing::{info, error};
use bot::Bot;
use std::error::Error;
use std::thread::sleep;
use std::time::Duration;
use state::start_state::StartState;
use state::State;
use crate::app::game_state_updater::GameStateUpdater;
use crate::app::card_library::build_card_library;


#[derive(Debug, Copy, Clone, PartialEq)]
pub struct Color {
    pub r: u8,
    pub g: u8,
    pub b: u8,
}

pub struct App {
    state: Box<dyn State<AppError>>,
    bot: Bot,
}

impl App {
    pub fn start(&mut self) {
        // Initial phase
        let mut current_phase = self.state.phase();
        let mut updater = GameStateUpdater::new();

        // Main run loop
        loop {
            // 1) Dispatch delayed effects for the current phase
            self.bot.gre.dispatch_delayed(current_phase);

            // 2) Update the state
            if let Err(e) = self.state.update(&mut self.bot) {
                error!("App error during state update: {:?}", e);
                break;
            }

            // 3) Check for a phase change *és csak akkor* lépjünk tovább a state machine-ben
            let next_phase = self.state.phase();
            if next_phase != current_phase {
                info!("Phase change: {:?} -> {:?}", current_phase, next_phase);
                self.bot.gre.trigger_event(
                    GameEvent::PhaseChange(next_phase),
                    &mut Vec::new(),
                    self.bot.gre.priority,
                );
                current_phase = next_phase;

                // Mielőtt frissítenénk a GameState-et, ugorjunk át az új State-re
                info!("App: Requesting next state from current state.");
                let next = self.state.next();
                info!("App: Transitioning to new state.");
                self.state = next;
            }

            // 4) Resolve the GRE stack
            self.bot.gre.resolve_stack();

            // 5) Frissítsük a GameState-et (OCR, stb.) mindig az aktuális state után
            updater.refresh_all(
                self.bot.screen_width as u32,
                self.bot.screen_height as u32,
                &self.bot.cards_texts,
                &build_card_library(),
                self.bot.land_number,
                self.bot.land_played_this_turn,
                &self.bot.gre.stack,
            );
            self.bot.updater.state = updater.state.clone();
        }
    }

    pub fn new() -> Self {
        info!("App: Creating new App instance with StartState and new Bot.");
        Self {
            state: Box::new(StartState::new()),
            bot: Bot::new(),
        }
    }


    pub fn update(&mut self) -> Result<(), AppError> {
        info!("App: Calling update() on current state.");
        self.state.update(&mut self.bot)
    }

    fn next_state(&mut self) {
        let _old_phase = self.state.phase();
        info!("App: Requesting next state from current state.");
        let next = self.state.next();
        info!("App: Transitioning to new state.");
        let new_phase = next.phase();
        self.state = next;
        // Notify the GRE of the phase change
        self.bot.gre.trigger_event(
            GameEvent::PhaseChange(new_phase),
            &mut Vec::new(),   // no cards in play at this point
            Player::Us,
        );
    }
}
