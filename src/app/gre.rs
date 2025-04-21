// app/gre.rs

use std::collections::VecDeque;
use crate::app::game_state::{GameEvent, GamePhase};
use crate::app::card_attribute::{Effect, Trigger};
use crate::app::card_library::Card;
use tracing::info;
use crate::app::game_state::Player;

/// An entry on the stack: spells, triggered or activated abilities.
#[derive(Debug, Clone)]
pub enum StackEntry {
    Spell { card: Card, controller: Player },
    TriggeredAbility { source: Option<Card>, effect: Effect, controller: Player },
    ActivatedAbility { source: Card, effect: Effect, controller: Player },
}

/// A delayed effect scheduled for a future phase
#[derive(Debug, Clone)]
pub struct DelayedEffect {
    pub effect: Effect,
    pub execute_phase: GamePhase,
    pub id: usize,
    pub depends_on: Vec<usize>,
}

pub struct Gre {
    /// The stack of spells and abilities.
    pub stack: VecDeque<StackEntry>,
    /// Scheduled delayed effects.
    pub delayed: Vec<DelayedEffect>,
    /// ID generator for delayed effects.
    pub next_id: usize,
    /// Who currently has priority.
    pub priority: Player,
    /// How many consecutive passes have occurred.
    pub passes: u8,
    /// Replacement effects.
    pub replacement_effects: Vec<Box<dyn Fn(&Effect) -> Option<Vec<Effect>>>>,
    /// Continuous effects.
    pub continuous_effects: Vec<Box<dyn Fn(&mut Effect)>>,
}

impl Default for Gre {
    fn default() -> Self {
        Gre::new(Player::Us)
    }
}

impl Gre {
    /// Initialize the GRE, starting with the given player on priority.
    pub fn new(starting_player: Player) -> Self {
        Self {
            stack: VecDeque::new(),
            delayed: Vec::new(),
            next_id: 0,
            priority: starting_player,
            passes: 0,
            replacement_effects: Vec::new(),
            continuous_effects: Vec::new(),
        }
    }

    /// Cast a spell: put it on the stack and reset pass count.
    pub fn cast_spell(&mut self, card: Card, controller: Player) {
        info!("{:?} casts {}", controller, card.name);
        self.stack.push_back(StackEntry::Spell { card, controller });
        self.reset_priority();
    }

    /// Schedule an effect for later execution in a specified phase, with optional dependencies.
    pub fn schedule_delayed(&mut self, effect: Effect, phase: GamePhase, depends_on: Vec<usize>) -> usize {
        let id = self.next_id;
        self.next_id += 1;
        self.delayed.push(DelayedEffect { effect, execute_phase: phase, id, depends_on });
        id
    }

    /// Dispatch delayed effects whose scheduled phase matches the current phase
    /// and whose dependencies have been met.
    pub fn dispatch_delayed(&mut self, current_phase: GamePhase) {
        let mut ready = Vec::new();
        let mut still = Vec::new();
        // IDs of effects scheduled for this phase
        let due_ids: Vec<usize> = self.delayed.iter()
            .filter(|d| d.execute_phase == current_phase)
            .map(|d| d.id)
            .collect();

        for effect in self.delayed.drain(..) {
            if effect.execute_phase == current_phase
                && effect.depends_on.iter().all(|dep| due_ids.contains(dep)) {
                ready.push(effect);
            } else {
                still.push(effect);
            }
        }
        self.delayed = still;

        for d in ready {
            info!("Dispatching delayed effect {} at {:?}", d.id, current_phase);
            self.queue_action(d.effect);
        }
    }

