// app/game_state.rs

use crate::app::card_library::Card;
use crate::app::card_attribute::Effect;
use tracing::{error, info, warn};
use crate::app::card_library::build_card_library;
use crate::app::bot::Bot;
use crate::app::{game_state, gre};
pub use crate::app::gre::StackEntry;

#[derive(Debug, Default, Clone)]
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
    CreatureDied(Card),
    TurnEnded,
    Custom(String),
    PhaseChange(GamePhase),
    OnCombatDamage,
    Targeted(u64),
}

/// Internal game phases for effect timing
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GamePhase {
    Beginning,
    PreCombatMain,
    BeginningCombat,
    Combat,
    CombatDamage,
    PostCombatMain,
    End,
}
#[derive(Debug, PartialEq)]
pub enum GameResult {
    Win,
    Loss,
    Draw,
    Ongoing,
}

impl GameState {
    pub fn new() -> Self {
        Default::default()
    }


    pub fn is_game_over(&self) -> bool {
        self.life_total <= 0 || self.opponent_life_total <= 0
    }

    pub fn result(&self) -> GameResult {
        let us_dead = self.life_total <= 0;
        let opp_dead = self.opponent_life_total <= 0;
        match (us_dead, opp_dead) {
            (true, true) => GameResult::Draw,
            (false, true) => GameResult::Win,
            (true, false) => GameResult::Loss,
            _ => GameResult::Ongoing,
        }
    }
    pub fn goto_phase(&mut self, _phase: GamePhase) {
        // TODO
    }
    /// Pull fields from the Bot into the persistent GameState.
    pub fn update_from_bot(&mut self, bot: &Bot) {
        let library = build_card_library();

        // Update hand from OCR texts
        self.hand = bot.cards_texts.iter()
            .filter_map(|text| library.get(text).cloned())
            .collect();

        // Update battlefields
        self.battlefield = bot.battlefield_creatures.values().cloned().collect();
        self.opponent_battlefield = bot.battlefield_opponent_creatures.values().cloned().collect();

        // Update mana and land flags
        self.mana_available = bot.land_number;
        self.land_played_this_turn = bot.land_played_this_turn;

        // Update stack snapshot
        self.stack = bot.gre.stack
            .iter()
            .map(|pe| pe.entry.clone())
            .collect();

    }
}



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
