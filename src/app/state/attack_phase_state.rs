// state/attack_phase_state.rs

use crate::app::bot::Bot;
use crate::app::state::State;
use std::thread::sleep;
use std::time::Duration;
use crate::app::ocr::check_main_region_text;
use crate::app::state::opponents_turn_state::OpponentsTurnState;
use crate::app::ui::{check_button_color, press_key};
use crate::app::state::second_main_phase_state::SecondMainPhaseState;
use crate::app::ui;

pub struct AttackPhaseState {
    no_attack: bool,
}
impl AttackPhaseState {
    pub fn new() -> Self {
        Self { no_attack: false }
    }
}

impl State for AttackPhaseState {
    fn update(&mut self, bot: &mut Bot) {
        tracing::info!("AttackPhaseState: starting attack phase.");
        if !Self::can_attack(bot) {
            tracing::info!("No creature can attack (all have summoning sickness or none exist). Transitioning to OpponentsTurnState.");
            self.no_attack = true;
            return;
        }
        self.process_attack_phase(bot);
    }

    fn next(&mut self) -> Box<dyn State> {
        if self.no_attack {
            tracing::info!("AttackPhaseState: transitioning to OpponentsTurnState due to no creature available.");
            Box::new(OpponentsTurnState::new())
        } else {
            tracing::info!("AttackPhaseState: transitioning to SecondMainPhaseState.");
            Box::new(SecondMainPhaseState::new())
        }
    }
}

impl AttackPhaseState {
    fn can_attack(bot: &Bot) -> bool {
        bot.battlefield_creatures.iter().any(|card| {
            if let crate::app::card_library::CardType::Creature(creature) = &card.card_type {
                !creature.summoning_sickness
            } else {
                false
            }
        })
    }

    fn process_attack_phase(&self, bot: &mut Bot) {
        let mut is_red = check_button_color(&bot.cords) == "red";
        loop {
            is_red = check_button_color(&bot.cords) == "red";
            let main_text = check_main_region_text(bot.screen_width as u32, bot.screen_height as u32, is_red);
            tracing::info!("(Attack phase) Main region text: {}", main_text);
            if main_text.contains("All Attack") {
                press_key(winapi::um::winuser::VK_SPACE as u16);
                break;
            } else if main_text.contains("Next") {
                press_key(winapi::um::winuser::VK_SPACE as u16);
            }
            sleep(Duration::from_secs(2));
        }
    }
}
