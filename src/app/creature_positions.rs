// app/creature_positions.rs

/// Bounds for creature OCR and click areas.
#[derive(Debug, Clone)]
pub struct CreaturePosition {
    pub ocr_x1: u32,
    pub ocr_x2: u32,
    pub ocr_y1: u32,
    pub ocr_y2: u32,
    pub click_x1: u32,
    pub click_y1: u32,
    pub click_x2: u32,
    pub click_y2: u32,
}

/// Compute creature positions for given count, screen size, and side.
fn get_creature_positions(
    creature_count: usize,
    screen_width: f64,
    screen_height: f64,
    is_opponent: bool,
) -> Vec<CreaturePosition> {
    // x koordináták (OCR és kattintási terület – ugyanolyan értékek az adott card esetén)
    // Az értékeket úgy adtuk meg, hogy az eredeti számok: 
    // pl. 311.284/677.292, 362.279/677.292, stb.
    let x_positions: Vec<(f64, f64, f64, f64)> = match creature_count {
        1 => vec![
            (311.284, 362.279, 311.284, 362.279)
        ],
        2 => vec![
            (276.777, 327.490, 276.777, 327.490),
            (345.801, 398.920, 345.801, 398.920)
        ],
        3 => vec![
            (242.343, 294.686, 242.343, 294.686),
            (311.284, 362.279, 311.284, 362.279),
            (380.432, 433.821, 380.432, 433.821)
        ],
        4 => vec![
            (207.994, 257.994, 207.994, 257.994),
            (276.777, 327.490, 276.777, 327.490),
            (345.801, 398.920, 345.801, 398.920),
            (414.707, 464.707, 414.707, 464.707)
        ],
        5 => vec![
            (173.569, 223.569, 173.569, 223.569),
            (242.343, 294.686, 242.343, 294.686),
            (311.284, 362.279, 311.284, 362.279),
            (380.432, 433.821, 380.432, 433.821),
            (449.238, 499.238, 449.238, 499.238)
        ],
        6 => vec![
            (138.984, 188.984, 138.984, 188.984),
            (207.994, 257.994, 207.994, 257.994),
            (276.777, 327.490, 276.777, 327.490),
            (345.801, 398.920, 345.801, 398.920),
            (414.707, 464.707, 414.707, 464.707),
            (483.615, 533.615, 483.615, 533.615)
        ],
        7 => vec![
            (104.555, 154.555, 104.555, 154.555),
            (173.569, 223.569, 173.569, 223.569),
            (242.343, 294.686, 242.343, 294.686),
            (311.284, 362.279, 311.284, 362.279),
            (380.432, 433.821, 380.432, 433.821),
            (449.238, 499.238, 449.238, 499.238),
            (518.099, 568.099, 518.099, 568.099)
        ],
        8 => {
            // A speciális eset: a 8 creature esetén az első és utolsó eltér, a középső 6 pedig megegyezik a 6-os koordinátákkal.
            let mut arr = Vec::new();
            arr.push((69.815, 119.815, 69.815, 119.815));
            // A 6-os creature koordináták (az eredeti 6 creature-es pozíciók)
            let inner6 = vec![
                (138.984, 188.984, 138.984, 188.984),
                (207.994, 257.994, 207.994, 257.994),
                (276.777, 327.490, 276.777, 327.490),
                (345.801, 398.920, 345.801, 398.920),
                (414.707, 464.707, 414.707, 464.707),
                (483.615, 533.615, 483.615, 533.615)
            ];
            arr.extend(inner6);
            arr.push((552.621, 602.621, 552.621, 602.621));
            arr
        },
        _ => {
            eprintln!("Unsupported creature count: {}", creature_count);
            vec![]
        }
    };

    // A vertikális (y) koordinátákhoz csak két esetünk van:
    // – Saját creature: OCR y: 185.141 és 188.731; kattintási y: 185.141 és 235.141.
    // – Ellenfél creature: OCR y: 101.761 és 104.891; kattintási y: 101.761 és 151.761.
    let (ocr_y1_norm, ocr_y2_norm, click_y1_norm, click_y2_norm) = if is_opponent {
        (101.761, 104.891, 101.761, 151.761)
    } else {
        (185.141, 188.731, 185.141, 235.141)
    };

    // Számoljuk ki a végső koordinátákat az adott képernyőméretek alapján.
    // Az x koordinátákat a 677.292-es alapérték, az y koordinátákat a 381.287-es érték szerint skálázzuk.
    let mut positions = Vec::new();
    for &(ocr_x1, ocr_x2, click_x1, click_x2) in &x_positions {
        positions.push(CreaturePosition {
            ocr_x1: ((ocr_x1 / 677.292) * screen_width).ceil() as u32,
            ocr_x2: ((ocr_x2 / 677.292) * screen_width).ceil() as u32,
            click_x1: ((click_x1 / 677.292) * screen_width).ceil() as u32,
            click_x2: ((click_x2 / 677.292) * screen_width).ceil() as u32,
            ocr_y1: ((ocr_y1_norm / 381.287) * screen_height).ceil() as u32,
            ocr_y2: ((ocr_y2_norm / 381.287) * screen_height).ceil() as u32,
            click_y1: ((click_y1_norm / 381.287) * screen_height).ceil() as u32,
            click_y2: ((click_y2_norm / 381.287) * screen_height).ceil() as u32,
        });
    }
    positions
}

/// Publikus függvény: visszaadja a saját (player) creature–jeinek pozícióit.
pub fn get_own_creature_positions(
    creature_count: usize,
    screen_width: u32,
    screen_height: u32,
) -> Vec<CreaturePosition> {
    get_creature_positions(creature_count, screen_width as f64, screen_height as f64, false)
}

/// Publikus függvény: visszaadja az ellenfél creature–jeinek pozícióit.
pub fn get_opponent_creature_positions(
    creature_count: usize,
    screen_width: u32,
    screen_height: u32,
) -> Vec<CreaturePosition> {
    get_creature_positions(creature_count, screen_width as f64, screen_height as f64, true)
}
