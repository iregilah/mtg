use std::any::Any;
use std::fmt::Debug;

use crate::app::card_library::{Card, ManaCost};
use crate::app::game_state::{GameEvent, GamePhase};


// -- UGYANAZ AZ ENUM, kiegészítve a Offspring { cost: u32 } mezővel:
#[derive(Debug, Clone, PartialEq, Eq)]
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

    CreateCreatureToken {
        name: String,
        power: i32,
        toughness: i32,
        creature_types: Vec<CreatureType>,
    },

    CreateEnchantmentToken {
        name: String,
        power_buff: i32,
        toughness_buff: i32,
        ability: KeywordAbility,
    },

    /// Képesség eltávolítása egy lényről
    RemoveAbility {
        ability: KeywordAbility,
        target: TargetFilter,
    },

    Offspring {
        cost: u32,
    },

    TargetedEffects {
        sub_effects: Vec<Effect>,
    },

    /// “Ha a targetált lény meghal a körben, fuss le effect.”
    WhenTargetDiesThisTurn {
        effect: Box<Effect>,
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
    PreventLifeGain {
        player: PlayerSelector,
        duration: Duration,
    },
    Conditional {
        condition: Condition,
        effect_if_true: Box<Effect>,
        effect_if_false: Option<Box<Effect>>,
    },
    ChooseSome {
        choose: usize,
        options: Vec<Effect>,
    },
    Delayed {
        effect: Box<Effect>,
        phase: GamePhase,
        deps: Vec<usize>,
    },
    AddMana {
        colorless: u32,
        red: u32,
        blue: u32,
        green: u32,
        black: u32,
        white: u32,
    },
}

/// A mennyiségek
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Amount {
    Fixed(i32),
    SourcePower,
    SourceToughness,
}

/// Counter-típusok
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CounterType {
    PlusOnePlusOne,
    Loyalty,
}

