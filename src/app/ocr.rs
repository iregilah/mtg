extern crate chrono;
extern crate screenshot;
extern crate image;

use chrono::Local;
use screenshot::get_screenshot;
use image::{DynamicImage, ImageBuffer, Rgba};
use image::imageops::{crop_imm, resize, FilterType};
use std::ffi::OsStr;
use std::os::windows::ffi::OsStrExt;
use std::slice;
use std::process::Command;
use crate::app::creature_positions::CreaturePosition;
use std::time::{SystemTime, UNIX_EPOCH};



/// Szűrő: csak betűk, számok, szóköz és néhány írásjel
pub fn sanitize_ocr_text(input: &str) -> String {
    input
        .chars()
        .filter(|c| c.is_alphanumeric() || c.is_whitespace() || matches!(c, '-' | ',' | '.' | '\''))
        .collect()
}

/// Egyszerű threshold
pub fn threshold_image(
    img: &ImageBuffer<image::Luma<u8>, Vec<u8>>,
    cutoff: u8,
) -> ImageBuffer<image::Luma<u8>, Vec<u8>> {
    let mut out = img.clone();
    for pixel in out.pixels_mut() {
        let new_val = if pixel[0] < cutoff { 0 } else { 255 };
        *pixel = image::Luma([new_val]);
    }
    out
}

/// Normál pipeline: grayscale, kontraszt, threshold, upscaling
pub fn preprocess_image(cropped: &DynamicImage) -> DynamicImage {
    let gray = cropped.to_luma8();
    let contrasted = DynamicImage::ImageLuma8(gray).adjust_contrast(40.0);
    let gray2 = contrasted.to_luma8();
    let thresholded = threshold_image(&gray2, 100);
    let binarized = DynamicImage::ImageLuma8(thresholded).to_rgba8();
    let upscaled = resize(
        &binarized,
        binarized.width() * 2,
        binarized.height() * 2,
        FilterType::Lanczos3,
    );
    DynamicImage::ImageRgba8(upscaled)
}

/// White invert pipeline: kis toleranciával inverz átalakítás
pub fn white_invert_image(cropped: &DynamicImage) -> DynamicImage {
    let rgb = cropped.to_rgb8();
    let (w, h) = (rgb.width(), rgb.height());
    let mut out = image::ImageBuffer::new(w, h);
    let tol = 50u8;
    for (x, y, pixel) in rgb.enumerate_pixels() {
        let (r, g, b) = (pixel[0], pixel[1], pixel[2]);
        if (255u8.saturating_sub(r) < tol)
            && (255u8.saturating_sub(g) < tol)
            && (255u8.saturating_sub(b) < tol)
        {
            out.put_pixel(x, y, image::Rgb([0, 0, 0]));
        } else {
            out.put_pixel(x, y, image::Rgb([255, 255, 255]));
        }
    }
    let out_rgba = DynamicImage::ImageRgb8(out).to_rgba8();
    DynamicImage::ImageRgba8(out_rgba)
}

/// A "start_order" felirat OCR-je – itt a white_invert pipeline-t használjuk.
pub fn check_start_order_text(screen_width: u32, screen_height: u32) -> String {
    let x1 = (238.294 / 677.292 * screen_width as f64).floor() as u32;
    let x2 = (437.764 / 677.292 * screen_width as f64).floor() as u32;
    let y1 = (21.432 / 381.287 * screen_height as f64).floor() as u32;
    let y2 = (43.454 / 381.287 * screen_height as f64).floor() as u32;
    let crop_width = x2.saturating_sub(x1);
    let crop_height = y2.saturating_sub(y1);

    let screenshot = match get_screenshot(0) {
        Ok(scn) => scn,
        Err(_) => return String::new(),
    };
    if x2 > screenshot.width() as u32 || y2 > screenshot.height() as u32 {
        return String::new();
    }
    let data_vec = unsafe {
        slice::from_raw_parts(screenshot.raw_data(), screenshot.raw_len()).to_vec()
    };
    let image_buf_opt = ImageBuffer::<Rgba<u8>, _>::from_raw(
        screenshot.width() as u32,
        screenshot.height() as u32,
        data_vec,
    );
    if image_buf_opt.is_none() {
        return String::new();
    }
    let dyn_img = DynamicImage::ImageRgba8(image_buf_opt.unwrap());
    let cropped_img = crop_imm(&dyn_img, x1, y1, crop_width, crop_height).to_image();
    let dynamic_cropped = DynamicImage::ImageRgba8(cropped_img);
    let processed = white_invert_image(&dynamic_cropped);
    let temp_filename = "temp_start_order.png";
    if processed.save(&temp_filename).is_err() {
        return String::new();
    }
    let output = Command::new(r"C:\Program Files\Tesseract-OCR\tesseract.exe")
        .arg(&temp_filename)
        .arg("stdout")
        .arg("-l")
        .arg("eng")
        .arg("--psm")
        .arg("7")
        .output();
    if let Ok(output) = output {
        if output.status.success() {
            let raw_text = String::from_utf8_lossy(&output.stdout).into_owned();
            return sanitize_ocr_text(raw_text.trim());
        }
    }
    String::new()
}

