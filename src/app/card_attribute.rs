use std::any::Any;
use std::fmt::Debug;
use tracing::{debug, info};

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
    DamageByTargetPower {
        source: TargetFilter,
        target: TargetFilter,
    },
    TapTarget {
        target: TargetFilter,
    },
    BuffAllByMaxPower {
        filter: TargetFilter,
        duration: Duration,
    },
    AddCounterAll {
        counter: CounterType,
        amount: Amount,
        filter: TargetFilter,
    },
    Destroy {
        target: TargetFilter,
    },
    Exile {
        target: TargetFilter,
    },
    DrawCardsCounted,
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
    Hexproof,
    Indestructible,
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
    Elf,
    Druid,
    Rabbit,
    Badger,
    Mole,
    Insect,
    Robot,
    Beast,
    Ooze,
    Plant,
    Wurm,
    Dinosaur,
    Hydra,
    Raccoon,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Trigger {
    OnEnterBattlefield {
        filter: TargetFilter,
    },
    OnDeath {
        filter: TargetFilter,
    },
    OnAttack {
        filter: TargetFilter,
    },
    OnBlock {
        filter: TargetFilter,
    },
    OnCombatDamage {
        filter: TargetFilter,
    },
    OnSpellCast {
        filter: SpellFilter,
    },
    OnTargetedFirstTimeEachTurn {
        filter: TargetFilter,
    },
    OnDealtDamage {
        filter: TargetFilter,
    },
    OnAttackWithCreatureType {
        creature_type: CreatureType,
    },
    AtPhase {
        phase: GamePhase,
        player: PlayerSelector,
    },
    OnCastResolved,
    OnTargeted {
        filter: TargetFilter,
        player: PlayerSelector,
    },
    OnAddMana {
        filter: TargetFilter,
    },
    OnCounterAdded {
        filter: TargetFilter,
    },
    OnCycle {
        filter: TargetFilter,
    },
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
    HasCreaturePower4OrMore,
    ExiledCardWasCreature,
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
    Artifact,
    Enchantment,
    Land,
    ControllerLand,
    CardInGraveyard,
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
        info!(
            "TriggeredEffectAttribute: Checking trigger {:?} against {:?}",
            trigger, self.trigger
        );
        let result = if &self.trigger == trigger {
            Some(self.effect.clone())
        } else {
            None
        };
        debug!("TriggeredEffectAttribute: on_trigger result = {:?}", result);
        result
    }
    fn as_any(&self) -> &dyn Any {
        self
    }
}

#[derive(Debug, Clone)]
pub struct CreateEnchantmentAttribute {
    pub name: String,
    pub power_buff: i32,
    pub toughness_buff: i32,
    pub ability: KeywordAbility,
    pub target: TargetFilter,
}

impl CardAttribute for CreateEnchantmentAttribute {
    fn on_trigger(&mut self, trigger: &Trigger) -> Option<Effect> {
        info!("CreateEnchantmentAttribute: received trigger {:?}", trigger);
        let res = if *trigger == Trigger::OnCastResolved {
            Some(Effect::TargetedEffects {
                sub_effects: vec![Effect::CreateEnchantmentToken {
                    name: self.name.clone(),
                    power_buff: self.power_buff,
                    toughness_buff: self.toughness_buff,
                    ability: self.ability.clone(),
                }],
            })
        } else {
            None
        };
        debug!("CreateEnchantmentAttribute: result = {:?}", res);
        res
    }
    fn as_any(&self) -> &dyn Any {
        self
    }
}

#[derive(Debug, Clone)]
pub struct BuffAttribute {
    pub power: i32,
    pub toughness: i32,
    pub duration: Duration,
    pub target: TargetFilter,
}

impl CardAttribute for BuffAttribute {
    fn on_trigger(&mut self, trigger: &Trigger) -> Option<Effect> {
        info!("BuffAttribute: Checking trigger {:?}", trigger);
        let res = if *trigger == Trigger::OnCastResolved
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
        };
        debug!("BuffAttribute: result = {:?}", res);
        res
    }
    fn as_any(&self) -> &dyn Any {
        self
    }
}

