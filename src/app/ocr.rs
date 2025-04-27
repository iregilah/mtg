// app/ocr.rs

use image::Rgba;
use std::{process::Command, thread::sleep, time::Duration};
use tracing::{error, info};
use thiserror::Error;

use screenshots::Screen;

use image::{DynamicImage, ImageBuffer};
use image::imageops::{crop_imm, resize, FilterType};

use crate::app::{
    cards_positions::get_card_positions,
    creature_positions::CreaturePosition,
    ui::{set_cursor_pos},
};

/// Keep only alphanumeric, whitespace, and a handful of punctuation.
pub fn sanitize_ocr_text(input: &str) -> String {
    input
        .chars()
        .filter(|c| c.is_alphanumeric() || c.is_whitespace() || matches!(c, '-' | ',' | '.' | '\''))
        .collect()
}

/// Simple black‐white threshold for Luma8 images.
pub fn threshold_image(
    img: &ImageBuffer<image::Luma<u8>, Vec<u8>>,
    cutoff: u8,
) -> ImageBuffer<image::Luma<u8>, Vec<u8>> {
    let mut out = img.clone();
    for pixel in out.pixels_mut() {
        pixel[0] = if pixel[0] < cutoff { 0 } else { 255 };
    }
    out
}

/// Standard OCR pre‐processing: grayscale → contrast → threshold → upscale.
pub fn preprocess_image(cropped: &DynamicImage) -> DynamicImage {
    let gray = cropped.to_luma8();
    let contrasted = DynamicImage::ImageLuma8(gray).adjust_contrast(40.0);
    let gray2 = contrasted.to_luma8();
    let thresh = threshold_image(&gray2, 100);
    let binarized = DynamicImage::ImageLuma8(thresh).to_rgba8();
    let upscaled = resize(
        &binarized,
        binarized.width() * 2,
        binarized.height() * 2,
        FilterType::Lanczos3,
    );
    DynamicImage::ImageRgba8(upscaled)
}


/// “Red button” pipeline: invert‐with‐tolerance.
pub fn white_invert_image(cropped: &DynamicImage) -> DynamicImage {
    let rgb = cropped.to_rgb8();
    let (w, h) = (rgb.width(), rgb.height());
    let mut out = image::ImageBuffer::new(w, h);
    let tol = 50u8;
    for (x, y, px) in rgb.enumerate_pixels() {
        let (r, g, b) = (px[0], px[1], px[2]);
        let inverted_black = (255u8.saturating_sub(r) < tol)
            && (255u8.saturating_sub(g) < tol)
            && (255u8.saturating_sub(b) < tol);
        out.put_pixel(x, y, if inverted_black {
            image::Rgb([0, 0, 0])
        } else {
            image::Rgb([255, 255, 255])
        });
    }
    DynamicImage::ImageRgba8(DynamicImage::ImageRgb8(out).to_rgba8())
}

/// Grab a full‐screen `DynamicImage`.
fn capture_screen() -> Option<DynamicImage> {
    // Capture the primary monitor (first available screen)
    let screens = Screen::all().ok()?;
    let screen = screens.get(0)?;
    // Capture a screenshot (screenshots::image::RgbaImage)
    let buffer = screen.capture().ok()?;
    // Convert screenshots::RgbaImage into your image::ImageBuffer<Rgba<u8>, Vec<u8>>
    let (w, h) = (buffer.width(), buffer.height());
    let raw = buffer.into_raw(); // Vec<u8> of RGBA pixels
    let img_buf = ImageBuffer::<Rgba<u8>, Vec<u8>>::from_raw(w, h, raw)
        .expect("from_raw failed: dimensions/raw length mismatch");
    Some(DynamicImage::ImageRgba8(img_buf))
}

/// If cropping fails, one of these errors will be returned.
#[derive(Debug, Error)]
pub enum CropError {
    #[error("Crop region {x1},{y1}-{x2},{y2} is outside of image bounds {width}×{height}")]
    OutOfBounds {
        x1: u32,
        y1: u32,
        x2: u32,
        y2: u32,
        width: u32,
        height: u32,
    },
    #[error("Crop region has zero width or height: {width}×{height}")]
    InvalidRegion {
        x1: u32,
        y1: u32,
        x2: u32,
        y2: u32,
        width: u32,
        height: u32,
    },
}

/// Crops the given image to the rectangle (x1,y1)–(x2,y2), or returns an error if
/// that rectangle is invalid or out of bounds.
pub fn crop_region(
    img: &DynamicImage,
    x1: u32,
    y1: u32,
    x2: u32,
    y2: u32,
) -> Result<DynamicImage, CropError> {
    let img_w = img.width();
    let img_h = img.height();

    // 1) Check bounds
    if x2 > img_w || y2 > img_h {
        error!("Crop out of bounds: requested ({},{})–({},{}) in image {}×{}", x1, y1, x2, y2, img_w, img_h);
        return Err(CropError::OutOfBounds { x1, y1, x2, y2, width: img_w, height: img_h });
    }

    // 2) Compute dimensions and validate
    let w = x2.saturating_sub(x1);
    let h = y2.saturating_sub(y1);
    if w == 0 || h == 0 {
        error!("Invalid crop region: zero dimension: requested ({},{})–({},{}) gives {}×{}", x1, y1, x2, y2, w, h);
        return Err(CropError::InvalidRegion { x1, y1, x2, y2, width: w, height: h });
    }

    // 3) Perform the crop
    let sub = crop_imm(img, x1, y1, w, h).to_image();
    Ok(DynamicImage::ImageRgba8(sub))
}

