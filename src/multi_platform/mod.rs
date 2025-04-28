// src/multiplatform/mod.rs
pub mod windows_platform;
pub mod wayland_platform;
pub mod x11_platform;
use imageproc::drawing::Canvas;
use screenshots::Screen;
#[cfg(target_os = "windows")]
use windows::Win32::UI::Input::KeyboardAndMouse::VIRTUAL_KEY;

pub enum Backend {
    Windows,
    X11,
    Wayland,
}

/// Returns (width, height) of the primary monitor.
pub fn screen_size() -> Result<(i32, i32), String> {
    let screens = Screen::all().map_err(|e| e.to_string())?;
    if let Some(screen) = screens.first() {
        let di = &screen.display_info;
        return Ok((di.width as i32, di.height as i32));
    }
    Err("No screens found".into())
}

pub fn detect_backend() -> Backend {
    #[cfg(target_os = "windows")]
    { return Backend::Windows; }

    #[cfg(target_os = "linux")]
    {
        if std::env::var("WAYLAND_DISPLAY").is_ok() ||
            std::env::var("XDG_SESSION_TYPE").map_or(false, |v| v == "wayland") {
            Backend::Wayland
        } else {
            Backend::X11
        }
    }
    #[cfg(target_os = "macos")]
    {
        // macOS can be handled separately if needed, else fallback to X11 or Wayland.
        Backend::X11
    }
}

#[cfg(target_os = "windows")]
use crate::multi_platform::windows_platform as current_platform;
#[cfg(all(target_os = "linux", not(feature = "force_x11")))]
use crate::wayland_platform as wayland_platform;
#[cfg(target_os = "linux")]
use crate::x11_platform as x11_platform;

#[cfg(all(target_os = "linux", not(feature = "force_x11")))]
static mut WL_PLATFORM: Option<wayland_platform::WaylandPlatform> = None;

/// Initialization: establishes connection and resources for Wayland.
pub fn init() -> Result<(), String> {
    match detect_backend() {
        Backend::Windows => Ok(()), // No separate init required on Windows
        Backend::X11 => Ok(()),     // No preliminary init needed for X11 either
        #[cfg(all(target_os = "linux", not(feature = "force_x11")))]
        Backend::Wayland => {
            unsafe {
                if WL_PLATFORM.is_none() {
                    WL_PLATFORM = Some(wayland_platform::WaylandPlatform::new()?);
                }
            }
            Ok(())
        }
        // on non-Linux builds, we still need to handle the variant,
        // but complain (it’ll never actually run on Windows)
        #[cfg(not(all(target_os = "linux", not(feature = "x11"))))]
        Backend::Wayland => Err("Wayland backend not supported on this platform".into()),
    }
}

pub fn get_pixel(x: i32, y: i32) -> Result<(u8, u8, u8), String> {
    match detect_backend() {
        Backend::Windows => current_platform::get_pixel_color(x, y),
        // only compile this arm when we actually have x11_platform available:
        #[cfg(target_os = "linux")]
        Backend::X11 => x11_platform::get_pixel_color(x, y),

        // on non-Linux builds, still cover the X11 variant so the match is exhaustive:
        #[cfg(not(target_os = "linux"))]
        Backend::X11 => Err("X11 backend not supported on this platform".into()),

        Backend::Wayland => {
            // Reading pixel color under Wayland is cumbersome: using the screenshot crate is recommended
            // Alternative: one would need to implement the screencopy protocol, which goes beyond the current task.
            Err("Wayland: Pixel szín olvasás nem támogatott közvetlenül".into())
        }
    }
}

pub fn move_cursor(x: i32, y: i32) -> Result<(), String> {
    match detect_backend() {
        Backend::Windows => current_platform::move_mouse(x, y),
        // only compile this on Linux, where x11_platform actually exists:
        #[cfg(target_os = "linux")]
        Backend::X11     => x11_platform::move_mouse(x, y),

        // on non-Linux builds still cover the variant so the match is exhaustive:
        #[cfg(not(target_os = "linux"))]
        Backend::X11     => Err("X11 backend not supported on this platform".into()),

        // only compile this path (and thus the reference to WL_PLATFORM) on Linux + Wayland
        #[cfg(all(target_os = "linux", not(feature = "force_x11")))]
        Backend::Wayland => unsafe {
            if WL_PLATFORM.is_none() {
                return Err("WaylandPlatform not initialized".into());
            }
            WL_PLATFORM
                .as_mut()
                .unwrap()
                .move_mouse_absolute(x as u32, y as u32)
        },

        // On all other platforms, still match Wayland but return an error
        #[cfg(not(all(target_os = "linux", not(feature = "x11"))))]
        Backend::Wayland => Err("WaylandPlatform not supported on this platform".into()),
    }
}

pub fn click_left() -> Result<(), String> {
    match detect_backend() {
        Backend::Windows => current_platform::mouse_click(true),

        // only on Linux, where x11_platform actually exists
        #[cfg(target_os = "linux")]
        Backend::X11 => {
            x11_platform::mouse_click(1, true)?;   // press
            x11_platform::mouse_click(1, false)?;  // release
            Ok(())
        }

        // reject on all other platforms
        #[cfg(not(target_os = "linux"))]
        Backend::X11 => Err("X11 backend not supported on this platform".into()),

        // The Wayland branches remain unchanged...
        #[cfg(all(target_os = "linux", not(feature = "force_x11")))]
        Backend::Wayland => unsafe {
            if WL_PLATFORM.is_none() {
                return Err("WaylandPlatform not initialized".into());
            }
            WL_PLATFORM.as_mut().unwrap().mouse_click(272)  // BTN_LEFT
        },

        #[cfg(not(all(target_os = "linux", not(feature = "x11"))))]
        Backend::Wayland => Err("WaylandPlatform not supported on this platform".into()),
    }
}

pub fn send_key(key: u32) -> Result<(), String> {
    match detect_backend() {
        Backend::Windows => {
            // On Windows using windows_platform::key_press / key_release
            current_platform::key_press(VIRTUAL_KEY(key as u16))?;    // press
            current_platform::key_release(VIRTUAL_KEY(key as u16))?;  // release
            Ok(())
        }

        // Only on Linux, where x11_platform actually exists
        #[cfg(target_os = "linux")]
        Backend::X11 => {
            x11_platform::key_event(key, true)?;   // press
            x11_platform::key_event(key, false)?;  // release
            Ok(())
        }

        // On other platforms, X11 is not available, signal error
        #[cfg(not(target_os = "linux"))]
        Backend::X11 => Err("X11 backend not supported on this platform".into()),

        // Wayland branches remain as before
        #[cfg(all(target_os = "linux", not(feature = "force_x11")))]
        Backend::Wayland => unsafe {
            if WL_PLATFORM.is_none() {
                return Err("WaylandPlatform not initialized".into());
            }
            WL_PLATFORM.as_mut().unwrap().key_press(key)?;   // press
            WL_PLATFORM.as_mut().unwrap().key_release(key)   // release, last call returns Result
        },

        #[cfg(not(all(target_os = "linux", not(feature = "x11"))))]
        Backend::Wayland => Err("WaylandPlatform not supported on this platform".into()),
    }
}
