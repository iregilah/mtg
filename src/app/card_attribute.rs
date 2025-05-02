// src/app/card_attribute.rs

use std::any::Any;
use std::fmt::Debug;
use crate::app::game_state::GamePhase;

/// How long an effect lasts.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Duration {
    EndOfTurn,
    NextTurnEnd,
    Permanent,
}

/// Everything an attribute can do when it fires.
#[derive
(Debug, Clone, PartialEq, Eq)]
pub enum Effect {
    Damage {
        amount: Amount,
        target: TargetFilter,
    },
    DrawCards {
        count: u32,
        player: PlayerSelector,
    },
    GainLife {
        amount: u32,
        player: PlayerSelector,
    },
    ModifyStats {
        power_delta: i32,
        toughness_delta: i32,
        duration: Duration,
        target: TargetFilter,
    },
    GrantAbility {
        ability: KeywordAbility,
        duration: Duration,
        target: TargetFilter,
    },
    AddCounter {
        counter: CounterType,
        amount: u32,
        target: TargetFilter,
    },
    Proliferate {
        counter_type: CounterType,
        player: PlayerSelector,
    },
    CreateToken {
        token: Token,
        player: PlayerSelector,
    },
    CreateEnchantmentToken {
        enchantment: Enchantment,
        target: TargetFilter,
    },
    ExileTop {
        count: u32,
        player: PlayerSelector,
    },
    ExileThenPlayFromExile {
        count: u32,
        player: PlayerSelector,
        duration: Duration,
    },
    Conditional {
        condition: Condition,
        effect_if_true: Box<Effect>,
        effect_if_false: Option<Box<Effect>>,
    },
    Delayed {
        effect: Box<Effect>,
        phase: GamePhase,
        deps: Vec<usize>,
    },
}

/// Dynamic or fixed numeric values (e.g., damage, buff size).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Amount {
    Fixed(i32),
    SourcePower,
    SourceToughness,
}

/// Ways to add counters.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CounterType {
    PlusOnePlusOne,
    Loyalty,
}

/// Keyword abilities granted by effects.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum KeywordAbility {
    Haste,
    Trample,
    Menace,
    Prowess,
    // Extend as needed
    Lifelink,
    Deathtouch,
}

/// Generic trigger conditions.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Trigger {
    OnEnterBattlefield { filter: TargetFilter },
    OnDeath { filter: TargetFilter },
    OnAttack { filter: TargetFilter },
    OnBlock { filter: TargetFilter },
    OnCombatDamage { filter: TargetFilter },
    OnSpellCast { filter: SpellFilter },
    OnTargeted { filter: TargetFilter },
    AtBeginPhase { phase: GamePhase, player: PlayerSelector },
    OnCastResolved,
}

/// Filters for spells that trigger abilities.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SpellFilter {
    InstantOrSorcery,
    Any,
}

/// Which players are affected by effects.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PlayerSelector {
    Controller,
    Opponent,
    AnyPlayer,
}

/// Conditions for conditional effects.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Condition {
    SpellWasKicked,
    FirstTimeThisTurn,
}

/// What can be targeted by effects or triggers.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TargetFilter {
    AnyTarget,
    Creature,
    Player,
    SelfCard,
    ControllerCreature,
    OpponentCreature,
}

/// A token to spawn.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Token {
    pub name: String,
}

/// An enchantment/Aura to attach.
#[derive(Debug, Clone, PartialEq, Eq)]
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

// CardAttribute trait kiegészítése
pub trait CardAttribute: Any + CardAttributeClone + Debug {
    fn on_trigger(&mut self, trigger: &Trigger) -> Option<Effect>;
    fn as_any(&self) -> &dyn Any;  // <<< ide kerül
}


// --- Generic: trigger an arbitrary effect ---
#[derive(Debug, Clone)]
pub struct TriggeredEffectAttribute {
    pub trigger: Trigger,
    pub effect: Effect,
}

impl CardAttribute for TriggeredEffectAttribute {
    fn on_trigger(&mut self, trigger: &Trigger) -> Option<Effect> {
        if *trigger == self.trigger {
            Some(self.effect.clone())
        } else {
            None
        }
    }
    fn as_any(&self) -> &dyn Any {
        self
    }
}


// --- Generic: buff until duration ---
#[derive(Debug, Clone)]
pub struct BuffAttribute {
    pub power: i32,
    pub toughness: i32,
    pub duration: Duration,
    pub target: TargetFilter,
}

