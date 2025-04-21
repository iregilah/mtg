use std::collections::HashMap;
use once_cell::sync::Lazy;
use crate::app::card_library::{build_card_library, CardType};
use crate::app::card_attribute::{Trigger, Effect};

//
// A tiny test harness for “Creature + Effects”
//
#[derive(Debug)]
struct CreatureState {
    base_power:   i32,
    base_tough:   i32,
    /// permanent +1/+1 counters
    counters:     i32,
    /// permanent enchantment buffs (Monster Role)
    enchants:     Vec<String>,
    /// temporary until‐EOT buffs, collected each time you cast
    temp_power:   i32,
    temp_tough:   i32,
    has_haste:    bool,
    has_trample:  bool,
}

impl CreatureState {
    fn new(p: i32, t: i32) -> Self {
        Self {
            base_power: p,
            base_tough: t,
            counters: 0,
            enchants: Vec::new(),
            temp_power: 0,
            temp_tough: 0,
            has_haste: false,
            has_trample: false,
        }
    }

    /// Apply one single Effect, interpreting it according to our rules.
    fn apply(&mut self, eff: &Effect) {
        match eff {
            Effect::SelfAttributeChange(ch) => {
                // decide whether this came from Valiant (permanent)
                // vs. Monstrous/Felonious Rage (temporary):
                // we treat any +1/+1 from Valiant as permanent,
                // and +2/+0 from Rage as temporary.
                if ch.power == 1 && ch.toughness == 1 {
                    // assume Valiant → permanent counter
                    self.counters += 1;
                } else {
                    // Rage: +2/+0 or Felonious
                    self.temp_power += ch.power;
                    self.temp_tough += ch.toughness;
                }
            }
            Effect::AttachEnchantment { enchantment } => {
                // Monster Role or Detective token
                if enchantment.name == "Monster Role" {
                    self.enchants.push(enchantment.name.clone());
                    // enchantment buff: +1/+1 & Trample
                    self.counters += 1;         // treat as a “pseudo‐counter”
                    self.has_trample = true;
                } else if enchantment.name == "Detective 2/2" {
                    // we put a fresh 2/2 token on battlefield
                    // but for now we just record it as an enchant
                    self.enchants.push("Detective 2/2".into());
                }
            }
            Effect::Haste => {
                self.has_haste = true;
            }
            _ => {
                // other Effects we ignore in this harness
            }
        }
    }

    /// What is the _current_ power/toughness of this creature?
    fn current_pt(&self) -> (i32,i32) {
        let mut p = self.base_power + self.counters + self.temp_power;
        let mut t = self.base_tough + self.counters + self.temp_tough;
        (p,t)
    }

    /// At end of turn, we clear all the *temporary* buffs
    fn end_of_turn(&mut self) {
        self.temp_power = 0;
        self.temp_tough = 0;
        self.has_haste = false;
    }

    /// Reset for a brand‐new turn (so valiant can trigger again)
    fn new_turn(&mut self) {
        self.end_of_turn();
        // ValiantAttribute::used would be reset in the real card,
        // but in this harness we just re‑build state each turn.
    }
}

//
// Build a globally‐shared library once
//
static LIB: Lazy<HashMap<String, crate::app::card_library::Card>> = Lazy::new(|| {
    build_card_library()
});

//
// Helpers to drive triggers
//
fn trigger_spell_resolved(name: &str, creature: &mut CreatureState) {
    let card = &LIB[name];
    let effects = card.trigger_by(&Trigger::Custom("OnCastResolved".into()));
    for eff in &effects {
        creature.apply(eff);
    }
}
fn trigger_targeted(name: &str, creature: &mut CreatureState) {
    let card = &LIB[name];
    let effects = card.trigger_by(&Trigger::OnTargeted);
    for eff in &effects {
        creature.apply(eff);
    }
}
fn trigger_death(name: &str, creature: &mut CreatureState, opponent_life: &mut i32) {
    let card = &LIB[name];
    let effects = card.trigger_by(&Trigger::OnDeath);
    for eff in &effects {
        match eff {
            Effect::DamageTarget { damage, .. } => {
                if let Some(ref special) = damage.special {
                    assert_eq!(special, "CURRENT_POWER");
                    let (p,_) = creature.current_pt();
                    *opponent_life -= p;
                } else {
                    *opponent_life -= damage.amount as i32;
                }
            }
            Effect::AttachEnchantment { token } if token.name == "Detective 2/2" => {
                creature.apply(eff);
            }
            _ => {}
        }
    }
}
