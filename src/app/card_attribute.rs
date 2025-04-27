// app/card_attribute.rs

use std::any::Any;
use crate::app::game_state::GamePhase;
use tracing::{debug, info, warn};

/// Everything an attribute can do when it fires.
#[derive(Debug, Clone, PartialEq)]
pub enum Effect {
    SelfAttributeChange(AttributeChange),
    DamageTarget { damage: Damage, target_filter: TargetFilter },
    DestroyTarget  { target_filter: TargetFilter },
    ExileTarget    { target_filter: TargetFilter },
    Poliferate     { counter_type: CounterType },
    HealSelfToFull,
    SpawnNewCreature,
    SetSelfHealthToOne,
    RemoveAttribute,
    AttachToken   { token: Token },
    AttachEnchantment { enchantment: Enchantment },
    Haste,
    DrawCard,
    AddMana       { mana_type: String },
    CreateRole    { role: String },

    /// Új variáns: késleltetett effektus egy későbbi fázisban, opcionális függőségekkel
    Delayed {
        effect: Box<Effect>,
        phase:   GamePhase,
        deps:    Vec<usize>,
    },
}

impl Eq for Effect {
    fn assert_receiver_is_total_eq(&self) {}
}

/// Ways to proliferate counters
#[derive(Debug, Clone, PartialEq)]
pub enum CounterType {
    PlusOnePlusOne,
    Loyalty,
}

/// The various moments in a game that attributes can listen for.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Trigger {
    OnDeath,
    OnTargeted,
    OnCombatDamage,
    OnSpellCast,
    EndOfTurn,
    Custom(String),
}

/// A plain delta to a creature’s own power/toughness.
#[derive(Debug, Clone)]
pub struct AttributeChange {
    pub power: i32,
    pub toughness: i32,
}
impl PartialEq for AttributeChange {
    fn eq(&self, other: &Self) -> bool {
        self.power == other.power && self.toughness == other.toughness
    }
}

/// A damage instance.
#[derive(Debug, Clone)]
pub struct Damage {
    pub amount: u32,
    pub special: Option<String>,
}

impl PartialEq for Damage {
    fn eq(&self, other: &Self) -> bool {
        self.amount == other.amount && self.special == other.special
    }
}

/// Filters for targets (placeholder implementation).
#[derive(Debug, Clone, PartialEq)]
pub struct TargetFilter {
    pub filter: u8,
}

/// A token to spawn.
#[derive(Debug, Clone, PartialEq)]
pub struct Token {
    pub name: String,
}

/// An enchantment/Aura to attach.
#[derive(Debug, Clone, PartialEq)]
pub struct Enchantment {
    pub name: String,
}

/// Helper trait to allow cloning of boxed trait objects.
pub trait CardAttributeClone {
    fn clone_box(&self) -> Box<dyn CardAttribute>;
}

impl<T> CardAttributeClone for T
where
    T: 'static + CardAttribute + Clone,
{
    fn clone_box(&self) -> Box<dyn CardAttribute> {
        Box::new(self.clone())
    }
}

impl Clone for Box<dyn CardAttribute> {
    fn clone(&self) -> Box<dyn CardAttribute> {
        self.clone_box()
    }
}

/// Core trait: card abilities implement this.
pub trait CardAttribute: Any + CardAttributeClone {
    /// Called when a trigger fires.
    fn on_trigger(&mut self, trigger: &Trigger) -> Option<Effect>;
}

// --- Modify base power/toughness ---
#[derive(Clone)]
pub struct ModifyAttackDefense {
    pub power: i32,
    pub toughness: i32,
}

impl CardAttribute for ModifyAttackDefense {
    fn on_trigger(&mut self, _trigger: &Trigger) -> Option<Effect> {
        Some(Effect::SelfAttributeChange(AttributeChange { power: self.power, toughness: self.toughness }))
    }
}

// --- Proliferate on combat damage ---
#[derive(Clone)]
pub struct PoliferateOnDamage;

impl CardAttribute for PoliferateOnDamage {
    fn on_trigger(&mut self, trigger: &Trigger) -> Option<Effect> {
        if *trigger == Trigger::OnCombatDamage {
            Some(Effect::Poliferate { counter_type: CounterType::PlusOnePlusOne })
        } else {
            None
        }
    }
}

// --- Spawn token on death ---
#[derive(Clone)]
pub struct SpawnTokenOnDeath;

impl CardAttribute for SpawnTokenOnDeath {
    fn on_trigger(&mut self, trigger: &Trigger) -> Option<Effect> {
        if *trigger == Trigger::OnDeath {
            Some(Effect::SpawnNewCreature)
        } else {
            None
        }
    }
}

// --- Damage equal to power on death ---
#[derive(Clone)]
pub struct DamageEqualPowerOnDeath {
    pub damage: Damage,
    pub target_filter: TargetFilter,
}

