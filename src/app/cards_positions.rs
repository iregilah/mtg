// card_library.rs
// Ez a modul tartalmazza a kártya pozíciókat leíró struktúrát és a képernyő szélessége alapján számoló függvényt.

/// --- Kártya pozíciók ---
/// - `hover_x`: azt a vízszintes koordinátát adja meg, ahová az egérmutatót kell mozgatni a tooltip megjelenítéséhez.
/// - `ocr_x1` és `ocr_x2`: az OCR számára azt a vízszintes intervallumot határozzák meg, ahol a kártyanév látható.
#[derive(Debug, Copy, Clone)]
pub struct CardPosition {
    pub hover_x: u32,
    pub ocr_x1: u32,
    pub ocr_x2: u32,
}



/// Számolja ki a kártya pozíciókat a képernyő szélessége alapján.
/// A relatív értékek a 677.292-es alapértékhez képest vannak megadva.
pub fn get_card_positions(card_count: usize, screen_width: u32) -> Vec<CardPosition> {
    let factor = screen_width as f64;
    match card_count {
        1 => vec![
            CardPosition {
                hover_x: (339.565 / 677.292 * factor).ceil() as u32,
                ocr_x1: (289.565 / 677.292 * factor).ceil() as u32,
                ocr_x2: (369.565 / 677.292 * factor).ceil() as u32,
            }
        ],
        2 => vec![
            CardPosition {
                hover_x: (308.755 / 677.292 * factor).ceil() as u32,
                ocr_x1: (258.755 / 677.292 * factor).ceil() as u32,
                ocr_x2: (338.755 / 677.292 * factor).ceil() as u32,
            },
            CardPosition {
                hover_x: (371.089 / 677.292 * factor).ceil() as u32,
                ocr_x1: (321.089 / 677.292 * factor).ceil() as u32,
                ocr_x2: (401.089 / 677.292 * factor).ceil() as u32,
            },
        ],
        3 => vec![
            CardPosition {
                hover_x: (277.547 / 677.292 * factor).ceil() as u32,
                ocr_x1: (227.547 / 677.292 * factor).ceil() as u32,
                ocr_x2: (307.547 / 677.292 * factor).ceil() as u32,
            },
            CardPosition {
                hover_x: (340.011 / 677.292 * factor).ceil() as u32,
                ocr_x1: (290.011 / 677.292 * factor).ceil() as u32,
                ocr_x2: (370.011 / 677.292 * factor).ceil() as u32,
            },
            CardPosition {
                hover_x: (402.428 / 677.292 * factor).ceil() as u32,
                ocr_x1: (352.428 / 677.292 * factor).ceil() as u32,
                ocr_x2: (432.428 / 677.292 * factor).ceil() as u32,
            },
        ],
        4 => vec![
            CardPosition {
                hover_x: (246.537 / 677.292 * factor).ceil() as u32,
                ocr_x1: (196.537 / 677.292 * factor).ceil() as u32,
                ocr_x2: (276.537 / 677.292 * factor).ceil() as u32,
            },
            CardPosition {
                hover_x: (309.118 / 677.292 * factor).ceil() as u32,
                ocr_x1: (259.118 / 677.292 * factor).ceil() as u32,
                ocr_x2: (339.118 / 677.292 * factor).ceil() as u32,
            },
            CardPosition {
                hover_x: (371.231 / 677.292 * factor).ceil() as u32,
                ocr_x1: (321.231 / 677.292 * factor).ceil() as u32,
                ocr_x2: (401.231 / 677.292 * factor).ceil() as u32,
            },
            CardPosition {
                hover_x: (433.435 / 677.292 * factor).ceil() as u32,
                ocr_x1: (383.435 / 677.292 * factor).ceil() as u32,
                ocr_x2: (463.435 / 677.292 * factor).ceil() as u32,
            },
        ],
        5 => vec![
            CardPosition {
                hover_x: (215.489 / 677.292 * factor).ceil() as u32,
                ocr_x1: (165.489 / 677.292 * factor).ceil() as u32,
                ocr_x2: (245.489 / 677.292 * factor).ceil() as u32,
            },
            CardPosition {
                hover_x: (277.376 / 677.292 * factor).ceil() as u32,
                ocr_x1: (227.376 / 677.292 * factor).ceil() as u32,
                ocr_x2: (307.376 / 677.292 * factor).ceil() as u32,
            },
            CardPosition {
                hover_x: (340.174 / 677.292 * factor).ceil() as u32,
                ocr_x1: (290.174 / 677.292 * factor).ceil() as u32,
                ocr_x2: (370.174 / 677.292 * factor).ceil() as u32,
            },
            CardPosition {
                hover_x: (402.464 / 677.292 * factor).ceil() as u32,
                ocr_x1: (352.464 / 677.292 * factor).ceil() as u32,
                ocr_x2: (432.464 / 677.292 * factor).ceil() as u32,
            },
            CardPosition {
                hover_x: (464.437 / 677.292 * factor).ceil() as u32,
                ocr_x1: (414.437 / 677.292 * factor).ceil() as u32,
                ocr_x2: (494.437 / 677.292 * factor).ceil() as u32,
            },
        ],
        6 => vec![
            CardPosition {
                hover_x: (184.912 / 677.292 * factor).ceil() as u32,
                ocr_x1: (134.912 / 677.292 * factor).ceil() as u32,
                ocr_x2: (214.912 / 677.292 * factor).ceil() as u32,
            },
            CardPosition {
                hover_x: (246.453 / 677.292 * factor).ceil() as u32,
                ocr_x1: (196.453 / 677.292 * factor).ceil() as u32,
                ocr_x2: (276.453 / 677.292 * factor).ceil() as u32,
            },
            CardPosition {
                hover_x: (308.416 / 677.292 * factor).ceil() as u32,
                ocr_x1: (258.416 / 677.292 * factor).ceil() as u32,
                ocr_x2: (338.416 / 677.292 * factor).ceil() as u32,
            },
            CardPosition {
                hover_x: (371.116 / 677.292 * factor).ceil() as u32,
                ocr_x1: (321.116 / 677.292 * factor).ceil() as u32,
                ocr_x2: (401.116 / 677.292 * factor).ceil() as u32,
            },
            CardPosition {
                hover_x: (433.416 / 677.292 * factor).ceil() as u32,
                ocr_x1: (383.416 / 677.292 * factor).ceil() as u32,
                ocr_x2: (463.416 / 677.292 * factor).ceil() as u32,
            },
            CardPosition {
                hover_x: (495.267 / 677.292 * factor).ceil() as u32,
                ocr_x1: (445.267 / 677.292 * factor).ceil() as u32,
                ocr_x2: (525.267 / 677.292 * factor).ceil() as u32,
            },
        ],
        7 => vec![
            CardPosition {
                hover_x: (154.345 / 677.292 * factor).ceil() as u32,
                ocr_x1: (104.345 / 677.292 * factor).ceil() as u32,
                ocr_x2: (184.345 / 677.292 * factor).ceil() as u32,
            },
            CardPosition {
                hover_x: (215.364 / 677.292 * factor).ceil() as u32,
                ocr_x1: (165.364 / 677.292 * factor).ceil() as u32,
                ocr_x2: (245.364 / 677.292 * factor).ceil() as u32,
            },
            CardPosition {
                hover_x: (277.277 / 677.292 * factor).ceil() as u32,
                ocr_x1: (227.277 / 677.292 * factor).ceil() as u32,
                ocr_x2: (307.277 / 677.292 * factor).ceil() as u32,
            },
            CardPosition {
                hover_x: (339.490 / 677.292 * factor).ceil() as u32,
                ocr_x1: (289.490 / 677.292 * factor).ceil() as u32,
                ocr_x2: (369.490 / 677.292 * factor).ceil() as u32,
            },
            CardPosition {
                hover_x: (402.492 / 677.292 * factor).ceil() as u32,
                ocr_x1: (352.492 / 677.292 * factor).ceil() as u32,
                ocr_x2: (432.492 / 677.292 * factor).ceil() as u32,
            },
            CardPosition {
                hover_x: (464.539 / 677.292 * factor).ceil() as u32,
                ocr_x1: (414.539 / 677.292 * factor).ceil() as u32,
                ocr_x2: (494.539 / 677.292 * factor).ceil() as u32,
            },
            CardPosition {
                hover_x: (525.680 / 677.292 * factor).ceil() as u32,
                ocr_x1: (475.680 / 677.292 * factor).ceil() as u32,
                ocr_x2: (555.680 / 677.292 * factor).ceil() as u32,
            },
        ],
        8 => vec![
            CardPosition {
                hover_x: (133.768 / 677.292 * factor).ceil() as u32,
                ocr_x1: (83.768 / 677.292 * factor).ceil() as u32,
                ocr_x2: (163.768 / 677.292 * factor).ceil() as u32,
            },
            CardPosition {
                hover_x: (191.709 / 677.292 * factor).ceil() as u32,
                ocr_x1: (141.709 / 677.292 * factor).ceil() as u32,
                ocr_x2: (221.709 / 677.292 * factor).ceil() as u32,
            },
            CardPosition {
                hover_x: (251.061 / 677.292 * factor).ceil() as u32,
                ocr_x1: (201.061 / 677.292 * factor).ceil() as u32,
                ocr_x2: (281.061 / 677.292 * factor).ceil() as u32,
            },
            CardPosition {
                hover_x: (310.147 / 677.292 * factor).ceil() as u32,
                ocr_x1: (260.147 / 677.292 * factor).ceil() as u32,
                ocr_x2: (340.147 / 677.292 * factor).ceil() as u32,
            },
            CardPosition {
                hover_x: (369.688 / 677.292 * factor).ceil() as u32,
                ocr_x1: (319.688 / 677.292 * factor).ceil() as u32,
                ocr_x2: (399.688 / 677.292 * factor).ceil() as u32,
            },
            CardPosition {
                hover_x: (428.619 / 677.292 * factor).ceil() as u32,
                ocr_x1: (378.619 / 677.292 * factor).ceil() as u32,
                ocr_x2: (458.619 / 677.292 * factor).ceil() as u32,
            },
            CardPosition {
                hover_x: (487.944 / 677.292 * factor).ceil() as u32,
                ocr_x1: (437.944 / 677.292 * factor).ceil() as u32,
                ocr_x2: (517.944 / 677.292 * factor).ceil() as u32,
            },
            CardPosition {
                hover_x: (545.972 / 677.292 * factor).ceil() as u32,
                ocr_x1: (495.972 / 677.292 * factor).ceil() as u32,
                ocr_x2: (575.972 / 677.292 * factor).ceil() as u32,
            },
        ],
        _ => {
            tracing::warn!("Unsupported card count: {}", card_count);
            vec![]
        }
    }
}
