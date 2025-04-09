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
        tracing::info!("App: Initializing new App instance.");
        let mut app = App::new();
        tracing::info!("App: Entering main loop.");
        loop {
            tracing::info!("App: Updating current state.");
            match app.update() {
                Ok(_) => tracing::info!("App: State update completed successfully."),
                Err(e) => {
                    tracing::error!("App: Error during state update: {:?}", e);
                    break;
                }
            }
            tracing::info!("App: Transitioning to next state.");
            app.next_state();
            // Egy rövid várakozás a loop körök között (opcionális)
            sleep(Duration::from_millis(100));
        }
    }

    fn new() -> Self {
        tracing::info!("App: Creating new App instance with StartState and new Bot.");
        Self {
            state: Box::new(StartState::new()),
            bot: Bot::new(),
        }
    }

    fn update(&mut self) -> Result<(), Box<dyn Error>> {
        tracing::info!("App: Calling update() on current state.");
        self.state.update(&mut self.bot);
        Ok(())
    }

    fn next_state(&mut self) {
        tracing::info!("App: Requesting next state from current state.");
        let next = self.state.next();
        tracing::info!("App: Transitioning to new state.");
        self.state = next;
    }
}
