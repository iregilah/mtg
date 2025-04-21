// app/bot.rs

use crate::app::card_attribute::Damage;
use crate::app::card_attribute::Effect;
use crate::app::game_state::Player;
use crate::app::game_state::Player as OtherPlayer;
use crate::app::game_state::GameEvent;
use crate::app::gre::Gre;
use crate::app::error::AppError;
use std::{
    collections::HashMap,
    thread::sleep,
    time::{Duration, Instant},
};
use tracing::{error, info, warn};

use crate::app::{
    card_library::{build_card_library, Card, CardType},
    cards_positions::get_card_positions,
    creature_positions::{get_own_creature_positions, get_opponent_creature_positions},
    ui::{Cords, set_cursor_pos, left_click, press_key, get_average_color, is_color_within_tolerance},
    ocr::{read_creature_text, get_card_text},
};

use crate::app::game_state::{GameState, Strategy, SimpleHeuristic};


pub struct Bot {
    pub end_game_counter: u32,         // Játék végi számláló (előbb, mint majd a teljes játéklogika részletezése megtörténik).
    pub end_game_threshold: u32,         // Küszöbérték, ami alapján a játék véget ér.
    pub time_game_started: Instant,      // A játék indításának időpontja.
    pub time_game_threshold: Duration,   // Maximális játékidő (pl. 20 perc).
    pub time_waiting_started: Instant,   // A várakozás (mulligan) kezdete.
    pub time_waiting_threshold: Duration, // Maximális várakozási idő (pl. 2 perc).
    pub cords: Cords,                    // A képernyő gomb koordinátáit tartalmazó struktúra.
    pub screen_width: i32,               // Képernyő szélessége.
    pub screen_height: i32,              // Képernyő magassága.
    pub card_count: usize,               // Mulligan után beállított kártyák száma (pl. 7 vagy 8).
    pub cards_texts: Vec<String>,        // Hooverelés során kiolvasott kártyaszövegek vektora.
    pub land_count: u32,                 // A játékban lévő összes land száma
    pub land_number: u32,                // Az adott körben kijátszott land-ek száma (egy körben csak 1 lehet).
    pub last_opponent_turn: bool,        // Logikai érték: utolsó körben ellenfél lépett-e.
    pub opponent_turn_counter: usize,    // Az ellenfél kör számlálója.
    pub land_played_this_turn: bool,     // true, ha a jelenlegi körben már kijátszottuk a land-et.
    pub battlefield_creatures: HashMap<String, Card>,
    pub battlefield_opponent_creatures: HashMap<String, Card>,
    pub next_state_override: Option<StateOverride>,
    pub last_cast_card_name: String,
    pub first_main_phase_done: bool,
    pub gre: Gre,
    pub game_state: GameState,
    pub strategy: Box<dyn Strategy>,
}

pub enum StateOverride {
    OpponentsTurn,
}

