use crate::app::bot::Bot;
use crate::app::state::State;
use std::thread::sleep;
use std::time::Duration;
use crate::app::ocr::check_main_region_text;
use crate::app::ui::press_key;
use crate::app::state::start_state::StartState;


pub struct SecondMainPhaseState {}

impl SecondMainPhaseState {
    pub fn new() -> Self {
        Self {}
    }
}

impl State for SecondMainPhaseState {
    fn update(&mut self, bot: &mut Bot) {
        tracing::info!("SecondMainPhaseState: handling second main phase and end turn.");
        Self::process_end_turn(self, bot);
        bot.land_played_this_turn = false;
        for card in &mut bot.battlefield_creatures {
            if let crate::app::card_library::CardType::Creature(ref mut creature) = card.card_type {
                creature.summoning_sickness = false;
            }
        }
    }

    fn next(&mut self) -> Box<dyn State> {
        tracing::info!("SecondMainPhaseState: transitioning to new round (StartState).");
        Box::new(StartState::new())
    }
}

impl SecondMainPhaseState {
    fn process_end_turn(&self, bot: &mut Bot) {
        loop {
            let main_text = check_main_region_text(bot.screen_width as u32, bot.screen_height as u32, false);
            tracing::info!("(Second main phase) Main region text: {}", main_text);
            if main_text.contains("End Turn") {
                press_key(winapi::um::winuser::VK_SPACE as u16);
                break;
            } else if main_text.contains("Next") {
                press_key(winapi::um::winuser::VK_SPACE as u16);
            }
            std::thread::sleep(Duration::from_secs(2));
        }
    }
}