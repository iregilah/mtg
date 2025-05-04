// tests/battlefield_ocr.rs
// Smoke test for battlefield creature OCR scanning

use std::env;
use tracing::info;
use tracing_subscriber;

// a te crate-ed neve lehet más, de az alábbi szerint importáld:
use MTGA_me::app::game_state_updater::{count_branch, detect_creature_count_for_side, load_side_creatures};
use MTGA_me::app::ui::{get_average_color, is_color_within_tolerance};

fn print_usage_and_exit() -> ! {
    eprintln!("Usage:");
    eprintln!("  mtga_test count-branch <y1> <region_h> <rect_w> <x> <tol> <r> <g> <b> <initial> <first_step> <step> <max>");
    eprintln!("  mtga_test detect-count <own|opp>");
    eprintln!("  mtga_test load-side <own|opp>");
    std::process::exit(1);
}

fn main() {
    // init mindenki által látott `info!` loghoz
    tracing_subscriber::fmt::init();

    let mut args = env::args().skip(1);
    let cmd = args.next().unwrap_or_else(|| print_usage_and_exit().to_string());

    // fix 2K felbontás
    let (w, h) = (2560u32, 1440u32);

    match cmd.as_str() {
        "count-branch" => {
            let y1: i32          = args.next().and_then(|s| s.parse().ok()).unwrap_or_else(|| print_usage_and_exit());
            let region_h: i32    = args.next().and_then(|s| s.parse().ok()).unwrap_or_else(|| print_usage_and_exit());
            let rect_w: i32      = args.next().and_then(|s| s.parse().ok()).unwrap_or_else(|| print_usage_and_exit());
            let x: i32           = args.next().and_then(|s| s.parse().ok()).unwrap_or_else(|| print_usage_and_exit());
            let tol: f64         = args.next().and_then(|s| s.parse().ok()).unwrap_or_else(|| print_usage_and_exit());
            let r: u8            = args.next().and_then(|s| s.parse().ok()).unwrap_or_else(|| print_usage_and_exit());
            let g: u8            = args.next().and_then(|s| s.parse().ok()).unwrap_or_else(|| print_usage_and_exit());
            let b: u8            = args.next().and_then(|s| s.parse().ok()).unwrap_or_else(|| print_usage_and_exit());
            let initial: usize   = args.next().and_then(|s| s.parse().ok()).unwrap_or_else(|| print_usage_and_exit());
            let first_step: i32  = args.next().and_then(|s| s.parse().ok()).unwrap_or_else(|| print_usage_and_exit());
            let step: i32        = args.next().and_then(|s| s.parse().ok()).unwrap_or_else(|| print_usage_and_exit());
            let max_count: usize = args.next().and_then(|s| s.parse().ok()).unwrap_or_else(|| print_usage_and_exit());

            info!("→ running count_branch on fixed {}×{}", w, h);
            let result = count_branch(
                y1, region_h, rect_w, x,
                tol, (r,g,b),
                initial, first_step, step, max_count,
            );
            println!("count_branch → {}", result);
        }

        "detect-count" => {
            let side = args.next().unwrap_or_else(|| print_usage_and_exit());
            let is_opp = match side.as_str() {
                "own" => false,
                "opp" => true,
                _     => print_usage_and_exit(),
            };
            info!("→ running detect_creature_count_for_side on fixed {}×{}", w, h);
            let cnt = detect_creature_count_for_side(w, h, is_opp);
            println!("detect-creature-count({}) → {}", side, cnt);
        }

        "load-side" => {
            let side = args.next().unwrap_or_else(|| print_usage_and_exit());
            let is_opp = match side.as_str() {
                "own" => false,
                "opp" => true,
                _     => print_usage_and_exit(),
            };
            info!("→ running load_side_creatures on fixed {}×{}", w, h);
            let map = load_side_creatures(w, h, is_opp);
            println!("loaded creatures on {} side: {} entries", side, map.len());
            for name in map.keys() {
                println!("  - {}", name);
            }
        }

        _ => print_usage_and_exit(),
    }
}
