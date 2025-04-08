use crate::app::card;
use crate::app::cards_positions;
use crate::app::ocr;
use crate::app::ui;
use crate::app::ui::Cords;
use std::ffi::OsStr;
use std::os::windows::ffi::OsStrExt;
use std::ptr::null_mut;
use std::sync::{Arc, Mutex};
use std::thread::sleep;
use std::time::{Duration, Instant};
use winapi::shared::windef::HWND;
use winapi::um::wingdi::GetPixel;
use winapi::um::winuser::{
    FindWindowW, GetDC, KEYEVENTF_KEYUP, MOUSEEVENTF_LEFTDOWN, MOUSEEVENTF_LEFTUP, ReleaseDC,
    SetCursorPos, keybd_event, mouse_event,
};

#[derive(Debug, Clone)]
pub struct Bot {
    pub cords: Cords,
    pub screen_width: i32,
    pub screen_height: i32,
    // A mulligan és játékmenethez szükséges mezők
    pub card_count: usize,
    pub cards_texts: Vec<String>,
    pub land_number: u32,
    pub last_opponent_turn: bool,
    pub opponent_turn_counter: usize,
    pub land_played_this_turn: bool,
    pub battlefield_creatures: Vec<card::Creature>,
    // Játékvégi és időzítéshez szükséges mezők
    pub end_game_counter: u32,
    pub end_game_threshold: u32,
    pub time_game_started: Instant,
    pub time_game_threshold: Duration,
    pub time_waiting_started: Instant,
    pub time_waiting_threshold: Duration,
}

impl Bot {
    pub fn new() -> Self {
        let screen_width;
        let screen_height;
        unsafe {
            screen_width = winapi::um::winuser::GetSystemMetrics(winapi::um::winuser::SM_CXSCREEN);
            screen_height = winapi::um::winuser::GetSystemMetrics(winapi::um::winuser::SM_CYSCREEN);
        }
        Self {
            cords: Cords::new(screen_width, screen_height),
            screen_width,
            screen_height,
            card_count: 0,
            cards_texts: Vec::new(),
            land_number: 0,
            last_opponent_turn: false,
            opponent_turn_counter: 0,
            land_played_this_turn: false,
            battlefield_creatures: Vec::new(),
            end_game_counter: 0,
            end_game_threshold: 3,
            time_game_started: Instant::now(),
            time_game_threshold: Duration::from_secs(1200), // 20 perc
            time_waiting_started: Instant::now(),
            time_waiting_threshold: Duration::from_secs(120), // 2 perc
        }
    }

    // Alap UI funkciók (még régi kódból is átemelve)
    pub fn left_click(&self) {
        unsafe {
            mouse_event(MOUSEEVENTF_LEFTDOWN, 0, 0, 0, 0);
            sleep(Duration::from_millis(100));
            mouse_event(MOUSEEVENTF_LEFTUP, 0, 0, 0, 0);
        }
        sleep(Duration::from_millis(100));
    }

    pub fn set_cursor_pos(&self, x: i32, y: i32) {
        unsafe {
            SetCursorPos(x, y);
        }
        sleep(Duration::from_millis(100));
    }

    pub fn press_key(&self, vk: u16) {
        unsafe {
            keybd_event(vk as u8, 0, 0, 0);
            sleep(Duration::from_millis(50));
            keybd_event(vk as u8, 0, KEYEVENTF_KEYUP, 0);
        }
        sleep(Duration::from_millis(100));
    }

    pub fn win32_get_color(&self, x: i32, y: i32) -> ui::Color {
        unsafe {
            let hdc = GetDC(null_mut());
            let pixel = GetPixel(hdc, x, y);
            ReleaseDC(null_mut(), hdc);
            ui::Color {
                r: (pixel & 0x0000FF) as u8,
                g: ((pixel & 0x00FF00) >> 8) as u8,
                b: ((pixel & 0xFF0000) >> 16) as u8,
            }
        }
    }

    pub fn find_window(&self, title: &str) -> Option<HWND> {
        use std::iter::once;
        let wide: Vec<u16> = OsStr::new(title).encode_wide().chain(once(0)).collect();
        unsafe {
            let hwnd = FindWindowW(null_mut(), wide.as_ptr());
            if hwnd.is_null() {
                None
            } else {
                Some(hwnd)
            }
        }
    }

    // --- Játékelemek az eredeti logika alapján ---

