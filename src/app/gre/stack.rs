// src/app/gre/stack.rs

use std::fmt;
use crate::app::game_state::Player;
use crate::app::card_library::Card;
use crate::app::gre::gre_structs::ActivatedAbility;

/// StackEntry: Spell, TriggeredAbility, ActivatedAbility
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StackEntry {
    Spell {
        card: Card,
        controller: Player,
        target_creature: Option<Card>,
    },
    TriggeredAbility {
        source: Option<Card>,
        effect: crate::app::card_attribute::Effect,
        controller: Player
    },
    ActivatedAbility {
        source: Card,
        ability: ActivatedAbility,
        controller: Player
    },
}

/// Prioritást is tartalmaz
#[derive(Debug, Clone, Eq)]
pub struct PriorityEntry {
    pub priority: u8,
    pub sequence: usize,
    pub entry: StackEntry,
}

impl Ord for PriorityEntry {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        // Nagyobb priority felül
        self.priority.cmp(&other.priority)
            .then_with(|| self.sequence.cmp(&other.sequence))
    }
}

impl PartialOrd for PriorityEntry {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl PartialEq for PriorityEntry {
    fn eq(&self, other: &Self) -> bool {
        self.priority == other.priority && self.sequence == other.sequence
    }
}