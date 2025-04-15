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
//use regex::Regex;


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
        tracing::info!("AttackPhaseState: transitioning to SecondMainPhaseState.");
        Box::new(SecondMainPhaseState::new())
    }
}

impl AttackPhaseState {
    fn is_attackers_text(s: &str) -> bool {
        // Egyszerű ellenőrzés: ha legalább két szó van, az első egy szám és a második "Attackers"
        let tokens: Vec<&str> = s.split_whitespace().collect();
        if tokens.len() >= 2 && tokens[1] == "Attackers" {
            return tokens[0].parse::<u32>().is_ok();
        }
        false
    }

    fn can_attack(bot: &Bot) -> bool {
        bot.battlefield_creatures.iter().any(|card| {
            if let crate::app::card_library::CardType::Creature(creature) = &card.card_type {
                !creature.summoning_sickness
            } else {
                false
            }
        })
    }

    pub fn process_attack_phase(&self, bot: &mut Bot) {
        // 1. Ciklus: addig várunk, amíg a main region text "All Attack"-et ad,
        //    itt mindig a red button (white_invert_image) feldolgozását használjuk.
        loop {
            let main_text = check_main_region_text(bot.screen_width as u32, bot.screen_height as u32, true);
            tracing::info!("(Attack phase) Main region text: {}", main_text);
            if main_text.contains("All Attack") {
                press_key(winapi::um::winuser::VK_SPACE as u16);
                sleep(Duration::from_secs(1));
                break;
            } else if main_text.contains("Next") {
                press_key(winapi::um::winuser::VK_SPACE as u16);
                sleep(Duration::from_secs(1));
            } else {
                sleep(Duration::from_secs(2));
            }
        }

        // 2. Ciklus: várjuk, hogy a main region text "X Attackers" formátumú legyen.
        loop {
            let main_text = check_main_region_text(bot.screen_width as u32, bot.screen_height as u32, true);
            tracing::info!("(Attack phase) Main region text after All Attack: {}", main_text);
            if Self::is_attackers_text(&main_text) {
                press_key(winapi::um::winuser::VK_SPACE as u16);
                sleep(Duration::from_secs(1));
                break;
            }
            sleep(Duration::from_secs(2));
        }

        // 3. Ciklus: amíg "Next" szerepel, kattintsuk a main region text-et
        loop {
            let main_text = check_main_region_text(bot.screen_width as u32, bot.screen_height as u32, true);
            tracing::info!("(Attack phase) Main region text in Next loop: {}", main_text);
            if main_text.contains("Next") {
                press_key(winapi::um::winuser::VK_SPACE as u16);
                sleep(Duration::from_secs(1));
            } else {
                break;
            }
        }
    }
}
