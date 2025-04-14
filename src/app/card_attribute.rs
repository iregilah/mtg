use std::fmt::Debug;
#[derive(Debug, Clone)]
pub enum Effect {
    SelfAttributeChange(AttributeChange),
    DamageTarget { damage: Damage, target_filter: TargetFilter },
    DestroyTarget { target_filter: TargetFilter },
    ExileTarget { target_filter: TargetFilter },
    Poliferate { counter_type: CounterType },
    HealSelfToFull,
    SpawnNewCreature,
    SetSelfHealthToOne,
    RemoveAttribute,
    AttachToken { token: Token },
    AttachEnchantment { enchantment: Enchantment },
}

#[derive(Debug, Clone)]
pub struct AttributeChange {
    pub power: i32,
    pub toughness: i32,
}

#[derive(Debug, Clone)]
pub struct Damage {
    pub amount: u32,
    pub special: Option<DamageType>,
}

#[derive(Debug, Clone)]
pub enum DamageType {
    Normal,
    Overflow,
    DeathTouch,
}

#[derive(Debug, Clone)]
pub struct TargetFilter {
    pub filter: u32,
}

#[derive(Debug, Clone)]
pub enum CounterType {
    PlusOnePlusOne,
    Oil,
    Poison,
}

#[derive(Debug, Clone)]
pub struct Token {
    pub name: String,
}

#[derive(Debug, Clone)]
pub struct Enchantment {
    pub name: String,
}

pub trait CloneCardAttribute {
    fn clone_box(&self) -> Box<dyn CardAttribute<Output = Effect>>;
}

impl<T> CloneCardAttribute for T
where
    T: 'static + CardAttribute<Output = Effect> + Clone,
{
    fn clone_box(&self) -> Box<dyn CardAttribute<Output = Effect>> {
        Box::new(self.clone())
    }
}

impl Clone for Box<dyn CardAttribute<Output = Effect>> {
    fn clone(&self) -> Self {
        self.clone_box()
    }
}

pub trait CardAttribute: Debug + CloneCardAttribute {
    type Output;
    fn on_trigger(&self) -> Option<Self::Output> {
        None
    }
    fn on_turn_ended(&mut self) -> Option<Self::Output> {
        None
    }
    fn on_owner_died(&mut self) -> Option<Self::Output> {
        None
    }
}

#[derive(Debug, Clone)]
pub struct ModifyAttackDefense {
    pub power: i32,
    pub toughness: i32,
}

impl CardAttribute for ModifyAttackDefense {
    type Output = Effect;

    fn on_trigger(&self) -> Option<Self::Output> {
        Some(Effect::SelfAttributeChange(AttributeChange {
            power: self.power,
            toughness: self.toughness,
        }))
    }
    fn on_turn_ended(&mut self) -> Option<Self::Output> {
        Some(Effect::RemoveAttribute)
    }
}

#[derive(Debug, Clone)]
pub struct PoliferateOnDamage;

impl CardAttribute for PoliferateOnDamage {
    type Output = Effect;

    fn on_trigger(&self) -> Option<Self::Output> {
        Some(Effect::Poliferate { counter_type: CounterType::Poison })
    }
}

#[derive(Debug, Clone)]
pub struct SpawnTokenOnDeath;

impl CardAttribute for SpawnTokenOnDeath {
    type Output = Effect;

    fn on_owner_died(&mut self) -> Option<Self::Output> {
        Some(Effect::SpawnNewCreature)
    }
}

#[derive(Debug, Clone)]
pub struct DamageEqualPowerOnDeath {
    pub damage: Damage,
    pub target_filter: TargetFilter,
}

impl CardAttribute for DamageEqualPowerOnDeath {
    type Output = Effect;

    fn on_owner_died(&mut self) -> Option<Self::Output> {
        Some(Effect::DamageTarget {
            damage: self.damage.clone(),
            target_filter: self.target_filter.clone(),
        })
    }
}