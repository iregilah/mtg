// src/app/game_state_updater.rs

use tracing::{info, warn};
use crate::app::game_state::GameState;
use crate::app::gre::{PriorityEntry, StackEntry};
use crate::app::card_library::{build_card_library, Card, CardTypeFlags};
use crate::app::ocr;
use crate::app::creature_positions::{get_own_creature_positions, get_opponent_creature_positions};
use crate::app::cards_positions::get_card_positions;
use crate::app::ocr::{read_creature_text};
use crate::app::ui::{get_average_color, is_color_within_tolerance};
use std::collections::{HashMap, BinaryHeap};
use crate::app::bot::Bot;


/// Generic branch counter (odd/even).
pub fn count_branch(
    y1: i32,
    region_height: i32,
    rect_width: i32,
    mut x: i32,
    tol: f64,
    target_color: (u8, u8, u8),
    initial_count: usize,
    first_step: i32,
    step: i32,
    max_count: usize,
) -> usize {
    let mut count = initial_count;
    info!(
        "count_branch: init={}, first_step={}, step={}, max={}",
        count, first_step, step, max_count
    );
    x -= first_step;
    while count < max_count && x >= 0 {
        let c = get_average_color(x, y1, rect_width, region_height);
        info!("  sample @ x={} → {:?}", x, c);
        if is_color_within_tolerance(c, target_color, tol) {
            count += 2;
            x -= step;
        } else {
            break;
        }
    }
    info!("count_branch final count={}", count);
    count
}

/// Detect creature count via OCR for a side.
pub fn detect_creature_count_for_side(
    screen_width: u32,
    screen_height: u32,
    is_opponent: bool,
) -> usize {
    info!(
        "detect_creature_count_for_side({}, {}, {})",
        screen_width, screen_height, is_opponent
    );
    let sw = screen_width as f64;
    let sh = screen_height as f64;
    let (y1_norm, y2_norm) = if is_opponent {
        (101.761, 104.891)
    } else {
        (185.141, 188.731)
    };
    let y1 = ((y1_norm / 381.287) * sh).floor() as i32;
    let y2 = ((y2_norm / 381.287) * sh).floor() as i32;
    let region_h = y2 - y1;
    let rect_w = ((4.4 / 677.292) * sw).floor() as i32;
    let center_x = (screen_width as i32) / 2 - rect_w / 2;

    let target_color = (210, 175, 157);
        let tol = 0.035;

    let center_color = get_average_color(center_x, y1, rect_w, region_h);
    info!("  center color: {:?}", center_color);
    let center_is_card = is_color_within_tolerance(center_color, target_color, tol);

    let scale = sw / 677.292;
    let step = (69.0 * scale).floor() as i32;
    let first = (34.492 * scale).floor() as i32;

    if center_is_card {
        count_branch(y1, region_h, rect_w, center_x, tol, target_color, 1, step, step, 7)
    } else {
        count_branch(y1, region_h, rect_w, center_x, tol, target_color, 0, first, step, 8)
    }
}

/// Load and OCR-recognize creatures on a side.
pub fn load_side_creatures(
    screen_width: u32,
    screen_height: u32,
    is_opponent: bool,
) -> HashMap<String, Card> {
    let mut map = HashMap::new();
    let library = build_card_library();
    let count = detect_creature_count_for_side(screen_width, screen_height, is_opponent);
    let positions = if is_opponent {
        get_opponent_creature_positions(count, screen_width, screen_height)
    } else {
        get_own_creature_positions(count, screen_width, screen_height)
    };

    for (i, pos) in positions.into_iter().enumerate() {
        let name = read_creature_text(pos, i + 1, is_opponent, screen_width, screen_height);
        if let Some(card) = library.get(&name) {
            map.insert(name.clone(), card.clone());
        } else if !name.is_empty() {
            warn!("Unknown OCR creature `{}` on {}", name, if is_opponent{"opponent"}else{"own"} );
        }
    }
    map
}

/// Centralized GameState updater.
pub struct GameStateUpdater {
    pub state: GameState,
}

impl GameStateUpdater {
    pub fn new() -> Self {
        Self { state: GameState::default() }
    }
    /// Helper to convert GRE StackEntries to GameState entries.
    pub fn update_stack(&mut self, gre_stack: &BinaryHeap<PriorityEntry>) {
        // just clone each GRE StackEntry
        self.state.stack = gre_stack
            .iter()
            .map(|pe| pe.entry.clone())
            .collect();
    }
    /// Update life totals via OCR.
    pub fn update_life_totals(&mut self, w: u32, h: u32) {
        self.state.life_total          = ocr::read_life_total(false, w, h);
        self.state.opponent_life_total = ocr::read_life_total(true,  w, h);
    }
    /// Update hand from OCR texts.
    pub fn update_hand(&mut self, cards_texts: &[String], library: &HashMap<String, Card>) {
        self.state.hand.clear();
        for txt in cards_texts {
            if let Some(card) = library.get(txt) {
                self.state.hand.push(card.clone());
            } else {
                warn!("OCR nem egyeztetett kártya: {}", txt);
            }
        }
    }
    /// Update battlefield creatures for both sides.
    pub fn update_battlefield_creatures(
        &mut self,
        battlefield_creatures: &mut HashMap<String, Card>,
        battlefield_opponent_creatures: &mut HashMap<String, Card>,
        width: u32,
        height: u32,
    ) {
        // 1) OCR mindkét oldalra
        let ours_ocr = load_side_creatures(width, height, false);
        let mut merged_ours = ours_ocr;
        // 2) Tokenek megtartása
        for (name, card) in battlefield_creatures.iter() {
            if card.type_flags.contains(CardTypeFlags::TOKEN) {
                merged_ours.insert(name.clone(), card.clone());
            }
        }
        *battlefield_creatures = merged_ours;

        let opp_ocr = load_side_creatures(width, height, true);
        let mut merged_opp = opp_ocr;
        for (name, card) in battlefield_opponent_creatures.iter() {
            if card.type_flags.contains(CardTypeFlags::TOKEN) {
                merged_opp.insert(name.clone(), card.clone());
            }
        }
        *battlefield_opponent_creatures = merged_opp;

        // 3) Perzisztens GameState mezők frissítése
        self.state.battlefield = battlefield_creatures
            .values()
            .cloned()
            .collect();
        self.state.opponent_battlefield = battlefield_opponent_creatures
            .values()
            .cloned()
            .collect();
    }
    /// Update mana and land-play flag.
    pub fn update_mana_and_land(&mut self, available_mana: u32, land_played: bool) {
        self.state.mana_available         = available_mana;
        self.state.land_played_this_turn  = land_played;
    }


    /// Refresh all GameState fields.
    pub fn refresh_all(
               &mut self,
                screen_width:  u32,
                screen_height: u32,
                cards_texts:   &[String],
                library:       &HashMap<String, Card>,
                available_mana:u32,
                land_played:   bool,
                gre_stack:     &BinaryHeap<PriorityEntry>,
                battlefield_creatures:        &mut HashMap<String, Card>,
                battlefield_opponent_creatures:&mut HashMap<String, Card>,
            ) {
                // 1) OCR-s életpontok
               self.update_life_totals(screen_width, screen_height);
                // 2) OCR-s kéz
                self.update_hand(cards_texts, library);
               // 3) OCR-s táblarajz + GameState mezők
               self.update_battlefield_creatures(
                    battlefield_creatures,
                    battlefield_opponent_creatures,
                    screen_width,
                    screen_height,
                );
                // 4) mana + land
                self.update_mana_and_land(available_mana, land_played);
                // 5) GRE stack
                self.update_stack(gre_stack);
            }
}