/// Save `img` to `temp_filename`, run Tesseract, return sanitized text.
fn run_tesseract_pipeline(img: &DynamicImage, temp_filename: &str) -> String {
    if img.save(&temp_filename).is_err() {
        error!("Cannot save temp OCR image: {}", temp_filename);
        return String::new();
    }
    let output = Command::new(r"C:\Program Files\Tesseract-OCR\tesseract.exe")
        .arg(&temp_filename)
        .arg("stdout")
        .arg("-l").arg("eng")
        .arg("--psm").arg("7")
        .output();
    if let Ok(o) = output {
        if o.status.success() {
            let raw = String::from_utf8_lossy(&o.stdout);
            return sanitize_ocr_text(raw.trim());
        } else {
            error!("Tesseract failed on {}: {}", temp_filename, String::from_utf8_lossy(&o.stderr));
        }
    } else {
        error!("Failed to run Tesseract on {}", temp_filename);
    }
    String::new()
}

/// Reads the “Start Order” label using the white‑invert pipeline and Tesseract,
/// with full tracing of capture, crop, temp file and OCR result.
pub fn check_start_order_text(screen_width: u32, screen_height: u32) -> String {
    info!("check_start_order_text()");
    // 1) compute region
    let x1 = (238.294 / 677.292 * screen_width as f64).floor() as u32;
    let x2 = (437.764 / 677.292 * screen_width as f64).floor() as u32;
    let y1 = (21.432 / 381.287 * screen_height as f64).floor() as u32;
    let y2 = (43.454 / 381.287 * screen_height as f64).floor() as u32;
    info!("  region coords: x1={}, y1={}, x2={}, y2={}", x1, y1, x2, y2);

    // 2) capture
    let screen = match capture_screen() {
        Some(img) => {
            info!("  screen captured {}×{}", img.width(), img.height());
            img
        }
        None => {
            error!("  failed to capture screen for start order OCR");
            return String::new();
        }
    };

    // 3) crop
    let cropped = match crop_region(&screen, x1, y1, x2, y2) {
        Ok(img) => img,
        Err(err) => {
            error!("  crop_region failed: {}", err);
            return String::new();
        }
    };

    // 4) preprocess (white‑invert)
    let processed = white_invert_image(&cropped);

    // 5) save temp + log
    let temp_filename = "temp_start_order.png";
    info!("  saving temp OCR image as '{}'", temp_filename);

    // 6) OCR
    let result = run_tesseract_pipeline(&processed, temp_filename);
    info!("  OCR start-order → {:?}", result);
    result
}

/// Reads whatever text is in the “main region” (the red vs non‑red button area),
/// choosing the pipeline via `is_red_button`, with full tracing.
pub fn check_main_region_text(
    screen_width: u32,
    screen_height: u32,
    is_red_button: bool,
) -> String {
    info!("check_main_region_text(is_red_button={})", is_red_button);
    // 1) compute region
    let x1 = (589.799 / 677.292 * screen_width as f64).floor() as u32;
    let x2 = (665.000 / 677.292 * screen_width as f64).floor() as u32;
    let y1 = (328.906 / 381.287 * screen_height as f64).floor() as u32;
    let y2 = (341.871 / 381.287 * screen_height as f64).floor() as u32;
    info!("  region coords: x1={}, y1={}, x2={}, y2={}", x1, y1, x2, y2);

    // 2) capture
    let screen = match capture_screen() {
        Some(img) => {
            info!("  screen captured {}×{}", img.width(), img.height());
            img
        }
        None => {
            error!("  failed to capture screen for main region OCR");
            return String::new();
        }
    };

    // 3) crop
    let cropped = match crop_region(&screen, x1, y1, x2, y2) {
        Ok(img) => img,
        Err(err) => {
            error!("  crop_region failed: {}", err);
            return String::new();
        }
    };

    // 4) preprocess
    let processed: DynamicImage = if is_red_button {
        info!("  using white_invert pipeline");
        white_invert_image(&cropped)
    } else {
        info!("  using normal preprocess pipeline");
        preprocess_image(&cropped)
    };

    // 5) save temp + log
    let temp_filename = if is_red_button {
        "temp_main_region_red.png"
    } else {
        "temp_main_region.png"
    };
    info!("  saving temp OCR image as '{}'", temp_filename);

    // 6) OCR
    let result = run_tesseract_pipeline(&processed, temp_filename);
    info!("  OCR main region → {:?}", result);
    result
}

