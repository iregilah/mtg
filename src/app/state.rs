// app/state.rs

use crate::app::game_state::GamePhase;

pub mod start_state;
pub mod mulligan_state;
pub mod first_main_phase_state;
pub mod attack_phase_state;
pub mod second_main_phase_state;
pub mod submit_state;
pub mod opponents_turn_state;
pub mod combat_damage_state;


pub trait State<AppError> {
    fn update(&mut self, bot: &mut crate::app::bot::Bot) -> Result<(), AppError>;
    fn next(&mut self) -> Box<dyn State<AppError>>;
    fn phase(&self) -> GamePhase;
}
