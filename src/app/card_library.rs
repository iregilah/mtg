// app/card_library.rs

use std::collections::HashMap;
use crate::app::card_attribute::{
    CardAttribute, ModifyAttackDefense, PoliferateOnDamage, SpawnTokenOnDeath, DamageEqualPowerOnDeath,
    Damage, TargetFilter,
};
#[derive(Debug, Copy, Clone)]
pub struct CardPosition {
    pub hover_x: u32,
    pub ocr_x1: u32,
    pub ocr_x2: u32,
}

// Hard-coded land és creature nevek (a play_land() függvényhez, stb.)
pub const LAND_NAMES: [&str; 2] = ["Mountain", "Rockface Village"];
pub const CREATURE_NAMES: [&str; 4] = [
    "Cacophony Scamp",
    "Heartfire Hero",
    "Monastery Swiftspear",
    "Electrostatic Infantry",
];
#[derive(Debug, Clone)]
pub struct Creature {
    pub name: String,
    pub summoning_sickness: bool,
    pub power: i32,
    pub toughness: i32,
}

#[derive(Debug, Clone)]
pub struct Instant_ {
    pub name: String,
}

#[derive(Debug, Clone)]
pub struct Enchantment {
    pub name: String,
}

#[derive(Debug, Clone)]
pub enum CardType {
    Creature(Creature),
    Instant(Instant_),
    Enchantment(Enchantment),
    Land,
}

/// Általános trigger típus, amely később kibővíthető
#[derive(Debug, Clone)]
pub enum Trigger {
    OnDeath,
    OnSpellCast,
    OnTargeted,
    OnCombatDamage,
    EndOfTurn,
    Custom(String),
}
#[derive(Debug, Clone)]
pub struct ManaCost {
    pub colorless: u32,
    pub red: u32,
    pub blue: u32,
    pub green: u32,
    pub black: u32,
    pub white: u32,
}

impl ManaCost {
    /// Színes mana költség
    pub fn colored(&self) -> u32 {
        self.red + self.blue + self.green + self.black + self.white
    }
    /// Teljes mana költség
    pub fn total(&self) -> u32 {
        self.colored() + self.colorless
    }
}


/// Az általános kártya reprezentáció – a típuson belül tároljuk például a creature, instant, enchantment stb. adatait,
/// valamint a hozzá tartozó manaköltséget, egy tetszőleges attribútumokból álló vektort és a trigger-eket.
#[derive(Debug, Clone)]
pub struct Card {
    pub name: String,
    pub card_type: CardType,
    pub mana_cost: ManaCost,
    pub attributes: Vec<Box<dyn CardAttribute<Output = crate::app::card_attribute::Effect>>>,
    pub triggers: Vec<Trigger>,
}

impl Card {
    /// Például a "halálkor" bekövetkező események hatásait kérhetjük le az attribútumokból.
    pub fn trigger_on_death(&mut self) -> Vec<crate::app::card_attribute::Effect> {
        self.attributes
            .iter_mut()
            .filter_map(|attr| attr.on_owner_died())
            .collect()
    }

    /// A kör végén hívható metódus, amely a turn-endhez kapcsolódó hatásokat kérdezi le.
    pub fn trigger_on_turn_end(&mut self) -> Vec<crate::app::card_attribute::Effect> {
        self.attributes
            .iter_mut()
            .filter_map(|attr| attr.on_turn_ended())
            .collect()
    }
}

