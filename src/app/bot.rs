// bot.rs

use std::process::Command;
use std::thread::sleep;
use std::time::{Duration, Instant};
use image::{DynamicImage, ImageBuffer, Rgba};
use image::imageops::crop_imm;
use screenshot::get_screenshot;
use crate::app::ui::{Cords, set_cursor_pos};
use crate::app::card::Creature;
use crate::app::cards_positions::get_card_positions;
use crate::app::ocr::{preprocess_image, sanitize_ocr_text};
use crate::app::ui;

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
    pub battlefield_creatures: Vec<Creature>, // A battlefield–en lévő creature–k.
}

impl Bot {
    pub fn new() -> Self {
        unsafe {
            let screen_width = winapi::um::winuser::GetSystemMetrics(winapi::um::winuser::SM_CXSCREEN);
            let screen_height = winapi::um::winuser::GetSystemMetrics(winapi::um::winuser::SM_CYSCREEN);
            let cords = ui::Cords::new(screen_width, screen_height);
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
            }
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

        // Ellenőrzés: az OCR horizontális intervallum helyes-e.
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
