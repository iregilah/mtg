use MTGA_me::app::*;
use tracing_subscriber;

fn main() {
    // Initialize tracing subscriber.
    tracing_subscriber::fmt::init();

    // DPI awareness beállítás
    unsafe {
        let _ = winapi::um::shellscalingapi::SetProcessDpiAwareness(winapi::um::shellscalingapi::PROCESS_PER_MONITOR_DPI_AWARE);
        winapi::um::winuser::SetProcessDPIAware();
    }

    // Parancssori argumentumok ellenőrzése
    let args: Vec<String> = std::env::args().collect();
    if args.len() > 1 {
        tracing::info!("RightClick to get coordinates. LeftClick for exit (Not implemented in Rust version).");
    } else {
        App::start();
    }
}