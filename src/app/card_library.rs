//app/card_library.rs

use crate::app::game_state::GamePhase;
use std::collections::HashMap;
use crate::app::card_attribute::*;
use crate::app::card_attribute::CardAttribute;
use tracing::{error, info, warn};
use std::cmp::PartialEq;

#[derive(Debug, Clone)]
#[derive(PartialEq)]
pub enum CardType { Creature(Creature), Instant(Instant_), Enchantment(Enchantment_), Land }

#[derive(Debug, Clone, PartialEq)]
pub struct Creature {
    pub name: String,
    pub summoning_sickness: bool,
    pub power: i32,
    pub toughness: i32,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Instant_ {
    pub name: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Enchantment_ {
    pub name: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ManaCost {
    pub colorless: u32,
    pub red: u32,
    pub blue: u32,
    pub green: u32,
    pub black: u32,
    pub white: u32,
}

impl ManaCost {
    pub fn default() -> Self {
        ManaCost { colorless: 0, red: 0, blue: 0, green: 0, black: 0, white: 0 }
    }
    pub fn colored(&self) -> u32 { self.red + self.blue + self.green + self.black + self.white }
    pub fn total(&self) -> u32 { self.colored() + self.colorless }
}

#[derive(Clone)]
pub struct Card {
    pub name: String,
    pub card_type: CardType,
    pub mana_cost: ManaCost,
    pub attributes: Vec<Box<dyn CardAttribute>>,
    pub triggers: Vec<Trigger>,
}
// Card Default + new() + with_trigger()
impl Default for Card {
    fn default() -> Self {
        Card {
            name: String::new(),
            card_type: CardType::Land,
            mana_cost: ManaCost::default(),
            attributes: Vec::new(),
            triggers: Vec::new(),
        }
    }
}
impl PartialEq for Card {
    fn eq(&self, other: &Self) -> bool {
        // Compare the relevant fields of `Card` to determine equality
        self.name == other.name && self.card_type == other.card_type && self.mana_cost == other.mana_cost
    }
}

impl Eq for Card {}

// kézzel implementáljuk, hogy ne kelljen Debug a CardAttribute-okra is
impl std::fmt::Debug for Card {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Card")
            .field("name", &self.name)
            .field("card_type", &self.card_type)
            .field("mana_cost", &self.mana_cost)
            .field("triggers", &self.triggers)
            .finish()
    }
}

impl Card {

    pub fn with_trigger(mut self, trigger: Trigger, _effect: Effect) -> Self {
        self.triggers.push(trigger);
        self
    }
    pub fn trigger_by(&mut self, trigger: &Trigger) -> Vec<Effect> {
        self.attributes.iter_mut().filter_map(|a| a.on_trigger(trigger)).collect()
    }
}

pub fn build_card_library() -> HashMap<String, Card> {
    let mut lib = HashMap::new();

    // Felonious Rage
    lib.insert("Felonious Rage".into(), Card {
        name: "Felonious Rage".into(),
        card_type: CardType::Instant(Instant_ { name: "Felonious Rage".into() }),
        mana_cost: ManaCost { colorless: 0, red: 1, blue: 0, green: 0, black: 0, white: 0 },
        attributes: vec![Box::new(FeloniousRageAttribute)],
        triggers: vec![
            Trigger::OnTargeted,
            Trigger::OnDeath,
            Trigger::Custom("OnCastResolved".into()),
        ],
    });

    // Burst Lightning
    lib.insert("Burst Lightning".into(), Card {
        name: "Burst Lightning".into(),
        card_type: CardType::Instant(Instant_ { name: "Burst Lightning".into() }),
        mana_cost: ManaCost { colorless: 4, red: 1, blue: 0, green: 0, black: 0, white: 0 },
        attributes: vec![Box::new(BurstLightningAttribute { kicked: false })],
        triggers: vec![
            Trigger::Custom("OnCastResolved".into()),
        ],
    });

    // Lightning Strike
    lib.insert("Lightning Strike".into(), Card {
        name: "Lightning Strike".into(),
        card_type: CardType::Instant(Instant_ { name: "Lightning Strike".into() }),
        mana_cost: ManaCost { colorless: 1, red: 1, blue: 0, green: 0, black: 0, white: 0 },
        attributes: vec![Box::new(DealDamageOnResolve { amount: 3 })],
        triggers: vec![
            Trigger::Custom("OnCastResolved".into()),
        ],
    });

    // Monstrous Rage
    lib.insert("Monstrous Rage".into(), Card {
        name: "Monstrous Rage".into(),
        card_type: CardType::Instant(Instant_ { name: "Monstrous Rage".into() }),
        mana_cost: ManaCost { colorless: 0, red: 1, blue: 0, green: 0, black: 0, white: 0 },
        // first buff +2/+0 until end of turn,
        // *then* attach the Monster Role token,
        // so we need two attributes:
        attributes: vec![
            Box::new(PlusTwoPlusZeroAndRole { role: "Monster".into() }),
            Box::new(AttachMonsterRole),
        ],
        triggers: vec![
            Trigger::Custom("OnCastResolved".into()),
        ],
    });
    // Monster Role token itself:
    lib.insert("Monster Role".into(), Card {
        name: "Monster Role".into(),
        card_type: CardType::Enchantment(Enchantment_ { name: "Monster Role".into() }),
        mana_cost: ManaCost { colorless: 0, red: 0, blue: 0, green: 0, black: 0, white: 0 },
        attributes: vec![ Box::new(MonsterRoleBuff) ],
        triggers: vec![], // no discrete triggers
    });

    // Cacophony Scamp
    lib.insert("Cacophony Scamp".into(), Card {
        name: "Cacophony Scamp".into(),
        card_type: CardType::Creature(Creature {
            name: "Cacophony Scamp".into(),
            summoning_sickness: true,
            power: 1,
            toughness: 1,
        }),
        mana_cost: ManaCost { colorless: 0, red: 1, blue: 0, green: 0, black: 0, white: 0 },
        attributes: vec![
            Box::new(PoliferateOnDamage),
            Box::new(SpawnTokenOnDeath),
            Box::new(DamageEqualPowerOnDeath {
                damage: Damage { amount: 1, special: None },
                target_filter: TargetFilter { filter: 0 },
            }),
        ],
        triggers: vec![
            Trigger::OnCombatDamage,
            Trigger::OnDeath,
        ],
    });

    // Electrostatic Infantry
    lib.insert("Electrostatic Infantry".into(), Card {
        name: "Electrostatic Infantry".into(),
        card_type: CardType::Creature(Creature {
            name: "Electrostatic Infantry".into(),
            summoning_sickness: true,
            power: 1,
            toughness: 2,
        }),
        mana_cost: ManaCost { colorless: 1, red: 1, blue: 0, green: 0, black: 0, white: 0 },
        attributes: vec![
            // trample-t statikusként most nem modellezzük, csak a +1/+1 counter triggerét
            Box::new(ProliferateOnSpellCast),
        ],
        triggers: vec![
            Trigger::OnSpellCast,
        ],
    });

    // Monastery Swiftspear
    lib.insert("Monastery Swiftspear".into(), Card {
        name: "Monastery Swiftspear".into(),
        card_type: CardType::Creature(Creature {
            name: "Monastery Swiftspear".into(),
            summoning_sickness: true,
            power: 1,
            toughness: 2,
        }),
        mana_cost: ManaCost { colorless: 0, red: 1, blue: 0, green: 0, black: 0, white: 0 },
        attributes: vec![
            Box::new(HasteAttribute),
            Box::new(ProwessAttribute),
        ],
        triggers: vec![
            Trigger::Custom("OnCastResolved".into()),
            Trigger::OnSpellCast,
        ],
    });

    // Heartfire Hero
    lib.insert("Heartfire Hero".into(), Card {
        name: "Heartfire Hero".into(),
        card_type: CardType::Creature(Creature {
            name: "Heartfire Hero".into(),
            summoning_sickness: true,
            power: 1,
            toughness: 1,
        }),
        mana_cost: ManaCost { colorless: 0, red: 1, blue: 0, green: 0, black: 0, white: 0 },
        attributes: vec![
            Box::new(ValiantAttribute { used: false }),
        ],
        triggers: vec![
            Trigger::OnTargeted,
            Trigger::OnDeath,
        ],
    });

    // Demonic Ruckus
    lib.insert("Demonic Ruckus".into(), Card {
        name: "Demonic Ruckus".into(),
        card_type: CardType::Enchantment(Enchantment_ { name: "Demonic Ruckus".into() }),
        mana_cost: ManaCost { colorless: 1, red: 1, blue: 0, green: 0, black: 0, white: 0 },
        attributes: vec![
            Box::new(EnchantCreatureBuff {
                power: 1,
                toughness: 1,
                abilities: vec!["Menace".into(), "Trample".into()],
            }),
            Box::new(DrawOnAuraDies),
        ],
        triggers: vec![
            Trigger::Custom("OnCastResolved".into()),
            Trigger::OnDeath,
        ],
    });
    lib.insert("Temporal Distortion".into(), Card {
        name: "Temporal Distortion".into(),
        card_type: CardType::Instant(Instant_ { name: "Temporal Distortion".into() }),
        mana_cost: ManaCost { colorless: 2, red: 0, blue: 2, green: 0, black: 0, white: 0 },
        attributes: vec![ Box::new(DelayedCounterAttribute { delay_phase: GamePhase::PostCombatMain }) ],
        triggers:   vec![ Trigger::Custom("OnCastResolved".into()) ],
    });


    // Rockface Village
    lib.insert("Rockface Village".into(), Card {
        name: "Rockface Village".into(),
        card_type: CardType::Land,
        mana_cost: ManaCost { colorless: 0, red: 0, blue: 0, green: 0, black: 0, white: 0 },
        attributes: vec![
            Box::new(AddManaAbility { mana_type: "Red".into(), condition: ManaCondition::Any }),
            Box::new(AddManaAbility { mana_type: "Colorless".into(), condition: ManaCondition::CreatureSpell }),
            Box::new(PlusOneZeroAndHasteOnSpell { color_filter: "".into() }),
        ],
        triggers: vec![
            Trigger::Custom("AddRedMana".into()),
            Trigger::Custom("AddColorlessMana".into()),
            Trigger::OnSpellCast,
        ],
    });


    lib
}