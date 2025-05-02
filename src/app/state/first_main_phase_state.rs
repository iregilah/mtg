// app/state/first_main_phase_state.rs

use std::any::Any;
//use std::{thread::sleep, time::Duration};
use crate::app::error::AppError;
use tracing::warn;
use crate::app::game_state::GamePhase;
use tracing::{info};

use crate::app::{
    bot::Bot,
    card_library::{build_card_library, CardType},
    state::{
        State,
        attack_phase_state::AttackPhaseState,
        opponents_turn_state::OpponentsTurnState,
    },
};
use crate::app::card_attribute::{GrantAbilityAttribute, KeywordAbility};
use crate::app::game_state_updater::load_side_creatures;

pub struct FirstMainPhaseState {
    skip_to_opponent: bool,
}

impl State<AppError> for FirstMainPhaseState {
    fn update(&mut self, bot: &mut Bot) -> Result<(), AppError> {
        info!("FirstMainPhaseState: handling first main phase.");

        self.refresh_battlefield_if_needed(bot);
        self.untap_and_clear_sickness(bot);
        self.play_land_step(bot);
        self.cast_main_phase_creatures(bot);
        self.cast_other_spells(bot);
        self.decide_attack_or_skip(bot);

        if bot.land_played_this_turn && bot.updater.state.mana_available == 0 {
            warn!("Már játszottunk land-et de nincs elérhető mana!");
        }

        Ok(())
    }

    fn next(&mut self) -> Box<dyn State<AppError>> {
        if self.skip_to_opponent {
            info!("Skipping AttackPhase -> OpponentsTurnState.");
            Box::new(OpponentsTurnState::new())
        } else {
            info!("Proceeding to AttackPhaseState.");
            Box::new(AttackPhaseState::new())
        }
    }

    fn phase(&self) -> GamePhase {
        GamePhase::PreCombatMain
    }
}

impl FirstMainPhaseState {
    pub fn new() -> Self {
        Self { skip_to_opponent: false }
    }

    fn refresh_battlefield_if_needed(&mut self, bot: &mut Bot) {
        if bot.first_main_phase_done {
            info!("Refreshing battlefield creatures from OCR at turn start.");
            let ours = load_side_creatures(
                bot.screen_width as u32,
                bot.screen_height as u32,
                false,
            );
            let opps = load_side_creatures(
                bot.screen_width as u32,
                bot.screen_height as u32,
                true,
            );
            bot.battlefield_creatures = ours;
            bot.battlefield_opponent_creatures = opps;
        } else {
            info!("First main phase of the game; skipping initial OCR refresh.");
            bot.first_main_phase_done = true;
        }
    }

    fn untap_and_clear_sickness(&self, bot: &mut Bot) {
        bot.land_number = bot.land_count;
        info!("Untap step: available mana = {}", bot.land_number);
        for (_key, card) in bot.battlefield_creatures.iter_mut() {
            if let CardType::Creature(ref mut cr) = card.card_type {
                cr.summoning_sickness = false;
            }
        }
        info!("Creatures after untap: {:?}", bot.battlefield_creatures);
    }

    fn play_land_step(&self, bot: &mut Bot) {
        bot.play_land();
        info!("Available mana after playing land: {}", bot.land_number);
    }

    fn cast_main_phase_creatures(&mut self, bot: &mut Bot) {
        let library = build_card_library();
        while bot.land_number > 0 {
            if let Some((name, cost)) = bot.cast_one_creature() {
                info!("Successfully cast '{}', spent {} mana.", name, cost);
                bot.land_number = bot.land_number.saturating_sub(cost);

                // allow duplicate keys
                let mut key = name.clone();
                if bot.battlefield_creatures.contains_key(&key) {
                    let dup = bot
                        .battlefield_creatures
                        .keys()
                        .filter(|k| k.starts_with(&name))
                        .count()
                        + 1;
                    key = format!("{}#{}", name, dup);
                }

                // insert new creature tapped
                if let Some(mut card) = library.get(&name).cloned() {
                    // 1) insert tapped
                    if let CardType::Creature(ref mut cr) = card.card_type {
                        cr.summoning_sickness = true;
                    }
                    bot.battlefield_creatures.insert(key.clone(), card.clone());

                    // 2) immediately clear summoning sickness if it has haste
                    if let Some(entry) = bot.battlefield_creatures.get_mut(&key) {
                        let has_haste = entry
                            .attributes
                            .iter()
                            .any(|attr| {
                                // coerce the CardAttribute trait object to &dyn Any, then downcast
                                let any = attr.as_ref() as &dyn Any;
                                any.downcast_ref::<GrantAbilityAttribute>()
                                    .map_or(false, |ga| ga.ability == KeywordAbility::Haste)
                            });
                        if has_haste {
                            if let CardType::Creature(ref mut cr) = entry.card_type {
                                cr.summoning_sickness = false;
                            }
                        }
                    }
                }

                info!(
                    "{} creature(s) on battlefield: {:?}",
                    bot.battlefield_creatures.len(),
                    bot.battlefield_creatures
                );

                // ha még van mana és van mit castolni, frissítjük az OCR-es battlefield-et
                if bot.land_number > 0 && bot.can_cast_creature() {
                    info!("Still have mana & creatures to cast: refreshing battlefield OCR.");

                    let ours = load_side_creatures(
                        bot.screen_width as u32,
                        bot.screen_height as u32,
                        false, // saját oldal
                    );
                    let opps = load_side_creatures(
                        bot.screen_width as u32,
                        bot.screen_height as u32,
                        true,  // ellenfél
                    );
                    bot.battlefield_creatures = ours;
                    bot.battlefield_opponent_creatures = opps;

                } else {
                    info!("Either out of mana or no more creatures—no further battlefield reads.");
                    break;
                }
            } else {
                info!("No more affordable creatures left to cast.");
                break;
            }
        }
    }

    fn cast_other_spells(&self, bot: &mut Bot) {
        let spent = bot.cast_creatures();
        bot.land_number = spent;
        if spent > 0 {
            info!("Mana after other spells: {}", spent);
        } else {
            info!("No affordable non‐creature spells left to cast.");
        }
    }

    fn decide_attack_or_skip(&mut self, bot: &mut Bot) {
        // if we still have non‐land spells to cast, go to attack
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

        // otherwise, if any creature is ready, attack; else skip
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
}
