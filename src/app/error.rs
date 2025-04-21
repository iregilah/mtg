// app/error.rs

use thiserror::Error;

#[derive(Debug, Error)]
pub enum AppError {
    #[error("OCR hiba: {0}")]
    OcrError(String),

    #[error("Nem található a kártya a könyvtárban: {0}")]
    CardNotFound(String),

    #[error("Nem elég mana: szükséges {required} (színes: {colored}, színtelen: {colorless}), elérhető: {available_colored} színes, {available_colorless} színtelen")]
    InsufficientMana {
        required: u32,
        colored: u32,
        colorless: u32,
        available_colored: u32,
        available_colorless: u32,
    },

    #[error("Állapotváltás nem várt állapotból: {0}")]
    InvalidStateTransition(String),

    #[error("Egyéb hiba: {0}")]
    Other(String),
}