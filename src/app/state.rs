// app/state.rs

pub mod start_state;
pub mod mulligan_state;
pub mod first_main_phase_state;
pub mod attack_phase_state;
pub mod second_main_phase_state;
pub mod submit_state;
pub mod opponents_turn_state;

pub trait State {
    fn update(&mut self, bot: &mut crate::app::bot::Bot);
    fn next(&mut self) -> Box<dyn State>;
}
