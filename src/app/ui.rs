// app/ui.rs

use tracing::{debug, error, info};

use std::{
    thread::sleep,
    time::Duration,
};

use screenshots::Screen;
use image::DynamicImage;

use image::{ImageBuffer, Rgba};

use chrono::Local;
use crate::multi_platform::{click_left, get_pixel, move_cursor, send_key, windows_platform};
#[cfg(target_os = "linux")]
use crate::multi_platform::x11_platform;
#[cfg(all(target_os = "linux", not(feature = "x11")))]
use crate::multi_platform::wayland_platform;

#[cfg(target_os = "windows")]
use {
    std::ffi::OsStr,
    std::os::windows::ffi::OsStrExt,
    windows::core::PCWSTR,
    windows::Win32::Foundation::HWND,
    windows::Win32::UI::WindowsAndMessaging::FindWindowW,
};

#[cfg(not(target_os = "windows"))]
use {
    x11::xlib,
    std::{ffi::CStr, ptr, slice},
};

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
        Ok((r, g, b)) => Color { r, g, b },
        Err(e) => {
            error!("get_color error: {}", e);
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
    for i in 0..width {
        for j in 0..height {
            let col = get_color(x + i, y + j);
            // Debug log minden egyes pixelért (ez info vagy debug szintű lehet, ha túl sok)
            debug!("Pixel at ({}, {}) has color: {:?}", x + i, y + j, col);
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
        error!("set_cursor_pos error: {}", e);
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
    if let Err(e) = send_key(keycode) {
        error!("press_key error: {}", e);
    }
    sleep(Duration::from_millis(100));
}

/// Takes a screenshot of the primary monitor and saves it with a timestamp.
pub fn make_screenshot() {
    if let Ok(screens) = Screen::all() {
        if let Some(screen) = screens.first() {
            if let Ok(buffer) = screen.capture() {
                let now = Local::now();
                let filename = format!("screenshot_{}.png", now.format("%Y-%m-%d_%H-%M"));
                let (w, h) = (buffer.width(), buffer.height());
                if let Some(img_buf) = ImageBuffer::<Rgba<u8>, _>::from_raw(w, h, buffer.into_raw()) {
                    let img = DynamicImage::ImageRgba8(img_buf);
                    let _ = img.save(&filename);
                } else {
                    error!("make_screenshot: buffer size mismatch");
                }
            }
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
    let color = get_color(cords.attack_button.0, cords.attack_button.1);
    if color.r > 200 {
        "red"
    } else if color.b > 200 {
        "blue"
    } else {
        "black"
    }
}

/// Finds a window by its title.
///
/// On Windows uses the `windows` crate; on Linux/X11 (and XWayland)
/// it enumerates X11 children and matches by window name.
#[cfg(target_os = "windows")]
pub fn find_window(title: &str) -> Option<HWND> {
    // Build a null-terminated UTF-16 string
    let wide: Vec<u16> = OsStr::new(title)
        .encode_wide()
        .chain(Some(0))
        .collect();
    let pw = PCWSTR(wide.as_ptr());

    // Most már Result<HWND, Error>
    let res = unsafe { FindWindowW(None, pw) };
    match res {
               Ok(hwnd) if !hwnd.0.is_null() => Some(hwnd),  // megtaláltuk, és nem null pointer
        Ok(_) | Err(_)          => None,         // hiba vagy null handle
    }
}
#[cfg(not(target_os = "windows"))]
pub fn find_window(title: &str) -> Option<()> {
    unsafe {
        // Connect to the X server
        let display = xlib::XOpenDisplay(ptr::null());
        if display.is_null() {
            return None;
        }
        let root = xlib::XDefaultRootWindow(display);

        // Query the window tree
        let mut root_ret = 0;
        let mut parent_ret = 0;
        let mut children_ptr: *mut xlib::Window = ptr::null_mut();
        let mut nchildren: u32 = 0;
        if xlib::XQueryTree(
            display,
            root,
            &mut root_ret,
            &mut parent_ret,
            &mut children_ptr,
            &mut nchildren,
        ) == 0 {
            xlib::XCloseDisplay(display);
            return None;
        }

        let children = slice::from_raw_parts(children_ptr, nchildren as usize);
        for &w in children {
            let mut name_ptr: *mut i8 = ptr::null_mut();
            if xlib::XFetchName(display, w, &mut name_ptr) != 0 && !name_ptr.is_null() {
                let name = CStr::from_ptr(name_ptr).to_string_lossy();
                xlib::XFree(name_ptr as *mut _);
                if name.contains(title) {
                    xlib::XCloseDisplay(display);
                    return Some(());
                }
            }
        }

        xlib::XCloseDisplay(display);
        None
    }
}