    /// A start fázis: ablak fókusz, home/play gombok stb.
    pub fn start_game(&mut self) {
        tracing::info!("{} Starting", chrono::Local::now().format("%Y-%m-%d %H:%M"));
        sleep(Duration::from_secs(5));
        self.set_cursor_pos(self.cords.home_button.0, self.cords.home_button.1);
        self.left_click();
        sleep(Duration::from_secs(1));
        if let Some(hwnd) = self.find_window("MTGA") {
            unsafe {
                winapi::um::winuser::SetForegroundWindow(hwnd);
                winapi::um::winuser::SetActiveWindow(hwnd);
            }
        }
        self.set_cursor_pos(self.cords.play_button.0, self.cords.play_button.1);
        sleep(Duration::from_millis(500));
        self.left_click();
        self.press_key(winapi::um::winuser::VK_SPACE as u16);
        sleep(Duration::from_millis(500));
        self.left_click();
        self.press_key(winapi::um::winuser::VK_SPACE as u16);
        sleep(Duration::from_millis(500));
        self.left_click();
        sleep(Duration::from_millis(500));
        self.left_click();
        tracing::info!("Start phase completed.");
    }

    /// A mulligan (loading) fázis, ahol az OCR alapján várjuk a start order szöveget.
    pub fn loading(&mut self) {
        self.time_waiting_started = Instant::now();
        tracing::info!("Waiting for Mulligan state... (Mulligan logic)");
        loop {
            let start_order_text =
                ocr::check_start_order_text(self.screen_width as u32, self.screen_height as u32);
            tracing::info!("Start order region text: {}", start_order_text);
            if start_order_text == "You Go First" || start_order_text == "Opponent Goes First" {
                if start_order_text == "Opponent Goes First" {
                    self.card_count = 8;
                    tracing::info!("Opponent started. Card count set to 8.");
                } else {
                    self.card_count = 7;
                    tracing::info!("We started. Card count remains 7.");
                }
                self.press_key(winapi::um::winuser::VK_SPACE as u16);
                break;
            }
            if Instant::now().duration_since(self.time_waiting_started) > self.time_waiting_threshold {
                tracing::warn!("Mulligan waiting time passed. Exiting mulligan loop...");
                break;
            }
            sleep(Duration::from_secs(2));
        }
        tracing::info!("Mulligan state completed. Ready.");
        sleep(Duration::from_secs(1));
    }

    /// A kártyák hooverelése – itt az OCR eredményeket kéne beolvasni.
    /// (A valós implementációban itt kellene iterálni a kártyapozíciókon.)
    pub fn examine_cards(&mut self) {
        self.cards_texts.clear();
        tracing::info!("Examining cards (dummy implementation).");
        // Például itt az OCR eredmények beolvasása történhetne.
    }

    /// A "Submit 0" fázis kezelése.
    pub fn handle_submit_phase(&mut self) {
        tracing::info!("Entering handle_submit_phase()");
        let positions = cards_positions::get_card_positions(self.card_count, self.screen_width as u32);
        if self.card_count >= 4 {
            let pos = positions[3]; // 4. kártya
            let card_y = ((self.screen_height as f64) * 0.97).ceil() as i32;
            self.set_cursor_pos(pos.hover_x as i32, card_y);
            self.left_click();
            tracing::info!("Clicked 4th card for 'Submit 0'.");
        } else {
            tracing::warn!("Not enough cards for Submit 0 action.");
        }
    }

    /// Az első main phase: land lehelyezése és creature kijátszás.
    pub fn handle_first_main_phase(&mut self) {
        tracing::info!("Entering first main phase...");
        if !self.land_played_this_turn {
            for (i, text) in self.cards_texts.iter().enumerate() {
                if card::LAND_NAMES.iter().any(|&land| text.contains(land)) {
                    tracing::info!("Found land card '{}' at index {}. Playing it.", text, i);
                    self.play_card(i);
                    self.land_number += 1;
                    self.land_played_this_turn = true;
                    break;
                }
            }
        }
        let mut mana_available = self.land_number;
        tracing::info!("Available mana for this turn after playing lands: {}", mana_available);

        let creature_indices: Vec<usize> = self
            .cards_texts
            .iter()
            .enumerate()
            .filter(|(_, text)| card::CREATURE_NAMES.iter().any(|&name| text.contains(name)))
            .map(|(i, _)| i)
            .collect();

        for &index in creature_indices.iter() {
            if let Some(card) = card::parse_card(&self.cards_texts[index]) {
                if let card::CardType::Creature(creature) = card {
                    let cost = card::parse_mana_cost(&creature.name);
                    let colored_cost = cost.colored();
                    let total_cost = cost.total();
                    if mana_available >= colored_cost {
                        let leftover = mana_available - colored_cost;
                        if leftover >= cost.colorless {
                            tracing::info!(
                                "Casting creature '{}' ({} colorless, {} colored), total cost = {}",
                                creature.name,
                                cost.colorless,
                                colored_cost,
                                total_cost
                            );
                            self.play_card(index);
                            self.battlefield_creatures.push(creature);
                            mana_available -= total_cost;
                        } else {
                            tracing::info!(
                                "Not enough leftover for colorless after paying colored mana for '{}'. Required: {} colorless, leftover: {}",
                                creature.name,
                                cost.colorless,
                                leftover
                            );
                        }
                    } else {
                        tracing::info!(
                            "Not enough colored mana to cast '{}'. Required: {} colored, available: {}",
                            creature.name,
                            colored_cost,
                            mana_available
                        );
                    }
                }
            }
        }
        tracing::info!("First main phase finished. Remaining mana: {}.", mana_available);
    }

