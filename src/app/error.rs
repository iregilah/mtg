// app/error.rs

use thiserror::Error;

#[derive(Debug, Error)]
pub enum AppError {
    #[error("OCR error: {0}")]
    OcrError(String),

    #[error("Card not found in library: {0}")]
    CardNotFound(String),

    #[error("Insufficient mana: required {required} (colored: {colored}, colorless: {colorless}), \
             available: {available_colored} colored, {available_colorless} colorless"
    )]
    InsufficientMana {
        required: u32,
        colored: u32,
        colorless: u32,
        available_colored: u32,
        available_colorless: u32,
    },

    #[error("Invalid state transition: {0}")]
    InvalidStateTransition(String),

    #[error("Other error: {0}")]
    Other(String),
}