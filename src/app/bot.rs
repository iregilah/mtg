// bot.rs

use std::process::Command;
use std::thread::sleep;
use std::time::{Duration, Instant};
use image::{DynamicImage, ImageBuffer, Rgba};
use image::imageops::crop_imm;
use screenshot::get_screenshot;

// Fontos: az app modulban legyenek a megfelelő folder struktúrák (ui, card, creature_positions stb.)
use crate::app::ui::{Cords, set_cursor_pos, left_click, press_key};
use crate::app::cards_positions::get_card_positions;
use crate::app::ocr::{preprocess_image, sanitize_ocr_text};
use crate::app::card_library::{CardType, CREATURE_NAMES, LAND_NAMES};

// <- Integráció: importáljuk a creature_positions modulból a két függvényt.
use crate::app::creature_positions::{get_own_creature_positions, get_opponent_creature_positions};
use crate::app::ui::{win32_get_color};

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
    pub land_number: u32,                // Az adott körben kijátszott land-ek száma (egy körben csak 1 lehet).
    pub last_opponent_turn: bool,        // Logikai érték: utolsó körben ellenfél lépett-e.
    pub opponent_turn_counter: usize,    // Az ellenfél kör számlálója.
    pub land_played_this_turn: bool,     // true, ha a jelenlegi körben már kijátszottuk a land-et.
    pub battlefield_creatures: Vec<crate::app::card_library::Card>,
    pub battlefield_opponent_creatures: Vec<crate::app::card_library::Card>,

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
                land_number: 0,
                last_opponent_turn: false,
                opponent_turn_counter: 0,
                land_played_this_turn: false,
                battlefield_creatures: Vec::new(),
                battlefield_opponent_creatures: Vec::new(),
            }
        }
    }

    pub fn play_land(&mut self) {
        if !self.land_played_this_turn {
            if let Some((index, card_text)) = self.cards_texts.iter().enumerate()
                .find(|(_i, text)| crate::app::card_library::LAND_NAMES.iter().any(|&land| text.contains(land)))
            {
                tracing::info!("Found land card '{}' at index {}. Playing it.", card_text, index);
                Self::play_card(self, index);
                self.land_number += 1;
                self.land_played_this_turn = true;
            }
        }
    }
    /// Megpróbálja kijátszani a paraméterként kapott kártyát, ha elég mana áll rendelkezésre.
    /// Ha sikeres, visszaadja a felhasznált teljes mana mennyiségét.
    fn try_cast_card(&mut self, pos: usize, card: &crate::app::card_library::Card) -> Option<u32> {
        let cost = card.mana_cost.clone();
        let colored_cost = cost.colored();
        let total_cost = cost.total();
        if self.land_number >= colored_cost {
            let leftover = self.land_number - colored_cost;
            if leftover >= cost.colorless {
                tracing::info!(
                    "Casting '{}' with cost: {} colored, {} colorless, total cost: {}",
                    card.name, colored_cost, cost.colorless, total_cost
                );
                Self::play_card(self, pos);
                sleep(Duration::from_secs(15));
                tracing::info!("!!!!!!waited 15 sec");
                return Some(total_cost);
            } else {
                tracing::info!(
                    "Not enough leftover mana for '{}'. Required {} colorless, leftover {}.",
                    card.name, cost.colorless, leftover
                );
            }
        } else {
            tracing::info!(
                "Not enough colored mana for '{}'. Required: {}, available: {}.",
                card.name, colored_cost, self.land_number
            );
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
        let card_library = crate::app::card_library::build_card_library();

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
                            // Ha creature típusú, frissítjük a battlefield creature-k tömbjét
                            if let CardType::Creature(_) = card.card_type {
                                self.battlefield_creatures.push(card.clone());
                            }
                            Bot::update_battlefield_creatures_from_ocr(self);
                        }
                    }
                }
            }
        }
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

    /// Új segédfüggvény, amely a creature casting esetét dolgozza fel.
    /// A SecondMainPhaseState-ben ezt hívjuk a process_casting() helyett,
    /// centralizálva ezzel a creature-k kijátszás logikáját.
    pub fn process_creature_casting(&mut self) {
        if self.land_number > 0 {
            let card_library = crate::app::card_library::build_card_library();
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
                tracing::info!("Creature card detected in hand. Attempting to cast creature.");
                self.cast_creatures();
                Bot::update_battlefield_creatures_from_ocr(self);
            }
        }
    }

    pub fn play_card(bot: &mut Bot, card_index: usize) {
        let positions = get_card_positions(bot.card_count, bot.screen_width as u32);
        if card_index >= positions.len() {
            tracing::error!("Error: Card index {} is out of range. Only {} cards available.", card_index, positions.len());
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
            tracing::info!("Removed card '{}' from hand at index {}.", removed, card_index);
            tracing::info!("Updated hand: {:?}", bot.cards_texts);
            bot.card_count = bot.cards_texts.len();
        } else {
            tracing::warn!("Attempted to remove card at invalid index {}.", card_index);
        }
    }


    pub fn text_contains(name: &str, ocr_text: &str) -> bool {
        //tracing::info!("text_contains() called with name = {:?} and ocr_text = {:?}", name, ocr_text);
        let result = ocr_text.contains(name);
       // tracing::info!("text_contains() returning: {}", result);
        result
    }
    // 1. Pixelátlagoló segédfüggvények
    fn get_average_color(x: i32, y: i32, width: i32, height: i32) -> (u8, u8, u8) {
        tracing::info!("get_average_color() called with x = {}, y = {}, width = {}, height = {}", x, y, width, height);
        let mut r_total: u32 = 0;
        let mut g_total: u32 = 0;
        let mut b_total: u32 = 0;
        let mut count = 0;
        for i in 0..width {
            for j in 0..height {
                let col = win32_get_color(x + i, y + j);
                // Debug log minden egyes pixelért (ez info vagy debug szintű lehet, ha túl sok)
                tracing::debug!("Pixel at ({}, {}) has color: {:?}", x + i, y + j, col);
                r_total += col.r as u32;
                g_total += col.g as u32;
                b_total += col.b as u32;
                count += 1;
            }
        }
        if count == 0 {
            tracing::error!("get_average_color(): count = 0, returning (0, 0, 0)");
            return (0, 0, 0);
        }
        let avg_r = (r_total / count) as u8;
        let avg_g = (g_total / count) as u8;
        let avg_b = (b_total / count) as u8;
        tracing::info!("get_average_color() returning average color: ({}, {}, {})", avg_r, avg_g, avg_b);
        (avg_r, avg_g, avg_b)
    }

    fn is_color_within_tolerance(color: (u8, u8, u8), target: (u8, u8, u8), tol: f64) -> bool {
        // Konvertáljuk a bemeneti értékeket f64-esre
        let (r, g, b) = (color.0 as f64, color.1 as f64, color.2 as f64);
        let (tr, tg, tb) = (target.0 as f64, target.1 as f64, target.2 as f64);

        // Számoljuk ki a három arányt, elkerülve a zéróval való osztást
        let ratio_rg = if g != 0.0 { r / g } else { 0.0 };
        let ratio_gb = if b != 0.0 { g / b } else { 0.0 };
        let ratio_rb = if b != 0.0 { r / b } else { 0.0 };

        let target_ratio_rg = if tg != 0.0 { tr / tg } else { 0.0 };
        let target_ratio_gb = if tb != 0.0 { tg / tb } else { 0.0 };
        let target_ratio_rb = if tb != 0.0 { tr / tb } else { 0.0 };

        // Számoljuk ki az arányok közötti abszolút különbségeket
        let diff_rg = (ratio_rg - target_ratio_rg).abs();
        let diff_gb = (ratio_gb - target_ratio_gb).abs();
        let diff_rb = (ratio_rb - target_ratio_rb).abs();

        let result = diff_rg <= tol && diff_gb <= tol && diff_rb <= tol;

        tracing::info!(
        "is_color_within_tolerance() called with color = {:?}, target = {:?}, tol = {}. \
         Computed ratios: (R/G: {:.3}, G/B: {:.3}, R/B: {:.3}), Target ratios: (R/G: {:.3}, G/B: {:.3}, R/B: {:.3}), \
         Differences: (R/G: {:.3}, G/B: {:.3}, R/B: {:.3}), result: {}",
        color, target, tol,
        ratio_rg, ratio_gb, ratio_rb,
        target_ratio_rg, target_ratio_gb, target_ratio_rb,
        diff_rg, diff_gb, diff_rb,
        result
    );
        result
    }
    // 2. Creature-számolási logika egy oldalon (saját vagy ellenfél)
    fn detect_creature_count_for_side(screen_width: u32, screen_height: u32, is_opponent: bool) -> usize {
        tracing::info!(
        "detect_creature_count_for_side() called with screen_width = {}, screen_height = {}, is_opponent = {}",
        screen_width, screen_height, is_opponent
    );
        let screen_width_f = screen_width as f64;
        let screen_height_f = screen_height as f64;

        // Választott y sáv az adott oldalhoz
        let (y1_norm, y2_norm) = if is_opponent {
            (101.761, 104.891)
        } else {
            (185.141, 188.731)
        };
        let y1 = ((y1_norm / 381.287) * screen_height_f).floor() as i32;
        let y2 = ((y2_norm / 381.287) * screen_height_f).floor() as i32;
        let region_height = y2 - y1;
        let rect_width = ((4.4 / 677.292) * screen_width_f).floor() as i32;
        let screen_center_x = (screen_width as i32) / 2;
        let center_rect_x = screen_center_x - rect_width / 2;
        tracing::info!(
        "Calculated values: y1 = {}, y2 = {}, region_height = {}, rect_width = {}, screen_center_x = {}, center_rect_x = {}",
        y1, y2, region_height, rect_width, screen_center_x, center_rect_x
    );

        let target_color = (210, 175, 157);
        let tol = 0.05;

        // Középső pixel vizsgálat
        let center_color = Self::get_average_color(center_rect_x, y1, rect_width, region_height);
        tracing::info!("Center area average color: {:?}", center_color);
        let center_is_card = Self::is_color_within_tolerance(center_color, target_color, tol);
        tracing::info!("Center area considered as card: {}", center_is_card);

        if center_is_card {
            // Páratlan ág: indulás 1 creature-vel, majd lépésenként +2
            let mut count = 1;
            let step = ((69.0 / 677.292) * screen_width_f).floor() as i32;
            let mut current_center_x = screen_center_x - step;
            tracing::info!("Starting odd branch: initial count = {}, step = {}", count, step);
            while count < 7 && (current_center_x - rect_width / 2 >= 0) {
                let sample_x = current_center_x - rect_width / 2;
                let sample_color = Self::get_average_color(sample_x, y1, rect_width, region_height);
                tracing::info!(
                "Odd branch: current_center_x = {}, sample_x = {}, sample_color = {:?}",
                current_center_x, sample_x, sample_color
            );
                if Self::is_color_within_tolerance(sample_color, target_color, tol) {
                    count += 2;
                    tracing::info!("Odd branch: increasing count, new count = {}", count);
                    current_center_x -= step;
                } else {
                    tracing::info!("Odd branch: sample color not within tolerance, breaking loop");
                    break;
                }
            }
            tracing::info!("Odd branch final creature count = {}", count);
            count
        } else {
            // Páros ág: indulás 0 creature-vel, majd lépésenként +2
            let mut count = 0;
            let start_x = ((34.492 / 677.292) * screen_width_f).floor() as i32;
            let step = ((69.0 / 677.292) * screen_width_f).floor() as i32;
            let mut current_x = start_x;
            tracing::info!("Starting even branch: initial count = {}, start_x = {}, step = {}", count, start_x, step);
            while count < 8 && (current_x - rect_width / 2 >= 0) {
                let sample_x = current_x - rect_width / 2;
                let sample_color = Self::get_average_color(sample_x, y1, rect_width, region_height);
                tracing::info!(
                "Even branch: current_x = {}, sample_x = {}, sample_color = {:?}",
                current_x, sample_x, sample_color
            );
                if Self::is_color_within_tolerance(sample_color, target_color, tol) {
                    count += 2;
                    tracing::info!("Even branch: increasing count, new count = {}", count);
                    current_x -= step;
                } else {
                    tracing::info!("Even branch: sample color not within tolerance, breaking loop");
                    break;
                }
            }
            tracing::info!("Even branch final creature count = {}", count);
            count
        }
    }

    // 3. Frissítő függvény, amely a creature_positions modul szerint feltölti a battlefield_creatures vektorokat
    pub fn update_battlefield_creatures_from_ocr(bot: &mut Bot) {
        let own_count = Self::detect_creature_count_for_side(bot.screen_width as u32, bot.screen_height as u32, false);
        let opp_count = Self::detect_creature_count_for_side(bot.screen_width as u32, bot.screen_height as u32, true);

        tracing::info!("Detected own creature count: {}", own_count);
        tracing::info!("Detected opponent creature count: {}", opp_count);

        // Saját creature–ök feltöltése
        let own_positions = get_own_creature_positions(own_count, bot.screen_width as u32, bot.screen_height as u32);
        bot.battlefield_creatures.clear();
        for (i, _pos) in own_positions.iter().enumerate() {
            let dummy_creature = crate::app::card_library::Card {
                name: format!("Saját Creature {}", i + 1),
                card_type: crate::app::card_library::CardType::Creature(
                    crate::app::card_library::Creature {
                        name: format!("Saját Creature {}", i + 1),
                        summoning_sickness: false,
                        power: 1,
                        toughness: 1,
                    }
                ),
                mana_cost: crate::app::card_library::ManaCost { colorless: 0, red: 0, blue: 0, green: 0, black: 0, white: 0 },
                attributes: vec![],
                triggers: vec![],
            };
            bot.battlefield_creatures.push(dummy_creature);
        }

        // Ellenfél creature–ök feltöltése
        let opp_positions = get_opponent_creature_positions(opp_count, bot.screen_width as u32, bot.screen_height as u32);
        bot.battlefield_opponent_creatures.clear();
        for (i, _pos) in opp_positions.iter().enumerate() {
            let dummy_creature = crate::app::card_library::Card {
                name: format!("Ellenfél Creature {}", i + 1),
                card_type: crate::app::card_library::CardType::Creature(
                    crate::app::card_library::Creature {
                        name: format!("Ellenfél Creature {}", i + 1),
                        summoning_sickness: false,
                        power: 1,
                        toughness: 1,
                    }
                ),
                mana_cost: crate::app::card_library::ManaCost { colorless: 0, red: 0, blue: 0, green: 0, black: 0, white: 0 },
                attributes: vec![],
                triggers: vec![],
            };
            bot.battlefield_opponent_creatures.push(dummy_creature);
        }
    }

    pub fn examine_cards(&mut self) {
        self.cards_texts.clear(); // Töröljük a korábbi eredményeket.
        for i in 0..self.card_count {
            let text = self.get_card_text(i);
            tracing::info!("Card {} text: {}", i, text);
            self.cards_texts.push(text);
        }
        tracing::info!("OCR results for cards: {:?}", self.cards_texts);
    }

    pub fn get_card_text(&self, index: usize) -> String {
        let positions = get_card_positions(self.card_count, self.screen_width as u32);
        if index >= positions.len() {
            tracing::error!("Index {} out of range", index);
            return String::new();
        }
        let pos = positions[index];
        // A képernyő alsó 97%-a
        let card_y = ((self.screen_height as f64) * 0.97).floor() as i32;
        set_cursor_pos(pos.hover_x as i32, card_y);
        tracing::info!("Hovering over card {} at ({}, {})", index, pos.hover_x, card_y);
        sleep(Duration::from_secs(2));

        // Képernyőkép készítése
        let screenshot = match get_screenshot(0) {
            Ok(scn) => scn,
            Err(_) => {
                tracing::error!("Screenshot error on card {}", index);
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
            tracing::error!("Image buffer error on card {}", index);
            return String::new();
        }
        let image_buf = image_buf_opt.unwrap();
        let dyn_img = DynamicImage::ImageRgba8(image_buf);

        if pos.ocr_x2 <= pos.ocr_x1 || pos.ocr_x2 > width {
            tracing::error!("Invalid OCR horizontal region for card {}", index);
            return String::new();
        }
        let ocr_y1 = ((232.606 / 381.287) * (self.screen_height as f64)).floor() as u32;
        let ocr_y2 = ((240.832 / 381.287) * (self.screen_height as f64)).floor() as u32;
        if ocr_y2 <= ocr_y1 {
            tracing::error!("Invalid OCR vertical region.");
            return String::new();
        }
        let cropped = crop_imm(&dyn_img, pos.ocr_x1, ocr_y1, pos.ocr_x2 - pos.ocr_x1, ocr_y2 - ocr_y1)
            .to_image();
        let preprocessed = preprocess_image(&DynamicImage::ImageRgba8(cropped));
        let temp_filename = format!("temp_card_{}.png", index);
        if let Err(e) = preprocessed.save(&temp_filename) {
            tracing::error!("Error saving temporary image for card {}: {:?}", index, e);
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
                tracing::error!("Tesseract error for card {}: {}", index, String::from_utf8_lossy(&output.stderr));
                String::from("OCR failed")
            }
            Err(e) => {
                tracing::error!("Error running Tesseract for card {}: {:?}", index, e);
                String::from("OCR failed")
            }
        };
        tracing::info!("Card {} text: {}", index, card_text);
        card_text
    }
}
