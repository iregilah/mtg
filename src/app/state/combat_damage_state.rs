// src/app/state/combat_damage_state.rs

use crate::app::card_attribute::{GrantAbilityAttribute, KeywordAbility};
use crate::app::error::AppError;
use crate::app::game_state::{GameEvent, GamePhase, Player};
use crate::app::bot::Bot;
use crate::app::state::{second_main_phase_state::SecondMainPhaseState, State};
use tracing::info;
use std::any::Any;
use crate::app::card_library::CardType;

pub struct CombatDamageState;

impl CombatDamageState {
    pub fn new() -> Self { Self }

    fn resolve_combat_damage(&self, bot: &mut Bot) {
        info!("CombatDamageState: resolving combat damage for attackers: {:?}", bot.attacking);

        let attackers = bot.attacking.clone();
        let mut total_unblocked: u32 = 0;
        let mut lifegain_us: i32 = 0;
        let mut lifegain_opp: i32 = 0;
        let mut to_kill_us = Vec::new();
        let mut to_kill_opp = Vec::new();

        // Snapshot defenders in order
        let defenders: Vec<String> = bot.battlefield_opponent_creatures.keys().cloned().collect();

        for (i, atk_name) in attackers.iter().enumerate() {
            if let Some(atk_card) = bot.battlefield_creatures.get_mut(atk_name) {
                if let CardType::Creature(atk_cr) = &mut atk_card.card_type {
                    let power = atk_cr.power as u32;

                    let has_lifelink = atk_card.attributes.iter().any(|attr| {
                        attr.as_any()
                            .downcast_ref::<GrantAbilityAttribute>()
                            .map_or(false, |ga| ga.ability == KeywordAbility::Lifelink)
                    });
                    let has_trample = atk_card.attributes.iter().any(|attr| {
                        attr.as_any()
                            .downcast_ref::<GrantAbilityAttribute>()
                            .map_or(false, |ga| ga.ability == KeywordAbility::Trample)
                    });
                    let has_deathtouch = atk_card.attributes.iter().any(|attr| {
                        attr.as_any()
                            .downcast_ref::<GrantAbilityAttribute>()
                            .map_or(false, |ga| ga.ability == KeywordAbility::Deathtouch)
                    });

                    if i < defenders.len() {
                        // Blocked by the i-th defender
                        let def_name = &defenders[i];
                        if let Some(def_card) = bot.battlefield_opponent_creatures.get_mut(def_name) {
                            if let CardType::Creature(def_cr) = &mut def_card.card_type {
                                // Attacker deals to blocker
                                let dmg_to_blocker = if has_deathtouch { 1 } else { power };
                                info!("{} deals {} to {}", atk_name, dmg_to_blocker, def_name);
                                def_cr.toughness -= dmg_to_blocker as i32;
                                // Deathtouch kills regardless of remaining toughness
                                if has_deathtouch || def_cr.toughness <= 0 {
                                    info!("{} died", def_name);
                                    bot.gre.trigger_event(
                                        GameEvent::CreatureDied(def_name.clone()),
                                        &mut Vec::new(),
                                        Player::Opponent,
                                    );
                                    to_kill_opp.push(def_name.clone());
                                }

                                // Blocker deals to attacker
                                let dmg_to_attacker = def_cr.power as u32;
                                info!("{} deals {} to {}", def_name, dmg_to_attacker, atk_name);
                                atk_cr.toughness -= dmg_to_attacker as i32;
                                if atk_cr.toughness <= 0 {
                                    info!("{} died", atk_name);
                                    bot.gre.trigger_event(
                                        GameEvent::CreatureDied(atk_name.clone()),
                                        &mut Vec::new(),
                                        Player::Us,
                                    );
                                    to_kill_us.push(atk_name.clone());
                                }

                                // Lifelink:
                                if has_lifelink {
                                    lifegain_us += dmg_to_blocker as i32;
                                }
                                // Trample excess:
                                if has_trample {
                                    let blocker_toughness = if has_deathtouch { 1 } else { dmg_to_blocker };
                                    if power > blocker_toughness {
                                        let excess = power - blocker_toughness;
                                        total_unblocked += excess;
                                        info!("{} tramples for {} excess", atk_name, excess);
                                    }
                                }
                            }
                        }
                    } else {
                        // Unblocked
                        info!("{} is unblocked: {} damage to opponent", atk_name, power);
                        total_unblocked += power;
                        if has_lifelink {
                            lifegain_us += power as i32;
                        }
                    }
                }
            }
        }

        // Remove dead creatures
        for name in to_kill_us {
            bot.battlefield_creatures.remove(&name);
        }
        for name in to_kill_opp {
            bot.battlefield_opponent_creatures.remove(&name);
        }

        // Apply unblocked damage to opponent
        if total_unblocked > 0 {
            info!("Applying {} unblocked damage to opponent", total_unblocked);
            bot.updater.state.opponent_life_total =
                bot.updater.state.opponent_life_total.saturating_sub(total_unblocked as i32);
        }
        // Apply lifegain
        if lifegain_us > 0 {
            info!("Applying lifelink: gain {} life", lifegain_us);
            bot.updater.state.life_total =
                bot.updater.state.life_total.saturating_add(lifegain_us);
        }
        if lifegain_opp > 0 {
            bot.updater.state.opponent_life_total =
                bot.updater.state.opponent_life_total.saturating_add(lifegain_opp);
        }

        // Resolve stack and refresh global state
        bot.gre.resolve_stack();
        bot.updater.refresh_all(
            bot.screen_width as u32,
            bot.screen_height as u32,
            &bot.cards_texts,
            &crate::app::card_library::build_card_library(),
            bot.land_number,
            bot.land_played_this_turn,
            &bot.gre.stack,
        );
        // Clear attackers list
        bot.attacking.clear();
    }
}

impl State<AppError> for CombatDamageState {
    fn update(&mut self, bot: &mut Bot) -> Result<(), AppError> {
        self.resolve_combat_damage(bot);
        Ok(())
    }

    fn next(&mut self) -> Box<dyn State<AppError>> {
        Box::new(SecondMainPhaseState::new())
    }

    fn phase(&self) -> GamePhase {
        GamePhase::CombatDamage
    }
}
