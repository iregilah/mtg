use crate::app::bot::Bot;
use crate::app::state::State;
use std::thread::sleep;
use std::time::Duration;
use crate::app::ocr::check_main_region_text;
use crate::app::state::first_main_phase_state::FirstMainPhaseState;
use crate::app::ui::check_button_color;

pub struct OpponentsTurnState {}

impl OpponentsTurnState {
    pub fn new() -> Self {
        Self {}
    }
}

impl State for OpponentsTurnState {
    fn update(&mut self, bot: &mut Bot) {
        bot.land_played_this_turn = false; // máshová kéne majd tenni később
        tracing::info!("OpponentsTurnState: handling opponent's turn.");
        Self::process_opponents_turn(self, bot);
        sleep(Duration::from_secs(1));
    }

    fn next(&mut self) -> Box<dyn State> {
        tracing::info!("OpponentsTurnState: transitioning to FirstMainPhaseState.");
        // Ha vége az ellenfél körének, visszalépünk az első main phase-be
        Box::new(FirstMainPhaseState::new())
    }
}

impl OpponentsTurnState {
    fn process_opponents_turn(&self, bot: &mut Bot) {
        let mut is_red = check_button_color(&bot.cords) == "red";
        loop {
            is_red = check_button_color(&bot.cords) == "red";
            let main_text = check_main_region_text(bot.screen_width as u32, bot.screen_height as u32, is_red);
            tracing::info!("(Opponent turn) Main region text: {}", main_text);
            if main_text.contains("Next") {
                tracing::info!("Opponent turn phase finished.");
                break;
            }
            sleep(Duration::from_secs(2));
        }
    }
}