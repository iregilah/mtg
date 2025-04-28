// src/multiplatform/x11_platform.rs
#![cfg(target_os = "linux")]

use std::ffi::CString;
use std::ptr;
use x11::xlib;
use x11::xlib::{Display, XImage};
use x11::xtest;
use std::{thread::sleep, time::Duration};

pub fn get_pixel_color(x: i32, y: i32) -> Result<(u8, u8, u8), String> {
    unsafe {
        // Csatlakozás a default X serverhez (DISPLAY env változó alapján)
        let display = xlib::XOpenDisplay(ptr::null());
        if display.is_null() {
            return Err("Nem sikerült csatlakozni az X11 display-hez".into());
        }
        // A gyökérablak (teljes képernyő) lekérése az alapértelmezett kijelzőn
        let root = xlib::XDefaultRootWindow(display);
        if root == 0 {
            xlib::XCloseDisplay(display);
            return Err("Nem található root window".into());
        }
        // 1x1 pixeles terület beolvasása a megadott koordinátától
        let img: *mut XImage = xlib::XGetImage(
            display,
            root,
            x as i32,
            y as i32,
            1, 1,    // width=1, height=1
            !0,      // all planes (~0 = 0xFFFFFFFF mask)
            xlib::ZPixmap,
        );
        if img.is_null() {
            xlib::XCloseDisplay(display);
            return Err("XGetImage visszatért NULL-lal".into());
        }
        // A pixelt az XImage struktúrából kinyerjük
        let pixel = xlib::XGetPixel(img, 0, 0);
        // Felszabadítjuk az XImage struktúrát a megfelelő függvénnyel
        ((*img).f.destroy_image.expect("destroy_image func"))(img);
        xlib::XCloseDisplay(display);
        // Az XGetPixel 32-bites pixelértéket ad vissza a kép pixelformatuma szerint (ált. XRGB8888)
        let r = ((pixel >> 16) & 0xFF) as u8;
        let g = ((pixel >> 8) & 0xFF) as u8;
        let b = (pixel & 0xFF) as u8;
        Ok((r, g, b))
    }
}

pub fn move_mouse(x: i32, y: i32) -> Result<(), String> {
    unsafe {
        let display = xlib::XOpenDisplay(ptr::null());
        if display.is_null() {
            return Err("XOpenDisplay failed".into());
        }
        // Relatív=0 (ez itt irreleváns, mert abszolút koordinátát adunk meg az egész képernyőn)
        let screen_num = xlib::XDefaultScreen(display);
        // A XTestFakeMotionEvent-nek megadjuk a screen indexet és az abszolút koordinátákat.
        // A következő paraméter (1) jelzi, hogy relatív mozgás helyett abszolút koordinátát értelmezzen.
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

pub fn mouse_click(button: u32, press: bool) -> Result<(), String> {
    // button: 1 = bal gomb, 2 = középső, 3 = jobb gomb (X11 hagyományos számozása)
    unsafe {
        let display = xlib::XOpenDisplay(ptr::null());
        if display.is_null() {
            return Err("XOpenDisplay failed".into());
        }
        let is_press = if press { xlib::True } else { xlib::False };
        // XTestFakeButtonEvent: gomb lenyomás/felengedés szimulálása
        let res = xtest::XTestFakeButtonEvent(display, button, is_press, 0);
        xlib::XFlush(display);
        xlib::XCloseDisplay(display);
        if res == 0 {
            Err("XTestFakeButtonEvent failed".into())
        }
    }
    if press {
        sleep(Duration::from_millis(10));
    }
    Ok(())
}

pub fn key_event(keycode: u32, press: bool) -> Result<(), String> {
    // keycode: X11 keycode (általában az Xlib keycode, nem pedig XK keysym),
    // pl. az 'A' betű keycode-ját XKeysymToKeycode segítségével kaphatnánk meg.
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
            Err("XTestFakeKeyEvent failed".into())
        }
    }
    if press {
        sleep(Duration::from_millis(10));
    }
}
