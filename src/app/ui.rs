// app/ui.rs

use crate::multiplatform::send_key;
use crate::multiplatform::click_left;
use crate::multiplatform::move_cursor;
use crate::multiplatform::get_pixel;
use tracing::{debug, info, error};
use std::{thread::sleep, time::Duration};
use screenshots::Screen;
use image::{DynamicImage, ImageBuffer, Rgba};
use chrono::Local;

/// Represents an RGB color.
#[derive(Debug, Copy, Clone, PartialEq)]
pub struct Color {
    pub r: u8,
    pub g: u8,
    pub b: u8,
}

/// Retrieves the color of the pixel at (x, y).
pub fn get_color(x: i32, y: i32) -> Color {
    match get_pixel(x, y) {
        Ok((r, g, b)) => {
            debug!("[get_color] Pixel @({}, {}) = ({},{},{})", x, y, r, g, b);
            Color { r, g, b }
        }
        Err(e) => {
            error!("[get_color] Error reading pixel @({}, {}): {}", x, y, e);
            Color { r: 0, g: 0, b: 0 }
        }
    }
}

/// Computes the average color over a rectangular region starting at (x, y).
pub fn get_average_color(x: i32, y: i32, width: i32, height: i32) -> (u8, u8, u8) {
    info!("get_average_color() called with x = {}, y = {}, width = {}, height = {}", x, y, width, height);
    let mut r_total: u32 = 0;
    let mut g_total: u32 = 0;
    let mut b_total: u32 = 0;
    let mut count = 0;
    for dx in 0..width {
        for dy in 0..height {
            let col = get_color(x + dx, y + dy);
            debug!("Pixel at ({}, {}) has color: {:?}", x + dx, y + dy, col);
            r_total += col.r as u32;
            g_total += col.g as u32;
            b_total += col.b as u32;
            count += 1;
        }
    }
    if count == 0 {
        error!("get_average_color(): count = 0, returning (0, 0, 0)");
        return (0, 0, 0);
    }
    let avg_r = (r_total / count) as u8;
    let avg_g = (g_total / count) as u8;
    let avg_b = (b_total / count) as u8;
    info!("get_average_color() returning average color: ({}, {}, {})", avg_r, avg_g, avg_b);
    (avg_r, avg_g, avg_b)
}


/// Checks if two colors are within a tolerance based on channel ratios.
pub fn is_color_within_tolerance(color: (u8, u8, u8), target: (u8, u8, u8), tol: f64) -> bool {
    // Convert channels to f64
    let (r, g, b) = (color.0 as f64, color.1 as f64, color.2 as f64);
    let (tr, tg, tb) = (target.0 as f64, target.1 as f64, target.2 as f64);

    // Compute ratios safely
    let ratio_rg = if g != 0.0 { r / g } else { 0.0 };
    let ratio_gb = if b != 0.0 { g / b } else { 0.0 };
    let ratio_rb = if b != 0.0 { r / b } else { 0.0 };

    let target_ratio_rg = if tg != 0.0 { tr / tg } else { 0.0 };
    let target_ratio_gb = if tb != 0.0 { tg / tb } else { 0.0 };
    let target_ratio_rb = if tb != 0.0 { tr / tb } else { 0.0 };

    // Calculate the absolute differences between the ratios
    let diff_rg = (ratio_rg - target_ratio_rg).abs();
    let diff_gb = (ratio_gb - target_ratio_gb).abs();
    let diff_rb = (ratio_rb - target_ratio_rb).abs();

    let result = diff_rg <= tol && diff_gb <= tol && diff_rb <= tol;

    info!(
        "is_color_within_tolerance() called with color = {:?}, target = {:?}, tol = {}. \
         Computed ratios: (R/G: {:.3}, G/B: {:.3}, R/B: {:.3}), Target ratios: (R/G: {:.3}, G/B: {:.3}, R/B: {:.3}), \
         Differences: (R/G: {:.3}, G/B: {:.3}, R/B: {:.3}), result: {}",
        color, target, tol,
        ratio_rg, ratio_gb, ratio_rb,
        target_ratio_rg, target_ratio_gb, target_ratio_rb,
        diff_rg, diff_gb, diff_rb,
        result
    );
    result
}

/// Moves the cursor to (x, y).
pub fn set_cursor_pos(x: i32, y: i32) {
    if let Err(e) = move_cursor(x, y) {
        error!("[set_cursor_pos] Error: {}", e);
    }
    sleep(Duration::from_millis(100));
}

/// Simulates a left mouse click.
pub fn left_click() {
    if let Err(e) = click_left() {
        error!("left_click error: {}", e);
    }
    sleep(Duration::from_millis(100));
}

/// Simulates a key press + release.
pub fn press_key(keycode: u32) {
    if let Err(e) = send_key(enigo::Key::Other(keycode)) {
        error!("press_key error: {}", e);
    }
    sleep(Duration::from_millis(100));
}

/// Takes a screenshot of the primary monitor and saves it with a timestamp.
pub fn make_screenshot() {
    match Screen::all() {
        Ok(screens) => {
            if let Some(screen) = screens.first() {
                match screen.capture() {
                    Ok(buffer) => {
                        let now = Local::now();
                        let filename = format!("screenshot_{}.png", now.format("%Y-%m-%d_%H-%M"));
                        let (w, h) = (buffer.width(), buffer.height());
                        match ImageBuffer::<Rgba<u8>, _>::from_raw(w, h, buffer.into_raw()) {
                            Some(img_buf) => {
                                let img = DynamicImage::ImageRgba8(img_buf);
                                if let Err(e) = img.save(&filename) {
                                    error!("[make_screenshot] Save failed: {}", e);
                                } else {
                                    info!("[make_screenshot] Saved {}", filename);
                                }
                            }
                            None => {
                                error!("[make_screenshot] Buffer size mismatch {}×{}", w, h);
                            }
                        }
                    }
                    Err(e) => error!("[make_screenshot] Capture failed: {}", e),
                }
            } else {
                error!("[make_screenshot] No screens available");
            }
        }
        Err(e) => error!("[make_screenshot] Screen enumeration failed: {}", e),
    }
}

/// Holds coordinates for UI elements like buttons.
#[derive(Debug, Copy, Clone)]
pub struct Cords {
    pub home_button: (i32, i32),
    pub play_button: (i32, i32),
    pub attack_button: (i32, i32),
}

impl Cords {
    /// Initialize UI coordinates based on screen size.
    pub fn new(screen_width: i32, screen_height: i32) -> Self {
        Self {
            home_button: (
                (screen_width as f64 * 0.052188).ceil() as i32,
                (screen_height as f64 * 0.03426).ceil() as i32,
            ),
            play_button: (
                (screen_width as f64 * 0.895).ceil() as i32,
                (screen_height as f64 * 0.938).ceil() as i32,
            ),
            attack_button: (
                (screen_width as f64 * 0.8815).ceil() as i32,
                (screen_height as f64 * 0.86666).ceil() as i32,
            ),
        }
    }
}


/// Returns "red", "blue" or "black" based on the attack button color.
pub fn check_button_color(cords: &Cords) -> &'static str {
    let (x, y) = cords.attack_button;
    let c = get_color(x, y);
    info!("[check_button_color] Pixel @({}, {}) = {:?}", x, y, c);
    if c.r > 200 {
        "red"
    } else if c.b > 200 {
        "blue"
    } else {
        "black"
    }
}

