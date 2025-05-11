// app/bot.rs

use crate::multiplatform::screen_size;
use crate::app::gre::Gre;
use crate::app::card_attribute::{Amount, Effect};
use crate::app::game_state::Player;
use crate::app::game_state::Player as OtherPlayer;
use crate::app::game_state::GameEvent;
use crate::app::gre;
use crate::app::error::AppError;
use crate::app::game_state::StackEntry;
use crate::app::game_state::StackEntry as GameStateStackEntry;
use crate::app::gre::StackEntry as GreStackEntry;
use crate::app::game_state_updater::{GameStateUpdater, load_side_creatures};
use std::{
    collections::HashMap,
    thread::sleep,
    time::{Duration, Instant},
};
use tracing::{error, info, warn};

use crate::app::{card_library::{build_card_library, Card, CardType}, cards_positions::get_card_positions, creature_positions::{get_own_creature_positions, get_opponent_creature_positions}, ui::{Cords, set_cursor_pos, left_click, press_key, get_average_color, is_color_within_tolerance}, ocr::{read_creature_text, get_card_text}, game_state};
use crate::app::card_library::CardTypeFlags;
use crate::app::game_state::{Strategy, SimpleHeuristic};

pub struct Bot {
    pub end_game_counter: u32,
    pub end_game_threshold: u32,
    pub time_game_started: Instant,
    pub time_game_threshold: Duration,
    pub time_waiting_started: Instant,
    pub time_waiting_threshold: Duration,
    pub cords: Cords,
    pub screen_width: i32,
    pub screen_height: i32,
    pub card_count: usize,
    pub cards_texts: Vec<String>,
    pub land_count: u32,
    pub land_number: u32,
    pub last_opponent_turn: bool,
    pub opponent_turn_counter: usize,
    pub land_played_this_turn: bool,
    pub battlefield_creatures: HashMap<String, Card>,
    pub battlefield_opponent_creatures: HashMap<String, Card>,
    pub next_state_override: Option<StateOverride>,
    pub last_cast_card_name: String,
    pub first_main_phase_done: bool,
    pub gre: Gre,
    pub strategy: Box<dyn Strategy>,
    pub updater: GameStateUpdater,
    pub attacking: Vec<String>,
    pub combat_attackers: Vec<usize>,
    pub combat_blocks: HashMap<usize, Vec<usize>>,
}

pub enum StateOverride {
    OpponentsTurn,
}

impl Bot {
    pub fn new() -> Self {
        // get a platform‐independent screen size
        let (screen_width, screen_height) = screen_size().unwrap_or_else(|e| {
            // fallback to a sensible default
            eprintln!("Warning: failed to get screen size: {}. Defaulting to 800×600", e);
            (800, 600)
        });
        let cords = Cords::new(screen_width, screen_height);

        // Build and configure the GRE
        let mut gre = Gre::new(Player::Us);

        // Példa: minden damage effektust +1-gyel növelünk
        gre.add_continuous_effect(|eff| {
            // klónozzuk, hogy könnyen mintázhassuk
            if let Effect::Damage { amount: Amount::Fixed(n), target } = eff.clone() {
                // építsük újra az Effect-et a megváltoztatott mennyiséggel
                *eff = Effect::Damage {
                    amount: Amount::Fixed(n + 1),
                    target,
                };
            }
        });

        // Instantiate Bot
        let bot = Self {
            end_game_counter: 0,
            end_game_threshold: 3,
            time_game_started: Instant::now(),
            time_game_threshold: Duration::from_secs(1200),
            time_waiting_started: Instant::now(),
            time_waiting_threshold: Duration::from_secs(120),
            cords,
            screen_width,
            screen_height,
            card_count: 0,
            cards_texts: Vec::new(),
            land_count: 0,
            land_number: 0,
            last_opponent_turn: false,
            opponent_turn_counter: 0,
            land_played_this_turn: false,
            battlefield_creatures: HashMap::new(),
            battlefield_opponent_creatures: HashMap::new(),
            next_state_override: None,
            last_cast_card_name: String::new(),
            first_main_phase_done: false,
            gre,
            strategy: Box::new(SimpleHeuristic),
            updater: GameStateUpdater::new(),
            attacking: Vec::new(),
            combat_attackers: Vec::new(),
            combat_blocks: HashMap::new(),
        };

        bot
    }