/// Az egyetlen közös függvény a main region szövegére.
pub fn check_main_region_text(screen_width: u32, screen_height: u32, is_red_button: bool) -> String {
    let x1 = (589.799 / 677.292 * screen_width as f64).floor() as u32;
    let x2 = (665.0 / 677.292 * screen_width as f64).floor() as u32;
    let y1 = (328.906 / 381.287 * screen_height as f64).floor() as u32;
    let y2 = (341.871 / 381.287 * screen_height as f64).floor() as u32;
    let crop_width = x2.saturating_sub(x1);
    let crop_height = y2.saturating_sub(y1);

    let screenshot = match get_screenshot(0) {
        Ok(scn) => scn,
        Err(_) => return String::new(),
    };
    if x2 > screenshot.width() as u32 || y2 > screenshot.height() as u32 {
        return String::new();
    }
    let data_vec = unsafe {
        slice::from_raw_parts(screenshot.raw_data(), screenshot.raw_len()).to_vec()
    };
    let image_buf_opt = ImageBuffer::<Rgba<u8>, _>::from_raw(
        screenshot.width() as u32,
        screenshot.height() as u32,
        data_vec,
    );
    if image_buf_opt.is_none() {
        return String::new();
    }
    let dyn_img = DynamicImage::ImageRgba8(image_buf_opt.unwrap());
    // Képernyőkép lekérése, kivágás stb.
    let cropped = crop_imm(&dyn_img, x1, y1, crop_width, crop_height).to_image();

    let processed = if is_red_button {
        // A red button esetén a white invertet használjuk
        white_invert_image(&DynamicImage::ImageRgba8(cropped))
    } else {
        // Nem red esetben a normál preprocess_image-t
        preprocess_image(&DynamicImage::ImageRgba8(cropped))
    };

    // DEBUG: mentés ellenőrzés céljából
    let debug_filename = if is_red_button {
        "debug_main_red.png"
    } else {
        "debug_main_nonred.png"
    };
    if let Err(e) = processed.save(debug_filename) {
        tracing::error!("Error saving debug main region image: {:?}", e);
    }

    // Tesseract hívás
    let temp_filename = "temp_main_region.png";
    if processed.save(&temp_filename).is_err() {
        return String::new();
    }
    let output = Command::new(r"C:\Program Files\Tesseract-OCR\tesseract.exe")
        .arg(&temp_filename)
        .arg("stdout")
        .arg("-l")
        .arg("eng")
        .arg("--psm")
        .arg("7")
        .output();
    if let Ok(output) = output {
        if output.status.success() {
            let raw_text = String::from_utf8_lossy(&output.stdout).into_owned();
            return sanitize_ocr_text(raw_text.trim());
        }
    }
    String::new()
}

