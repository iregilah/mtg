// src/app/card_library.rs


use crate::app::game_state::GamePhase;
use std::collections::HashMap;
use crate::app::card_attribute::*;
use crate::app::card_attribute::CardAttribute;
use crate::app::gre::ActivatedAbility;

const CACOPHONY_SCAMP: &str = "Cacophony Scamp";
const MONASTERY_SWIFTSPEAR: &str = "Monastery Swiftspear";
const ELECTROSTATIC_INFANTRY: &str = "Electrostatic Infantry";
const HEARTFIRE_HERO: &str = "Heartfire Hero";
const FELONIOUS_RAGE: &str = "Felonious Rage";
const MONSTROUS_RAGE: &str = "Monstrous Rage";
const MONSTER_ROLE: &str = "Monster Role";
const BLAZING_CRESCENDO: &str = "Blazing Crescendo";
const DEMONIC_RUCKUS: &str = "Demonic Ruckus";
const BURST_LIGHTNING: &str = "Burst Lightning";
const LIGHTNING_STRIKE: &str = "Lightning Strike";
const MOUNTAIN: &str = "Mountain";
const ROCKFACE_VILLAGE: &str = "Rockface Village";

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CardType {
    Creature(Creature),
    Instant(Instant_),
    Enchantment(Enchantment),
    Land,
    Token,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Creature {
    pub name: String,
    pub power: i32,
    pub toughness: i32,
    pub summoning_sickness: bool,
    pub abilities: Vec<KeywordAbility>,
    pub types: Vec<CreatureType>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Instant_ {
    pub name: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Enchantment {
    pub name: String,
}

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
    pub fn free() -> Self { ManaCost::new(0, 0, 0, 0, 0, 0) }
    pub fn total(&self) -> u32 { self.colorless + self.red + self.blue + self.green + self.black + self.white }
    pub fn colored(&self) -> u32 {
        self.red + self.blue + self.green + self.black + self.white
    }
}

#[derive(Clone, Debug)]
pub struct Card {
    pub name: String,
    pub card_types: Vec<CardType>,
    pub mana_cost: ManaCost,
    pub attributes: Vec<Box<dyn CardAttribute>>,
    pub triggers: Vec<Trigger>,
    pub activated_abilities: Vec<ActivatedAbility>,
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
    pub fn new(name: &str, card_types: Vec<CardType>, mana_cost: ManaCost) -> Self {
        Card {
            name: name.into(),
            card_types,
            mana_cost,
            attributes: Vec::new(),
            triggers: Vec::new(),
            activated_abilities: Vec::new(),
        }
    }
    //TODO: where-esíteni
    pub fn with(mut self, trigger: Trigger, attr: impl CardAttribute + 'static)
                -> Self {
        self.triggers.push(trigger);
        self.attributes.push(Box::new(attr));
        self
    }

    pub fn with_activated(mut self, ability: ActivatedAbility) -> Self {
        self.activated_abilities.push(ability);
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
                   CardType::Creature(Creature {
                       name: CACOPHONY_SCAMP.into(),
                       power: 1,
                       toughness: 1,
                       summoning_sickness: true,
                       abilities: Vec::new(),
                       types: vec![CreatureType::Phyrexian, CreatureType::Goblin, CreatureType::Warrior],
                   }),
                   ManaCost::new(0, 1, 0, 0, 0, 0),
               )
                   .with(
                       Trigger::OnCombatDamage { filter: TargetFilter::SelfCard },
                       ProliferateAttribute { counter: CounterType::PlusOnePlusOne, player: PlayerSelector::Controller },
                   )
                   .with(
                       Trigger::OnDeath { filter: TargetFilter::SelfCard },
                       TriggeredEffectAttribute {
                           trigger: Trigger::OnDeath { filter: TargetFilter::SelfCard },
                           effect: Effect::Damage { amount: Amount::SourcePower, target: TargetFilter::AnyTarget },
                       },
                   ),
    );

    lib.insert(MONASTERY_SWIFTSPEAR.into(),
               Card::new(
                   MONASTERY_SWIFTSPEAR,
                   CardType::Creature(Creature {
                       name: MONASTERY_SWIFTSPEAR.into(),
                       power: 1,
                       toughness: 2,
                       summoning_sickness: true,
                       abilities: Vec::new(),
                       types: vec![CreatureType::Human, CreatureType::Monk],
                   }),
                   ManaCost::new(0, 1, 0, 0, 0, 0),
               )
                   .with(
                       Trigger::OnEnterBattlefield { filter: TargetFilter::SelfCard },
                       GrantAbilityAttribute { ability: KeywordAbility::Haste, duration: Duration::EndOfTurn, target: TargetFilter::SelfCard },
                   )
                   .with(
                       Trigger::OnSpellCast { filter: SpellFilter::InstantOrSorcery },
                       ProwessAttribute { filter: SpellFilter::InstantOrSorcery, power: 1, toughness: 1, duration: Duration::EndOfTurn },
                   ),
    );
    // Electrostatic Infantry
    lib.insert(ELECTROSTATIC_INFANTRY.into(),
               Card::new(
                   ELECTROSTATIC_INFANTRY,
                   CardType::Creature(Creature {
                       name: ELECTROSTATIC_INFANTRY.into(),
                       power: 1,
                       toughness: 2,
                       summoning_sickness: true,
                       abilities: Vec::new(),
                       types: vec![CreatureType::Dwarf, CreatureType::Wizard],
                   }),
                   ManaCost::new(1, 1, 0, 0, 0, 0),
               )
                   .with(
                       Trigger::OnEnterBattlefield { filter: TargetFilter::SelfCard },
                       GrantAbilityAttribute { ability: KeywordAbility::Trample, duration: Duration::Permanent, target: TargetFilter::SelfCard },
                   )
                   .with(
                       Trigger::OnSpellCast { filter: SpellFilter::InstantOrSorcery },
                       AddCounterAttribute { counter: CounterType::PlusOnePlusOne, amount: 1, target: TargetFilter::SelfCard },
                   ),
    );

    // Heartfire Hero
    lib.insert("Heartfire Hero".into(),
               Card::new(
                   "Heartfire Hero",
                   CardType::Creature(Creature {
                       name: "Heartfire Hero".into(),
                       power: 1,
                       toughness: 1,
                       summoning_sickness: true,
                       abilities: Vec::new(),
                       types: vec![CreatureType::Mouse, CreatureType::Soldier],
                   }),
                   ManaCost::new(0, 1, 0, 0, 0, 0),
               )
                   .with(
                       Trigger::OnTargetedFirstTimeEachTurn { filter: TargetFilter::SelfCard },
                       FirstTimePerTurnAttribute {
                           base_trigger: Trigger::OnTargetedFirstTimeEachTurn { filter: TargetFilter::SelfCard },
                           reset_phase: GamePhase::End,
                           action: Effect::AddCounter {
                               counter: CounterType::PlusOnePlusOne,
                               amount: 1,
                               target: TargetFilter::SelfCard,
                           },
                           used: false,
                       },
                   ),
    );

    // Screaming Nemesis komplexitásának részletes kezelése:
    lib.insert("Screaming Nemesis".into(),
               Card::new(
                   "Screaming Nemesis",
                   CardType::Creature(Creature {
                       name: "Screaming Nemesis".into(),
                       power: 3,
                       toughness: 3,
                       summoning_sickness: true,
                       abilities: vec![KeywordAbility::Haste],
                       types: vec![CreatureType::Spirit],
                   }),
                   ManaCost::new(2, 1, 0, 0, 0, 0),
               )
                   .with(
                       Trigger::OnEnterBattlefield { filter: TargetFilter::SelfCard },
                       GrantAbilityAttribute { ability: KeywordAbility::Haste, duration: Duration::Permanent, target: TargetFilter::SelfCard },
                   )
                   .with(
                       Trigger::OnDealtDamage { filter: TargetFilter::SelfCard },
                       TriggeredEffectAttribute {
                           trigger: Trigger::OnDealtDamage { filter: TargetFilter::SelfCard },
                           effect: Effect::Damage { amount: Amount::SourcePower, target: TargetFilter::AnyTarget },
                       },
                   )
                   .with(
                       Trigger::OnDealtDamage { filter: TargetFilter::SelfCard },
                       TriggeredEffectAttribute {
                           trigger: Trigger::OnDealtDamage { filter: TargetFilter::SelfCard },
                           effect: Effect::PreventLifeGain { player: PlayerSelector::Opponent, duration: Duration::Permanent },
                       },
                   ),
    );

    // Card library-be:
    lib.insert("Hired Claw".into(),
               Card::new(
                   "Hired Claw",
                   CardType::Creature(Creature {
                       name: "Hired Claw".into(),
                       power: 1,
                       toughness: 2,
                       summoning_sickness: true,
                       abilities: Vec::new(),
                       types: vec![CreatureType::Lizard, CreatureType::Mercenary],
                   }),
                   ManaCost::new(0, 1, 0, 0, 0, 0),
               )
                   // 1) OnAttackWithCreatureType trigger
                   .with(
                       Trigger::OnAttackWithCreatureType { creature_type: CreatureType::Lizard },
                       TriggeredEffectAttribute {
                           trigger: Trigger::OnAttackWithCreatureType { creature_type: CreatureType::Lizard },
                           effect: Effect::Damage { amount: Amount::Fixed(1), target: TargetFilter::OpponentCreature },
                       },
                   )
                   // 2) Activated ability – építs be egy mezőt Card-ban: activated_abilities: Vec<ActivatedAbility>
                   // majd bot.rs-ben, amikor PassPriority, ott engedélyezd:
                   .with_activated(
                       ActivatedAbility {
                           cost: ManaCost::new(1, 1, 0, 0, 0, 0),
                           condition: Condition::OpponentLostLifeThisTurn,
                           effect: Effect::AddCounter { counter: CounterType::PlusOnePlusOne, amount: 1, target: TargetFilter::SelfCard },
                           activated_this_turn: false,
                       }
                   ),
    );
    lib.insert("Manifold Mouse".into(),
               Card::new(
                   "Manifold Mouse",
                   vec![
                       CardType::Creature(Creature {
                           name: "Manifold Mouse".into(),
                           power: 1,
                           toughness: 2,
                           summoning_sickness: true,
                           abilities: Vec::new(),
                           types: vec![CreatureType::Mouse, CreatureType::Soldier],

                       }),
                   ],
                   ManaCost::new(1, 1, 0, 0, 0, 0),
               )
                   .with(
                       Trigger::AtPhase { phase: GamePhase::BeginningCombat, player: PlayerSelector::Controller },
                       ChooseOnConditionAttribute {
                           choose: 1,
                           options: vec![
                               Effect::GrantAbility {
                                   ability: KeywordAbility::DoubleStrike,
                                   duration: Duration::EndOfTurn,
                                   target: TargetFilter::CreatureType(CreatureType::Mouse),
                               },
                               Effect::GrantAbility {
                                   ability: KeywordAbility::Trample,
                                   duration: Duration::EndOfTurn,
                                   target: TargetFilter::CreatureType(CreatureType::Mouse),
                               },
                           ],
                       },
                   )
                   .with(
                       Trigger::OnCastResolved,
                       OffspringAttribute {
                           template: Card::new(
                               "Manifold Mouse",
                               vec![CardType::Creature(Creature {
                                   name: "Manifold Mouse".into(),
                                   power: 1,
                                   toughness: 2,
                                   summoning_sickness: true,
                                   abilities: Vec::new(),
                                   types: vec![CreatureType::Mouse, CreatureType::Soldier],
                               })
                               ],
                               ManaCost::new(1, 1, 0, 0, 0, 0),
                           ),
                           player: PlayerSelector::Controller,
                       },
                   ),
    );

    // Slickshot Show-Off
    lib.insert("Slickshot Show-Off".into(),
               Card::new(
                   "Slickshot Show-Off",
                   CardType::Creature(Creature {
                       name: "Slickshot Show-Off".into(),
                       power: 1,
                       toughness: 2,
                       summoning_sickness: true,
                       abilities: vec![KeywordAbility::Flying, KeywordAbility::Haste],
                       types: vec![CreatureType::Bird, CreatureType::Wizard],
                   }),
                   ManaCost::new(1, 1, 0, 0, 0, 0),
               )
                   .with(
                       Trigger::OnSpellCast { filter: SpellFilter::InstantOrSorcery },
                       BuffAttribute {
                           power: 2,
                           toughness: 0,
                           duration: Duration::EndOfTurn,
                           target: TargetFilter::SelfCard,
                       },
                   ),
    );

    // Sunset Strikemaster
    lib.insert("Sunset Strikemaster".into(),
               Card::new(
                   "Sunset Strikemaster",
                   CardType::Creature(Creature {
                       name: "Sunset Strikemaster".into(),
                       power: 3,
                       toughness: 1,
                       summoning_sickness: true,
                       abilities: vec![],
                       types: vec![CreatureType::Human, CreatureType::Monk],
                   }),
                   ManaCost::new(1, 1, 0, 0, 0, 0),
               )
                   .with_activated(
                       ActivatedAbility {
                           cost: ManaCost::free(),
                           condition: Condition::Always, // placeholder; implement tap/sacrifice logic
                           effect: Effect::AddMana { red: 1, colorless: 0, blue: 0, green: 0, black: 0, white: 0 },
                           activated_this_turn: false,
                       }
                   ),
    );

    // Emberheart Challenger
    lib.insert("Emberheart Challenger".into(),
               Card::new(
                   "Emberheart Challenger",
                   CardType::Creature(Creature {
                       name: "Emberheart Challenger".into(),
                       power: 2,
                       toughness: 2,
                       summoning_sickness: true,
                       abilities: vec![KeywordAbility::Haste],
                       types: vec![CreatureType::Mouse, CreatureType::Warrior],
                   }),
                   ManaCost::new(1, 1, 0, 0, 0, 0),
               )
                   .with(
                       Trigger::OnSpellCast { filter: SpellFilter::InstantOrSorcery },
                       ProwessAttribute { filter: SpellFilter::InstantOrSorcery, power: 1, toughness: 1, duration: Duration::EndOfTurn },
                   )

                   .with(
                       Trigger::OnTargetedFirstTimeEachTurn { filter: TargetFilter::SelfCard },
                       FirstTimePerTurnAttribute {
                           base_trigger: Trigger::OnTargetedFirstTimeEachTurn { filter: TargetFilter::SelfCard },
                           reset_phase: GamePhase::End,
                           action: Effect::ExileThenPlayFromExile {
                               count: 1,
                               player: PlayerSelector::Controller,
                               duration: Duration::EndOfTurn,
                           },
                           used: false,
                       },
                   ),
    );


    // Felonious Rage
    lib.insert(FELONIOUS_RAGE.into(),
               Card::new(
                   FELONIOUS_RAGE,
                   CardType::Instant(Instant_ { name: FELONIOUS_RAGE.into() }),
                   ManaCost::new(0, 1, 0, 0, 0, 0),
               )
                   .with(
                       Trigger::OnCastResolved,
                       BuffAttribute { power: 2, toughness: 0, duration: Duration::EndOfTurn, target: TargetFilter::ControllerCreature },
                   )
                   .with(
                       Trigger::OnCastResolved,
                       GrantAbilityAttribute { ability: KeywordAbility::Haste, duration: Duration::EndOfTurn, target: TargetFilter::ControllerCreature },
                   )
                   .with(
                       Trigger::OnDeath { filter: TargetFilter::SelfCard },
                       CreateTokenAttribute { token: Token { name: "Detective 2/2".into() }, player: PlayerSelector::Controller },
                   ),
    );

    // Monstrous Rage
    lib.insert(MONSTROUS_RAGE.into(),
               Card::new(
                   MONSTROUS_RAGE,
                   CardType::Instant(Instant_ { name: MONSTROUS_RAGE.into() }),
                   ManaCost::new(0, 1, 0, 0, 0, 0),
               )
                   .with(
                       Trigger::OnCastResolved,
                       BuffAttribute { power: 2, toughness: 0, duration: Duration::EndOfTurn, target: TargetFilter::Creature },
                   )
                   .with(
                       Trigger::OnCastResolved,
                       CreateEnchantmentAttribute {
                           enchantment: crate::app::card_attribute::Enchantment { name: MONSTER_ROLE.into() },
                           target: TargetFilter::Creature,
                       },
                   ),
    );

    // Monster Role token (enchantment)
    lib.insert(MONSTER_ROLE.into(),
               Card::new(
                   MONSTER_ROLE,
                   CardType::Enchantment(Enchantment { name: MONSTER_ROLE.into() }),
                   ManaCost::free(),
               ),
               // continuous effect (enchantment) handled by rules engine
    );

    // Blazing Crescendo
    lib.insert(BLAZING_CRESCENDO.into(),
               Card::new(
                   BLAZING_CRESCENDO,
                   CardType::Instant(Instant_ { name: BLAZING_CRESCENDO.into() }),
                   ManaCost::new(1, 1, 0, 0, 0, 0),
               )
                   .with(
                       Trigger::OnCastResolved,
                       BuffAttribute { power: 3, toughness: 1, duration: Duration::EndOfTurn, target: TargetFilter::Creature },
                   )
                   .with(
                       Trigger::OnCastResolved,
                       ExileAndPlayAttribute { count: 1, player: PlayerSelector::Controller, duration: Duration::NextTurnEnd },
                   ),
    );

    // Demonic Ruckus (Aura)
    lib.insert(DEMONIC_RUCKUS.into(),
               Card::new(
                   DEMONIC_RUCKUS,
                   CardType::Enchantment(Enchantment { name: DEMONIC_RUCKUS.into() }),
                   ManaCost::new(0, 1, 0, 0, 0, 0),
               )
                   .with(
                       Trigger::OnCastResolved,
                       BuffAttribute { power: 1, toughness: 1, duration: Duration::Permanent, target: TargetFilter::ControllerCreature },
                   )
                   .with(
                       Trigger::OnCastResolved,
                       GrantAbilityAttribute { ability: KeywordAbility::Menace, duration: Duration::Permanent, target: TargetFilter::ControllerCreature },
                   )
                   .with(
                       Trigger::OnCastResolved,
                       GrantAbilityAttribute { ability: KeywordAbility::Trample, duration: Duration::Permanent, target: TargetFilter::ControllerCreature },
                   )
                   .with(
                       Trigger::OnDeath { filter: TargetFilter::SelfCard },
                       TriggeredEffectAttribute {
                           trigger: Trigger::OnDeath { filter: TargetFilter::SelfCard },
                           effect: Effect::DrawCards { count: 1, player: PlayerSelector::Controller },
                       },
                   ),
    );

    // Burst Lightning (with kicker)
    lib.insert(BURST_LIGHTNING.into(),
               Card::new(
                   BURST_LIGHTNING,
                   CardType::Instant(Instant_ { name: BURST_LIGHTNING.into() }),
                   ManaCost::new(4, 1, 0, 0, 0, 0),
               )
                   .with(
                       Trigger::OnCastResolved,
                       ConditionalAttribute {
                           trigger: Trigger::OnCastResolved,
                           condition: Condition::SpellWasKicked,
                           effect_if_true: Effect::Damage { amount: Amount::Fixed(4), target: TargetFilter::AnyTarget },
                           effect_if_false: Some(Effect::Damage { amount: Amount::Fixed(2), target: TargetFilter::AnyTarget }),
                       },
                   ),
    );

    // Lightning Strike
    lib.insert(LIGHTNING_STRIKE.into(),
               Card::new(
                   LIGHTNING_STRIKE,
                   CardType::Instant(Instant_ { name: LIGHTNING_STRIKE.into() }),
                   ManaCost::new(0, 1, 0, 0, 0, 0),
               )
                   .with(
                       Trigger::OnCastResolved,
                       TriggeredEffectAttribute { trigger: Trigger::OnCastResolved, effect: Effect::Damage { amount: Amount::Fixed(3), target: TargetFilter::AnyTarget } },
                   ),
    );

    // Basic lands (no on-chain attributes)
    lib.insert(MOUNTAIN.into(), Card::new(MOUNTAIN, CardType::Land, ManaCost::free()));
    lib.insert(ROCKFACE_VILLAGE.into(), Card::new(ROCKFACE_VILLAGE, CardType::Land, ManaCost::free()));

    lib
}