/// Keyword-ek
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum KeywordAbility {
    Haste,
    Trample,
    Menace,
    Prowess,
    Lifelink,
    Deathtouch,
    Flying,
    DoubleStrike,
    FirstStrike,
    Reach,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum CreatureType {
    Mouse,
    Lizard,
    Bird,
    Human,
    Spirit,
    Soldier,
    Wizard,
    Monk,
    Warrior,
    Mercenary,
    Phyrexian,
    Goblin,
    Dwarf,
    Detective,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Trigger {
    OnEnterBattlefield { filter: TargetFilter },
    OnDeath { filter: TargetFilter },
    OnAttack { filter: TargetFilter },
    OnBlock { filter: TargetFilter },
    OnCombatDamage { filter: TargetFilter },
    OnSpellCast { filter: SpellFilter },
    OnTargetedFirstTimeEachTurn { filter: TargetFilter },
    OnDealtDamage { filter: TargetFilter },
    OnAttackWithCreatureType { creature_type: CreatureType },
    AtPhase { phase: GamePhase, player: PlayerSelector },
    OnCastResolved,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SpellFilter {
    InstantOrSorcery,
    Any,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PlayerSelector {
    Controller,
    Opponent,
    AnyPlayer,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Condition {
    OpponentLostLifeThisTurn,
    FirstTimeThisTurn,
    SpellWasNonCreature,
    Tap,
    SacrificeSelf,
    Always,
    SpellWasKicked,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TargetFilter {
    AnyTarget,
    Creature,
    Player,
    SelfCard,
    ControllerCreature,
    OpponentCreature,
    CreatureType(CreatureType),
    ExactCardID(u64),
}


#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Duration {
    EndOfTurn,
    NextTurnEnd,
    Permanent,
}

/// CardAttribute trait – változatlan
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

pub trait CardAttribute: Any + CardAttributeClone + Debug {
    fn on_trigger(&mut self, trigger: &Trigger) -> Option<Effect>;
    fn as_any(&self) -> &dyn Any;
}

/// Egyszerű triggered effect
#[derive(Debug, Clone)]
pub struct TriggeredEffectAttribute {
    pub trigger: Trigger,
    pub effect: Effect,
}

impl CardAttribute for TriggeredEffectAttribute {
    fn on_trigger(&mut self, trigger: &Trigger) -> Option<Effect> {
        match (&self.trigger, trigger) {
            (Trigger::OnAttackWithCreatureType { creature_type: t1 },
                Trigger::OnAttackWithCreatureType { creature_type: t2 })
            if t1 == t2 => {
                Some(self.effect.clone())
            }
            (Trigger::OnTargetedFirstTimeEachTurn { .. },
                Trigger::OnTargetedFirstTimeEachTurn { .. }) => {
                Some(self.effect.clone())
            }
            (Trigger::OnDealtDamage { filter: f1 }, Trigger::OnDealtDamage { filter: f2 })
            if f1 == f2 => {
                Some(self.effect.clone())
            }
            _ if trigger == &self.trigger => {
                Some(self.effect.clone())
            }
            _ => None,
        }
    }
    fn as_any(&self) -> &dyn Any { self }
}

#[derive(Debug, Clone)]
pub struct CreateEnchantmentAttribute {
    /// A "token enchantment" neve (pl. "Monster")
    pub name: String,
    /// Mennyi power/toughness buffot ad
    pub power_buff: i32,
    pub toughness_buff: i32,
    /// Milyen keyword ability-t ad (pl. Trample)
    pub ability: KeywordAbility,
    /// Kit szeretnénk célozni (pl. `TargetFilter::Creature`)
    pub target: TargetFilter,
}

impl CardAttribute for CreateEnchantmentAttribute {
    fn on_trigger(&mut self, trigger: &Trigger) -> Option<Effect> {
        if *trigger == Trigger::OnCastResolved {
            // Kibocsátunk egy "TargetedEffects" hatást,
            // amelyen belül CreateEnchantmentToken jön létre
            Some(Effect::TargetedEffects {
                sub_effects: vec![
                    Effect::CreateEnchantmentToken {
                        name: self.name.clone(),
                        power_buff: self.power_buff,
                        toughness_buff: self.toughness_buff,
                        ability: self.ability.clone(),
                    },
                ],
            })
        } else {
            None
        }
    }
    fn as_any(&self) -> &dyn Any { self }
}

/// Buff
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

/// GrantAbility
#[derive(Debug, Clone)]
pub struct GrantAbilityAttribute {
    pub ability: KeywordAbility,
    pub duration: Duration,
    pub target: TargetFilter,
}

impl CardAttribute for GrantAbilityAttribute {
    fn on_trigger(&mut self, trigger: &Trigger) -> Option<Effect> {
        match trigger {
            Trigger::OnCastResolved
            | Trigger::OnEnterBattlefield { .. } => {
                Some(Effect::GrantAbility {
                    ability: self.ability.clone(),
                    duration: self.duration.clone(),
                    target: self.target.clone(),
                })
            }
            _ => None,
        }
    }
    fn as_any(&self) -> &dyn Any { self }
}

/// ChooseOnConditionAttribute
#[derive(Debug, Clone)]
pub struct ChooseOnConditionAttribute {
    pub choose: usize,
    pub options: Vec<Effect>,
}

impl CardAttribute for ChooseOnConditionAttribute {
    fn on_trigger(&mut self, _trigger: &Trigger) -> Option<Effect> {
        // Mindig kiprovokálja a "ChooseSome" effectet
        Some(Effect::ChooseSome {
            choose: self.choose,
            options: self.options.clone(),
        })
    }
    fn as_any(&self) -> &dyn Any { self }
}

/// **OffspringAttribute** – ez hozza létre belépéskor a token copy‐t
#[derive(Debug, Clone)]
pub struct OffspringAttribute {
    pub additional_cost: u32, // pl. 2, 3 stb. – colorless
}

impl CardAttribute for OffspringAttribute {
    fn on_trigger(&mut self, trigger: &Trigger) -> Option<Effect> {
        match trigger {
            Trigger::OnEnterBattlefield { filter: TargetFilter::SelfCard } => {
                Some(Effect::Offspring { cost: self.additional_cost })
            }
            _ => None,
        }
    }
    fn as_any(&self) -> &dyn Any { self }
}

/// Prowess
#[derive(Debug, Clone)]
pub struct ProwessAttribute {
    pub filter: SpellFilter,
    pub power: i32,
    pub toughness: i32,
    pub duration: Duration,
}

impl CardAttribute for ProwessAttribute {
    fn on_trigger(&mut self, trigger: &Trigger) -> Option<Effect> {
        if let Trigger::OnSpellCast { filter } = trigger {
            if *filter == self.filter {
                Some(Effect::ModifyStats {
                    power_delta: self.power,
                    toughness_delta: self.toughness,
                    duration: self.duration.clone(),
                    target: TargetFilter::SelfCard,
                })
            } else {
                None
            }
        } else {
            None
        }
    }
    fn as_any(&self) -> &dyn Any { self }
}

/// Lifelink
#[derive(Debug, Clone)]
pub struct LifelinkAttribute;

impl CardAttribute for LifelinkAttribute {
    fn on_trigger(&mut self, trigger: &Trigger) -> Option<Effect> {
        if let Trigger::OnCombatDamage { filter } = trigger {
            if *filter == TargetFilter::SelfCard {
                return Some(Effect::Conditional {
                    condition: Condition::Always,
                    effect_if_true: Box::new(Effect::GainLife { amount: 0, player: PlayerSelector::Controller }),
                    effect_if_false: None,
                });
            }
        }
        None
    }
    fn as_any(&self) -> &dyn Any { self }
}

/// Deathtouch
#[derive(Debug, Clone)]
pub struct DeathtouchAttribute;

impl CardAttribute for DeathtouchAttribute {
    fn on_trigger(&mut self, trigger: &Trigger) -> Option<Effect> {
        if let Trigger::OnCombatDamage { filter } = trigger {
            if *filter == TargetFilter::Creature {
                return Some(Effect::Damage { amount: Amount::Fixed(1), target: TargetFilter::SelfCard });
            }
        }
        None
    }
    fn as_any(&self) -> &dyn Any { self }
}

/// Trample
#[derive(Debug, Clone)]
pub struct TrampleAttribute;

impl CardAttribute for TrampleAttribute {
    fn on_trigger(&mut self, trigger: &Trigger) -> Option<Effect> {
        if let Trigger::OnCombatDamage { filter } = trigger {
            if *filter == TargetFilter::SelfCard {
                return Some(Effect::Conditional {
                    condition: Condition::Always,
                    effect_if_true: Box::new(Effect::Damage { amount: Amount::SourcePower, target: TargetFilter::AnyTarget }),
                    effect_if_false: None,
                });
            }
        }
        None
    }
    fn as_any(&self) -> &dyn Any { self }
}

/// DoubleStrike
#[derive(Debug, Clone)]
pub struct DoubleStrikeAttribute;

impl CardAttribute for DoubleStrikeAttribute {
    fn on_trigger(&mut self, trigger: &Trigger) -> Option<Effect> {
        if let Trigger::OnCombatDamage { filter } = trigger {
            if *filter == TargetFilter::SelfCard {
                return Some(Effect::Delayed {
                    effect: Box::new(Effect::Damage { amount: Amount::SourcePower, target: TargetFilter::AnyTarget }),
                    phase: GamePhase::CombatDamage,
                    deps: Vec::new(),
                });
            }
        }
        None
    }
    fn as_any(&self) -> &dyn Any { self }
}

/// TypeSpecificTargetAttribute
#[derive(Debug, Clone)]
pub struct TypeSpecificTargetAttribute {
    pub creature_type: CreatureType,
    pub effect: Effect,
}

impl CardAttribute for TypeSpecificTargetAttribute {
    fn on_trigger(&mut self, trigger: &Trigger) -> Option<Effect> {
        if matches!(trigger, Trigger::AtPhase { phase: GamePhase::Combat, player: PlayerSelector::Controller }) {
            Some(self.effect.clone())
        } else {
            None
        }
    }
    fn as_any(&self) -> &dyn Any { self }
}

/// AddCounterAttribute
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

/// ProliferateAttribute
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


/// ExileAndPlayAttribute
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

/// ConditionalAttribute
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

/// FirstTimePerTurnAttribute
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
            Trigger::AtPhase { phase, player: _ } if *phase == self.reset_phase => {
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

/// DelayedAttribute
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
