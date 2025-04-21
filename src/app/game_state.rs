// app/game_state.rs

use crate::app::card_library::Card;
use crate::app::card_attribute::Effect;
use tracing::{error, info, warn};

#[derive(Debug, Default)]
pub struct GameState {
    pub hand: Vec<Card>,
    pub battlefield: Vec<Card>,
    pub opponent_battlefield: Vec<Card>,
    pub graveyard: Vec<Card>,
    pub opponent_graveyard: Vec<Card>,
    pub exile: Vec<Card>,
    pub opponent_exile: Vec<Card>,
    pub library_count: usize,
    pub opponent_library_count: usize,
    pub life_total: i32,
    pub opponent_life_total: i32,
    pub mana_available: u32,
    pub land_played_this_turn: bool,
    pub stack: Vec<StackEntry>,
}
#[derive(Debug, Clone)]
pub enum GameEvent {
    SpellResolved(String),
    CreatureDied(String),
    TurnEnded,
    Custom(String),
    PhaseChange(GamePhase),
}

// A GamePhase enum a játék belső fázisait reprezentálja, függetlenül attól,
// hogy a bot épp melyik UI-state-ben van – így tudunk szabályosan időzített effektusokat kezelni.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GamePhase {
    Beginning,
    PreCombatMain,
    Combat,
    PostCombatMain,
    End,
}
#[derive(Debug, Clone)]
pub enum StackEntry {
    Spell { card: Card, controller: Player },
    TriggeredAbility { source: Card, effect: Effect, controller: Player },
    ActivatedAbility { source: Card, effect: Effect, controller: Player },
}
//TODO valószínűleg hiba
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Player { Us, Opponent }

impl Player {
    /// Returns the opposing player.
    pub fn opponent(self) -> Self {
        match self {
            Player::Us => Player::Opponent,
            Player::Opponent => Player::Us,
        }
    }
}

#[derive(Debug)]
pub enum GameAction {
    PlayLand(usize),
    CastSpell(usize),
    AttackWith(Vec<usize>),
    ActivateAbility{card_idx:usize,ability_idx:usize},
    PassPriority,
}

pub trait Strategy {
    fn decide(&mut self, state:&GameState)->GameAction;
}

pub struct SimpleHeuristic;
impl Strategy for SimpleHeuristic {
    fn decide(&mut self, state:&GameState)->GameAction {
        // play land if possible
        if !state.land_played_this_turn {
            if let Some(i) = state.hand.iter().position(|c| matches!(c.card_type, crate::app::card_library::CardType::Land)) {
                return GameAction::PlayLand(i);
            }
        }
        // cast first affordable
        for (i,c) in state.hand.iter().enumerate() {
            let cost = c.mana_cost.total();
            if cost <= state.mana_available { return GameAction::CastSpell(i); }
        }
        GameAction::PassPriority
    }
}