#[derive(Debug, Clone)]
pub struct GrantAbilityAttribute {
    pub ability: KeywordAbility,
    pub duration: Duration,
    pub target: TargetFilter,
}

impl CardAttribute for GrantAbilityAttribute {
    fn on_trigger(&mut self, trigger: &Trigger) -> Option<Effect> {
        info!("GrantAbilityAttribute: Checking trigger {:?}", trigger);
        let res = if matches!(trigger, Trigger::OnCastResolved)
            || matches!(trigger, Trigger::OnEnterBattlefield { .. })
        {
            Some(Effect::GrantAbility {
                ability: self.ability.clone(),
                duration: self.duration.clone(),
                target: self.target.clone(),
            })
        } else {
            None
        };
        debug!("GrantAbilityAttribute: result = {:?}", res);
        res
    }
    fn as_any(&self) -> &dyn Any {
        self
    }
}

#[derive(Debug, Clone)]
pub struct ChooseOnConditionAttribute {
    pub choose: usize,
    pub options: Vec<Effect>,
}

impl CardAttribute for ChooseOnConditionAttribute {
    fn on_trigger(&mut self, _trigger: &Trigger) -> Option<Effect> {
        info!("ChooseOnConditionAttribute: triggering always returns ChooseSome");
        let res = Some(Effect::ChooseSome {
            choose: self.choose,
            options: self.options.clone(),
        });
        debug!("ChooseOnConditionAttribute: result = {:?}", res);
        res
    }
    fn as_any(&self) -> &dyn Any {
        self
    }
}

#[derive(Debug, Clone)]
pub struct OffspringAttribute {
    pub additional_cost: u32,
}

impl CardAttribute for OffspringAttribute {
    fn on_trigger(&mut self, trigger: &Trigger) -> Option<Effect> {
        info!("OffspringAttribute: Checking trigger {:?}", trigger);
        let res = if let Trigger::OnEnterBattlefield {
            filter: TargetFilter::SelfCard,
        } = trigger
        {
            Some(Effect::Offspring {
                cost: self.additional_cost,
            })
        } else {
            None
        };
        debug!("OffspringAttribute: result = {:?}", res);
        res
    }
    fn as_any(&self) -> &dyn Any {
        self
    }
}

#[derive(Debug, Clone)]
pub struct ProwessAttribute {
    pub filter: SpellFilter,
    pub power: i32,
    pub toughness: i32,
    pub duration: Duration,
}

impl CardAttribute for ProwessAttribute {
    fn on_trigger(&mut self, trigger: &Trigger) -> Option<Effect> {
        debug!("ProwessAttribute: trigger = {:?}", trigger);
        if let Trigger::OnSpellCast { filter } = trigger {
            if *filter == self.filter {
                let res = Some(Effect::ModifyStats {
                    power_delta: self.power,
                    toughness_delta: self.toughness,
                    duration: self.duration.clone(),
                    target: TargetFilter::SelfCard,
                });
                info!(
                    "ProwessAttribute: matched filter {:?}, result = {:?}",
                    filter, res
                );
                return res;
            }
        }
        None
    }
    fn as_any(&self) -> &dyn Any {
        self
    }
}

#[derive(Debug, Clone)]
pub struct LifelinkAttribute;

impl CardAttribute for LifelinkAttribute {
    fn on_trigger(&mut self, trigger: &Trigger) -> Option<Effect> {
        debug!("LifelinkAttribute: trigger = {:?}", trigger);
        if let Trigger::OnCombatDamage { filter } = trigger {
            if *filter == TargetFilter::SelfCard {
                info!("LifelinkAttribute: granting lifelink conditional");
                return Some(Effect::Conditional {
                    condition: Condition::Always,
                    effect_if_true: Box::new(Effect::GainLife {
                        amount: 0,
                        player: PlayerSelector::Controller,
                    }),
                    effect_if_false: None,
                });
            }
        }
        None
    }
    fn as_any(&self) -> &dyn Any {
        self
    }
}

