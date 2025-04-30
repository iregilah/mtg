// src/multiplatform.rs

//! Cross-platform input and screen utilities using `enigo` and `screenshots`.
//! Includes extensive tracing for debugging and edge-case handling.

use std::sync::Mutex;
use tracing::{debug, info, error};

use enigo::{Enigo, Settings, Mouse, Keyboard, Button, Direction, Key, InputError, Coordinate};
use std::thread::sleep;
use std::time::Duration;
use once_cell::sync::Lazy;
use screenshots::Screen;

/// Global, lazily-initialized Enigo instance guarded by a Mutex.
/// Wrapped in a Mutex so that future multi-threaded use is safe and to satisfy Rust's
/// requirement that statics be Sync. If only single-threaded, the lock cost is minimal.
static ENIGO: Lazy<Mutex<Enigo>> = Lazy::new(|| {
    // Create Enigo with default settings
    let enigo = Enigo::new(&Settings::default())
        .expect("Failed to initialize Enigo input context");
    Mutex::new(enigo)
});

/// Tracks the last cursor position so our clicks can say “Clicking at (x,y)”.
static CURSOR_POS: Lazy<Mutex<(i32, i32)>> = Lazy::new(|| Mutex::new((0, 0)));


/// Initialize the input backend.
/// Currently, this simply forces the creation of the global Enigo instance.
/// Can return Err if initialization panics.
pub fn init() -> Result<(), String> {
    Lazy::force(&ENIGO);
    info!("Multi-platform input initialized");
    Ok(())
}

/// Returns the (width, height) of the primary monitor in pixels.
/// Errors if no screens found or if capture fails.
pub fn screen_size() -> Result<(i32, i32), String> {
    let screens = Screen::all()
        .map_err(|e| format!("[screen_size] Screenshot enumeration failed: {}", e))?;
    let screen = screens
        .first()
        .ok_or_else(|| "[screen_size] No screens detected".to_string())?;
    let di = &screen.display_info;
    info!("Primary screen size detected: {}x{}", di.width, di.height);
    Ok((di.width as i32, di.height as i32))
}

/// Read the RGB color of the pixel at (x, y).
/// Includes bounds checks and detailed error messages.
pub fn get_pixel(x: i32, y: i32) -> Result<(u8, u8, u8), String> {
    let screens = Screen::all().map_err(|e| format!("[get_pixel] Screenshot enumeration failed: {}", e))?;
    let screen = screens
        .first()
        .ok_or_else(|| "[get_pixel] No screens detected".to_string())?;
    let cap = screen.capture().map_err(|e| format!("[get_pixel] Capture failed: {}", e))?;
    let w = cap.width() as usize;
    let h = cap.height() as usize;
    if x < 0 || y < 0 || (x as usize) >= w || (y as usize) >= h {
        error!("[get_pixel] Coordinates out of bounds: ({}, {}), screen {}x{}", x, y, w, h);
        return Err(format!("Coordinates ({},{}) outside screen bounds {}×{}", x, y, w, h));
    }
    let buf = cap.into_raw();
    let idx = ((y as usize) * w + (x as usize)) * 4;
    let r = buf[idx];
    let g = buf[idx + 1];
    let b = buf[idx + 2];
    debug!("[get_pixel] Pixel @({}, {}) = ({},{},{})", x, y, r, g, b);
    Ok((r, g, b))
}

/// Moves the mouse cursor to absolute (x, y) coordinates.
/// Updates our tracked position so that subsequent clicks know where they occur.
pub fn move_cursor(x: i32, y: i32) -> Result<(), String> {
    {
        let mut pos = CURSOR_POS
            .lock()
            .map_err(|e| format!("[move_cursor] CURSOR_POS lock error: {}", e))?;
        *pos = (x, y);
    }
    let mut enigo = ENIGO
        .lock()
        .map_err(|e| format!("[move_cursor] Enigo lock error: {}", e))?;
    info!("[move_cursor] Moving cursor to ({}, {})", x, y);
    enigo
        .move_mouse(x, y, Coordinate::Abs)
        .map_err(|e| format!("[move_cursor] Failed: {}", e))?;
    Ok(())
}

/// Internal helper to click a given mouse button at the tracked position.
fn click_button(button: Button, action: &str) -> Result<(), String> {
    let (x, y) = *CURSOR_POS
        .lock()
        .map_err(|e| format!("[{}] CURSOR_POS lock error: {}", action, e))?;
    let mut enigo = ENIGO
        .lock()
        .map_err(|e| format!("[{}] Enigo lock error: {}", action, e))?;
    info!("[{}] Clicking {:?} at ({}, {})", action, button, x, y);
    enigo
        .button(button, Direction::Press)
        .map_err(|e| format!("[{}] Press failed: {}", action, e))?;
    // delay between press and release
    sleep(Duration::from_millis(50));
    // release
    enigo
        .button(button, Direction::Release)
        .map_err(|e| format!("[{}] Release failed: {}", action, e))?;
    Ok(())
}

/// Simulates a left mouse click (press + release) at the last moved-to position.
pub fn click_left() -> Result<(), String> {
    click_button(Button::Left, "click_left")
}

/// Simulates a right mouse click (press + release) at the last moved-to position.
pub fn click_right() -> Result<(), String> {
    click_button(Button::Right, "click_right")
}

/// Sends a single key by raw keycode (press + release).
pub fn send_key(key: Key) -> Result<(), String> {
    // Press
    let mut enigo = ENIGO
        .lock()
        .map_err(|e| format!("[send_key] Enigo lock error: {}", e))?;
    info!("[send_key] Pressing   key {:?}", key);
    enigo
        .key(key, Direction::Press)
        .map_err(|e| format!("[send_key] Press failed: {}", e))?;
    // delay between press and release
    sleep(Duration::from_millis(50));
    // release
    info!("[send_key] Releasing key {:?}", key);
    enigo
        .key(key, Direction::Release)
        .map_err(|e| format!("[send_key] Release failed: {}", e))?;
    Ok(())
}