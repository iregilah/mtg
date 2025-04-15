use crate::app::bot::Bot;
use crate::app::state::State;
use std::thread::sleep;
use std::time::Duration;
use crate::app::card_library::{CardType, CREATURE_NAMES, LAND_NAMES};
use crate::app::cards_positions::get_card_positions;
use crate::app::ui::{set_cursor_pos, left_click, press_key};
use crate::app::state::attack_phase_state::AttackPhaseState;
use crate::app::ui;
use crate::app::creature_positions::{get_own_creature_positions, get_opponent_creature_positions};

pub struct FirstMainPhaseState {}

impl State for FirstMainPhaseState {
    fn update(&mut self, bot: &mut Bot) {
        tracing::info!("FirstMainPhaseState: handling first main phase.");
        // 1. Először játsszuk ki a land-et (ha még nem történt meg)
        self.play_land_phase(bot);
        tracing::info!("Available mana after playing land: {}", bot.land_number);

        // 2. Frissítjük a battlefield creature–eket az új OCR-alapú módszerrel
        Self::update_battlefield_creatures_from_ocr(bot);

        // 3. Ha van (legalább 1) érvényes saját creature, akkor targetoljuk a kézben található első instantot;
        //    különben kijátszunk egy creature-t.
        if bot.battlefield_creatures.len() > 0 {
            tracing::info!("Creature(ök) találhatók a battlefield-en – instant castolása következik.");
            let mana_left = self.cast_instants_phase(bot);
            tracing::info!("Main phase finished (instant cast). Remaining mana: {}.", mana_left);
        } else {
            tracing::info!("Nincs érvényes creature a battlefield-en – creature castolása következik.");
            let mana_left = self.cast_creatures_phase(bot);
            tracing::info!("Main phase finished (creature cast). Remaining mana: {}.", mana_left);
        }
    }

    fn next(&mut self) -> Box<dyn State> {
        tracing::info!("FirstMainPhaseState: transitioning to AttackPhaseState.");
        Box::new(AttackPhaseState::new())
    }
}

impl FirstMainPhaseState {
    pub fn new() -> Self {
        Self {}
    }

    /// Játssza ki a land-et, ha még nem történt meg
    fn play_land_phase(&mut self, bot: &mut Bot) {
        if !bot.land_played_this_turn {
            if let Some((index, card_text)) = bot.cards_texts.iter().enumerate()
                .find(|(_i, text)| LAND_NAMES.iter().any(|&land| text.contains(land)))
            {
                tracing::info!("Found land card '{}' at index {}. Playing it.", card_text, index);
                Self::play_card(bot, index);
                bot.land_number += 1;
                bot.land_played_this_turn = true;
            }
        }
    }