    /// Draw exactly one card, OCR it, update hand and GameState.
    pub fn draw_card(&mut self) {
        let new_index = self.cards_texts.len();
        let new_count = new_index + 1;

        let positions = get_card_positions(new_count, self.screen_width as u32);
        let pos = positions[new_index];
        let card_y = ((self.screen_height as f64) * 0.97).floor() as i32;
        info!("Drawing → hovering new slot at index {} @ {:?}", new_index, pos);
        set_cursor_pos(pos.hover_x as i32, card_y);
        sleep(Duration::from_secs(2));

        let text = get_card_text(
            new_index,
            new_count,
            self.screen_width as u32,
            self.screen_height as u32,
        );

        // most frissítjük az állapotot
        self.cards_texts.push(text.clone());
        self.card_count = self.cards_texts.len();

        info!("Drew card '{}' → Updated hand: {:?}", text, self.cards_texts);
    }

    pub fn play_land(&mut self) {
        if !self.land_played_this_turn {
            // find a land in hand via library
            let library = build_card_library();
            if let Some((idx, _text)) = self.cards_texts.iter().enumerate()
                .find(|(_, txt)| library.values().any(|c| matches!(c.card_type, CardType::Land) && txt.contains(&c.name)))
            {
                info!("Playing land at hand idx {}", idx);
                Self::play_card(self, idx);
                self.land_played_this_turn = true;
                self.land_count += 1;
                self.land_number += 1;
            }
        }
        sleep(Duration::from_secs(1));
    }
    /// Refresh battlefield OCR and merge tracked tokens
    pub fn refresh_battlefield(&mut self) {
        let ours_ocr = load_side_creatures(
            self.screen_width as u32,
            self.screen_height as u32,
            false,
        );
        let mut merged = ours_ocr;
        // Preserve existing tracked tokens
        for (name, card) in self.battlefield_creatures.iter() {
            if card.type_flags.contains(CardTypeFlags::TOKEN) {
                merged.insert(name.clone(), card.clone());
            }
        }
        self.battlefield_creatures = merged;

        let opp_ocr = load_side_creatures(
            self.screen_width as u32,
            self.screen_height as u32,
            true,
        );
        let mut opp_merged = opp_ocr;
        for (name, card) in self.battlefield_opponent_creatures.iter() {
            if card.type_flags.contains(CardTypeFlags::TOKEN) {
                opp_merged.insert(name.clone(), card.clone());
            }
        }
        self.battlefield_opponent_creatures = opp_merged;
    }
    pub fn on_spell_resolved(&mut self) {
        let name = self.last_cast_card_name.clone();
        let mut targets: Vec<Card> = self.battlefield_creatures.values().cloned().collect();
        self.gre.trigger_event(GameEvent::SpellResolved(name.clone()), &mut targets, Player::Us);
        self.gre.resolve_stack();
    }

    pub fn on_turn_end(&mut self) {
        let mut all_creatures: Vec<Card> = self.battlefield_creatures.values().cloned().collect();
        self.gre.trigger_event(GameEvent::TurnEnded, &mut all_creatures, Player::Us);
        self.gre.resolve_stack();
        self.land_played_this_turn = false;
    }