#[derive(Debug, Clone)]
pub struct DeathtouchAttribute;

impl CardAttribute for DeathtouchAttribute {
    fn on_trigger(&mut self, trigger: &Trigger) -> Option<Effect> {
        debug!("DeathtouchAttribute: trigger = {:?}", trigger);
        if let Trigger::OnCombatDamage { filter } = trigger {
            if *filter == TargetFilter::Creature {
                info!("DeathtouchAttribute: returning fixed damage = 1");
                return Some(Effect::Damage {
                    amount: Amount::Fixed(1),
                    target: TargetFilter::SelfCard,
                });
            }
        }
        None
    }
    fn as_any(&self) -> &dyn Any {
        self
    }
}

#[derive(Debug, Clone)]
pub struct TrampleAttribute;

impl CardAttribute for TrampleAttribute {
    fn on_trigger(&mut self, trigger: &Trigger) -> Option<Effect> {
        debug!("TrampleAttribute: trigger = {:?}", trigger);
        if let Trigger::OnCombatDamage { filter } = trigger {
            if *filter == TargetFilter::SelfCard {
                info!("TrampleAttribute: returning trample conditional");
                return Some(Effect::Conditional {
                    condition: Condition::Always,
                    effect_if_true: Box::new(Effect::Damage {
                        amount: Amount::SourcePower,
                        target: TargetFilter::AnyTarget,
                    }),
                    effect_if_false: None,
                });
            }
        }
        None
    }
    fn as_any(&self) -> &dyn Any {
        self
    }
}

#[derive(Debug, Clone)]
pub struct DoubleStrikeAttribute;

impl CardAttribute for DoubleStrikeAttribute {
    fn on_trigger(&mut self, trigger: &Trigger) -> Option<Effect> {
        debug!("DoubleStrikeAttribute: trigger = {:?}", trigger);
        if let Trigger::OnCombatDamage { filter } = trigger {
            if *filter == TargetFilter::SelfCard {
                info!("DoubleStrikeAttribute: scheduling delayed damage");
                return Some(Effect::Delayed {
                    effect: Box::new(Effect::Damage {
                        amount: Amount::SourcePower,
                        target: TargetFilter::AnyTarget,
                    }),
                    phase: GamePhase::CombatDamage,
                    deps: Vec::new(),
                });
            }
        }
        None
    }
    fn as_any(&self) -> &dyn Any {
        self
    }
}

#[derive(Debug, Clone)]
pub struct TypeSpecificTargetAttribute {
    pub creature_type: CreatureType,
    pub effect: Effect,
}

impl CardAttribute for TypeSpecificTargetAttribute {
    fn on_trigger(&mut self, trigger: &Trigger) -> Option<Effect> {
        info!(
            "TypeSpecificTargetAttribute: checking trigger {:?}",
            trigger
        );
        if matches!(
            trigger,
            Trigger::AtPhase {
                phase: GamePhase::Combat,
                player: PlayerSelector::Controller
            }
        ) {
            info!("TypeSpecificTargetAttribute: matched combat phase, returning effect");
            return Some(self.effect.clone());
        }
        None
    }
    fn as_any(&self) -> &dyn Any {
        self
    }
}

#[derive(Debug, Clone)]
pub struct AddCounterAttribute {
    pub counter: CounterType,
    pub amount: u32,
    pub target: TargetFilter,
}

impl CardAttribute for AddCounterAttribute {
    fn on_trigger(&mut self, trigger: &Trigger) -> Option<Effect> {
        info!("AddCounterAttribute: received trigger {:?}", trigger);
        if matches!(trigger, Trigger::OnSpellCast { .. }) {
            let res = Some(Effect::AddCounter {
                counter: self.counter.clone(),
                amount: self.amount,
                target: self.target.clone(),
            });
            debug!("AddCounterAttribute: result = {:?}", res);
            return res;
        }
        None
    }
    fn as_any(&self) -> &dyn Any {
        self
    }
}

#[derive(Debug, Clone)]
pub struct ProliferateAttribute {
    pub counter: CounterType,
    pub player: PlayerSelector,
}

