// src/app/card_library.rs

use crate::app::game_state::GamePhase;
use std::collections::HashMap;
use crate::app::card_attribute::*;
use crate::app::card_attribute::CardAttribute;

const CACOPHONY_SCAMP: &str        = "Cacophony Scamp";
const MONASTERY_SWIFTSPEAR: &str   = "Monastery Swiftspear";
const ELECTROSTATIC_INFANTRY: &str = "Electrostatic Infantry";
const HEARTFIRE_HERO: &str         = "Heartfire Hero";
const FELONIOUS_RAGE: &str         = "Felonious Rage";
const MONSTROUS_RAGE: &str         = "Monstrous Rage";
const MONSTER_ROLE: &str           = "Monster Role";
const BLAZING_CRESCENDO: &str      = "Blazing Crescendo";
const DEMONIC_RUCKUS: &str         = "Demonic Ruckus";
const BURST_LIGHTNING: &str        = "Burst Lightning";
const LIGHTNING_STRIKE: &str       = "Lightning Strike";
const MOUNTAIN: &str               = "Mountain";
const ROCKFACE_VILLAGE: &str       = "Rockface Village";

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CardType {
    Creature(Creature),
    Instant(Instant_),
    Enchantment(Enchantment),
    Land,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Creature {
    pub name: String,
    pub power: i32,
    pub toughness: i32,
    pub summoning_sickness: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Instant_ { pub name: String }

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Enchantment { pub name: String }

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ManaCost {
    pub colorless: u32,
    pub red: u32,
    pub blue: u32,
    pub green: u32,
    pub black: u32,
    pub white: u32,
}

impl ManaCost {
    pub fn new(colorless: u32, red: u32, blue: u32, green: u32, black: u32, white: u32) -> Self {
        ManaCost { colorless, red, blue, green, black, white }
    }
    pub fn free() -> Self { ManaCost::new(0,0,0,0,0,0) }
    pub fn total(&self) -> u32 { self.colorless + self.red + self.blue + self.green + self.black + self.white }
    pub fn colored(&self) -> u32 {
        self.red + self.blue + self.green + self.black + self.white
    }
}

#[derive(Clone, Debug)]
pub struct Card {
    pub name: String,
    pub card_type: CardType,
    pub mana_cost: ManaCost,
    pub attributes: Vec<Box<dyn CardAttribute>>,
    pub triggers: Vec<Trigger>,
}

impl PartialEq for Card {
    fn eq(&self, other: &Self) -> bool {
        self.name == other.name
            && self.card_type == other.card_type
            && self.mana_cost == other.mana_cost
            && self.triggers == other.triggers
    }
}
impl Eq for Card {}

impl Card {
    pub fn new(name: &str, card_type: CardType, mana_cost: ManaCost)
               -> Self {
        Card { name: name.into(), card_type, mana_cost, attributes: Vec::new(), triggers: Vec::new() }
    }
    pub fn with(mut self, trigger: Trigger, attr: impl CardAttribute + 'static)
                -> Self {
        self.triggers.push(trigger);
        self.attributes.push(Box::new(attr));
        self
    }
    pub fn trigger_by(&mut self, trigger: &Trigger) -> Vec<Effect> {
        self.triggers
            .iter()
            .zip(self.attributes.iter_mut())
            .filter_map(|(t, attr)| {
                if t == trigger {
                    attr.on_trigger(trigger)
                } else {
                    None
                }
            })
            .collect()
    }
}

pub fn build_card_library() -> HashMap<String, Card> {
    let mut lib = HashMap::new();

    // Cacophony Scamp
    lib.insert(CACOPHONY_SCAMP.into(),
               Card::new(
                   CACOPHONY_SCAMP,
                   CardType::Creature(Creature { name: CACOPHONY_SCAMP.into(), power:1, toughness:1, summoning_sickness:true }),
                   ManaCost::new(0,1,0,0,0,0)
               )
                   .with(
                       Trigger::OnCombatDamage { filter: TargetFilter::SelfCard },
                       ProliferateAttribute { counter: CounterType::PlusOnePlusOne, player: PlayerSelector::Controller }
                   )
                   .with(
                       Trigger::OnDeath { filter: TargetFilter::SelfCard },
                       TriggeredEffectAttribute {
                           trigger: Trigger::OnDeath { filter: TargetFilter::SelfCard },
                           effect: Effect::Damage { amount: Amount::SourcePower, target: TargetFilter::AnyTarget }
                       }
                   )
    );

    // Monastery Swiftspear
    lib.insert(MONASTERY_SWIFTSPEAR.into(),
               Card::new(
                   MONASTERY_SWIFTSPEAR,
                   CardType::Creature(Creature { name: MONASTERY_SWIFTSPEAR.into(), power:1, toughness:2, summoning_sickness:true }),
                   ManaCost::new(0,1,0,0,0,0)
               )
                   .with(
                       Trigger::OnEnterBattlefield { filter: TargetFilter::SelfCard },
                       GrantAbilityAttribute { ability: KeywordAbility::Haste, duration: Duration::EndOfTurn, target: TargetFilter::SelfCard }
                   )
                   .with(
                       Trigger::OnSpellCast { filter: SpellFilter::InstantOrSorcery },
                       AddCounterAttribute { counter: CounterType::PlusOnePlusOne, amount:1, target: TargetFilter::SelfCard }
                   )
    );

    // Electrostatic Infantry
    lib.insert(ELECTROSTATIC_INFANTRY.into(),
               Card::new(
                   ELECTROSTATIC_INFANTRY,
                   CardType::Creature(Creature { name: ELECTROSTATIC_INFANTRY.into(), power:1, toughness:2, summoning_sickness:true }),
                   ManaCost::new(1,1,0,0,0,0)
               )
                   .with(
                       Trigger::OnEnterBattlefield { filter: TargetFilter::SelfCard },
                       GrantAbilityAttribute { ability: KeywordAbility::Trample, duration: Duration::Permanent, target: TargetFilter::SelfCard }
                   )
                   .with(
                       Trigger::OnSpellCast { filter: SpellFilter::InstantOrSorcery },
                       AddCounterAttribute { counter: CounterType::PlusOnePlusOne, amount:1, target: TargetFilter::SelfCard }
                   )
    );

    // Heartfire Hero
    lib.insert(HEARTFIRE_HERO.into(),
               Card::new(
                   HEARTFIRE_HERO,
                   CardType::Creature(Creature { name: HEARTFIRE_HERO.into(), power:1, toughness:1, summoning_sickness:true }),
                   ManaCost::new(0,1,0,0,0,0)
               )
                   .with(
                       // Valiant: first time targeted each turn
                       Trigger::OnTargeted { filter: TargetFilter::SelfCard },
                       FirstTimePerTurnAttribute {
                           base_trigger: Trigger::OnTargeted { filter: TargetFilter::SelfCard },
                           reset_phase: GamePhase::End,
                           action: Effect::AddCounter { counter: CounterType::PlusOnePlusOne, amount:1, target: TargetFilter::SelfCard },
                           used: false,
                       }
                   )
                   .with(
                       // Death trigger
                       Trigger::OnDeath { filter: TargetFilter::SelfCard },
                       TriggeredEffectAttribute {
                           trigger: Trigger::OnDeath { filter: TargetFilter::SelfCard },
                           effect: Effect::Damage { amount: Amount::SourcePower, target: TargetFilter::Player }
                       }
                   )
    );

    // Felonious Rage
    lib.insert(FELONIOUS_RAGE.into(),
               Card::new(
                   FELONIOUS_RAGE,
                   CardType::Instant(Instant_ { name: FELONIOUS_RAGE.into() }),
                   ManaCost::new(0,1,0,0,0,0)
               )
                   .with(
                       Trigger::OnCastResolved,
                       BuffAttribute { power:2, toughness:0, duration: Duration::EndOfTurn, target: TargetFilter::ControllerCreature }
                   )
                   .with(
                       Trigger::OnCastResolved,
                       GrantAbilityAttribute { ability: KeywordAbility::Haste, duration: Duration::EndOfTurn, target: TargetFilter::ControllerCreature }
                   )
                   .with(
                       Trigger::OnDeath { filter: TargetFilter::SelfCard },
                       CreateTokenAttribute { token: Token { name: "Detective 2/2".into() }, player: PlayerSelector::Controller }
                   )
    );

    // Monstrous Rage
    lib.insert(MONSTROUS_RAGE.into(),
               Card::new(
                   MONSTROUS_RAGE,
                   CardType::Instant(Instant_ { name: MONSTROUS_RAGE.into() }),
                   ManaCost::new(0,1,0,0,0,0)
               )
                   .with(
                       Trigger::OnCastResolved,
                       BuffAttribute { power:2, toughness:0, duration: Duration::EndOfTurn, target: TargetFilter::Creature }
                   )
                   .with(
                       Trigger::OnCastResolved,
                       CreateEnchantmentAttribute {
                           enchantment: crate::app::card_attribute::Enchantment { name: MONSTER_ROLE.into() },
                           target: TargetFilter::Creature,
                       }
                   )
    );

    // Monster Role token (enchantment)
    lib.insert(MONSTER_ROLE.into(),
               Card::new(
                   MONSTER_ROLE,
                   CardType::Enchantment(Enchantment { name: MONSTER_ROLE.into() }),
                   ManaCost::free()
               )
               // continuous effect (enchantment) handled by rules engine
    );

    // Blazing Crescendo
    lib.insert(BLAZING_CRESCENDO.into(),
               Card::new(
                   BLAZING_CRESCENDO,
                   CardType::Instant(Instant_ { name: BLAZING_CRESCENDO.into() }),
                   ManaCost::new(1,1,0,0,0,0)
               )
                   .with(
                       Trigger::OnCastResolved,
                       BuffAttribute { power:3, toughness:1, duration: Duration::EndOfTurn, target: TargetFilter::Creature }
                   )
                   .with(
                       Trigger::OnCastResolved,
                       ExileAndPlayAttribute { count:1, player: PlayerSelector::Controller, duration: Duration::NextTurnEnd }
                   )
    );

    // Demonic Ruckus (Aura)
    lib.insert(DEMONIC_RUCKUS.into(),
               Card::new(
                   DEMONIC_RUCKUS,
                   CardType::Enchantment(Enchantment { name: DEMONIC_RUCKUS.into() }),
                   ManaCost::new(0,1,0,0,0,0)
               )
                   .with(
                       Trigger::OnCastResolved,
                       BuffAttribute { power:1, toughness:1, duration: Duration::Permanent, target: TargetFilter::ControllerCreature }
                   )
                   .with(
                       Trigger::OnCastResolved,
                       GrantAbilityAttribute { ability: KeywordAbility::Menace, duration: Duration::Permanent, target: TargetFilter::ControllerCreature }
                   )
                   .with(
                       Trigger::OnCastResolved,
                       GrantAbilityAttribute { ability: KeywordAbility::Trample, duration: Duration::Permanent, target: TargetFilter::ControllerCreature }
                   )
                   .with(
                       Trigger::OnDeath { filter: TargetFilter::SelfCard },
                       TriggeredEffectAttribute {
                           trigger: Trigger::OnDeath { filter: TargetFilter::SelfCard },
                           effect: Effect::DrawCards { count:1, player: PlayerSelector::Controller }
                       }
                   )
    );

    // Burst Lightning (with kicker)
    lib.insert(BURST_LIGHTNING.into(),
               Card::new(
                   BURST_LIGHTNING,
                   CardType::Instant(Instant_ { name: BURST_LIGHTNING.into() }),
                   ManaCost::new(4,1,0,0,0,0)
               )
                   .with(
                       Trigger::OnCastResolved,
                       ConditionalAttribute {
                           trigger: Trigger::OnCastResolved,
                           condition: Condition::SpellWasKicked,
                           effect_if_true: Effect::Damage { amount: Amount::Fixed(4), target: TargetFilter::AnyTarget },
                           effect_if_false: Some(Effect::Damage { amount: Amount::Fixed(2), target: TargetFilter::AnyTarget }),
                       }
                   )
    );

    // Lightning Strike
    lib.insert(LIGHTNING_STRIKE.into(),
               Card::new(
                   LIGHTNING_STRIKE,
                   CardType::Instant(Instant_ { name: LIGHTNING_STRIKE.into() }),
                   ManaCost::new(0,1,0,0,0,0)
               )
                   .with(
                       Trigger::OnCastResolved,
                       TriggeredEffectAttribute { trigger: Trigger::OnCastResolved, effect: Effect::Damage { amount: Amount::Fixed(3), target: TargetFilter::AnyTarget } }
                   )
    );

    // Basic lands (no on-chain attributes)
    lib.insert(MOUNTAIN.into(), Card::new(MOUNTAIN, CardType::Land, ManaCost::free()));
    lib.insert(ROCKFACE_VILLAGE.into(), Card::new(ROCKFACE_VILLAGE, CardType::Land, ManaCost::free()));

    lib
}