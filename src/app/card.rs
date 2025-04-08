/// --- Kártya pozíciók ---
#[derive(Debug, Copy, Clone)]
pub struct CardPosition {
    pub hover_x: u32,
    pub ocr_x1: u32,
    pub ocr_x2: u32,
}

/// A hard-coded land és creature nevek az eredeti kódból
pub const LAND_NAMES: [&str; 2] = ["Mountain", "Rockface Village"];
pub const CREATURE_NAMES: [&str; 4] = [
    "Cacophony Scamp",
    "Heartfire Hero",
    "Monastery Swiftspear",
    "Electrostatic Infantry",
];

/// ManaCost struktúra és segédfüggvények.
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

pub fn parse_mana_cost(card_name: &str) -> ManaCost {
    match card_name {
        "Burst Lightning" => ManaCost {
            colorless: 0,
            red: 1,
            blue: 0,
            green: 0,
            black: 0,
            white: 0,
        },
        "Lightning Strike" => ManaCost {
            colorless: 1,
            red: 1,
            blue: 0,
            green: 0,
            black: 0,
            white: 0,
        },
        "Blazing Crescendo" => ManaCost {
            colorless: 1,
            red: 1,
            blue: 0,
            green: 0,
            black: 0,
            white: 0,
        },
        "Monstrous Rage" => ManaCost {
            colorless: 0,
            red: 1,
            blue: 0,
            green: 0,
            black: 0,
            white: 0,
        },
        "Felonious Rage" => ManaCost {
            colorless: 0,
            red: 1,
            blue: 0,
            green: 0,
            black: 0,
            white: 0,
        },
        "Heartfire Hero" => ManaCost {
            colorless: 0,
            red: 1,
            blue: 0,
            green: 0,
            black: 0,
            white: 0,
        },
        "Cacophony Scamp" => ManaCost {
            colorless: 0,
            red: 1,
            blue: 0,
            green: 0,
            black: 0,
            white: 0,
        },
        "Electrostatic Infantry" => ManaCost {
            colorless: 1,
            red: 1,
            blue: 0,
            green: 0,
            black: 0,
            white: 0,
        },
        "Monastery Swiftspear" => ManaCost {
            colorless: 0,
            red: 1,
            blue: 0,
            green: 0,
            black: 0,
            white: 0,
        },
        "Demonic Ruckus" => ManaCost {
            colorless: 1,
            red: 1,
            blue: 0,
            green: 0,
            black: 0,
            white: 0,
        },
        _ => ManaCost {
            colorless: 0,
            red: 0,
            blue: 0,
            green: 0,
            black: 0,
            white: 0,
        },
    }
}

/// Kártya típusok
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct Creature {
    pub name: String,
    pub summoning_sickness: bool,
    pub power: i32,
    pub toughness: i32,
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct Instant_ {
    pub name: String,
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct Enchantment {
    pub name: String,
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum CardType {
    Creature(Creature),
    Instant(Instant_),
    Enchantment(Enchantment),
}

/// Kártya parse függvény – az eredeti logika alapján.
pub fn parse_card(card_text: &str) -> Option<CardType> {
    if card_text.contains("Cacophony Scamp") {
        Some(CardType::Creature(Creature {
            name: "Cacophony Scamp".to_string(),
            summoning_sickness: true,
            power: 1,
            toughness: 1,
        }))
    } else if card_text.contains("Heartfire Hero") {
        Some(CardType::Creature(Creature {
            name: "Heartfire Hero".to_string(),
            summoning_sickness: true,
            power: 1,
            toughness: 1,
        }))
    } else if card_text.contains("Monastery Swiftspear") {
        Some(CardType::Creature(Creature {
            name: "Monastery Swiftspear".to_string(),
            summoning_sickness: true,
            power: 1,
            toughness: 2,
        }))
    } else if card_text.contains("Electrostatic Infantry") {
        Some(CardType::Creature(Creature {
            name: "Electrostatic Infantry".to_string(),
            summoning_sickness: true,
            power: 1,
            toughness: 2,
        }))
    } else if card_text.contains("Burst Lightning") {
        Some(CardType::Instant(Instant_ {
            name: "Burst Lightning".to_string(),
        }))
    } else if card_text.contains("Lightning Strike") {
        Some(CardType::Instant(Instant_ {
            name: "Lightning Strike".to_string(),
        }))
    } else if card_text.contains("Blazing Crescendo") {
        Some(CardType::Instant(Instant_ {
            name: "Blazing Crescendo".to_string(),
        }))
    } else if card_text.contains("Monstrous Rage") {
        Some(CardType::Instant(Instant_ {
            name: "Monstrous Rage".to_string(),
        }))
    } else if card_text.contains("Felonious Rage") {
        Some(CardType::Instant(Instant_ {
            name: "Felonious Rage".to_string(),
        }))
    } else if card_text.contains("Demonic Ruckus") {
        Some(CardType::Enchantment(Enchantment {
            name: "Demonic Ruckus".to_string(),
        }))
    } else {
        None
    }
}
