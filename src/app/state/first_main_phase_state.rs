// app/state/first_main_phase_state.rs

use std::{thread::sleep, time::Duration};
use tracing::{info};

use crate::app::{
    bot::Bot,
    card_library::{build_card_library, CardType, LAND_NAMES},
    state::{
        State,
        attack_phase_state::AttackPhaseState,
        opponents_turn_state::OpponentsTurnState,
    },
};

pub struct FirstMainPhaseState {
    skip_to_opponent: bool,
}

impl State for FirstMainPhaseState {
    fn update(&mut self, bot: &mut Bot) {
        info!("FirstMainPhaseState: handling first main phase.");

        // —— NEW: Only refresh battlefield OCR on subsequent turns, not the very first.
        // We track this with a flag in Bot (e.g. `first_main_phase_done`).
        if bot.first_main_phase_done {
            info!("Refreshing battlefield creatures from OCR at turn start.");
            Bot::update_battlefield_creatures_from_ocr(bot);
        } else {
            info!("First main phase of the game; skipping initial OCR refresh.");
            bot.first_main_phase_done = true;
        }

        // 0) Untap lands and clear summoning sickness
        bot.land_number = bot.land_count;
        info!("Untap step: available mana = {}", bot.land_number);
        for (_key, card) in bot.battlefield_creatures.iter_mut() {
            if let CardType::Creature(ref mut cr) = card.card_type {
                cr.summoning_sickness = false;
            }
        }
        info!("Creatures after untap: {:?}", bot.battlefield_creatures);

        // 1) Play land if not yet played this turn
        bot.play_land();
        info!("Available mana after playing land: {}", bot.land_number);

        // 2) Cast creatures one by one, inserting each into our map with unique keys
        loop {
            if bot.land_number == 0 {
                info!("No mana remaining—stopping creature casting loop.");
                break;
            }
            if let Some((name, cost)) = bot.cast_one_creature() {
                info!("Successfully cast '{}', spent {} mana.", name, cost);
                bot.land_number = bot.land_number.saturating_sub(cost);

                // —— FIX #1: Allow duplicate names by generating a unique key per copy.
                let mut key = name.clone();
                if bot.battlefield_creatures.contains_key(&key) {
                    // count existing copies
                    let dup_count = bot
                        .battlefield_creatures
                        .keys()
                        .filter(|k| k.starts_with(&name))
                        .count()
                        + 1;
                    key = format!("{}#{}", name, dup_count);
                }

                // clone the card from our library and insert under `key`
                if let Some(mut card) = build_card_library().get(&name).cloned() {
                    if let CardType::Creature(ref mut cr) = card.card_type {
                        cr.summoning_sickness = true; // new creatures enter tapped
                    }
                    bot.battlefield_creatures.insert(key.clone(), card);
                }

                info!(
                    "{} creature(s) on battlefield: {:?}",
                    bot.battlefield_creatures.len(),
                    bot.battlefield_creatures
                );

                // only re‐OCR battlefield if we have both mana and further creatures castable
                if bot.land_number > 0 && bot.can_cast_creature() {
                    info!("Still have mana & creatures to cast: refreshing battlefield OCR.");
                    Bot::update_battlefield_creatures_from_ocr(bot);
                } else {
                    info!("Either out of mana or no more creatures—no further battlefield reads.");
                    break;
                }
            } else {
                info!("No more affordable creatures left to cast.");
                break;
            }
        }

        // 3) Cast any other spells (instants/enchantments)
        let spent = bot.cast_creatures();
        bot.land_number = spent;
        if spent > 0 {
            info!("Mana after other spells: {}", spent);
        } else {
            info!("No affordable non‐creature spells left to cast.");
        }

        // 4) Decide whether to enter AttackPhase or skip directly to OpponentsTurn
        // 4a) If any non‐land card still affordable, proceed to AttackPhase
        if bot.land_number > 0 {
            let lib = build_card_library();
            let can_cast_more = bot.cards_texts.iter().any(|ocr| {
                lib.values().any(|card| {
                    !matches!(card.card_type, CardType::Land)
                        && Bot::text_contains(&card.name, ocr)
                        && {
                        let c = &card.mana_cost;
                        let col = c.colored();
                        let leftover = bot.land_number.saturating_sub(col);
                        bot.land_number >= col && leftover >= c.colorless
                    }
                })
            });
            if can_cast_more {
                info!("Still spells to cast with available mana—proceeding to AttackPhase.");
                self.skip_to_opponent = false;
                return;
            }
        }

        // 4b) Otherwise, if any creature is ready (no summoning sickness), go to AttackPhase
        let can_attack = bot
            .battlefield_creatures
            .values()
            .any(|card| match &card.card_type {
                CardType::Creature(cr) => !cr.summoning_sickness,
                _ => false,
            });

        if can_attack {
            info!("Creatures available to attack—proceeding to AttackPhase.");
            self.skip_to_opponent = false;
        } else {
            info!("No attackers available—skipping to OpponentsTurn.");
            self.skip_to_opponent = true;
        }
    }

    fn next(&mut self) -> Box<dyn State> {
        if self.skip_to_opponent {
            info!("Skipping AttackPhase -> OpponentsTurnState.");
            Box::new(OpponentsTurnState::new())
        } else {
            info!("Proceeding to AttackPhaseState.");
            Box::new(AttackPhaseState::new())
        }
    }
}

impl FirstMainPhaseState {
    pub fn new() -> Self {
        Self { skip_to_opponent: false }
    }
}