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

/// Reads the color of the pixel at (x, y) from the screen.
pub fn get_pixel_color(x: i32, y: i32) -> Result<(u8, u8, u8), String> {
    unsafe {
        // Get device context for the entire desktop (NULL handle)
        let hdc = GetDC(None);
        if hdc.0.is_null() {
            return Err("GetDC failed".to_string());
        }
        // Read the pixel color
        let color = GetPixel(hdc, x, y);
        // Release the device context
        ReleaseDC(None, hdc);
        // COLORREF.0 holds the raw u32 value
        if color.0 == u32::MAX {
            // GetPixel returns u32::MAX on failure
            return Err("GetPixel failed".to_string());
        }
        // COLORREF.0 format: 0x00BBGGRR
        let raw = color.0;
        let r = (raw & 0xFF) as u8;
        let g = ((raw >> 8) & 0xFF) as u8;
        let b = ((raw >> 16) & 0xFF) as u8;
        Ok((r, g, b))
    }
}

/// Moves the mouse cursor to absolute screen coordinates.
pub fn move_mouse(x: i32, y: i32) -> Result<(), String> {
    // SetCursorPos moves the cursor using absolute screen coordinates and returns a Result
    unsafe {
        SetCursorPos(x, y)
            .map_err(|e| format!("SetCursorPos failed: {}", e))?;
    }
    Ok(())
}

/// Simulate a mouse click: press → short delay → release
pub fn mouse_click(left_button: bool) -> Result<(), String> {
    unsafe {
        // --- Press down ---
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

        // Short pause to register the press
        sleep(Duration::from_millis(10));

        // --- Release ---
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

/// Simulate a key press: down → short delay → up
pub fn key_press(vk: VIRTUAL_KEY) -> Result<(), String> {
    unsafe {
        // Press down
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

        // Short pause to ensure the system registers the press
        sleep(Duration::from_millis(10));

        // Release key
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

/// KEYEVENTF_KEYUP indicates a key release event.
pub fn key_release(vkey: VIRTUAL_KEY) -> Result<(), String> {
    unsafe {
        let mut input: INPUT = std::mem::zeroed();
        input.r#type = INPUT_KEYBOARD;
        input.Anonymous.ki = KEYBDINPUT {
            wVk: vkey,
            wScan: 0,
            dwFlags: KEYEVENTF_KEYUP,  // KEYEVENTF_KEYUP indicates key release
            time: 0,
            dwExtraInfo: 0,
        };
        let sent = SendInput(std::slice::from_ref(&input), size_of::<INPUT>() as i32);
        if sent == 1 { Ok(()) } else { Err("Key release SendInput failed".to_string()) }
    }
}