impl CardAttribute for ProliferateAttribute {
    fn on_trigger(&mut self, trigger: &Trigger) -> Option<Effect> {
        info!("ProliferateAttribute: trigger = {:?}", trigger);
        if matches!(trigger, Trigger::OnCombatDamage { .. }) {
            let res = Some(Effect::Proliferate {
                counter_type: self.counter.clone(),
                player: self.player.clone(),
            });
            debug!("ProliferateAttribute: result = {:?}", res);
            return res;
        }
        None
    }
    fn as_any(&self) -> &dyn Any {
        self
    }
}

#[derive(Debug, Clone)]
pub struct ExileAndPlayAttribute {
    pub count: u32,
    pub player: PlayerSelector,
    pub duration: Duration,
}

impl CardAttribute for ExileAndPlayAttribute {
    fn on_trigger(&mut self, trigger: &Trigger) -> Option<Effect> {
        info!("ExileAndPlayAttribute: trigger = {:?}", trigger);
        if *trigger == Trigger::OnCastResolved {
            let res = Some(Effect::ExileThenPlayFromExile {
                count: self.count,
                player: self.player.clone(),
                duration: self.duration.clone(),
            });
            debug!("ExileAndPlayAttribute: result = {:?}", res);
            return res;
        }
        None
    }
    fn as_any(&self) -> &dyn Any {
        self
    }
}

#[derive(Debug, Clone)]
pub struct ConditionalAttribute {
    pub trigger: Trigger,
    pub condition: Condition,
    pub effect_if_true: Effect,
    pub effect_if_false: Option<Effect>,
}

impl CardAttribute for ConditionalAttribute {
    fn on_trigger(&mut self, trigger: &Trigger) -> Option<Effect> {
        info!(
            "ConditionalAttribute: checking trigger {:?} against {:?}",
            trigger, self.trigger
        );
        if *trigger == self.trigger {
            let res = Some(Effect::Conditional {
                condition: self.condition.clone(),
                effect_if_true: Box::new(self.effect_if_true.clone()),
                effect_if_false: self.effect_if_false.clone().map(Box::new),
            });
            debug!("ConditionalAttribute: result = {:?}", res);
            return res;
        }
        None
    }
    fn as_any(&self) -> &dyn Any {
        self
    }
}

#[derive(Debug, Clone)]
pub struct FirstTimePerTurnAttribute {
    pub base_trigger: Trigger,
    pub reset_phase: GamePhase,
    pub action: Effect,
    pub used: bool,
}

impl CardAttribute for FirstTimePerTurnAttribute {
    fn on_trigger(&mut self, trigger: &Trigger) -> Option<Effect> {
        info!(
            "FirstTimePerTurnAttribute: trigger = {:?}, used = {}",
            trigger, self.used
        );
        match trigger {
            t if *t == self.base_trigger && !self.used => {
                self.used = true;
                let res = Some(self.action.clone());
                info!("FirstTimePerTurnAttribute: firing action = {:?}", res);
                res
            }
            Trigger::AtPhase { phase, player: _ } if *phase == self.reset_phase => {
                self.used = false;
                info!("FirstTimePerTurnAttribute: reset used flag");
                None
            }
            _ => None,
        }
    }
    fn as_any(&self) -> &dyn Any {
        self
    }
}

#[derive(Debug, Clone)]
pub struct DelayedAttribute {
    pub effect: Effect,
    pub phase: GamePhase,
    pub deps: Vec<usize>,
}

impl CardAttribute for DelayedAttribute {
    fn on_trigger(&mut self, trigger: &Trigger) -> Option<Effect> {
        info!("DelayedAttribute: received trigger {:?}", trigger);
        if *trigger == Trigger::OnCastResolved {
            let res = Some(Effect::Delayed {
                effect: Box::new(self.effect.clone()),
                phase: self.phase,
                deps: self.deps.clone(),
            });
            debug!("DelayedAttribute: scheduling delayed effect = {:?}", res);
            return res;
        }
        None
    }
    fn as_any(&self) -> &dyn Any {
        self
    }
}