impl CardAttribute for DamageEqualPowerOnDeath {
    fn on_trigger(&mut self, trigger: &Trigger) -> Option<Effect> {
        if *trigger == Trigger::OnDeath {
            Some(Effect::DamageTarget { damage: self.damage.clone(), target_filter: self.target_filter.clone() })
        } else {
            None
        }
    }
}

// --- Haste on resolution ---
#[derive(Clone)]
pub struct HasteAttribute;

impl CardAttribute for HasteAttribute {
    fn on_trigger(&mut self, trigger: &Trigger) -> Option<Effect> {
        if let Trigger::Custom(s) = trigger {
            if s == "OnCastResolved" {
                return Some(Effect::Haste);
            }
        }
        None
    }
}

// --- Burst Lightning with kicker ---
#[derive(Clone)]
pub struct BurstLightningAttribute {
    pub kicked: bool,
}

impl CardAttribute for BurstLightningAttribute {
    fn on_trigger(&mut self, trigger: &Trigger) -> Option<Effect> {
        if let Trigger::Custom(s) = trigger {
            if s == "OnCastResolved" {
                let dmg = if self.kicked { 4 } else { 2 };
                return Some(Effect::DamageTarget {
                    damage: Damage { amount: dmg, special: None },
                    target_filter: TargetFilter { filter: 0 },
                });
            }
        }
        None
    }
}

// --- Simple damage on resolve (Lightning Strike) ---
#[derive(Clone)]
pub struct DealDamageOnResolve {
    pub amount: u32,
}

impl CardAttribute for DealDamageOnResolve {
    fn on_trigger(&mut self, trigger: &Trigger) -> Option<Effect> {
        if let Trigger::Custom(s) = trigger {
            if s == "OnCastResolved" {
                return Some(Effect::DamageTarget {
                    damage: Damage { amount: self.amount, special: None },
                    target_filter: TargetFilter { filter: 0 },
                });
            }
        }
        None
    }
}

// --- +2/+0 and Role bonus ---
#[derive(Clone)]
pub struct PlusTwoPlusZeroAndRole {
    pub role: String,
}

impl CardAttribute for PlusTwoPlusZeroAndRole {
    fn on_trigger(&mut self, trigger: &Trigger) -> Option<Effect> {
        if let Trigger::Custom(s) = trigger {
            if s == "OnCastResolved" {
                return Some(Effect::SelfAttributeChange(AttributeChange { power: 2, toughness: 0 }));
            }
        }
        None
    }
}

// --- Felonious Rage combo ---
#[derive(Clone)]
pub struct FeloniousRageAttribute;

impl CardAttribute for FeloniousRageAttribute {
    fn on_trigger(&mut self, trigger: &Trigger) -> Option<Effect> {
        match trigger {
            Trigger::OnTargeted => Some(Effect::SelfAttributeChange(AttributeChange { power: 2, toughness: 0 })),
            Trigger::Custom(s) if s == "OnCastResolved" => Some(Effect::Haste),
            Trigger::OnDeath => Some(Effect::AttachToken { token: Token { name: "Detective 2/2".into() } }),
            _ => None,
        }
    }
}

// --- Proliferate on spell cast (Electrostatic Infantry) ---
#[derive(Clone)]
pub struct ProliferateOnSpellCast;

impl CardAttribute for ProliferateOnSpellCast {
    fn on_trigger(&mut self, trigger: &Trigger) -> Option<Effect> {
        if *trigger == Trigger::OnSpellCast {
            Some(Effect::Poliferate { counter_type: CounterType::PlusOnePlusOne })
        } else {
            None
        }
    }
}

// --- Prowess (Monastery Swiftspear) ---
#[derive(Clone)]
pub struct ProwessAttribute;

impl CardAttribute for ProwessAttribute {
    fn on_trigger(&mut self, trigger: &Trigger) -> Option<Effect> {
        if *trigger == Trigger::OnSpellCast {
            Some(Effect::SelfAttributeChange(AttributeChange { power: 1, toughness: 1 }))
        } else {
            None
        }
    }
}

// --- Valiant (Heartfire Hero) ---
#[derive(Clone)]
pub struct ValiantAttribute {
    pub used: bool,
}

impl CardAttribute for ValiantAttribute {
    fn on_trigger(&mut self, trigger: &Trigger) -> Option<Effect> {
        match trigger {
            Trigger::OnTargeted if !self.used => {
                self.used = true;
                Some(Effect::SelfAttributeChange(AttributeChange { power: 1, toughness: 1 }))
            }
            Trigger::OnDeath => Some(Effect::DamageTarget {
                damage: Damage {
                    // special payload: "CurrentPower"
                    amount: 0,
                    special: Some("CURRENT_POWER".into()),
                },
                target_filter: TargetFilter { filter: 0 },
            }),
            _ => None,
        }
    }
}