    /// Új metódus: battlefield creature–ök frissítése OCR segítségével
    /// Mindkét módszert (páratlan és páros) lefuttatjuk, és csak akkor tekintjük érvényesnek,
    /// ha a kiolvasott eredményekből legalább 2 "értelmes" (két szó) név származott.
    fn update_battlefield_creatures_from_ocr(bot: &mut Bot) {
        // Saját creature–ök beolvasása:
        let odd_positions = get_own_creature_positions(7, bot.screen_width as u32, bot.screen_height as u32);
        let even_positions = get_own_creature_positions(8, bot.screen_width as u32, bot.screen_height as u32);

        let mut valid_own_names = Vec::new();
        for pos in odd_positions.into_iter().chain(even_positions.into_iter()) {
            let text = crate::app::ocr::read_creature_text(pos, bot.screen_width as u32, bot.screen_height as u32);
            if Self::is_valid_creature_name(&text) {
                valid_own_names.push(text);
            }
        }
        bot.battlefield_creatures.clear();
        if valid_own_names.len() >= 2 {
            for (i, name) in valid_own_names.into_iter().enumerate() {
                let dummy_creature = crate::app::card_library::Card {
                    name: name.clone(),
                    card_type: crate::app::card_library::CardType::Creature(
                        crate::app::card_library::Creature {
                            name,
                            summoning_sickness: false,
                            power: 1,
                            toughness: 1,
                        }
                    ),
                    mana_cost: crate::app::card_library::ManaCost {
                        colorless: 0, red: 0, blue: 0, green: 0, black: 0, white: 0
                    },
                    attributes: vec![],
                    triggers: vec![],
                };
                bot.battlefield_creatures.push(dummy_creature);
            }
            tracing::info!("Own battlefield creatures updated via OCR: {:?}", bot.battlefield_creatures.iter().map(|c| &c.name).collect::<Vec<_>>());
        } else {
            tracing::warn!("Nem sikerült legalább 2 érvényes creature nevet beolvasni a saját oldalon.");
        }

        // Az ellenfél creature–eit is lehet hasonló módon olvasni, itt csak demonstráció:
        let odd_opp_positions = get_opponent_creature_positions(7, bot.screen_width as u32, bot.screen_height as u32);
        let even_opp_positions = get_opponent_creature_positions(8, bot.screen_width as u32, bot.screen_height as u32);

        let mut valid_opp_names = Vec::new();
        for pos in odd_opp_positions.into_iter().chain(even_opp_positions.into_iter()) {
            let text = crate::app::ocr::read_creature_text(pos, bot.screen_width as u32, bot.screen_height as u32);
            if Self::is_valid_creature_name(&text) {
                valid_opp_names.push(text);
            }
        }
        bot.battlefield_opponent_creatures.clear();
        if valid_opp_names.len() >= 2 {
            for (i, name) in valid_opp_names.into_iter().enumerate() {
                let dummy_creature = crate::app::card_library::Card {
                    name: name.clone(),
                    card_type: crate::app::card_library::CardType::Creature(
                        crate::app::card_library::Creature {
                            name,
                            summoning_sickness: false,
                            power: 1,
                            toughness: 1,
                        }
                    ),
                    mana_cost: crate::app::card_library::ManaCost {
                        colorless: 0, red: 0, blue: 0, green: 0, black: 0, white: 0
                    },
                    attributes: vec![],
                    triggers: vec![],
                };
                bot.battlefield_opponent_creatures.push(dummy_creature);
            }
            tracing::info!("Opponent battlefield creatures updated via OCR: {:?}", bot.battlefield_opponent_creatures.iter().map(|c| &c.name).collect::<Vec<_>>());
        } else {
            tracing::warn!("Nem sikerült legalább 2 érvényes creature nevet beolvasni az ellenfél oldalon.");
        }
    }

    /// Segédfüggvény, mely azt vizsgálja, hogy a beolvasott szöveg érvényes creature névnek számít-e
    /// (legalább 2 szóból áll)
    fn is_valid_creature_name(name: &str) -> bool {
        name.split_whitespace().count() >= 2
    }

    /// Az instantok kijátszásának meglévő metódusa
    fn cast_instants_phase(&mut self, bot: &mut Bot) -> u32 {
        let mut mana_available = bot.land_number;
        let card_library = crate::app::card_library::build_card_library();
        let instant_names: Vec<String> = bot.cards_texts.iter()
            .filter(|text| text.contains("Burst Lightning") || text.contains("Lightning Strike"))
            .cloned()
            .collect();

        for instant_name in instant_names {
            if let Some(pos) = bot.cards_texts.iter().position(|text| text.contains(&instant_name)) {
                if let Some(card) = card_library.values().find(|c| text_contains(&c.name, &bot.cards_texts[pos])) {
                    if let crate::app::card_library::CardType::Instant(_) = card.card_type {
                        let cost = card.mana_cost.clone();
                        let colored_cost = cost.colored();
                        let total_cost = cost.total();
                        if mana_available >= colored_cost {
                            let leftover = mana_available - colored_cost;
                            if leftover >= cost.colorless {
                                tracing::info!(
                                "Casting instant '{}' ({} colorless, {} colored), total cost = {}",
                                card.name, cost.colorless, colored_cost, total_cost
                            );
                                Self::play_card(bot, pos);
                                mana_available -= total_cost;
                                // Helyettesítjük a Bot metódushívását a free függvény hívásával:
                                Self::update_battlefield_creatures_from_ocr(bot);
                            } else {
                                tracing::info!(
                                "Not enough leftover for colorless mana after paying colored cost for '{}'.",
                                card.name
                            );
                            }
                        } else {
                            tracing::info!(
                            "Not enough colored mana to cast instant '{}'. Required: {} colored, available: {}",
                            card.name, colored_cost, mana_available
                        );
                        }
                    }
                }
            } else {
                tracing::warn!("Instant '{}' not found in hand.", instant_name);
            }
        }
        mana_available
    }

