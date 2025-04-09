// state/first_main_phase_state.rs

use crate::app::bot::Bot;
use crate::app::state::State;
use std::thread::sleep;
use std::time::Duration;
use crate::app::card::{CardType, CREATURE_NAMES, LAND_NAMES, parse_card, parse_mana_cost};
use crate::app::cards_positions::get_card_positions;
use crate::app::ui::{set_cursor_pos, left_click};
use crate::app::state::attack_phase_state::AttackPhaseState;
use crate::app::ui;

pub struct FirstMainPhaseState {}

impl FirstMainPhaseState {
    pub fn new() -> Self {
        Self {}
    }

    fn play_card(bot: &mut Bot, card_index: usize) {
        let positions = get_card_positions(bot.card_count, bot.screen_width as u32);
        if card_index >= positions.len() {
            tracing::error!("Error: Card index {} is out of range. Only {} cards available.", card_index, positions.len());
            return;
        }
        let pos = positions[card_index];
        let card_y = ((bot.screen_height as f64) * 0.97).ceil() as i32;
        set_cursor_pos(pos.hover_x as i32, card_y);
        left_click();
        left_click();
        set_cursor_pos(bot.screen_width - 1, bot.screen_height - 1);
        ui::press_key(0x5A); // 'Z' billentyű
        left_click();
        sleep(Duration::from_millis(150));
        if card_index < bot.cards_texts.len() {
            tracing::info!("Removing card at index {} from hand.", card_index);
            tracing::info!("Cards in hand after the removal: {:?}", bot.cards_texts);
            bot.cards_texts.remove(card_index);
            bot.card_count = bot.cards_texts.len();
        }
    }
}

impl State for FirstMainPhaseState {
    fn update(&mut self, bot: &mut Bot) {
        tracing::info!("FirstMainPhaseState: handling first main phase.");
        if !bot.land_played_this_turn {
            if let Some((index, card_text)) = bot.cards_texts.iter().enumerate()
                .find(|(_i, text)| LAND_NAMES.iter().any(|&land| text.contains(land))) {
                tracing::info!("Found land card '{}' at index {}. Playing it.", card_text, index);
                Self::play_card(bot, index);
                bot.land_number += 1;
                bot.land_played_this_turn = true;
            }
        }
        let mut mana_available = bot.land_number;
        tracing::info!("Available mana for this turn after playing lands: {}", mana_available);
        let creature_indices: Vec<usize> = bot.cards_texts.iter().enumerate()
            .filter(|(_i, text)| CREATURE_NAMES.iter().any(|&name| text.contains(name)))
            .map(|(i, _)| i)
            .collect();

        for &index in &creature_indices {
            let card_text = &bot.cards_texts[index];
            if let Some(card) = parse_card(card_text) {
                if let CardType::Creature(creature) = card {
                    let cost = parse_mana_cost(&creature.name);
                    let colored_cost = cost.colored();
                    let total_cost = cost.total();
                    if mana_available >= colored_cost {
                        let leftover = mana_available - colored_cost;
                        if leftover >= cost.colorless {
                            tracing::info!(
                                "Casting creature '{}' ({} colorless, {} colored), total cost = {}",
                                creature.name, cost.colorless, colored_cost, total_cost
                            );
                            Self::play_card(bot, index);
                            bot.battlefield_creatures.push(creature);
                            mana_available -= total_cost;
                        } else {
                            tracing::info!(
                                "Not enough leftover for colorless after paying colored mana for '{}'. Required: {} colorless, leftover: {}",
                                creature.name, cost.colorless, leftover
                            );
                        }
                    } else {
                        tracing::info!(
                            "Not enough colored mana to cast '{}'. Required: {} colored, available: {}",
                            creature.name, colored_cost, mana_available
                        );
                    }
                }
            }
        }
        tracing::info!("Main phase finished. Remaining mana: {}.", mana_available);
    }

    fn next(&mut self) -> Box<dyn State> {
        tracing::info!("FirstMainPhaseState: transitioning to AttackPhaseState.");
        Box::new(AttackPhaseState::new())
    }
}