/// Olvassa a creature név szövegét az adott pozíció alapján.
///
/// A függvény a következő lépéseket hajtja végre:
/// 1. Lekéri a teljes képernyős képet.
/// 2. Ellenőrzi, hogy a megadott OCR koordináták érvényesek-e az aktuális screenshot-on.
/// 3. Kivágja azt a képrészletet, amelyet a CreaturePosition mezői (ocr_x1, ocr_y1, ocr_x2, ocr_y2) határoznak meg.
/// 4. Az előfeldolgozás során a preprocess_image függvényt hívja (amely grayscale, kontraszt növelést, thresholdolást és upscaling-et végez).
/// 5. Elment egy ideiglenes képfájlt (például "temp_creature.png").
/// 6. Tesseract-ot hívja meg a temp_creature.png állomány feldolgozására, majd az eredményt a sanitize_ocr_text segítségével tisztítja meg.
/// 7. Visszaadja az így kapott, OCR által kiolvasott szöveget.
pub fn read_creature_text(pos: CreaturePosition, index: usize, is_opponent: bool, _screen_width: u32, _screen_height: u32) -> String {
    // 1. Capture full‐screen
    let screenshot = match get_screenshot(0) {
        Ok(scn) => scn,
        Err(e) => {
            tracing::error!("Screenshot hiba: {:?}", e);
            return String::new();
        }
    };
    tracing::info!("Screenshot mérete: {}x{}", screenshot.width(), screenshot.height());

    // 2. Bounds check
    if pos.ocr_x2 > screenshot.width() as u32 || pos.ocr_y2 > screenshot.height() as u32 {
        tracing::error!(
            "OCR koordináták kívül esnek a képen: pos=({}, {}, {}, {}), screenshot = {}x{}",
            pos.ocr_x1, pos.ocr_y1, pos.ocr_x2, pos.ocr_y2,
            screenshot.width(), screenshot.height()
        );
        return String::new();
    }

    // 3. Build image buffer
    let raw = unsafe { slice::from_raw_parts(screenshot.raw_data(), screenshot.raw_len()).to_vec() };
    let image_buf = match ImageBuffer::<Rgba<u8>, _>::from_raw(
        screenshot.width() as u32,
        screenshot.height() as u32,
        raw,
    ) {
        Some(buf) => buf,
        None => {
            tracing::error!("Hiba az image buffer létrehozásánál");
            return String::new();
        }
    };
    let dyn_img = DynamicImage::ImageRgba8(image_buf);

    // 4. Crop to OCR region
    let w = pos.ocr_x2.saturating_sub(pos.ocr_x1);
    let h = pos.ocr_y2.saturating_sub(pos.ocr_y1);
    tracing::info!("Kivágás: x1={}, y1={}, width={}, height={}", pos.ocr_x1, pos.ocr_y1, w, h);
    if w == 0 || h == 0 {
        tracing::error!("Érvénytelen kivágási méret: {}x{}", w, h);
        return String::new();
    }
    let cropped = crop_imm(&dyn_img, pos.ocr_x1, pos.ocr_y1, w, h).to_image();

    // 5. Preprocess pipeline
    let processed = preprocess_image(&DynamicImage::ImageRgba8(cropped));

    // 6. Save a single stable temp file for Tesseract
    let parity = if index % 2 == 1 { "odd" } else { "even" };
    let side   = if is_opponent      { "opponents_creature" } else { "creature" };
    let temp_filename = format!("temp_{}_{}_{}.png", parity, side, index);
    if let Err(e) = processed.save(&temp_filename) {
        tracing::error!("Hiba a temp creature kép mentésekor: {:?}", e);
        return String::new();
    }
    tracing::info!("Temp creature kép mentve: {}", temp_filename);

    // 7. OCR via Tesseract
    let output = Command::new(r"C:\Program Files\Tesseract-OCR\tesseract.exe")
        .arg(&temp_filename)
        .arg("stdout")
        .arg("-l").arg("eng")
        .arg("--psm").arg("7")
        .output();

    match output {
        Ok(o) if o.status.success() => {
            let raw = String::from_utf8_lossy(&o.stdout);
            sanitize_ocr_text(raw.trim())
        }
        Ok(o) => {
            tracing::error!("Tesseract hiba a {} fájlnál: {}",
                           temp_filename,
                           String::from_utf8_lossy(&o.stderr));
            String::new()
        }
        Err(e) => {
            tracing::error!("Tesseract futtatási hiba: {:?}", e);
            String::new()
        }
    }
}

