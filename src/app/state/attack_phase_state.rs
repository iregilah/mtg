// app/state/attack_phase_state.rs

use crate::app::error::AppError;
use crate::app::game_state::GamePhase;
use std::{thread::sleep, time::Duration};
use tracing::{info};
use crate::app::combat_engine::CombatEngine;
use regex::Regex;

use crate::app::{
    bot::Bot,
    ocr::check_main_region_text,
    state::{State, second_main_phase_state::SecondMainPhaseState},
    ui::{set_cursor_pos, left_click},
};
use crate::app::ui::check_button_color;

pub struct AttackPhaseState {
    no_attack: bool,
}

impl AttackPhaseState {
    pub fn new() -> Self {
        Self { no_attack: false }
    }
}

impl State<AppError> for AttackPhaseState {
    fn update(&mut self, bot: &mut Bot) -> Result<(), AppError> {
        info!("AttackPhaseState: starting attack phase.");
        if !Self::can_attack(bot) {
            info!("No creature can attack (all have summoning sickness or none exist). Transitioning to OpponentsTurnState.");
            self.no_attack = true;
            return Ok(());
        }
        self.process_attack_phase(bot);
        Ok(())
    }

    fn next(&mut self) -> Box<dyn State<AppError>> {
        info!("AttackPhaseState: transitioning to CombatDamageState.");
        Box::new(crate::app::state::combat_damage_state::CombatDamageState::new())
    }

    fn phase(&self) -> GamePhase {
        GamePhase::Combat
    }
}

impl AttackPhaseState {
    fn is_attackers_text(s: &str) -> bool {
        // match “1 Attacker” or “2 or more Attackers”, in any case, with optional surrounding whitespace
        let re = Regex::new(r"^\s*\d+\s+Attackers?\s*$").unwrap();
        let result = re.is_match(s.trim());
        info!("is_attackers_text(): input = {:?}, matches regex: {}", s, result);
        result
    }

    fn can_attack(bot: &Bot) -> bool {
        bot.battlefield_creatures.values().any(|card| {
            if let crate::app::card_library::CardType::Creature(cr) = &card.card_type {
                !cr.summoning_sickness
            } else {
                false
            }
        })
    }

    pub fn process_attack_phase(&self, bot: &mut Bot) {
        // 1) Wait for "All Attack" on red button
        loop {
            let main_text = check_main_region_text(
                bot.screen_width as u32,
                bot.screen_height as u32,
                true,
            );
            info!("(Attack phase - red) Main region text: {}", main_text);

            if main_text.contains("All Attack") {
                // record which creatures will attack
                bot.attacking = bot
                    .battlefield_creatures
                    .iter()
                    .filter_map(|(name, card)| {
                        if let crate::app::card_library::CardType::Creature(cr) = &card.card_type {
                            if !cr.summoning_sickness {
                                return Some(name.clone());
                            }
                        }
                        None
                    })
                    .collect();
                info!("Attacking creatures: {:?}", bot.attacking);

                let (x, y) = bot.cords.attack_button;
                set_cursor_pos(x, y);
                left_click();
                sleep(Duration::from_secs(1));
                break;
            }
            if main_text.contains("Next") {
                let (x, y) = bot.cords.attack_button;
                set_cursor_pos(x, y);
                left_click();
                sleep(Duration::from_secs(1));
            } else {
                sleep(Duration::from_secs(2));
            }
        }

        // 2) Wait for "X Attackers"
        loop {
            let is_red = check_button_color(&bot.cords) == "red";
            let main_text = check_main_region_text(
                bot.screen_width as u32,
                bot.screen_height as u32,
                is_red,
            );
            info!("(Attack phase - white) Main region text: {}", main_text);

            if Self::is_attackers_text(&main_text) {
                let (x, y) = bot.cords.attack_button;
                set_cursor_pos(x, y);
                left_click();
                sleep(Duration::from_secs(1));
                break;
            } else {
                sleep(Duration::from_secs(2));
            }
        }

        // 3) Click "Next" until it goes away
        loop {
            let main_text = check_main_region_text(
                bot.screen_width as u32,
                bot.screen_height as u32,
                false,
            );
            info!("(Attack phase - post-attack Next loop) Main region text: {}", main_text);

            if main_text.contains("Next") {
                let (x, y) = bot.cords.attack_button;
                set_cursor_pos(x, y);
                left_click();
                sleep(Duration::from_secs(1));
            } else {
                break;
            }
        }
    }
}