    /// Cast the first affordable instant, then click on one of our creatures as target.
    pub fn cast_instants_targeting_creature(&mut self, creature_index: usize) {
        // find and cast one instant
        if let Some((i, _text)) = self.cards_texts.iter().enumerate().find(|(_, txt)| {
            self.can_cast_instant() &&
                build_card_library().values().any(|card| {
                    matches!(card.card_type, CardType::Instant) &&
                        Bot::text_contains(&card.name, txt)
                })
        }) {
            // get the Card struct
            let card_library = build_card_library();
            if let Some(card) = card_library.values().find(|c| {
                matches!(c.card_type, CardType::Instant) &&
                    Bot::text_contains(&c.name, &self.cards_texts[i])
            }) {
                match self.try_cast_card(i, card) {
                    Ok(cost) => {
                        self.land_number = self.land_number.saturating_sub(cost);
                        // now click on our creature as target
                        let positions = get_own_creature_positions(
                            self.battlefield_creatures.len(),
                            self.screen_width as u32,
                            self.screen_height as u32,
                        );
                        if creature_index < positions.len() {
                            let p = &positions[creature_index];
                            let click_x = ((p.click_x1 + p.click_x2) / 2) as i32;
                            let click_y = ((p.click_y1 + p.click_y2) / 2) as i32;
                            set_cursor_pos(click_x, click_y);
                            left_click();
                            info!("Targeted instant at creature #{}", creature_index);
                        }
                    }
                    Err(e) => warn!("Cannot cast {}: {:?}", card.name, e),
                }
            }
        }
    }
    /// Attempt to cast a card at `pos`, update mana in GameState on success.
    fn try_cast_card(&mut self, pos: usize, card: &Card) -> Result<u32, AppError> {
        let cost = &card.mana_cost;
        let available_colored = self.land_number;
        let needed_colored = cost.colored();
        let needed_colorless = cost.colorless;

        if available_colored < needed_colored {
            return Err(AppError::InsufficientMana {
                required: cost.total(),
                colored: needed_colored,
                colorless: needed_colorless,
                available_colored,
                available_colorless: available_colored,
            });
        }
        let leftover = available_colored - needed_colored;
        if leftover < needed_colorless {
            return Err(AppError::InsufficientMana {
                required: cost.total(),
                colored: needed_colored,
                colorless: needed_colorless,
                available_colored,
                available_colorless: available_colored,
            });
        }

        info!(
            "Casting '{}' költség: {} színes, {} színtelen (össz: {})",
            card.name, needed_colored, needed_colorless, cost.total()
        );

        let cost_total = cost.total();
        Self::play_card(self, pos);
        self.last_cast_card_name = card.name.clone();
        self.land_number = self.land_number.saturating_sub(cost_total);
        Ok(cost_total)
    }

    /// Attempts to cast all cards for which `predicate` returns `true`, updates battlefield creatures, and returns the remaining mana.
    pub fn cast_cards_by_filter<F>(&mut self, predicate: F) -> u32
    where
        F: Fn(&CardType) -> bool,
    {
        let mut mana_available = self.land_number;
        let card_library = build_card_library();

        // Iterálunk visszafelé az aktuális self.cards_texts tömbön.
        let mut i = self.cards_texts.len();
        while i > 0 {
            i -= 1; // for ciklusban visszafelé: i = len - 1, len - 2, ... , 0
            // Biztosan lekérjük a kártya szöveget a kézből
            if let Some(text) = self.cards_texts.get(i) {
                // Megkeressük a card_library-ben azt a kártyát, amelynek neve megtalálható a text-ben
                if let Some(card) = card_library.values().find(|card| Bot::text_contains(&card.name, text)) {
                    // Csak azokat a kártyákat próbáljuk meg kijátszani, amelyek megfelelnek a predicate-nek
                    if predicate(&card.card_type) {
                        if let Ok(cost_used) = self.try_cast_card(i, card) {
                            mana_available = mana_available.saturating_sub(cost_used);
                            if let CardType::Creature(mut cr) = card.card_type.clone() {
                                cr.summoning_sickness = true;
                                let mut new_card = card.clone();
                                new_card.card_type = CardType::Creature(cr);
                                self.battlefield_creatures.insert(new_card.name.clone(), new_card);
                            }
                            self.updater.update_battlefield_creatures(
                                self,
                                self.screen_width as u32,
                                self.screen_height as u32
                            );
                        } else if let Err(e) = self.try_cast_card(i, card) {
                            warn!("Cannot cast {}: {:?}", card.name, e);
                        }
                    }
                }
            }
        }
        self.land_number = mana_available;
        mana_available
    }
    pub fn cast_instants(&mut self) -> u32 {
        self.cast_cards_by_filter(|card_type| match card_type {
            CardType::Instant => true,
            _ => false,
        })
    }

    pub fn cast_creatures(&mut self) -> u32 {
        self.cast_cards_by_filter(|card_type| match card_type {
            CardType::Creature(_) => true,
            _ => false,
        })
    }
    pub fn can_cast_card<F>(&self, predicate: F) -> bool
    where
        F: Fn(&CardType) -> bool,
    {
        let library = build_card_library();
        self.cards_texts.iter().any(|ocr_text| {
            library.values().any(|card| {
                // Match card type
                if predicate(&card.card_type) && Bot::text_contains(&card.name, ocr_text) {
                    // Mana cost check
                    let colored = card.mana_cost.colored() as u32;
                    let leftover = self.land_number.saturating_sub(colored);
                    self.land_number >= colored && leftover >= card.mana_cost.colorless
                } else {
                    false
                }
            })
        })
    }