    /// Kártya kijátszás: elvégzi az egérmozgásokat, klikkeléseket, billentyű lenyomásokat, majd törli a kártyát a kézből.
    pub fn play_card(&mut self, card_index: usize) {
        let positions = cards_positions::get_card_positions(self.card_count, self.screen_width as u32);
        if card_index >= positions.len() {
            tracing::error!(
                "Error: Card index {} is out of range. Only {} cards available.",
                card_index,
                positions.len()
            );
            return;
        }
        let pos = positions[card_index];
        let card_y = ((self.screen_height as f64) * 0.97).ceil() as i32;
        self.set_cursor_pos(pos.hover_x as i32, card_y);
        self.left_click();
        self.left_click();
        self.set_cursor_pos(self.screen_width - 1, self.screen_height - 1);
        self.press_key(0x5A); // 'Z' billentyű
        self.left_click();
        sleep(Duration::from_millis(150));
        if card_index < self.cards_texts.len() {
            tracing::info!("Removing card at index {} from hand.", card_index);
            self.cards_texts.remove(card_index);
            self.card_count = self.cards_texts.len();
        }
    }

    /// Támadási fázis: amennyiben van támadható creature, az egér és OCR alapján végrehajtja a támadást.
    pub fn handle_attack_phase(&mut self) {
        tracing::info!("Entering attack phase...");
        let can_attack = self
            .battlefield_creatures
            .iter()
            .any(|creature| !creature.summoning_sickness);
        if !can_attack {
            tracing::info!("No creature can attack. Transitioning to opponent's turn.");
            self.handle_opponents_turn();
            return;
        }
        loop {
            let is_red = ui::check_button_color(&self.cords) == "red";
            let main_text =
                ocr::check_main_region_text(self.screen_width as u32, self.screen_height as u32, is_red);
            tracing::info!("(Attack phase) Main region text: {}", main_text);
            if main_text.contains("All Attack") {
                self.press_key(winapi::um::winuser::VK_SPACE as u16);
                break;
            } else if main_text.contains("Next") {
                self.press_key(winapi::um::winuser::VK_SPACE as u16);
            }
            sleep(Duration::from_secs(2));
        }
    }

    /// Második main phase: az End Turn kezelés.
    pub fn second_main_phase(&self) {
        tracing::info!("Entering second main phase...");
        loop {
            let main_text =
                ocr::check_main_region_text(self.screen_width as u32, self.screen_height as u32, false);
            tracing::info!("(Second main phase) Main region text: {}", main_text);
            if main_text.contains("End Turn") {
                ui::press_key(winapi::um::winuser::VK_SPACE as u16);
                break;
            } else if main_text.contains("Next") {
                ui::press_key(winapi::um::winuser::VK_SPACE as u16);
            }
            sleep(Duration::from_secs(2));
        }
    }

    /// Az ellenfél körének kezelése.
    pub fn handle_opponents_turn(&mut self) {
        tracing::info!("Handling opponent's turn...");
        loop {
            let is_red = ui::check_button_color(&self.cords) == "red";
            let main_text =
                ocr::check_main_region_text(self.screen_width as u32, self.screen_height as u32, is_red);
            tracing::info!("(Opponent turn) Main region text: {}", main_text);
            if main_text.contains("Next") {
                tracing::info!("Opponent turn phase finished.");
                break;
            }
            sleep(Duration::from_secs(2));
        }
    }
}
