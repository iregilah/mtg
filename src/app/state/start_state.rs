use crate::app::error::AppError;
use crate::app::game_state::GamePhase;
use std::{thread::sleep, time::Duration};
use tracing::info;
use chrono::Local;
use windows::Win32::UI::Input::KeyboardAndMouse::SetActiveWindow;

use crate::app::{
    bot::Bot,
    state::{State, mulligan_state::MulliganState},
    ui::{find_window, left_click, press_key, set_cursor_pos},
};
#[cfg(target_os = "windows")]
use windows::Win32::UI::WindowsAndMessaging::SetForegroundWindow;

#[cfg(target_os = "windows")]
use windows::Win32::UI::Input::KeyboardAndMouse::VK_SPACE;

// On non-Windows we still define VK_SPACE so our `press_key(VK_SPACE)` calls compile.
// It will no-op via the platform abstraction.
#[cfg(not(target_os = "windows"))]
const VK_SPACE: u32 = 0x20;
pub struct StartState {}

impl StartState {
    pub fn new() -> Self { Self {} }
}

impl State<AppError> for StartState {
    fn update(&mut self, bot: &mut Bot) -> Result<(), AppError> {
        info!("StartState: initiating game start.");
        info!("{} Starting", Local::now().format("%Y-%m-%d %H:%M"));
        sleep(Duration::from_secs(5));

        // Click the “Home” button in the launcher UI
        set_cursor_pos(bot.cords.home_button.0, bot.cords.home_button.1);
        left_click();
        sleep(Duration::from_secs(1));

        // Try to find and focus the main MTGA window
        if let Some(hwnd) = find_window("MTGA_me") {
            #[cfg(target_os = "windows")]
            unsafe {
                let _ = SetForegroundWindow(hwnd);
                let _ = SetActiveWindow(hwnd);
            }
            // On Linux/X11/Wayland we rely on the launcher click having raised it already
        }

        // Click “Play” and advance through startup prompts
        set_cursor_pos(bot.cords.play_button.0, bot.cords.play_button.1);
        sleep(Duration::from_millis(500));
        left_click();

        // Press Space to confirm dialogs twice
        sleep(Duration::from_millis(500));
        #[cfg(target_os = "windows")]
                let space_code = VK_SPACE.0 as u32;
                #[cfg(not(target_os = "windows"))]
                let space_code = VK_SPACE;
                press_key(space_code);
        sleep(Duration::from_millis(500));
        left_click();
        #[cfg(target_os = "windows")]
                let space_code = VK_SPACE.0 as u32;
                #[cfg(not(target_os = "windows"))]
                let space_code = VK_SPACE;
                press_key(space_code);

        // Final clicks to get into the game
        sleep(Duration::from_millis(500));
        left_click();
        sleep(Duration::from_millis(500));
        left_click();

        info!("StartState: Start phase completed.");
        Ok(())
    }

    fn next(&mut self) -> Box<dyn State<AppError>> {
        info!("StartState: transitioning to MulliganState.");
        Box::new(MulliganState::new())
    }

    fn phase(&self) -> GamePhase {
        GamePhase::Beginning
    }
}