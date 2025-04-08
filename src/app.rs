pub mod bot;
pub mod card;
pub mod cards_positions;
pub mod ocr;
pub mod ui;
pub mod state;

use bot::Bot;
use std::error::Error;
use std::rc::Rc;
use std::sync::{Arc, Mutex};
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
    state: Box<dyn State>,
    bot: Bot,
}

impl App {
    pub fn start() {
        let mut app = App::new();
        loop {
            // Az aktuális állapot futtatása a boton
            app.update().unwrap();
            // Állapotváltás az egyes fázisok között
            app.next_state();
        }
    }

    fn new() -> Self {
        Self {
            state: Box::new(StartState::new()),
            bot: Bot::new(),
        }
    }

    fn update(&mut self) -> Result<(), Box<dyn Error>> {
        self.state.update(&mut self.bot);
        Ok(())
    }

    fn next_state(&mut self) {
        self.state = self.state.next();
    }
}
