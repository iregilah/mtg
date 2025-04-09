// card_attribute.rs

#[derive(Debug, Clone)]
pub enum Effect {
    // Saját attribútum módosítás: például +1/+1 counter hozzáadása vagy más stat változás.
    SelfAttributeChange(AttributeChange),
    // Célkártya sebzése: tartalmazza a Damage értéket és a cél szűrőt.
    DamageTarget { damage: Damage, target_filter: TargetFilter },
    // Célkártya megsemmisítése.
    DestroyTarget { target_filter: TargetFilter },
    // Célkártya elűzése.
    ExileTarget { target_filter: TargetFilter },
    // Poliferate: a kiválasztott counter típus növelése.
    Poliferate { counter_type: CounterType },
    // Teljes életpont visszaállítása.
    HealSelfToFull,
    // Új creature létrehozása.
    SpawnNewCreature,
    // A kártya saját életpontjának 1-re állítása.
    SetSelfHealthToOne,
    // Egy attribútum eltávolítása.
    RemoveAttribute,
    // Token csatolása.
    AttachToken { token: Token },
    // Enchantment csatolása.
    AttachEnchantment { enchantment: Enchantment },
}

#[derive(Debug, Clone)]
pub struct AttributeChange {
    pub attack_change: i32,
    pub health_change: i32,
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

pub trait CardAttribute {
    fn trigger(&self) -> Option<Effect> {
        None
    }
    fn turn_ended(&mut self) -> Option<Effect> {
        None
    }
    fn owner_died(&mut self) -> Option<Effect> {
        None
    }
}
