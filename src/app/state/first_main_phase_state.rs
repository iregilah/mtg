// state/first_main_phase_state.rs

use crate::app::bot::Bot;
use crate::app::state::State;
use std::thread::sleep;
use std::time::Duration;
use crate::app::card::{CardType, CREATURE_NAMES, LAND_NAMES, parse_card, parse_mana_cost};
use crate::app::cards_positions::get_card_positions;
use crate::app::ui::{set_cursor_pos, left_click, press_key};
use crate::app::state::attack_phase_state::AttackPhaseState;
use crate::app::ui;

pub struct FirstMainPhaseState {}

impl State for FirstMainPhaseState {
    fn update(&mut self, bot: &mut Bot) {
        tracing::info!("FirstMainPhaseState: handling first main phase.");
        self.play_land_phase(bot);
        tracing::info!("Available mana for this turn after playing lands: {}", bot.land_number);
        let mana_available = self.cast_creatures_phase(bot);
        tracing::info!("Main phase finished. Remaining mana: {}.", mana_available);
    }

    fn next(&mut self) -> Box<dyn State> {
        tracing::info!("FirstMainPhaseState: transitioning to AttackPhaseState.");
        Box::new(AttackPhaseState::new())
    }
}

impl FirstMainPhaseState {
    pub fn new() -> Self {
        Self {}
    }
    /// Eltávolítja a kézből a megadott indexű kártyát, majd logolja a frissített kezet.
    fn remove_card_from_hand(bot: &mut Bot, card_index: usize) {
        if card_index < bot.cards_texts.len() {
            let removed = bot.cards_texts.remove(card_index);
            tracing::info!("Removed card '{}' from hand at index {}.", removed, card_index);
            tracing::info!("Updated hand: {:?}", bot.cards_texts);
            bot.card_count = bot.cards_texts.len();
        } else {
            tracing::warn!("Attempted to remove card at invalid index {}.", card_index);
        }
    }
    /// Végrehajtja a land kijátszását, ha még nem történt meg.
    fn play_land_phase(&mut self, bot: &mut Bot) {
        if !bot.land_played_this_turn {
            if let Some((index, card_text)) = bot.cards_texts.iter().enumerate()
                .find(|(_i, text)| LAND_NAMES.iter().any(|&land| text.contains(land))) {
                tracing::info!("Found land card '{}' at index {}. Playing it.", card_text, index);
                Self::play_card(bot, index);
                bot.land_number += 1; // Egy land 1 mana forrást jelent
                bot.land_played_this_turn = true;
            }
        }
    }

    /// Végrehajtja a creature castolás logikáját: végigiterál a kézben lévő creature-okon,
    /// és ha elegendő mana áll rendelkezésre, kijátssza az adott kártyát, majd eltávolítja azt a kézből.
    /// A függvény visszaadja a maradék mana értékét.
    fn cast_creatures_phase(&mut self, bot: &mut Bot) -> u32 {
        let mut mana_available = bot.land_number;
        // Gyűjtsük össze a kézben lévő creature neveket (a statikus lista alapján)
        let creature_names: Vec<String> = bot.cards_texts.iter()
            .filter(|text| CREATURE_NAMES.iter().any(|&name| text.contains(name)))
            .cloned()
            .collect();

        // Iterálunk a creature neveken; a kéz tartalma eltűnhet az eltávolítás miatt,
        // ezért mindig keresünk a frissített listában.
        for creature_name in creature_names {
            if let Some(pos) = bot.cards_texts.iter().position(|text| text.contains(&creature_name)) {
                if let Some(card) = parse_card(&bot.cards_texts[pos]) {
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
                                Self::play_card(bot, pos);
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
            } else {
                tracing::warn!("Creature '{}' not found in hand for removal.", creature_name);
            }
        }
        mana_available
    }

    /// Végrehajtja a kijátszás műveletét: mozgatás, kattintás, majd a kijátszott kártya eltávolítása a kézből.
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
        press_key(0x5A); // 'Z' billentyű
        left_click();
        sleep(Duration::from_millis(150));
        Self::remove_card_from_hand(bot, card_index);
    }
}
