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


    // 1. Pixelátlagoló segédfüggvények
    fn get_average_color(x: i32, y: i32, width: i32, height: i32) -> (u8, u8, u8) {
        let mut r_total: u32 = 0;
        let mut g_total: u32 = 0;
        let mut b_total: u32 = 0;
        let mut count = 0;
        for i in 0..width {
            for j in 0..height {
                let col = win32_get_color(x + i, y + j);
                r_total += col.r as u32;
                g_total += col.g as u32;
                b_total += col.b as u32;
                count += 1;
            }
        }
        if count == 0 {
            return (0, 0, 0);
        }
        (
            (r_total / count) as u8,
            (g_total / count) as u8,
            (b_total / count) as u8,
        )
    }

    fn is_color_within_tolerance(color: (u8, u8, u8), target: (u8, u8, u8), tol: i32) -> bool {
        let (r, g, b) = color;
        let (tr, tg, tb) = target;
        (r as i32 - tr as i32).abs() <= tol &&
            (g as i32 - tg as i32).abs() <= tol &&
            (b as i32 - tb as i32).abs() <= tol
    }

    // 2. Creature-számolási logika egy oldalon (saját vagy ellenfél)
    fn detect_creature_count_for_side(screen_width: u32, screen_height: u32, is_opponent: bool) -> usize {
        let screen_width_f = screen_width as f64;
        let screen_height_f = screen_height as f64;
        // Választott y sáv az adott oldalhoz
        let (y1_norm, y2_norm) = if is_opponent {
            (101.761, 104.891)
        } else {
            (185.141, 188.731)
        };
        let y1 = ((y1_norm / 381.287) * screen_height_f).ceil() as i32;
        let y2 = ((y2_norm / 381.287) * screen_height_f).ceil() as i32;
        let region_height = y2 - y1;
        // A vizsgálandó téglalap szélessége: (4.4 / 677.292) * screen_width
        let rect_width = ((4.4 / 677.292) * screen_width_f).ceil() as i32;
        let screen_center_x = (screen_width as i32) / 2;
        let center_rect_x = screen_center_x - rect_width / 2;

        let target_color = (210, 175, 157);
        let tol = 10;

        // Először vizsgáljuk a képernyő közepén levő téglalapot
        let center_color = Self::get_average_color(center_rect_x, y1, rect_width, region_height);
        let center_is_card = Self::is_color_within_tolerance(center_color, target_color, tol);

        if center_is_card {
            // Páratlan eset: indulás 1 creature-vel, majd lépésenként +2 addig, amíg maximum 7 creature van
            let mut count = 1;
            let step = ((69.0 / 677.292) * screen_width_f).ceil() as i32;
            let mut current_center_x = screen_center_x - step;
            while count < 7 && current_center_x - rect_width / 2 >= 0 {
                let sample_x = current_center_x - rect_width / 2;
                let sample_color = Self::get_average_color(sample_x, y1, rect_width, region_height);
                if Self::is_color_within_tolerance(sample_color, target_color, tol) {
                    count += 2;
                    current_center_x -= step;
                } else {
                    break;
                }
            }
            count
        } else {
            // Páros eset: indulás 0 creature-vel, kezdő pozíció: (34.492/677.292)*screen_width,
            // majd lépésenként +2, maximum 8 creature
            let mut count = 0;
            let start_x = ((34.492 / 677.292) * screen_width_f).ceil() as i32;
            let step = ((69.0 / 677.292) * screen_width_f).ceil() as i32;
            let mut current_x = start_x;
            while count < 8 && current_x - rect_width / 2 >= 0 {
                let sample_x = current_x - rect_width / 2;
                let sample_color = Self::get_average_color(sample_x, y1, rect_width, region_height);
                if Self::is_color_within_tolerance(sample_color, target_color, tol) {
                    count += 2;
                    current_x -= step;
                } else {
                    break;
                }
            }
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
        let card_y = ((self.screen_height as f64) * 0.97).ceil() as i32;
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
        let ocr_y1 = ((232.606 / 381.287) * (self.screen_height as f64)).ceil() as u32;
        let ocr_y2 = ((240.832 / 381.287) * (self.screen_height as f64)).ceil() as u32;
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
