// windows_platform.rs
#![cfg(target_os = "windows")]

use windows::Win32::Foundation::HWND;
use windows::Win32::Graphics::Gdi::{GetPixel, GetDC, ReleaseDC};
// these two live in WindowsAndMessaging:
use windows::Win32::UI::WindowsAndMessaging::SetCursorPos;

// the raw INPUT structs & flags come from the KeyboardAndMouse module:
use windows::Win32::UI::Input::KeyboardAndMouse::{
    SendInput, INPUT, MOUSEINPUT, KEYBDINPUT,
    MOUSEEVENTF_LEFTDOWN, MOUSEEVENTF_LEFTUP,
    MOUSEEVENTF_RIGHTDOWN, MOUSEEVENTF_RIGHTUP,
    KEYEVENTF_KEYUP, INPUT_MOUSE, INPUT_KEYBOARD,
    VIRTUAL_KEY, KEYBD_EVENT_FLAGS,
};
use std::{mem::size_of, thread::sleep, time::Duration};


pub fn get_pixel_color(x: i32, y: i32) -> Result<(u8, u8, u8), String> {
    unsafe {
        // Képernyő device context lekérése (NULL handle = teljes desktop)
        let hdc = GetDC(None);
        if hdc.0.is_null() {
            return Err("GetDC failed".to_string());
        }
        // Pixel szín lekérése
        let color = GetPixel(hdc, x, y);
            // Release the DC
            ReleaseDC(None, hdc);
            // COLORREF.0 a belső u32
            if color.0 == u32::MAX {
            // GetPixel hibára u32::MAX (-1) tér vissza
            return Err("GetPixel failed".to_string());
        }
    // A COLORREF.0 formátum: 0x00BBGGRR
            let raw = color.0;
            let r = (raw & 0xFF) as u8;
            let g = ((raw >> 8) & 0xFF) as u8;
            let b = ((raw >> 16) & 0xFF) as u8;
        Ok((r, g, b))
    }
}

pub fn move_mouse(x: i32, y: i32) -> Result<(), String> {
    // Az SetCursorPos az egész képernyő koordináta-rendszerében mozgatja az egeret,
        // és hibát Result-ként ad vissza.
        unsafe {
            SetCursorPos(x, y)
                .map_err(|e| format!("SetCursorPos failed: {}", e))?;
        }
        Ok(())
}

/// Simulál egy egérkattintást: lenyomás → rövid késleltetés → felengedés
pub fn mouse_click(left_button: bool) -> Result<(), String> {
    unsafe {
        // --- Lenézés ---
        let mut down: INPUT = std::mem::zeroed();
        down.r#type = INPUT_MOUSE;
        down.Anonymous.mi = MOUSEINPUT {
            dx: 0,
            dy: 0,
            mouseData: 0,
            dwFlags: if left_button { MOUSEEVENTF_LEFTDOWN } else { MOUSEEVENTF_RIGHTDOWN },
            time: 0,
            dwExtraInfo: 0,
        };
        let sent_down = SendInput(std::slice::from_ref(&down), size_of::<INPUT>() as i32);
        if sent_down != 1 {
            return Err(format!("SendInput down failed: sent {}", sent_down));
        }

        // Pici szünet, hogy tényleges lenyomásként hasson
        sleep(Duration::from_millis(10));

        // --- Felengedés ---
        let mut up: INPUT = std::mem::zeroed();
        up.r#type = INPUT_MOUSE;
        up.Anonymous.mi = MOUSEINPUT {
            dx: 0,
            dy: 0,
            mouseData: 0,
            dwFlags: if left_button { MOUSEEVENTF_LEFTUP } else { MOUSEEVENTF_RIGHTUP },
            time: 0,
            dwExtraInfo: 0,
        };
        let sent_up = SendInput(std::slice::from_ref(&up), size_of::<INPUT>() as i32);
        if sent_up != 1 {
            return Err(format!("SendInput up failed: sent {}", sent_up));
        }

        Ok(())
    }
}

/// Simulál egy billentyűnyomást: lenyomás → rövid késleltetés → felengedés
pub fn key_press(vk: VIRTUAL_KEY) -> Result<(), String> {
    unsafe {
        // Len yomás
        let mut down: INPUT = std::mem::zeroed();
        down.r#type = INPUT_KEYBOARD;
        down.Anonymous.ki = KEYBDINPUT {
            wVk: vk,
            wScan: 0,
            dwFlags: KEYBD_EVENT_FLAGS(0),
            time: 0,
            dwExtraInfo: 0,
        };
        let sent_down = SendInput(std::slice::from_ref(&down), size_of::<INPUT>() as i32);
        if sent_down != 1 {
            return Err(format!("SendInput key_down failed: sent {}", sent_down));
        }

        // Rövid pauza, hogy a rendszer érzékelje a lenyomást
        sleep(Duration::from_millis(10));

        // Felengedés
        let mut up: INPUT = std::mem::zeroed();
        up.r#type = INPUT_KEYBOARD;
        up.Anonymous.ki = KEYBDINPUT {
            wVk: vk,
            wScan: 0,
            dwFlags: KEYEVENTF_KEYUP,
            time: 0,
            dwExtraInfo: 0,
        };
        let sent_up = SendInput(std::slice::from_ref(&up), size_of::<INPUT>() as i32);
        if sent_up != 1 {
            return Err(format!("SendInput key_up failed: sent {}", sent_up));
        }

        Ok(())
    }
}
pub fn key_release(vkey: VIRTUAL_KEY) -> Result<(), String> {
    unsafe {
        let mut input: INPUT = std::mem::zeroed();
        input.r#type = INPUT_KEYBOARD;
        input.Anonymous.ki = KEYBDINPUT {
            wVk: vkey,
            wScan: 0,
            dwFlags: KEYEVENTF_KEYUP,  // KEYEVENTF_KEYUP = felengedés jelzés
            time: 0,
            dwExtraInfo: 0,
        };
        let sent = SendInput(std::slice::from_ref(&input), size_of::<INPUT>() as i32);
        if sent == 1 { Ok(()) } else { Err("Key release SendInput failed".to_string()) }
    }
}
