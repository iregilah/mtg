use image::Rgba;
use std::ptr::null_mut;
use std::thread::sleep;
use std::time::Duration;
use std::ffi::OsStr;
use std::os::windows::ffi::OsStrExt;
use winapi::um::wingdi::*;
use winapi::um::winuser::*;
use winapi::um::shellscalingapi::*;
use winapi::shared::windef::*;
use screenshot::get_screenshot;
use chrono::Local;
use image::ImageBuffer;

/// Represents an RGB color.
#[derive(Debug, Copy, Clone, PartialEq)]
pub struct Color {
    pub r: u8,
    pub g: u8,
    pub b: u8,
}


/// Retrieves the color of the pixel at (x, y).
pub fn win32_get_color(x: i32, y: i32) -> Color {
    unsafe {
        let hdc = GetDC(null_mut());
        let pixel = GetPixel(hdc, x, y);
        ReleaseDC(null_mut(), hdc);
        Color {
            r: (pixel & 0x0000FF) as u8,
            g: ((pixel & 0x00FF00) >> 8) as u8,
            b: ((pixel & 0xFF0000) >> 16) as u8,
        }
    }
}

/// Computes the average color over a rectangular region starting at (x, y).
pub fn get_average_color(x: i32, y: i32, width: i32, height: i32) -> (u8, u8, u8) {
    tracing::info!("get_average_color() called with x = {}, y = {}, width = {}, height = {}", x, y, width, height);
    let mut r_total: u32 = 0;
    let mut g_total: u32 = 0;
    let mut b_total: u32 = 0;
    let mut count = 0;
    for i in 0..width {
        for j in 0..height {
            let col = win32_get_color(x + i, y + j);
            // Debug log minden egyes pixelért (ez info vagy debug szintű lehet, ha túl sok)
            tracing::debug!("Pixel at ({}, {}) has color: {:?}", x + i, y + j, col);
            r_total += col.r as u32;
            g_total += col.g as u32;
            b_total += col.b as u32;
            count += 1;
        }
    }
    if count == 0 {
        tracing::error!("get_average_color(): count = 0, returning (0, 0, 0)");
        return (0, 0, 0);
    }
    let avg_r = (r_total / count) as u8;
    let avg_g = (g_total / count) as u8;
    let avg_b = (b_total / count) as u8;
    tracing::info!("get_average_color() returning average color: ({}, {}, {})", avg_r, avg_g, avg_b);
    (avg_r, avg_g, avg_b)
}


/// Checks if two colors are within a tolerance based on channel ratios.
pub fn is_color_within_tolerance(color: (u8, u8, u8), target: (u8, u8, u8), tol: f64) -> bool {
    // Konvertáljuk a bemeneti értékeket f64-esre
    let (r, g, b) = (color.0 as f64, color.1 as f64, color.2 as f64);
    let (tr, tg, tb) = (target.0 as f64, target.1 as f64, target.2 as f64);

    // Számoljuk ki a három arányt, elkerülve a zéróval való osztást
    let ratio_rg = if g != 0.0 { r / g } else { 0.0 };
    let ratio_gb = if b != 0.0 { g / b } else { 0.0 };
    let ratio_rb = if b != 0.0 { r / b } else { 0.0 };

    let target_ratio_rg = if tg != 0.0 { tr / tg } else { 0.0 };
    let target_ratio_gb = if tb != 0.0 { tg / tb } else { 0.0 };
    let target_ratio_rb = if tb != 0.0 { tr / tb } else { 0.0 };

    // Számoljuk ki az arányok közötti abszolút különbségeket
    let diff_rg = (ratio_rg - target_ratio_rg).abs();
    let diff_gb = (ratio_gb - target_ratio_gb).abs();
    let diff_rb = (ratio_rb - target_ratio_rb).abs();

    let result = diff_rg <= tol && diff_gb <= tol && diff_rb <= tol;

    tracing::info!(
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

/// Moves the cursor to (x, y) and waits for the OS to catch up.
pub fn set_cursor_pos(x: i32, y: i32) {
    unsafe { SetCursorPos(x, y); }
    sleep(Duration::from_millis(100));
}

/// Simulates a left mouse click.
pub fn left_click() {
    unsafe {
        mouse_event(MOUSEEVENTF_LEFTDOWN, 0, 0, 0, 0);
        sleep(Duration::from_millis(50));
        mouse_event(MOUSEEVENTF_LEFTUP, 0, 0, 0, 0);
    }
    sleep(Duration::from_millis(100));
}

/// Simulates a key press for the given virtual-key code.
pub fn press_key(vk: u16) {
    unsafe {
        keybd_event(vk as u8, 0, 0, 0);
        sleep(Duration::from_millis(50));
        keybd_event(vk as u8, 0, KEYEVENTF_KEYUP, 0);
    }
    sleep(Duration::from_millis(100));
}

/// Takes a screenshot of the primary monitor and saves it with a timestamp.
pub fn make_screenshot() {
    if let Ok(scn) = get_screenshot(0) {
        let width = scn.width() as u32;
        let height = scn.height() as u32;
        let data = unsafe { std::slice::from_raw_parts(scn.raw_data(), scn.raw_len()).to_vec() };
        if let Some(buffer) = ImageBuffer::<Rgba<u8>, _>::from_raw(width, height, data) {
            let now = Local::now();
            let filename = format!("screenshot_{}.png", now.format("%Y-%m-%d_%H-%M"));
            let _ = buffer.save(&filename);
        }
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

/// Finds a window by its title.
pub fn find_window(title: &str) -> Option<HWND> {
    use std::iter::once;
    let wide: Vec<u16> = OsStr::new(title)
        .encode_wide()
        .chain(once(0))
        .collect();
    unsafe {
        let hwnd = FindWindowW(null_mut(), wide.as_ptr());
        if hwnd.is_null() {
            None
        } else {
            Some(hwnd)
        }
    }
}

/// Returns "red", "blue" or "black" based on the attack button color.
pub fn check_button_color(cords: &Cords) -> &'static str {
    let color = win32_get_color(cords.attack_button.0, cords.attack_button.1);
    if color.r > 200 {
        "red"
    } else if color.b > 200 {
        "blue"
    } else {
        "black"
    }
}