/// Az alábbi függvény létrehozza a (például a jelenlegi kombóhoz szükséges) kártyalétárat,
/// melyben minden kártya a neve alapján érhető el, és a hozzá tartozó adatok (típus, manaköltség, attribútumok, trigger-ek) szerepelnek.
pub fn build_card_library() -> HashMap<String, Card> {
    let mut library: HashMap<String, Card> = HashMap::new();

    // --- Cacophony Scamp ---
    library.insert(
        "Cacophony Scamp".to_string(),
        Card {
            name: "Cacophony Scamp".to_string(),
            card_type: CardType::Creature(Creature {
                name: "Cacophony Scamp".to_string(),
                summoning_sickness: true,
                power: 1,
                toughness: 1,
            }),
            mana_cost: ManaCost { colorless: 0, red: 1, blue: 0, green: 0, black: 0, white: 0 },
            attributes: vec![
                // Példa: a kártyára jellemző "spawn" jellegű műveletet (kijátszáskor a battlefieldre kerül)
                Box::new(ModifyAttackDefense { power: 0, toughness: 0 }),
                // Ha combat damage-t okoz, proliferate történik
                Box::new(PoliferateOnDamage),
                // Amikor meghal, új lény (token) kerül létrehozásra
                Box::new(SpawnTokenOnDeath),
                // Amikor meghal, a power-ával megegyező sebzést okoz
                Box::new(DamageEqualPowerOnDeath {
                    damage: Damage { amount: 1, special: None },
                    target_filter: TargetFilter { filter: 0 },
                }),
            ],
            triggers: vec![
                Trigger::OnDeath,       // „Whenever this creature dies…”
                Trigger::OnCombatDamage, // "... and when it deals combat damage to a player, you may sacrifice it"
            ],
        },
    );

    // --- Monastery Swiftspear ---
    library.insert(
        "Monastery Swiftspear".to_string(),
        Card {
            name: "Monastery Swiftspear".to_string(),
            card_type: CardType::Creature(Creature {
                name: "Monastery Swiftspear".to_string(),
                summoning_sickness: true,
                power: 1,
                toughness: 2,
            }),
            mana_cost: ManaCost { colorless: 0, red: 1, blue: 0, green: 0, black: 0, white: 0 },
            attributes: vec![
                // Haste: a summoning_sickness tulajdonság feloldása játékmenet közbeni triggerrel történhet.
                Box::new(ModifyAttackDefense { power: 0, toughness: 0 }),
                // Prowess: a non-creature spell kijátszásakor +1/+1 az aktuális kör végéig
                Box::new(ModifyAttackDefense { power: 1, toughness: 1 }),
            ],
            triggers: vec![
                Trigger::OnSpellCast, // Prowess aktiválása
                Trigger::EndOfTurn,   // A +1/+1 hatás visszaállítása a kör végén
            ],
        },
    );

    // --- Electrostatic Infantry ---
    library.insert(
        "Electrostatic Infantry".to_string(),
        Card {
            name: "Electrostatic Infantry".to_string(),
            card_type: CardType::Creature(Creature {
                name: "Electrostatic Infantry".to_string(),
                summoning_sickness: true,
                power: 1,
                toughness: 2,
            }),
            mana_cost: ManaCost { colorless: 1, red: 1, blue: 0, green: 0, black: 0, white: 0 },
            attributes: vec![
                // Trample és +1/+1 counter (tartós növekedés) – itt egyszerűsítve, az attribútum elvileg mérhető hatást ad.
                Box::new(ModifyAttackDefense { power: 0, toughness: 0 }),
            ],
            triggers: vec![
                Trigger::OnSpellCast, // A spell kijátszásakor +1/+1 counter adható
            ],
        },
    );

    // --- Heartfire Hero ---
    library.insert(
        "Heartfire Hero".to_string(),
        Card {
            name: "Heartfire Hero".to_string(),
            card_type: CardType::Creature(Creature {
                name: "Heartfire Hero".to_string(),
                summoning_sickness: true,
                power: 1,
                toughness: 1,
            }),
            mana_cost: ManaCost { colorless: 0, red: 1, blue: 0, green: 0, black: 0, white: 0 },
            attributes: vec![
                // Valiant: amikor célponttá válik, +1/+1 counter
                Box::new(ModifyAttackDefense { power: 0, toughness: 0 }),
                // Amikor meghal, sebzés a power-ával megegyező értékkel minden ellenfélre
                Box::new(DamageEqualPowerOnDeath {
                    damage: Damage { amount: 1, special: None },
                    target_filter: TargetFilter { filter: 0 },
                }),
            ],
            triggers: vec![
                Trigger::OnDeath,
                Trigger::OnTargeted,
            ],
        },
    );

    // --- Felonious Rage ---
    library.insert(
        "Felonious Rage".to_string(),
        Card {
            name: "Felonious Rage".to_string(),
            card_type: CardType::Instant(Instant_ {
                name: "Felonious Rage".to_string(),
            }),
            mana_cost: ManaCost { colorless: 0, red: 1, blue: 0, green: 0, black: 0, white: 0 },
            attributes: vec![
                // A targetált creature +2/+0 kap, illetve haste (itt egyszerűsítve)
                Box::new(ModifyAttackDefense { power: 2, toughness: 0 }),
            ],
            triggers: vec![
                Trigger::OnTargeted,
                // Ha a targetált creature meghal ebben a körben: token létrehozás (például OnDeath triggerrel)
                Trigger::OnDeath,
                Trigger::EndOfTurn,
            ],
        },
    );

    // --- Monstrous Rage ---
    library.insert(
        "Monstrous Rage".to_string(),
        Card {
            name: "Monstrous Rage".to_string(),
            card_type: CardType::Instant(Instant_ {
                name: "Monstrous Rage".to_string(),
            }),
            mana_cost: ManaCost { colorless: 0, red: 1, blue: 0, green: 0, black: 0, white: 0 },
            attributes: vec![
                // A targetált creature +2/+0 kap
                Box::new(ModifyAttackDefense { power: 2, toughness: 0 }),
            ],
            triggers: vec![
                Trigger::OnTargeted,
                Trigger::EndOfTurn,
            ],
        },
    );

    // --- Monster Role Token (Enchantment) ---
    library.insert(
        "Monster Role Token".to_string(),
        Card {
            name: "Monster Role Token".to_string(),
            card_type: CardType::Enchantment(Enchantment {
                name: "Monster Role Token".to_string(),
            }),
            mana_cost: ManaCost { colorless: 0, red: 0, blue: 0, green: 0, black: 0, white: 0 },
            attributes: vec![
                // Állandó +1/+1 és trample
                Box::new(ModifyAttackDefense { power: 1, toughness: 1 }),
            ],
            triggers: vec![],
        },
    );

    // --- Blazing Crescendo ---
    library.insert(
        "Blazing Crescendo".to_string(),
        Card {
            name: "Blazing Crescendo".to_string(),
            card_type: CardType::Instant(Instant_ {
                name: "Blazing Crescendo".to_string(),
            }),
            mana_cost: ManaCost { colorless: 1, red: 1, blue: 0, green: 0, black: 0, white: 0 },
            attributes: vec![
                // +3/+1 hatás
                Box::new(ModifyAttackDefense { power: 3, toughness: 1 }),
            ],
            triggers: vec![
                Trigger::OnTargeted,
                Trigger::EndOfTurn,
            ],
        },
    );

    // --- Demonic Ruckus ---
    library.insert(
        "Demonic Ruckus".to_string(),
        Card {
            name: "Demonic Ruckus".to_string(),
            card_type: CardType::Enchantment(Enchantment {
                name: "Demonic Ruckus".to_string(),
            }),
            mana_cost: ManaCost { colorless: 1, red: 1, blue: 0, green: 0, black: 0, white: 0 },
            attributes: vec![
                // Állandó +1/+1 bonus, menace és trample (itt a bonus egyszerűsítve)
                Box::new(ModifyAttackDefense { power: 1, toughness: 1 }),
            ],
            triggers: vec![
                Trigger::OnTargeted,
                Trigger::OnDeath, // Ha az Aura a graveyard-ba kerül, kártyahúzás
                Trigger::Custom("Plot".to_string()),
            ],
        },
    );

    // --- Burst Lightning ---
    library.insert(
        "Burst Lightning".to_string(),
        Card {
            name: "Burst Lightning".to_string(),
            card_type: CardType::Instant(Instant_ {
                name: "Burst Lightning".to_string(),
            }),
            mana_cost: ManaCost { colorless: 0, red: 1, blue: 0, green: 0, black: 0, white: 0 },
            attributes: vec![],
            triggers: vec![
                Trigger::OnTargeted,
                Trigger::Custom("Kick".to_string()),
            ],
        },
    );

    // --- Lightning Strike ---
    library.insert(
        "Lightning Strike".to_string(),
        Card {
            name: "Lightning Strike".to_string(),
            card_type: CardType::Instant(Instant_ {
                name: "Lightning Strike".to_string(),
            }),
            mana_cost: ManaCost { colorless: 0, red: 1, blue: 0, green: 0, black: 0, white: 0 },
            attributes: vec![],
            triggers: vec![
                Trigger::OnTargeted,
            ],
        },
    );

    // --- Mountain --- (Land)
    library.insert(
        "Mountain".to_string(),
        Card {
            name: "Mountain".to_string(),
            card_type: CardType::Land,  // Most már Land típus
            mana_cost: ManaCost { colorless: 0, red: 0, blue: 0, green: 0, black: 0, white: 0 },
            attributes: vec![],
            triggers: vec![Trigger::Custom("AddRedMana".to_string())],
        },
    );

    // --- Rockface Village --- (Land)
    library.insert(
        "Rockface Village".to_string(),
        Card {
            name: "Rockface Village".to_string(),
            card_type: CardType::Land,  // Mostantól Land típus
            mana_cost: ManaCost { colorless: 0, red: 0, blue: 0, green: 0, black: 0, white: 0 },
            attributes: vec![],
            triggers: vec![
                Trigger::Custom("AddColorlessMana".to_string()),
                Trigger::Custom("AddRedMana".to_string()),
                Trigger::OnTargeted,
            ],
        },
    );

    library
}
