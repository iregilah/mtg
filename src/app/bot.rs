// app/bot.rs


use crate::app::ocr::sanitize_ocr_text;
use crate::app::ocr::preprocess_image;
use image::imageops::crop_imm;
use image::Rgba;
use image::{DynamicImage, ImageBuffer};
use std::{
    collections::HashMap,
    process::Command,
    thread::sleep,
    time::{Duration, Instant},
};
use tracing::{error, info, warn};

use screenshot::get_screenshot;

use crate::app::{
    card_library::{build_card_library, Card, CardType, LAND_NAMES},
    cards_positions::get_card_positions,
    creature_positions::{get_own_creature_positions, get_opponent_creature_positions},
    ui::{Cords, set_cursor_pos, left_click, press_key, get_average_color, is_color_within_tolerance},
    ocr::read_creature_text,
};

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

}

pub enum StateOverride {
    OpponentsTurn,
}

impl Bot {
    pub fn new() -> Self {
        unsafe {
            let screen_width = winapi::um::winuser::GetSystemMetrics(winapi::um::winuser::SM_CXSCREEN);
            let screen_height = winapi::um::winuser::GetSystemMetrics(winapi::um::winuser::SM_CYSCREEN);
            let cords = Cords::new(screen_width, screen_height);
            Self {
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
            }
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
            let t = self.get_card_text(new_index);
            self.card_count = old;
            t
        };

        self.cards_texts.push(text.clone());
        self.card_count = new_count;
        info!("Drew card '{}' → Updated hand: {:?}", text, self.cards_texts);
    }
    pub fn play_land(&mut self) {
        if !self.land_played_this_turn {
            if let Some((index, card_text)) = self.cards_texts.iter().enumerate()
                .find(|(_i, text)| LAND_NAMES.iter().any(|&land| text.contains(land)))
            {
                info!("Found land card '{}' at index {}. Playing it.", card_text, index);
                Self::play_card(self, index);
                self.land_number += 1;
                self.land_count += 1;
                self.land_played_this_turn = true;
            }
        }
        sleep(Duration::from_secs(1));
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
                if let Some(cost) = self.try_cast_card(i, card) {
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
            }
        }
    }
    /// Megpróbálja kijátszani a paraméterként kapott kártyát, ha elég mana áll rendelkezésre.
    /// Ha sikeres, visszaadja a felhasznált teljes mana mennyiségét.
    fn try_cast_card(&mut self, pos: usize, card: &Card) -> Option<u32> {
        let cost = card.mana_cost.clone();
        let colored_cost = cost.colored();
        let total_cost = cost.total();
        if self.land_number >= colored_cost {
            let leftover = self.land_number - colored_cost;
            if leftover >= cost.colorless {
                info!("Casting '{}' with cost: {} colored, {} colorless, total cost: {}", card.name, colored_cost, cost.colorless, total_cost);
                Self::play_card(self, pos);
                self.last_cast_card_name = card.name.clone();
                sleep(Duration::from_secs(3)); //TODO this time should me smaller
                info!("!!!!!!waited 3 sec");
                return Some(total_cost);
            } else {
                info!("Not enough leftover mana for '{}'. Required {} colorless, leftover {}.", card.name, cost.colorless, leftover);
            }
        } else {
            info!("Not enough colored mana for '{}'. Required: {}, available: {}.", card.name, colored_cost, self.land_number);
        }
        None
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
                        if let Some(cost_used) = self.try_cast_card(i, card) {
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
                if let Some(cost_used) = self.try_cast_card(i, card) {
                    // sikeres cast: térjünk vissza a névvel és a költséggel
                    return Some((card.name.clone(), cost_used));
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


    // 3. Frissítő függvény, amely a creature_positions modul szerint feltölti a battlefield_creatures vektorokat
    pub fn update_battlefield_creatures_from_ocr(bot: &mut Bot) {
        // először is előállítjuk a teljes kártyatárat
        let library: HashMap<String, Card> = build_card_library();

        // 1) hány creature van a mezőn mindkét oldalon?
        let own_count = Self::detect_creature_count_for_side(bot.screen_width as u32, bot.screen_height as u32, false);
        let opp_count = Self::detect_creature_count_for_side(bot.screen_width as u32, bot.screen_height as u32, true);
        info!("Detected own creature count: {}", own_count);
        info!("Detected opponent creature count: {}", opp_count);

        // 2) saját creature-k betöltése
        bot.battlefield_creatures.clear();
        let own_positions = get_own_creature_positions(own_count, bot.screen_width as u32, bot.screen_height as u32);
        for (i, pos) in own_positions.into_iter().enumerate() {
            let name = read_creature_text(pos, i + 1, false, bot.screen_width as u32, bot.screen_height as u32);
            if let Some(card) = library.get(&name) {
                info!("Found own battlefield creature: {}", name);
                bot.battlefield_creatures.insert(name.clone(), card.clone());
            } else if !name.is_empty() {
                warn!("Unknown own battlefield creature OCR’d as `{}`", name);
            }
        }
        info!("Own battlefield map keys: {:?}", bot.battlefield_creatures.keys());

        // 3) ellenfél creature-k betöltése
        bot.battlefield_opponent_creatures.clear();
        let opp_positions = get_opponent_creature_positions(opp_count, bot.screen_width as u32, bot.screen_height as u32);
        for (i, pos) in opp_positions.into_iter().enumerate() {
            let name = read_creature_text(pos, i + 1, true, bot.screen_width as u32, bot.screen_height as u32);
            if let Some(card) = library.get(&name) {
                info!("Found opponent battlefield creature: {}", name);
                bot.battlefield_opponent_creatures.insert(name.clone(), card.clone());
            } else if !name.is_empty() {
                warn!("Unknown opponent battlefield creature OCR’d as `{}`", name);
            }
        }
        info!("Opponent battlefield map keys: {:?}", bot.battlefield_opponent_creatures.keys());
    }


    pub fn examine_cards(&mut self) {
        self.cards_texts.clear(); // Töröljük a korábbi eredményeket.
        for i in 0..self.card_count {
            let text = self.get_card_text(i);
            info!("Card {} text: {}", i, text);
            self.cards_texts.push(text);
        }
        info!("OCR results for cards: {:?}", self.cards_texts);
    }

    pub fn get_card_text(&self, index: usize) -> String {
        let positions = get_card_positions(self.card_count, self.screen_width as u32);
        if index >= positions.len() {
            error!("Index {} out of range", index);
            return String::new();
        }
        let pos = positions[index];
        // A képernyő alsó 97%-a
        let card_y = ((self.screen_height as f64) * 0.97).floor() as i32;
        set_cursor_pos(pos.hover_x as i32, card_y);
        info!("Hovering over card {} at ({}, {})", index, pos.hover_x, card_y);
        sleep(Duration::from_secs(2));

        // Képernyőkép készítése
        let screenshot = match get_screenshot(0) {
            Ok(scn) => scn,
            Err(_) => {
                error!("Screenshot error on card {}", index);
                return String::new();
            }
        };
        let width = screenshot.width() as u32;
        let height = screenshot.height() as u32;
        let data_vec = unsafe {
            std::slice::from_raw_parts(screenshot.raw_data(), screenshot.raw_len()).to_vec()
        };
        let image_buf_opt = ImageBuffer::<Rgba<u8>, _>::from_raw(width, height, data_vec);
        if image_buf_opt.is_none() {
            error!("Image buffer error on card {}", index);
            return String::new();
        }
        let image_buf = image_buf_opt.unwrap();
        let dyn_img = DynamicImage::ImageRgba8(image_buf);

        if pos.ocr_x2 <= pos.ocr_x1 || pos.ocr_x2 > width {
            error!("Invalid OCR horizontal region for card {}", index);
            return String::new();
        }
        let ocr_y1 = ((232.606 / 381.287) * (self.screen_height as f64)).floor() as u32;
        let ocr_y2 = ((240.832 / 381.287) * (self.screen_height as f64)).floor() as u32;
        if ocr_y2 <= ocr_y1 {
            error!("Invalid OCR vertical region.");
            return String::new();
        }
        let cropped = crop_imm(&dyn_img, pos.ocr_x1, ocr_y1, pos.ocr_x2 - pos.ocr_x1, ocr_y2 - ocr_y1)
            .to_image();
        let preprocessed = preprocess_image(&DynamicImage::ImageRgba8(cropped));
        let temp_filename = format!("temp_card_{}.png", index);
        if let Err(e) = preprocessed.save(&temp_filename) {
            error!("Error saving temporary image for card {}: {:?}", index, e);
            return String::new();
        }
        let output = Command::new(r"C:\Program Files\Tesseract-OCR\tesseract.exe")
            .arg(&temp_filename)
            .arg("stdout")
            .arg("-l")
            .arg("eng")
            .arg("--psm")
            .arg("7")
            .output();
        let card_text = match output {
            Ok(output) if output.status.success() => {
                let raw_text = String::from_utf8_lossy(&output.stdout).into_owned();
                sanitize_ocr_text(raw_text.trim())
            }
            Ok(output) => {
                error!("Tesseract error for card {}: {}", index, String::from_utf8_lossy(&output.stderr));
                String::from("OCR failed")
            }
            Err(e) => {
                error!("Error running Tesseract for card {}: {:?}", index, e);
                String::from("OCR failed")
            }
        };
        info!("Card {} text: {}", index, card_text);
        card_text
    }
}