    /// Trigger abilities in response to a game event, grouping multiple triggers atomically.
    pub fn trigger_event(&mut self, event: GameEvent, battlefield: &mut Vec<Card>, controller: Player) {
        info!("Firing event: {:?}", event);
        let mut batch = Vec::new();
        for card in battlefield.iter_mut() {
            let effects = match &event {
                GameEvent::SpellResolved(_) => card.trigger_by(&Trigger::Custom("OnCastResolved".into())),
                GameEvent::CreatureDied(_) => card.trigger_by(&Trigger::OnDeath),
                GameEvent::TurnEnded     => card.trigger_by(&Trigger::EndOfTurn),
                GameEvent::Custom(s)     => card.trigger_by(&Trigger::Custom(s.clone())),
                GameEvent::PhaseChange(_) => Vec::new(),
            };
            batch.extend(effects);
        }
        self.reset_priority();
        for eff in batch {
            match &eff {
                Effect::Delayed{ effect, phase, deps } => {
                    let id = self.schedule_delayed(*effect.clone(), *phase, deps.clone());
                    info!("Scheduled delayed effect id {}", id);
                }
                _ => {
                    self.stack.push_back(StackEntry::TriggeredAbility { source: None, effect: eff, controller });
                }
            }
        }
    }

    /// Player passes priority. After two passes, resolve the top of the stack.
    pub fn pass_priority(&mut self) {
        self.passes += 1;
        if self.passes >= 2 {
            self.resolve_top_of_stack();
            self.reset_priority();
        } else {
            self.priority = self.priority.opponent();
            info!("Priority passed to {:?}", self.priority);
        }
    }

    /// Reset consecutive pass count without changing priority.
    fn reset_priority(&mut self) {
        self.passes = 0;
    }

    /// Resolve the top object on the stack.
    fn resolve_top_of_stack(&mut self) {
        if let Some(entry) = self.stack.pop_back() {
            info!("Resolving {:?}", entry);
            match entry {
                StackEntry::Spell { card, controller } => {
                    info!("Resolving spell: {}", card.name);
                    let mut battlefield = Vec::new();
                    self.trigger_event(GameEvent::SpellResolved(card.name.clone()), &mut battlefield, controller);
                }
                StackEntry::TriggeredAbility { source: _, effect, controller: _ }
                | StackEntry::ActivatedAbility { source: _, effect, controller: _ } => {
                    self.handle_effect(effect);
                }
            }
        }
    }

    /// Handle an effect: apply replacement, continuous modifications, then execute.
    pub fn handle_effect(&mut self, effect: Effect) {
        let mut to_execute = if let Some(replaced) = self.replacement_effects.iter()
            .find_map(|rep| rep(&effect)) {
            replaced
        } else {
            vec![effect]
        };
        for cont in &self.continuous_effects {
            for e in &mut to_execute {
                cont(e);
            }
        }
        for e in to_execute {
            self.execute(e);
        }
    }

    /// Immediately execute or queue an effect without stacking.
    pub fn queue_action(&mut self, effect: Effect) {
        self.handle_effect(effect);
    }

    /// Resolve all remaining stack entries (e.g., at phase end).
    pub fn resolve_stack(&mut self) {
        while !self.stack.is_empty() {
            self.resolve_top_of_stack();
        }
    }

    /// Register a replacement effect (e.g., replace destroy with exile).
    pub fn add_replacement_effect<F>(&mut self, effect: F)
    where
        F: 'static + Fn(&Effect) -> Option<Vec<Effect>>
    {
        self.replacement_effects.push(Box::new(effect));
    }

    /// Register a continuous effect (e.g., damage modification).
    pub fn add_continuous_effect<F>(&mut self, effect: F)
    where
        F: 'static + Fn(&mut Effect)
    {
        self.continuous_effects.push(Box::new(effect));
    }

    /// The raw execution of an effect: mutate game state here.
    fn execute(&self, effect: Effect) {
        info!("Executing effect: {:?}", effect);
        // TODO: implement state mutation
    }

    /// Add an entry to the stack as an activated ability, resetting priority.
    pub fn push_to_stack(&mut self, entry: StackEntry) {
        self.stack.push_back(entry);
        self.reset_priority();
    }
}
