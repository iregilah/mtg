#![cfg(target_os = "linux")]

use std::ffi::CString;
use std::ptr;
use x11::xlib;
use x11::xlib::{Display, XImage};
use x11::xtest;
use std::{thread::sleep, time::Duration};

/// Reads the color of the pixel at (x, y) from the default X11 display.
pub fn get_pixel_color(x: i32, y: i32) -> Result<(u8, u8, u8), String> {
    unsafe {
        // Open default X display (DISPLAY env var)
        let display = xlib::XOpenDisplay(ptr::null());
        if display.is_null() {
            return Err("Failed to connect to X11 display".into());
        }
        // Get the root window for the display
        let root = xlib::XDefaultRootWindow(display);
        if root == 0 {
            xlib::XCloseDisplay(display);
            return Err("Root window not found".into());
        }
        // Capture a 1x1 pixel area at the given coordinates
        let img: *mut XImage = xlib::XGetImage(
            display,
            root,
            x,
            y,
            1, 1,
            !0,
            xlib::ZPixmap,
        );
        if img.is_null() {
            xlib::XCloseDisplay(display);
            return Err("XGetImage returned NULL".into());
        }
        // Extract pixel data
        let pixel = xlib::XGetPixel(img, 0, 0);
        // Destroy the XImage
        ((*img).f.destroy_image.expect("destroy_image func"))(img);
        xlib::XCloseDisplay(display);
        // Parse 32-bit pixel (XRGB8888 format)
        let r = ((pixel >> 16) & 0xFF) as u8;
        let g = ((pixel >> 8) & 0xFF) as u8;
        let b = (pixel & 0xFF) as u8;
        Ok((r, g, b))
    }
}

/// Moves the mouse to the specified absolute screen coordinates.
pub fn move_mouse(x: i32, y: i32) -> Result<(), String> {
    unsafe {
        let display = xlib::XOpenDisplay(ptr::null());
        if display.is_null() {
            return Err("XOpenDisplay failed".into());
        }
        let screen_num = xlib::XDefaultScreen(display);
        // Simulate mouse motion event (absolute coordinates)
        let res = xtest::XTestFakeMotionEvent(display, screen_num, x, y, 0);
        xlib::XFlush(display);
        xlib::XCloseDisplay(display);
        if res == 0 {
            Err("XTestFakeMotionEvent failed".into())
        } else {
            Ok(())
        }
    }
}

/// Simulates a mouse button press or release (1=left, 2=middle, 3=right).
pub fn mouse_click(button: u32, press: bool) -> Result<(), String> {
    unsafe {
        let display = xlib::XOpenDisplay(ptr::null());
        if display.is_null() {
            return Err("XOpenDisplay failed".into());
        }
        let is_press = if press { xlib::True } else { xlib::False };
        // Simulate button event
        let res = xtest::XTestFakeButtonEvent(display, button, is_press, 0);
        xlib::XFlush(display);
        xlib::XCloseDisplay(display);
        if res == 0 {
            return Err("XTestFakeButtonEvent failed".into());
        }
    }
    // Short delay after press
    if press {
        sleep(Duration::from_millis(10));
    }
    Ok(())
}

/// Simulates a key press or release using X11 keycodes.
pub fn key_event(keycode: u32, press: bool) -> Result<(), String> {
    unsafe {
        let display = xlib::XOpenDisplay(ptr::null());
        if display.is_null() {
            return Err("XOpenDisplay failed".into());
        }
        let is_press = if press { xlib::True } else { xlib::False };
        let res = xtest::XTestFakeKeyEvent(display, keycode, is_press, 0);
        xlib::XFlush(display);
        xlib::XCloseDisplay(display);
        if res == 0 {
            return Err("XTestFakeKeyEvent failed".into());
        }
    }
    // Short delay after press
    if press {
        sleep(Duration::from_millis(10));
    }
    Ok(())
}
