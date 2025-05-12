// src/app/gre/effect_resolution.rs

use tracing::{debug, info, warn};
use crate::app::card_attribute::{Effect, TargetFilter, Duration, Amount, PlayerSelector};
use crate::app::card_library::{Card, CardType, Creature, ManaCost};
use crate::app::card_library::CardTypeFlags;
use crate::app::game_state::{GamePhase, GameEvent, Player};
use crate::app::gre::Gre;
use crate::app::gre::stack::StackEntry;
use crate::app::gre::gre_structs::DelayedEffect;
use crate::app::gre::gre_structs::ReplacementEffect;

/// Végső effectkezelő: Replacement + Continuous + Execute
impl Gre {
    pub fn handle_effect(&mut self, effect: Effect) {
        let replaced = if self.replacement_effects.is_empty() {
            vec![effect]
        } else {
            self.apply_replacement(&effect, 0)
        };

        let mut final_effects = Vec::new();
        for mut e in replaced {
            for cont in &self.continuous_effects {
                cont(&mut e);
            }
            final_effects.push(e);
        }

        for e in final_effects {
            self.execute(e);
        }
    }

    pub fn apply_replacement(&self, effect: &Effect, idx: usize) -> Vec<Effect> {
        if idx >= self.replacement_effects.len() {
            return vec![effect.clone()];
        }
        let replacer = &self.replacement_effects[idx];
        if let Some(repls) = (replacer.f)(effect) {
            repls.into_iter()
                .flat_map(|eff| self.apply_replacement(&eff, idx + 1))
                .collect()
        } else {
            self.apply_replacement(effect, idx + 1)
        }
    }

    pub fn execute(&mut self, effect: Effect) {
        info!("GRE.execute() → Indul az effect végrehajtása: {:?}", effect);
        match effect {
            // --- Ehhez bemásoljuk a korábbi match-ágakat (CreateEnchantmentToken, stb.)
            // Ahol plusz logok is voltak, maradnak.

            _ => {
                info!("Executing effect: {:?}", effect);
            }
        }
    }
}

/// Néhány segédfüggvény
pub fn clone_card(
    original: &Card,
    new_power: Option<i32>,
    new_toughness: Option<i32>,
    added_flags: Option<CardTypeFlags>,
) -> Card {
    let mut cloned = original.clone();
    if let CardType::Creature(ref mut cr) = cloned.card_type {
        if let Some(p) = new_power {
            cr.power = p;
        }
        if let Some(t) = new_toughness {
            cr.toughness = t;
        }
    }
    if let Some(flags) = added_flags {
        cloned.type_flags |= flags;
    }
    cloned
}

pub fn create_creature_token(
    gre: &mut Gre,
    name: &str,
    power: i32,
    toughness: i32,
    creature_types: Vec<crate::app::card_attribute::CreatureType>,
) {
    info!("create_creature_token() -> name='{}', power={}, toughness={}, types={:?}",
          name, power, toughness, creature_types);

    let mut new_card = Card::new(
        name,
        CardType::Creature(Creature {
            power,
            toughness,
            summoning_sickness: true,
            abilities: Vec::new(),
            types: creature_types,
        }),
        ManaCost::free(),
    )
        .with_added_type(CardTypeFlags::CREATURE)
        .with_added_type(CardTypeFlags::TOKEN);

    gre.enter_battlefield(&mut new_card);
}

pub fn create_clone_card(gre: &mut Gre, mut cloned: Card) {
    info!("create_clone_card() -> cloning card '{}' (id={}) and placing on battlefield", cloned.name, cloned.card_id);
    gre.enter_battlefield(&mut cloned);
}

pub fn current_stack_target(gre: &Gre) -> Option<Card> {
    if let Some(pe) = gre.stack.peek() {
        match &pe.entry {
            StackEntry::Spell { target_creature: Some(t), .. } => Some(t.clone()),
            _ => None,
        }
    } else {
        None
    }
}

pub fn replace_targeted_filter_with_exact(gre: &Gre, effect: Effect, tcard: &Card) -> Effect {
    debug!(
        "replace_targeted_filter_with_exact() - Start: effect={:?}, tcard='{}'(id={})",
        effect,
        tcard.name,
        tcard.card_id
    );

    let result = match effect {
        Effect::ModifyStats {
            power_delta,
            toughness_delta,
            duration,
            target,
        } => {
            debug!("  → ModifyStats branch. Eredeti target={:?}", target);

            let new_t = if target == TargetFilter::Creature {
                info!("    TargetFilter::Creature → lecseréljük ExactCardID({})-re", tcard.card_id);
                TargetFilter::ExactCardID(tcard.card_id)
            } else {
                target
            };

            Effect::ModifyStats {
                power_delta,
                toughness_delta,
                duration,
                target: new_t,
            }
        }

        Effect::GrantAbility {
            ability,
            duration,
            target,
        } => {
            debug!("  → GrantAbility branch. Eredeti target={:?}", target);

            let new_t = if target == TargetFilter::Creature {
                info!("    TargetFilter::Creature → ExactCardID({})", tcard.card_id);
                TargetFilter::ExactCardID(tcard.card_id)
            } else {
                target
            };

            Effect::GrantAbility {
                ability,
                duration,
                target: new_t,
            }
        }

        Effect::TargetedEffects { sub_effects } => {
            debug!("  → TargetedEffects branch, sub_effects len={}", sub_effects.len());

            let replaced_subs = sub_effects
                .into_iter()
                .map(|sub| {
                    debug!("    TargetedEffects - belső sub: {:?}", sub);
                    replace_targeted_filter_with_exact(gre, sub, tcard)
                })
                .collect();

            Effect::TargetedEffects {
                sub_effects: replaced_subs
            }
        }

        Effect::WhenTargetDiesThisTurn { effect } => {
            debug!("  → WhenTargetDiesThisTurn, effect={:?}", effect);
            Effect::WhenTargetDiesThisTurn { effect }
        }

        other => {
            debug!("  → Egyéb effect, nincs célcserélés: {:?}", other);
            other
        }
    };

    debug!("replace_targeted_filter_with_exact() - End: returning={:?}", result);
    result
}