// --- Enchantment buff (Demonic Ruckus) ---
#[derive(Clone)]
pub struct EnchantCreatureBuff {
    pub power: i32,
    pub toughness: i32,
    pub abilities: Vec<String>,
}

impl CardAttribute for EnchantCreatureBuff {
    fn on_trigger(&mut self, trigger: &Trigger) -> Option<Effect> {
        if let Trigger::Custom(s) = trigger {
            if s == "OnCastResolved" {
                return Some(Effect::AttachEnchantment { enchantment: Enchantment { name: "Demonic Ruckus".into() } });
            }
        }
        None
    }
}

#[derive(Clone)]
pub struct DrawOnAuraDies;

impl CardAttribute for DrawOnAuraDies {
    fn on_trigger(&mut self, trigger: &Trigger) -> Option<Effect> {
        if *trigger == Trigger::OnDeath {
            Some(Effect::DrawCard)
        } else {
            None
        }
    }
}

// --- Mana abilities (Rockface Village) ---
#[derive(Clone)]
pub enum ManaCondition { Any, CreatureSpell }

#[derive(Clone)]
pub struct AddManaAbility {
    pub mana_type: String,
    pub condition: ManaCondition,
}

impl CardAttribute for AddManaAbility {
    fn on_trigger(&mut self, trigger: &Trigger) -> Option<Effect> {
        if let Trigger::Custom(s) = trigger {
            match (s.as_str(), &self.condition) {
                ("AddRedMana", _) => return Some(Effect::AddMana { mana_type: "Red".into() }),
                ("AddColorlessMana", ManaCondition::CreatureSpell) => {
                    return Some(Effect::AddMana { mana_type: "Colorless".into() })
                }
                _ => {}
            }
        }
        None
    }
}

// --- +1/+0 & Haste on any spell (example) ---
#[derive(Clone)]
pub struct PlusOneZeroAndHasteOnSpell {
    pub color_filter: String,
}

impl CardAttribute for PlusOneZeroAndHasteOnSpell {
    fn on_trigger(&mut self, trigger: &Trigger) -> Option<Effect> {
        if *trigger == Trigger::OnSpellCast {
            Some(Effect::SelfAttributeChange(AttributeChange { power: 1, toughness: 0 }))
        } else {
            None
        }
    }
}

#[derive(Clone)]
pub struct AttachMonsterRole;

impl CardAttribute for AttachMonsterRole {
    fn on_trigger(&mut self, trigger: &Trigger) -> Option<Effect> {
        if let Trigger::Custom(s) = trigger {
            if s == "OnCastResolved" {
                return Some(Effect::AttachEnchantment {
                    enchantment: Enchantment { name: "Monster Role".into() }
                });
            }
        }
        None
    }
}

// --- Monster Role token: grants permanent +1/+1 & Trample ---
#[derive(Clone)]
pub struct MonsterRoleBuff;

impl CardAttribute for MonsterRoleBuff {
    fn on_trigger(&mut self, _trigger: &Trigger) -> Option<Effect> {
        // static buff, no trigger needed
        // we register this as a continuous effect in GRE instead
        None
    }
}

// Az új attribute, ami egy késleltetett +1/+1 counter-t ütemez:
#[derive(Clone)]
pub struct DelayedCounterAttribute {
    pub delay_phase: GamePhase,
}

impl CardAttribute for DelayedCounterAttribute {
    fn on_trigger(&mut self, trigger: &Trigger) -> Option<Effect> {
        if let Trigger::Custom(s) = trigger {
            if s == "OnCastResolved" {
                return Some(Effect::Delayed {
                    effect: Box::new(Effect::SelfAttributeChange(AttributeChange { power: 0, toughness: 1 })),
                    phase:   self.delay_phase,
                    deps:    vec![],
                });
            }
        }
        None
    }
}

/// Lifelink: a harci sebzés után életet nyersz ugyanolyan mértékben, de nem termel Effect-et.
#[derive(Clone)]
pub struct LifelinkAttribute;

impl CardAttribute for LifelinkAttribute {
    fn on_trigger(&mut self, _trigger: &Trigger) -> Option<Effect> {
        None
    }
}

/// Deathtouch: 1 sebzés is öl, passzív marker
#[derive(Clone)]
pub struct DeathtouchAttribute;

impl CardAttribute for DeathtouchAttribute {
    fn on_trigger(&mut self, _trigger: &Trigger) -> Option<Effect> {
        None
    }
}

/// Trample: blokkoló után felesleges sebzés a játékosra, passzív marker
#[derive(Clone)]
pub struct TrampleAttribute;

impl CardAttribute for TrampleAttribute {
    fn on_trigger(&mut self, _trigger: &Trigger) -> Option<Effect> {
        None
    }
}