/// Read a single creature slot by index (1-based) and side.
pub fn read_creature_text(
    pos: CreaturePosition,
    index: usize,       // 1-based
    is_opponent: bool,  // false = ours, true = theirs
    _screen_width: u32,
    _screen_height: u32,
) -> String {
    // 1) Log intent
    info!(
        "read_creature_text: slot #{} for {}",
        index,
        if is_opponent { "opponent" } else { "self" }
    );

    // 2) Capture full‐screen
    let screen = match capture_screen() {
        Some(img) => {
            info!("Captured screen: {}x{}", img.width(), img.height());
            img
        }
        None => {
            error!("Failed to capture screen for creature OCR");
            return String::new();
        }
    };

    // 3) Log and perform crop
    info!("  cropping creature region: x1={}, y1={}, x2={}, y2={}", pos.ocr_x1, pos.ocr_y1, pos.ocr_x2, pos.ocr_y2);
    let cropped = match crop_region(&screen, pos.ocr_x1, pos.ocr_y1, pos.ocr_x2, pos.ocr_y2) {
        Ok(img) => img,
        Err(err) => {
            error!("  crop_region failed: {}", err);
            return String::new();
        }
    };

    // 4) Preprocess
    let processed = preprocess_image(&cropped);

    // 5) Build stable temp filename
    let parity = if index % 2 == 1 { "odd" } else { "even" };
    let side = if is_opponent { "opponents_creature" } else { "creature" };
    let temp_filename = format!("temp_{}_{}_{}.png", parity, side, index);
    info!("Saving temporary OCR image as '{}'", temp_filename);

    // 6) Run Tesseract
    let result = run_tesseract_pipeline(&processed, &temp_filename);
    info!("OCR result for slot #{}: {:?}", index, result);
    result
}

/// Reads the visible text of one hand‐card by index on the battlefield.
pub fn get_card_text(
    index: usize,
    card_count: usize,
    screen_width: u32,
    screen_height: u32,
) -> String {
    // 1) Hover over the card slot
    let positions = get_card_positions(card_count, screen_width);
    let pos = match positions.get(index) {
        Some(p) => p,
        None => {
            error!("get_card_text(): index {} out of range ({} cards)", index, card_count);
            return String::new();
        }
    };
    let hover_y = ((screen_height as f64) * 0.97).floor() as i32;
    set_cursor_pos(pos.hover_x as i32, hover_y);
    sleep(Duration::from_secs(2));

    // 2) Capture the full screen
    let screen = match capture_screen() {
        Some(img) => img,
        None => {
            error!("Failed to capture screen for card {}", index);
            return String::new();
        }
    };

    // 3) Crop to the card’s OCR region
    let y1 = ((232.606 / 381.287) * screen_height as f64).floor() as u32;
    let y2 = ((240.832 / 381.287) * screen_height as f64).floor() as u32;
    let cropped = match crop_region(&screen, pos.ocr_x1, y1, pos.ocr_x2, y2) {
        Ok(img) => img,
        Err(e) => {
            error!("crop_region failed for card {}: {}", index, e);
            return String::new();
        }
    };

    // 4) Preprocess for OCR
    let processed = preprocess_image(&cropped);

    // 5) Save a stable temp file & run Tesseract
    let tmp = format!("temp_card_{}.png", index);
    info!("Saving temp OCR image as '{}'", tmp);
    let result = run_tesseract_pipeline(&processed, &tmp);

    info!("OCR result for card {}: {:?}", index, result);
    result
}

pub fn read_life_total(
    is_opponent: bool,
    screen_width: u32,
    screen_height: u32,
) -> i32 {
    // Relatív koordináták: felső sáv ellenfélnek, alsó sáv nekünk
    let (x1f, y1f, x2f, y2f) = if is_opponent {
        (0.45, 0.03, 0.55, 0.10)
    } else {
        (0.45, 0.90, 0.55, 0.97)
    };
    let x1 = (x1f * screen_width as f64).floor() as u32;
    let x2 = (x2f * screen_width as f64).floor() as u32;
    let y1 = (y1f * screen_height as f64).floor() as u32;
    let y2 = (y2f * screen_height as f64).floor() as u32;

    // Képernyőfogás és vágás
    let screen = match capture_screen() {
        Some(img) => img,
        None => return 0,
    };
    let cropped = match crop_region(&screen, x1, y1, x2, y2) {
        Ok(img) => img,
        Err(_) => return 0,
    };

    // Előfeldolgozás és OCR
    let processed = preprocess_image(&cropped);
    let temp_file = if is_opponent { "temp_opp_life.png" } else { "temp_us_life.png" };
    let raw_text = run_tesseract_pipeline(&processed, temp_file);

    // Csak a számjegyeket tartjuk meg, parse-oljuk
    raw_text
        .chars()
        .filter(|c| c.is_ascii_digit())
        .collect::<String>()
        .parse::<i32>()
        .unwrap_or(0)
}