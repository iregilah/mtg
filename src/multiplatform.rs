//! Cross-platform input and screen utilities using message-driven `enigo` and `screenshots`.
//! Input is dispatched to a dedicated thread to avoid Send/Sync issues on macOS.

use std::sync::{Mutex, mpsc::{Sender, Receiver, channel}};
use std::thread;
use std::time::Duration;
use once_cell::sync::Lazy;
use tracing::{debug, info, error};

use enigo::{Enigo, Settings, Button, Direction, Key, InputError, Coordinate, Mouse, Keyboard};
use screenshots::Screen;

/// Commands for the input thread.
enum InputCommand {
    MoveMouse { x: i32, y: i32 },
    ClickLeft,
    ClickRight,
    SendKey(Key),
}

/// Sender for input commands.
static INPUT_SENDER: Lazy<Sender<InputCommand>> = Lazy::new(|| {
    let (tx, rx): (Sender<InputCommand>, Receiver<InputCommand>) = channel();
    thread::spawn(move || {
        // Initialize Enigo on this dedicated thread
        let mut enigo = Enigo::new(&Settings::default())
            .expect("Failed to initialize Enigo input context");
        while let Ok(cmd) = rx.recv() {
            match cmd {
                InputCommand::MoveMouse { x, y } => {
                    debug!("[input-thread] MoveMouse to ({}, {})", x, y);
                    if let Err(e) = enigo.move_mouse(x, y, Coordinate::Abs) {
                        error!("[input-thread] MoveMouse failed: {}", e);
                    }
                }
                InputCommand::ClickLeft => {
                    info!("[input-thread] ClickLeft");
                    if let Err(e) = enigo.button(Button::Left, Direction::Press) {
                        error!("[input-thread] ClickLeft press failed: {}", e);
                    }
                    thread::sleep(Duration::from_millis(50));
                    if let Err(e) = enigo.button(Button::Left, Direction::Release) {
                        error!("[input-thread] ClickLeft release failed: {}", e);
                    }
                }
                InputCommand::ClickRight => {
                    info!("[input-thread] ClickRight");
                    if let Err(e) = enigo.button(Button::Right, Direction::Press) {
                        error!("[input-thread] ClickRight press failed: {}", e);
                    }
                    thread::sleep(Duration::from_millis(50));
                    if let Err(e) = enigo.button(Button::Right, Direction::Release) {
                        error!("[input-thread] ClickRight release failed: {}", e);
                    }
                }
                InputCommand::SendKey(key) => {
                    info!("[input-thread] SendKey {:?}", key);
                    if let Err(e) = enigo.key(key, Direction::Press) {
                        error!("[input-thread] Key press failed: {}", e);
                    }
                    thread::sleep(Duration::from_millis(50));
                    if let Err(e) = enigo.key(key, Direction::Release) {
                        error!("[input-thread] Key release failed: {}", e);
                    }
                }
            }
        }
    });
    tx
});

/// Tracks the last cursor position so clicks know where they occur.
static CURSOR_POS: Lazy<Mutex<(i32, i32)>> = Lazy::new(|| Mutex::new((0, 0)));

/// Initialize the input backend (starts the input thread).
pub fn init() -> Result<(), String> {
    Lazy::force(&INPUT_SENDER);
    info!("Multi-platform input thread initialized");
    Ok(())
}

/// Returns the (width, height) of the primary monitor in pixels.
pub fn screen_size() -> Result<(i32, i32), String> {
    let screens = Screen::all()
        .map_err(|e| format!("[screen_size] Screenshot enumeration failed: {}", e))?;
    let screen = screens
        .first()
        .ok_or_else(|| "[screen_size] No screens detected".to_string())?;
    let di = &screen.display_info;
    info!("Primary screen size: {}x{}", di.width, di.height);
    Ok((di.width as i32, di.height as i32))
}

/// Read the RGB color of the pixel at (x, y).
pub fn get_pixel(x: i32, y: i32) -> Result<(u8, u8, u8), String> {
    let screens = Screen::all().map_err(|e| format!("[get_pixel] {}", e))?;
    let screen = screens.first().ok_or_else(|| "[get_pixel] No screens".to_string())?;
    let cap = screen.capture().map_err(|e| format!("[get_pixel] {}", e))?;
    let (w, h) = (cap.width() as usize, cap.height() as usize);
    if x < 0 || y < 0 || (x as usize) >= w || (y as usize) >= h {
        return Err(format!("Coordinates ({},{}) outside {}×{}", x, y, w, h));
    }
    let buf = cap.into_raw();
    let idx = ((y as usize) * w + (x as usize)) * 4;
    let (r, g, b) = (buf[idx], buf[idx + 1], buf[idx + 2]);
    debug!("[get_pixel] Pixel @({}, {}) = ({},{},{})", x, y, r, g, b);
    Ok((r, g, b))
}

/// Moves the mouse cursor to absolute (x, y) coordinates.
pub fn move_cursor(x: i32, y: i32) -> Result<(), String> {
    {
        let mut pos = CURSOR_POS
            .lock()
            .map_err(|e| format!("[move_cursor] CURSOR_POS lock error: {}", e))?;
        *pos = (x, y);
    }
    INPUT_SENDER
        .send(InputCommand::MoveMouse { x, y })
        .map_err(|e| format!("[move_cursor] send error: {}", e))?;
    Ok(())
}

/// Simulates a left mouse click at the last moved-to position.
pub fn click_left() -> Result<(), String> {
    INPUT_SENDER
        .send(InputCommand::ClickLeft)
        .map_err(|e| format!("[click_left] send error: {}", e))?;
    Ok(())
}

/// Simulates a right mouse click at the last moved-to position.
pub fn click_right() -> Result<(), String> {
    INPUT_SENDER
        .send(InputCommand::ClickRight)
        .map_err(|e| format!("[click_right] send error: {}", e))?;
    Ok(())
}

/// Sends a single key (press + release).
pub fn send_key(key: Key) -> Result<(), String> {
    INPUT_SENDER
        .send(InputCommand::SendKey(key))
        .map_err(|e| format!("[send_key] send error: {}", e))?;
    Ok(())
}