impl CardAttribute for BuffAttribute {
    fn on_trigger(&mut self, trigger: &Trigger) -> Option<Effect> {
        if *trigger == Trigger::OnCastResolved
            || matches!(trigger, Trigger::OnEnterBattlefield { .. })
        {
            Some(Effect::ModifyStats {
                power_delta: self.power,
                toughness_delta: self.toughness,
                duration: self.duration.clone(),
                target: self.target.clone(),
            })
        } else {
            None
        }
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

// --- Generic: grant a keyword ability until duration ---
#[derive(Debug, Clone)]
pub struct GrantAbilityAttribute {
    pub ability: KeywordAbility,
    pub duration: Duration,
    pub target: TargetFilter,
}

impl CardAttribute for GrantAbilityAttribute {
    fn on_trigger(&mut self, trigger: &Trigger) -> Option<Effect> {
        if *trigger == Trigger::OnCastResolved {
            Some(Effect::GrantAbility {
                ability: self.ability.clone(),
                duration: self.duration.clone(),
                target: self.target.clone(),
            })
        } else {
            None
        }
    }
    fn as_any(&self) -> &dyn Any {
        self
    }
}

// --- Generic: add counters ---
#[derive(Debug, Clone)]
pub struct AddCounterAttribute {
    pub counter: CounterType,
    pub amount: u32,
    pub target: TargetFilter,
}

impl CardAttribute for AddCounterAttribute {
    fn on_trigger(&mut self, trigger: &Trigger) -> Option<Effect> {
        if matches!(trigger, Trigger::OnSpellCast { .. }) {
            Some(Effect::AddCounter {
                counter: self.counter.clone(),
                amount: self.amount,
                target: self.target.clone(),
            })
        } else {
            None
        }
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

// --- Generic: proliferate ---
#[derive(Debug, Clone)]
pub struct ProliferateAttribute {
    pub counter: CounterType,
    pub player: PlayerSelector,
}

impl CardAttribute for ProliferateAttribute {
    fn on_trigger(&mut self, trigger: &Trigger) -> Option<Effect> {
        if matches!(trigger, Trigger::OnCombatDamage { .. }) {
            Some(Effect::Proliferate { counter_type: self.counter.clone(), player: self.player.clone() })
        } else {
            None
        }
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

// --- Generic: create token ---
#[derive(Debug, Clone)]
pub struct CreateTokenAttribute {
    pub token: Token,
    pub player: PlayerSelector,
}

// CreateTokenAttribute
impl CardAttribute for CreateTokenAttribute {
    fn on_trigger(&mut self, trigger: &Trigger) -> Option<Effect> {
        if matches!(trigger, Trigger::OnDeath { .. })
            || *trigger == Trigger::OnCastResolved
        {
            Some(Effect::CreateToken {
                token: self.token.clone(),
                player: self.player.clone(),
            })
        } else {
            None
        }
    }
    fn as_any(&self) -> &dyn Any { self }
}

// --- Generic: create enchantment token ---
#[derive(Debug, Clone)]
pub struct CreateEnchantmentAttribute {
    pub enchantment: Enchantment,
    pub target: TargetFilter,
}

impl CardAttribute for CreateEnchantmentAttribute {
    fn on_trigger(&mut self, trigger: &Trigger) -> Option<Effect> {
        if *trigger == Trigger::OnCastResolved {
            Some(Effect::CreateEnchantmentToken { enchantment: self.enchantment.clone(), target: self.target.clone() })
        } else {
            None
        }
    }
    fn as_any(&self) -> &dyn Any {
        self
    }
}

// --- Generic: exile and allow playing from exile ---
#[derive(Debug, Clone)]
pub struct ExileAndPlayAttribute {
    pub count: u32,
    pub player: PlayerSelector,
    pub duration: Duration,
}

impl CardAttribute for ExileAndPlayAttribute {
    fn on_trigger(&mut self, trigger: &Trigger) -> Option<Effect> {
        if *trigger == Trigger::OnCastResolved {
            Some(Effect::ExileThenPlayFromExile {
                count: self.count,
                player: self.player.clone(),
                duration: self.duration.clone(),
            })
        } else {
            None
        }
    }
    fn as_any(&self) -> &dyn Any {
        self
    }
}

// --- Generic: conditional effect on resolve ---
#[derive(Debug, Clone)]
pub struct ConditionalAttribute {
    pub trigger: Trigger,
    pub condition: Condition,
    pub effect_if_true: Effect,
    pub effect_if_false: Option<Effect>,
}

impl CardAttribute for ConditionalAttribute {
    fn on_trigger(&mut self, trigger: &Trigger) -> Option<Effect> {
        if *trigger == self.trigger {
            Some(Effect::Conditional {
                condition: self.condition.clone(),
                effect_if_true: Box::new(self.effect_if_true.clone()),
                effect_if_false: self.effect_if_false.clone().map(Box::new),
            })
        } else {
            None
        }
    }
    fn as_any(&self) -> &dyn Any {
        self
    }
}

// --- Generic: first time per turn attribute (e.g., Valiant) ---
#[derive(Debug, Clone)]
pub struct FirstTimePerTurnAttribute {
    pub base_trigger: Trigger,
    pub reset_phase: GamePhase,
    pub action: Effect,
    pub used: bool,
}

impl CardAttribute for FirstTimePerTurnAttribute {
    fn on_trigger(&mut self, trigger: &Trigger) -> Option<Effect> {
        match trigger {
            t if *t == self.base_trigger && !self.used => {
                self.used = true;
                Some(self.action.clone())
            }
            Trigger::AtBeginPhase { phase, player } if *phase == self.reset_phase => {
                self.used = false;
                None
            }
            _ => None,
        }
    }
    fn as_any(&self) -> &dyn Any {
        self
    }
}

// --- Generic: delayed effect in a later phase ---
#[derive(Debug, Clone)]
pub struct DelayedAttribute {
    pub effect: Effect,
    pub phase: GamePhase,
    pub deps: Vec<usize>,
}

impl CardAttribute for DelayedAttribute {
    fn on_trigger(&mut self, trigger: &Trigger) -> Option<Effect> {
        if *trigger == Trigger::OnCastResolved {
            Some(Effect::Delayed {
                effect: Box::new(self.effect.clone()),
                phase: self.phase,
                deps: self.deps.clone(),
            })
        } else {
            None
        }
    }
    fn as_any(&self) -> &dyn Any {
        self
    }
}
