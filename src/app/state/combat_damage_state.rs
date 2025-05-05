// src/app/state/combat_damage_state.rs

use crate::app::card_attribute::{KeywordAbility, Trigger, TargetFilter};
use crate::app::combat_engine::CombatEngine;
use crate::app::error::AppError;
use crate::app::game_state::{GameEvent, GamePhase, Player};
use crate::app::bot::Bot;
use crate::app::card_library::CardType;
use crate::app::state::{second_main_phase_state::SecondMainPhaseState, State};
use tracing::info;

pub struct CombatDamageState;

impl CombatDamageState {
    pub fn new() -> Self { Self }

    fn resolve_combat_damage(&self, bot: &mut Bot) {
        info!("CombatDamageState: resolving combat damage for attackers: {:?}", bot.attacking);

        // 1) GRE triggers: OnCombatDamage képességek (pl. Lifelink, Deathtouch, stb.)
        for atk_name in bot.attacking.iter() {
            if let Some(atk_card) = bot.battlefield_creatures.get_mut(atk_name) {
                let effects = atk_card.trigger_by(&Trigger::OnCombatDamage { filter: TargetFilter::SelfCard });
                for eff in effects {
                    bot.gre.handle_effect(eff);
                }
            }
        }

        // 2) Harci eredmények kiszámolása a CombatEngine-nel
        let attack_vec = bot
            .battlefield_creatures
            .values()
            .filter_map(|c| {
                if let CardType::Creature(cr) = &c.card_type {
                    Some(cr.clone())
                } else {
                    None
                }
            })
            .collect::<Vec<_>>();
        let block_vec = bot
            .battlefield_opponent_creatures
            .values()
            .filter_map(|c| {
                if let CardType::Creature(cr) = &c.card_type {
                    Some(cr.clone())
                } else {
                    None
                }
            })
            .collect::<Vec<_>>();

        let (surv_att, surv_blk, unblocked_dmg, life_gain) = CombatEngine::resolve_combat(
            &bot.combat_attackers,
            &attack_vec,
            &block_vec,
            &bot.combat_blocks,
            &mut bot.gre.prevent_life_gain_opponent,
        );

        // 3) Túlélők kiszűrése a HashMap-ben
        bot.battlefield_creatures = bot
            .battlefield_creatures
            .iter()
            .zip(surv_att.iter())
            .filter(|&(_, &keep)| keep)
            .map(|((name, card), _)| (name.clone(), card.clone()))
            .collect();
        bot.battlefield_opponent_creatures = bot
            .battlefield_opponent_creatures
            .iter()
            .zip(surv_blk.iter())
            .filter(|&(_, &keep)| keep)
            .map(|((name, card), _)| (name.clone(), card.clone()))
            .collect();

        // 4) Unblocked damage az ellenfél életére
        if unblocked_dmg > 0 {
            info!("Applying {} unblocked damage to opponent", unblocked_dmg);
            bot.updater.state.opponent_life_total =
                bot.updater.state.opponent_life_total.saturating_sub(unblocked_dmg as i32);
        }

        // 5) Lifelinkből származó életerő-nyerés
        if life_gain > 0 {
            info!("Gaining {} life from lifelink", life_gain);
            bot.updater.state.life_total += life_gain;
        }

        // 6) GRE stack és delayed efektek
        bot.gre.resolve_stack();
        bot.gre.dispatch_delayed(GamePhase::CombatDamage);

        // 7) Végső GameState frissítés és takarítás
        bot.updater.refresh_all(
            bot.screen_width as u32,
            bot.screen_height as u32,
            &bot.cards_texts,
            &crate::app::card_library::build_card_library(),
            bot.land_number,
            bot.land_played_this_turn,
            &bot.gre.stack,
        );
        bot.attacking.clear();
        bot.combat_attackers.clear();
        bot.combat_blocks.clear();
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