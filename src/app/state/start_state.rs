use crate::app::bot::Bot;
use crate::app::state::State;
use std::thread::sleep;
use std::time::Duration;
use chrono::Local;
use crate::app::state::mulligan_state::MulliganState;
use crate::app::ui::{find_window, left_click, press_key, set_cursor_pos};

pub struct StartState {}

impl StartState {
    pub fn new() -> Self {
        Self {}
    }
}

impl State for StartState {
    fn update(&mut self, bot: &mut Bot) {
        tracing::info!("StartState: initiating game start.");
        tracing::info!("{} Starting", Local::now().format("%Y-%m-%d %H:%M"));
        sleep(Duration::from_secs(5));
        set_cursor_pos(bot.cords.home_button.0, bot.cords.home_button.1);
        left_click();
        sleep(Duration::from_secs(1));
        if let Some(hwnd) = find_window("MTGA_me") {
            unsafe {
                winapi::um::winuser::SetForegroundWindow(hwnd);
                winapi::um::winuser::SetActiveWindow(hwnd);
            }
        }
        set_cursor_pos(bot.cords.play_button.0, bot.cords.play_button.1);
        sleep(Duration::from_millis(500));
        left_click();
        press_key(winapi::um::winuser::VK_SPACE as u16);
        sleep(Duration::from_millis(500));
        left_click();
        press_key(winapi::um::winuser::VK_SPACE as u16);
        sleep(Duration::from_millis(500));
        left_click();
        sleep(Duration::from_millis(500));
        left_click();
        tracing::info!("StartState: Start phase completed.");
    }

    fn next(&mut self) -> Box<dyn State> {
        tracing::info!("StartState: transitioning to MulliganState.");
        // Átmegyünk a mulligan fázisra
        Box::new(MulliganState::new())
    }
}
