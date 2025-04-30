// src/main.rs

use tracing::info;
use tracing_subscriber;


use MTGA_me::app::*;
use MTGA_me::multiplatform::init;

fn main() {
    // Init logging
    tracing_subscriber::fmt::init();

    // Initialize cross-platform input and screen utilities
    if let Err(e) = init() {
        eprintln!("Failed to initialize multiplatform utilities: {}", e);
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
