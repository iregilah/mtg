// main.rs

use tracing::info;
use tracing_subscriber;

use MTGA_me::app::*;

#[cfg(target_os = "linux")]
use std::os::linux::io::RawFd;
use MTGA_me::multi_platform::init;
use windows::Win32::UI::HiDpi::{
    SetProcessDpiAwareness,
    PROCESS_PER_MONITOR_DPI_AWARE,
};
#[cfg(target_os = "windows")]
fn enable_dpi_awareness() {
    unsafe {
        // Próbáld meg használni a DPI awareness beállítást
        let result = SetProcessDpiAwareness(PROCESS_PER_MONITOR_DPI_AWARE);
        if let Err(e) = result {
            eprintln!("Failed to set DPI awareness: {:?}", e);
        }
    }
}

#[cfg(not(target_os = "windows"))]
fn enable_dpi_awareness() {
    // no-op on non-Windows
}


fn main() {
    // Init logging
    tracing_subscriber::fmt::init();

    // Try to enable DPI awareness on Windows
    enable_dpi_awareness();
    // Try to initialize the platform (Wayland, X11, Windows)
    if let Err(e) = init() {
        eprintln!("Failed to initialize platform: {}", e);
        return;
    }

    // CLI switch: extra arg for coordinate-mode
    let args: Vec<String> = std::env::args().collect();
    if args.len() > 1 {
        info!("RightClick to get coordinates. LeftClick to exit.");
    } else {
        let mut app = App::new();
        app.start();
    }
}