    //// Attempts to cast one creature, returning its name and mana spent if successful.
    pub fn cast_one_creature(&mut self) -> Option<(String, u32)> {
        let library = crate::app::card_library::build_card_library();

        // Use index-based loop to avoid borrow conflicts
        for i in 0..self.cards_texts.len() {
            // Clone OCR text to avoid borrowing self
            let ocr_text = self.cards_texts[i].clone();

            // Filter only Creature cards matching the text
            if let Some(card) = library
                .values()
                .find(|card| {
                    matches!(card.card_type, CardType::Creature(_))
                        && Bot::text_contains(&card.name, &ocr_text)
                })
            {
                // Safe to mutably borrow self now
                match self.try_cast_card(i, card) {
                    Ok(cost_used) => {
                        // Successfully cast: return name and mana
                        return Some((card.name.clone(), cost_used));
                    }
                    Err(e) => {
                        warn!("Nem sikerült kirakni {}: {:?}", card.name, e);
                        // On failure, try next card
                    }
                }
            }
        }

        None
    }

    /// Returns true if there's an instant in hand you can afford.
    pub fn can_cast_instant(&self) -> bool {
        self.can_cast_card(|t| matches!(t, CardType::Instant))
    }

    /// Returns true if there's a creature in hand you can afford.
    pub fn can_cast_creature(&self) -> bool {
        self.can_cast_card(|t| matches!(t, CardType::Creature(_)))
    }

    /// Central helper for casting creatures in SecondMainPhaseState.
    pub fn process_creature_casting(&mut self) {
        if self.land_number > 0 {
            let card_library = build_card_library();
            let creature_exists = self.cards_texts.iter().any(|text| {
                card_library.values().any(|card| {
                    if let CardType::Creature(_) = card.card_type {
                        Bot::text_contains(&card.name, text)
                    } else {
                        false
                    }
                })
            });
            if creature_exists {
                info!("Creature card detected in hand. Attempting to cast creature.");
                self.cast_creatures();
                self.updater.update_battlefield_creatures(
                    self,
                    self.screen_width as u32,
                    self.screen_height as u32
                );
            }
        }
    }

    pub fn play_card(bot: &mut Bot, card_index: usize) {
        let positions = get_card_positions(bot.card_count, bot.screen_width as u32);
        if card_index >= positions.len() {
            error!("Error: Card index {} is out of range. Only {} cards available.", card_index, positions.len());
            return;
        }
        let pos = positions[card_index];
        let card_y = ((bot.screen_height as f64) * 0.97).floor() as i32;
        set_cursor_pos(pos.hover_x as i32, card_y);
        left_click();
        left_click();
        set_cursor_pos(bot.screen_width - 1, bot.screen_height - 1);
        press_key(0x5A); // 'Z' billentyű
        left_click();
        sleep(Duration::from_millis(150));
        Bot::remove_card_from_hand(bot, card_index);
    }

    pub fn remove_card_from_hand(bot: &mut Bot, card_index: usize) {
        if card_index < bot.cards_texts.len() {
            let removed = bot.cards_texts.remove(card_index);
            info!("Removed card '{}' from hand at index {}.", removed, card_index);
            info!("Updated hand: {:?}", bot.cards_texts);
            bot.card_count = bot.cards_texts.len();
        } else {
            warn!("Attempted to remove card at invalid index {}.", card_index);
        }
    }

    pub fn text_contains(name: &str, ocr_text: &str) -> bool {
        //info!("text_contains() called with name = {:?} and ocr_text = {:?}", name, ocr_text);
        let result = ocr_text.contains(name);
        // info!("text_contains() returning: {}", result);
        result
    }

    pub fn examine_cards(&mut self) {
        self.cards_texts.clear(); // Töröljük a korábbi eredményeket.
        for i in 0..self.card_count {
            let text = get_card_text(
                i,
                self.card_count,
                self.screen_width as u32,
                self.screen_height as u32,
            );
            info!("Card {} text: {}", i, text);
            self.cards_texts.push(text);
        }
        info!("OCR results for cards: {:?}", self.cards_texts);
    }
}

// Provide a Default impl to allow `..Default::default()` in `new()`
impl Default for Bot {
    fn default() -> Self {
        panic!("Bot::default() is not supported; use Bot::new() instead");
    }
}
