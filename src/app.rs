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
        // Kezdeti fázis
        let mut current_phase = self.state.phase();

        // Fő futóciklus
        loop {
            // 1) Késleltetett effektusok dispatch-olása az aktuális fázisra
            self.bot.gre.dispatch_delayed(current_phase);

            // 2) State update
            if let Err(e) = self.state.update(&mut self.bot) {
                error!("App hiba az állapotfrissítés során: {:?}", e);
                break;
            }

            // 3) Ellenőrizzük, hogy változott‑e a fázis
            let next_phase = self.state.phase();
            if next_phase != current_phase {
                info!("Phase change: {:?} -> {:?}", current_phase, next_phase);
                self.bot.gre.trigger_event(
                    GameEvent::PhaseChange(next_phase),
                    &mut Vec::new(),
                    self.bot.gre.priority,
                );
                current_phase = next_phase;
            }

            // 4) Resolve-oljuk a GRE stackjét (spell-ek, triggered abilket)
            self.bot.gre.resolve_stack();

            // 5) State váltás, ha szükséges
            self.next_state();
            // Nincs szükség a current_phase újra-beállítására, mert a ciklus elején újra lekérjük
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
        // Értesítjük a GRE-t a fázisváltásról
        self.bot.gre.trigger_event(
            GameEvent::PhaseChange(new_phase),
            &mut Vec::new(),   // ekkor még nincs kártya-terület
            Player::Us,
        );

    }
}