impl Bot {
    pub fn new() -> Self {
        unsafe {
            // Screen metrics
            let screen_width = winapi::um::winuser::GetSystemMetrics(winapi::um::winuser::SM_CXSCREEN);
            let screen_height = winapi::um::winuser::GetSystemMetrics(winapi::um::winuser::SM_CYSCREEN);
            let cords = Cords::new(screen_width, screen_height);

            // Build and configure the GRE
            let mut gre = Gre::new(Player::Us);

            // Replacement effect example:
            // If something would be destroyed, exile it instead.
            gre.add_replacement_effect(move |eff| {
                if let Effect::DestroyTarget { target_filter } = eff {
                    Some(vec![Effect::ExileTarget { target_filter: target_filter.clone() }])
                } else {
                    None
                }
            });

            // Continuous effect example:
            // All damage instances deal +1 extra.
            gre.add_continuous_effect(|eff| {
                if let Effect::DamageTarget { damage, .. } = eff {
                    let new_amount = damage.amount.saturating_add(1);
                    *damage = Damage { amount: new_amount, special: damage.special.clone() };
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
                game_state: GameState::default(),
                strategy: Box::new(SimpleHeuristic),
            };

            bot
        }
    }
    /// Draws exactly one card (the rightmost) and OCR‐reads only that card.
    pub fn draw_card(&mut self) {
        let new_index = self.card_count;
        let new_count = new_index + 1;
        let positions = get_card_positions(new_count, self.screen_width as u32);
        let pos = positions[new_index];
        let card_y = ((self.screen_height as f64) * 0.97).floor() as i32;

        info!("Drawing → hovering new slot at index {} @ {:?}", new_index, pos);
        set_cursor_pos(pos.hover_x as i32, card_y);
        sleep(Duration::from_secs(2));

        // Now OCR exactly that one card:
        let text = {
            // Temporarily override card_count so get_card_text uses the right positions
            let old = self.card_count;
            self.card_count = new_count;
            let t = get_card_text(
                new_index,
                new_count,
                self.screen_width as u32,
                self.screen_height as u32,
            );
            self.card_count = old;
            t
        };

        self.cards_texts.push(text.clone());
        self.card_count = new_count;
        info!("Drew card '{}' → Updated hand: {:?}", text, self.cards_texts);
    }
    pub fn play_land(&mut self) {
        if !self.land_played_this_turn {
            // find a land in hand via library
            let library = build_card_library();
            if let Some((idx, text)) = self.cards_texts.iter().enumerate()
                .find(|(_, txt)| library.values().any(|c| matches!(c.card_type, CardType::Land) && txt.contains(&c.name)))
            {
                info!("Playing land '{}' at hand index {}", text, idx);
                Self::play_card(self, idx);
                self.land_played_this_turn = true;
                self.game_state.mana_available += 1;
                self.game_state.land_played_this_turn = true;
                // remove from game state hand
                self.game_state.hand.remove(idx);
            }
        }
        sleep(Duration::from_secs(1));
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
        self.game_state.land_played_this_turn = false;
    }

    /// Cast the first affordable instant, then click on one of our creatures as target.
    pub fn cast_instants_targeting_creature(&mut self, creature_index: usize) {
        // find and cast one instant
        if let Some((i, _text)) = self.cards_texts.iter().enumerate().find(|(_, txt)| {
            self.can_cast_instant() &&
                build_card_library().values().any(|card| {
                    matches!(card.card_type, CardType::Instant(_)) &&
                        Bot::text_contains(&card.name, txt)
                })
        }) {
            // get the Card struct
            let card_library = build_card_library();
            if let Some(card) = card_library.values().find(|c| {
                matches!(c.card_type, CardType::Instant(_)) &&
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
    /// Megpróbálja kijátszani a paraméterként kapott kártyát, ha elég mana áll rendelkezésre.
    /// Ha sikeres, visszaadja a felhasznált teljes mana mennyiségét.
    fn try_cast_card(&mut self, pos: usize, card: &Card) -> Result<u32, AppError> {
        let cost = &card.mana_cost;
        let available_colored = self.game_state.mana_available;
        let available_colorless = self.game_state.mana_available;

        let needed_colored = cost.colored();
        let needed_colorless = cost.colorless;

        // Ha nincs elég színes mana:
        if available_colored < needed_colored {
            info!(
                "Not enough colored mana for '{}'. Required: {}, available: {}.",
                card.name, needed_colored, available_colored
            );
            return Err(AppError::InsufficientMana {
                required: cost.total(),
                colored: needed_colored,
                colorless: needed_colorless,
                available_colored,
                available_colorless,
            });
        }

        // Ha nincs elég maradék színtelen manára:
        let leftover = available_colored - needed_colored;
        if leftover < needed_colorless {
            info!(
                "Not enough leftover mana for '{}'. Required {} colorless, leftover {}.",
                card.name, needed_colorless, leftover
            );
            return Err(AppError::InsufficientMana {
                required: cost.total(),
                colored: needed_colored,
                colorless: needed_colorless,
                available_colored,
                available_colorless,
            });
        }

        // Visszatért a régi részletes info-log:
        info!(
            "Casting '{}' költség: {} színes, {} színtelen (össz: {})",
            card.name, needed_colored, needed_colorless, cost.total()
        );

        // sikeres cast
        Self::play_card(self, pos);
        self.last_cast_card_name = card.name.clone();
        Ok(cost.total())
    }


    /// Generic függvény, amely a kapott predikátum alapján megpróbálja kijátszani a kártyákat.
    /// A `predicate` closure eldönti, hogy az adott kártya megfelel-e a feltételnek (például Instant vagy Creature).
    /// Visszaadja a megmaradt mana mennyiségét.
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
                            // if it's a creature, clone it with summoning_sickness = true
                            if let CardType::Creature(mut cr) = card.card_type.clone() {
                                cr.summoning_sickness = true;
                                let mut new_card = card.clone();
                                new_card.card_type = CardType::Creature(cr);
                                self.battlefield_creatures.insert(new_card.name.clone(), new_card);
                            }
                            // always OCR‐refresh opponent side too
                            Bot::update_battlefield_creatures_from_ocr(self);
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
            CardType::Instant(_) => true,
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

    /// Megpróbál egyetlen creature-t kijátszani.
    /// Ha sikerül, visszaadja a castolt kártya nevét és a felhasznált mana mennyiségét.
    pub fn cast_one_creature(&mut self) -> Option<(String, u32)> {
        let library = crate::app::card_library::build_card_library();

        // index‑alapú ciklus, így az ocr_text clonolása után nincs immut borrow amikor majd mut-ot kérünk
        for i in 0..self.cards_texts.len() {
            // klónozzuk a szöveget, hogy ne tartsunk borrow‑ot self‑en
            let ocr_text = self.cards_texts[i].clone();

            // csak creature típusú cardokra szűrünk
            if let Some(card) = library
                .values()
                .find(|card| {
                    matches!(card.card_type, CardType::Creature(_))
                        && Bot::text_contains(&card.name, &ocr_text)
                })
            {
                // most, hogy nincs élő borrow self‑en, jöhet a mutable borrow
                match self.try_cast_card(i, card) {
                    Ok(cost_used) => {
                        // sikeres cast: név + mana
                        return Some((card.name.clone(), cost_used));
                    }
                    Err(e) => {
                        warn!("Nem sikerült kirakni {}: {:?}", card.name, e);
                        // próbálja tovább a következő kártyát
                    }
                }
            }
        }

        None
    }

    /// Returns true if there's an instant in hand you can afford.
    pub fn can_cast_instant(&self) -> bool {
        self.can_cast_card(|t| matches!(t, CardType::Instant(_)))
    }

    /// Returns true if there's a creature in hand you can afford.
    pub fn can_cast_creature(&self) -> bool {
        self.can_cast_card(|t| matches!(t, CardType::Creature(_)))
    }

    /// Új segédfüggvény, amely a creature casting esetét dolgozza fel.
    /// A SecondMainPhaseState-ben ezt hívjuk a process_casting() helyett,
    /// centralizálva ezzel a creature-k kijátszás logikáját.
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
                Bot::update_battlefield_creatures_from_ocr(self);
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


    // 2. Creature-számolási logika egy oldalon (saját vagy ellenfél)
    /// Generalized counting for both odd and even branches.
    fn count_branch(
        y1: i32,
        region_height: i32,
        rect_width: i32,
        mut examined_rect_x: i32,
        tol: f64,
        target_color: (u8, u8, u8),
        initial_count: usize,
        first_step: i32,
        step: i32,
        max_count: usize,
    ) -> usize {
        let mut count = initial_count;
        info!("Starting branch: initial_count={}, first_step={}, step={}, max_count={}", count, first_step, step, max_count);
        // Move to the first neighboring slot
        examined_rect_x -= first_step;

        // Walk until we hit max_count or run out of screen
        while count < max_count && examined_rect_x >= 0 {
            let sample_color = get_average_color(examined_rect_x, y1, rect_width, region_height);
            info!("Branch: examined_rect_x={}, sample_color={:?}",examined_rect_x, sample_color);

            if is_color_within_tolerance(sample_color, target_color, tol) {
                count += 2;
                info!("Branch: increasing count to {}", count);
                examined_rect_x -= step;
            } else {
                info!("Branch: color out of tolerance, stopping");
                break;
            }
        }
        info!("Branch final creature count={}", count);
        count
    }


    /// Detects how many creatures are on one side of the board.
    fn detect_creature_count_for_side(screen_width: u32, screen_height: u32, is_opponent: bool) -> usize {
        info!("detect_creature_count_for_side() called with screen_width = {}, screen_height = {}, is_opponent = {}", screen_width, screen_height, is_opponent);
        let screen_width_f = screen_width as f64;
        let screen_height_f = screen_height as f64;

        // Normalized Y positions differ for opponent vs. own side
        let (y1_norm, y2_norm) = if is_opponent {
            (101.761, 104.891)
        } else {
            (185.141, 188.731)
        };

        // Convert norms to pixel coordinates
        let y1 = ((y1_norm / 381.287) * screen_height_f).floor() as i32;
        let y2 = ((y2_norm / 381.287) * screen_height_f).floor() as i32;
        let region_height = y2 - y1;
        let rect_width = ((4.4 / 677.292) * screen_width_f).floor() as i32;

        let screen_center_x = (screen_width as i32) / 2;
        let examined_rect_x = screen_center_x - rect_width / 2;
        info!("Calculated values: y1 = {}, y2 = {}, region_height = {}, rect_width = {}, screen_center_x = {}, center_rect_x = {}", y1, y2, region_height, rect_width, screen_center_x, examined_rect_x);

        // Target color and tolerance for card detection
        let target_color = (210, 175, 157);
        let tol = 0.035;

        // Check the center slot first
        let center_color = get_average_color(examined_rect_x, y1, rect_width, region_height);
        info!("Center area average color: {:?}", center_color);
        let center_is_card = is_color_within_tolerance(center_color, target_color, tol);
        info!("Center area considered as card: {}", center_is_card);

        // Precompute stepping distances
        let scale = screen_width_f / 677.292;
        let step = (69.0 * scale).floor() as i32;
        let first_step_even = (34.492 * scale).floor() as i32;

        // Delegate to the generalized branch counter
        if center_is_card {
            // Odd branch: start from center (count=1), use same step for first move
            Self::count_branch(y1, region_height, rect_width, examined_rect_x, tol, target_color, 1, step, step, 7)
        } else {
            // Even branch: start empty (count=0), initial offset differs
            Self::count_branch(y1, region_height, rect_width, examined_rect_x, tol, target_color, 0, first_step_even, step, 8)
        }
    }

    /// Load creatures for one side (own or opponent) into a fresh HashMap.
    fn load_side_creatures(
        &self,
        is_opponent: bool,
        count: usize,
        library: &HashMap<String, Card>,
    ) -> HashMap<String, Card> {
        let mut map = HashMap::new();
        let positions = if is_opponent {
            get_opponent_creature_positions(count, self.screen_width as u32, self.screen_height as u32)
        } else {
            get_own_creature_positions(count, self.screen_width as u32, self.screen_height as u32)
        };
        for (i, pos) in positions.into_iter().enumerate() {
            let name = read_creature_text(pos, i + 1, is_opponent, self.screen_width as u32, self.screen_height as u32);
            if let Some(card) = library.get(&name) {
                info!(
                    "Found {} battlefield creature: {}",
                    if is_opponent { "opponent" } else { "own" },
                    name
                );
                map.insert(name.clone(), card.clone());
            } else if !name.is_empty() {
                warn!(
                    "Unknown {} battlefield creature OCR’d as `{}`",
                    if is_opponent { "opponent" } else { "own" },
                    name
                );
            }
        }
        info!(
            "{} battlefield map keys: {:?}",
            if is_opponent { "Opponent" } else { "Own" },
            map.keys()
        );
        map
    }

    // 3. Frissítő függvény, amely a creature_positions modul szerint feltölti a battlefield_creatures vektorokat
    pub fn update_battlefield_creatures_from_ocr(bot: &mut Bot) {
        let library = build_card_library();

        // detect counts
        let own_count = Self::detect_creature_count_for_side(
            bot.screen_width as u32,
            bot.screen_height as u32,
            false,
        );
        let opp_count = Self::detect_creature_count_for_side(
            bot.screen_width as u32,
            bot.screen_height as u32,
            true,
        );
        info!("Detected own creature count: {}", own_count);
        info!("Detected opponent creature count: {}", opp_count);

        // load both sides in one line each
        bot.battlefield_creatures = bot.load_side_creatures(false, own_count, &library);
        bot.battlefield_opponent_creatures = bot.load_side_creatures(true, opp_count, &library);
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
