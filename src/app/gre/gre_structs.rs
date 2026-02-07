// src/app/gre/gre_structs.rs

use crate::app::card_attribute::{Condition, Effect};
use crate::app::card_library::ManaCost;
use crate::app::game_state::GamePhase;
use std::fmt;

/// Aktivált képesség struktúrája
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ActivatedAbility {
    pub cost: ManaCost,
    pub condition: Condition,
    pub effect: Effect,
    pub activated_this_turn: bool,
    pub loyalty_change: i32,
}

/// Késleltetett effekt
#[derive(Debug, Clone)]
pub struct DelayedEffect {
    pub effect: Effect,
    pub execute_phase: GamePhase,
    pub id: usize,
    pub depends_on: Vec<usize>,
}

/// ReplacementEffect a replacement effectekhez.
/// Itt a beépített Debug és Clone implementációk speciálisak a closure miatt.
pub struct ReplacementEffect {
    pub priority: u8,
    pub f: Box<dyn Fn(&Effect) -> Option<Vec<Effect>>>,
}

// Debug: ne próbáljuk a closure-t formázni
impl fmt::Debug for ReplacementEffect {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ReplacementEffect")
            .field("priority", &self.priority)
            .finish()
    }
}

// Ha clone-olni akarjuk, pánikol
impl Clone for ReplacementEffect {
    fn clone(&self) -> Self {
        panic!("ReplacementEffect is not clonable in a simple way!");
    }
}