    /// A creature–kijátszás meglévő metódusa
    fn cast_creatures_phase(&mut self, bot: &mut Bot) -> u32 {
        let mut mana_available = bot.land_number;
        let card_library = crate::app::card_library::build_card_library();
        let creature_names: Vec<String> = bot.cards_texts.iter()
            .filter(|text| crate::app::card_library::CREATURE_NAMES.iter().any(|&name| text.contains(name)))
            .cloned()
            .collect();

        for creature_name in creature_names {
            if let Some(pos) = bot.cards_texts.iter().position(|text| text.contains(&creature_name)) {
                if let Some(card) = card_library.values().find(|c| text_contains(&c.name, &bot.cards_texts[pos])) {
                    if let crate::app::card_library::CardType::Creature(ref creature) = card.card_type {
                        let cost = card.mana_cost.clone();
                        let colored_cost = cost.colored();
                        let total_cost = cost.total();
                        if mana_available >= colored_cost {
                            let leftover = mana_available - colored_cost;
                            if leftover >= cost.colorless {
                                tracing::info!(
                                    "Casting creature '{}' ({} colorless, {} colored), total cost = {}",
                                    creature.name, cost.colorless, colored_cost, total_cost
                                );
                                Self::play_card(bot, pos);
                                bot.battlefield_creatures.push(card.clone());
                                mana_available -= total_cost;
                            } else {
                                tracing::info!(
                                    "Not enough leftover for colorless mana after paying colored mana for '{}'. Required: {} colorless, leftover: {}",
                                    creature.name, cost.colorless, leftover
                                );
                            }
                        } else {
                            tracing::info!(
                                "Not enough colored mana to cast '{}'. Required: {} colored, available: {}",
                                creature.name, colored_cost, mana_available
                            );
                        }
                    }
                }
            } else {
                tracing::warn!("Creature '{}' not found in hand for removal.", creature_name);
            }
        }
        mana_available
    }

    /// Általános függvény a kijátszás lépéseihez: az egér mozgatása, kattintás, billentyű lenyomás, majd a kártya eltávolítása a kézből.
    fn play_card(bot: &mut Bot, card_index: usize) {
        let positions = get_card_positions(bot.card_count, bot.screen_width as u32);
        if card_index >= positions.len() {
            tracing::error!("Error: Card index {} is out of range. Only {} cards available.", card_index, positions.len());
            return;
        }
        let pos = positions[card_index];
        let card_y = ((bot.screen_height as f64) * 0.97).ceil() as i32;
        set_cursor_pos(pos.hover_x as i32, card_y);
        left_click();
        left_click();
        set_cursor_pos(bot.screen_width - 1, bot.screen_height - 1);
        press_key(0x5A); // 'Z' billentyű
        left_click();
        sleep(Duration::from_millis(150));
        Self::remove_card_from_hand(bot, card_index);
    }

    fn remove_card_from_hand(bot: &mut Bot, card_index: usize) {
        if card_index < bot.cards_texts.len() {
            let removed = bot.cards_texts.remove(card_index);
            tracing::info!("Removed card '{}' from hand at index {}.", removed, card_index);
            tracing::info!("Updated hand: {:?}", bot.cards_texts);
            bot.card_count = bot.cards_texts.len();
        } else {
            tracing::warn!("Attempte d to remove card at invalid index {}.", card_index);
        }
    }
}

// Egyszerű segédfüggvény, hogy az OCR eredmény tartalmazza-e a kártya nevét (nem csak részlet)
fn text_contains(name: &str, ocr_text: &str) -> bool {
    ocr_text.contains(name)
}