// app/state/attack_phase_state.rs

use std::{thread::sleep, time::Duration};
use tracing::{info};

use regex::Regex;

use crate::app::{
    bot::Bot,
    ocr::check_main_region_text,
    state::{State, second_main_phase_state::SecondMainPhaseState},
    ui::press_key,
};

pub struct AttackPhaseState {
    no_attack: bool,
}

impl AttackPhaseState {
    pub fn new() -> Self {
        Self { no_attack: false }
    }
}

impl State for AttackPhaseState {
    fn update(&mut self, bot: &mut Bot) {
        info!("AttackPhaseState: starting attack phase.");
        if !Self::can_attack(bot) {
            info!("No creature can attack (all have summoning sickness or none exist). Transitioning to OpponentsTurnState.");
            self.no_attack = true;
            return;
        }
        self.process_attack_phase(bot);
    }

    fn next(&mut self) -> Box<dyn State> {
        info!("AttackPhaseState: transitioning to SecondMainPhaseState.");
        Box::new(SecondMainPhaseState::new())
    }
}

impl AttackPhaseState {
    fn is_attackers_text(s: &str) -> bool {
        // A regex, ami egyezik a következővel:
        // - opcionális szóközök elején,
        // - majd egy vagy több számjegy (\d+),
        // - utána legalább egy szóköz (\s+),
        // - majd az "Attacker" szó, ahol az "s" opcionális ("Attackers?" az "s" kérdőjellel opcionális),
        // - végül opcionális szóközök, és a szöveg vége.
        let re = Regex::new(r"^\s*\d+\s+Attackers?\s*$").unwrap();
        let result = re.is_match(s);
        info!("is_attackers_text(): input = {:?}, matches regex: {}", s, result);
        result
    }

    fn can_attack(bot: &Bot) -> bool {
        bot.battlefield_creatures.iter().any(|(_name, card)| {
            if let crate::app::card_library::CardType::Creature(creature) = &card.card_type {
                !creature.summoning_sickness
            } else {
                false
            }
        })
    }
    pub fn process_attack_phase(&self, bot: &mut Bot) {
        // 1. Ciklus: addig várunk, amíg a main region text "All Attack"-et ad,
        //    itt mindig a red button (white_invert_image) feldolgozását használjuk.
        loop {
            let main_text = check_main_region_text(bot.screen_width as u32, bot.screen_height as u32, true);
            info!("(Attack phase) Main region text: {}", main_text);
            if main_text.contains("All Attack") {
                press_key(winapi::um::winuser::VK_SPACE as u16);
                sleep(Duration::from_secs(1));
                break;
            } else if main_text.contains("Next") {
                press_key(winapi::um::winuser::VK_SPACE as u16);
                sleep(Duration::from_secs(1));
            } else {
                sleep(Duration::from_secs(2));
            }
        }

        // 2. Ciklus: várjuk, hogy a main region text "X Attackers" formátumú legyen.
        loop {
            let main_text = check_main_region_text(bot.screen_width as u32, bot.screen_height as u32, true);
            info!("(Attack phase) Main region text after All Attack: {}", main_text);
            if Self::is_attackers_text(&main_text) {
                press_key(winapi::um::winuser::VK_SPACE as u16);
                sleep(Duration::from_secs(1));
                break;
            }
            sleep(Duration::from_secs(2));
        }

        // 3. Ciklus: amíg "Next" szerepel, kattintsuk a main region text-et
        loop {
            let main_text = check_main_region_text(bot.screen_width as u32, bot.screen_height as u32, true);
            info!("(Attack phase) Main region text in Next loop: {}", main_text);
            if main_text.contains("Next") {
                press_key(winapi::um::winuser::VK_SPACE as u16);
                sleep(Duration::from_secs(1));
            } else {
                break;
            }
        }
    }
}
