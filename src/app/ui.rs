use image::Rgba;
use std::ptr::null_mut;
use std::thread::sleep;
use std::time::Duration;
use std::ffi::OsStr;
use std::os::windows::ffi::OsStrExt;
use winapi::um::wingdi::*;
use winapi::um::winuser::*;
use winapi::um::shellscalingapi::*;
use winapi::shared::minwindef::*;
use winapi::shared::windef::*;

#[derive(Debug, Copy, Clone, PartialEq)]
pub struct Color {
    pub r: u8,
    pub g: u8,
    pub b: u8,
}

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

pub fn set_cursor_pos(x: i32, y: i32) {
    unsafe { SetCursorPos(x, y); }
    sleep(Duration::from_millis(100));
}

pub fn left_click() {
    unsafe {
        mouse_event(MOUSEEVENTF_LEFTDOWN, 0, 0, 0, 0);
        sleep(Duration::from_millis(100));
        mouse_event(MOUSEEVENTF_LEFTUP, 0, 0, 0, 0);
    }
    sleep(Duration::from_millis(100));
}

pub fn press_key(vk: u16) {
    unsafe {
        keybd_event(vk as u8, 0, 0, 0);
        sleep(Duration::from_millis(50));
        keybd_event(vk as u8, 0, KEYEVENTF_KEYUP, 0);
    }
    sleep(Duration::from_millis(100));
}

/// Képernyőkép készítése (opcionális)
pub fn make_screenshot() {
    use chrono::Local;
    use image::ImageBuffer;
    use screenshot::get_screenshot;
    if let Ok(scn) = get_screenshot(0) {
        let width = scn.width() as u32;
        let height = scn.height() as u32;
        let data_vec = unsafe {
            std::slice::from_raw_parts(scn.raw_data(), scn.raw_len()).to_vec()
        };
        if let Some(buffer) = ImageBuffer::<Rgba<u8>, _>::from_raw(width, height, data_vec) {
            let now = Local::now();
            let filename = format!("screenshot_{}.png", now.format("%Y-%m-%d_%H-%M"));
            let _ = buffer.save(&filename);
        }
    }
}

/// Gombok koordinátái
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

/// Például az attack_button színének lekérdezése.
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
