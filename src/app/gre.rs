use std::collections::{BinaryHeap, VecDeque, HashSet};
use crate::app::game_state::{GameEvent, GamePhase};
use crate::app::card_attribute::{Effect, Trigger};
use crate::app::card_library::Card;
use tracing::info;
use crate::app::game_state::Player;


/// Wrapper for prioritized stack entries, supports custom ordering.
#[derive(Debug, Clone, Eq)]
pub struct PriorityEntry {
    priority: u8,
    sequence: usize,
    entry: StackEntry,
}

impl Ord for PriorityEntry {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        // Higher priority first; if equal, higher sequence first (LIFO among same priority)
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
impl Eq for StackEntry {}
/// An entry on the stack: spells, triggered or activated abilities.
#[derive(Debug, Clone, PartialEq)]
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
    /// Priority queue for stack entries.
    pub stack: BinaryHeap<PriorityEntry>,
    /// Scheduled delayed effects.
    pub delayed: Vec<DelayedEffect>,
    /// IDs of delayed effects already dispatched.
    pub executed_delayed: HashSet<usize>,
    /// ID generator for delayed effects and sequence counter.
    pub next_id: usize,
    /// Sequence counter for stack ordering.
    pub sequence: usize,
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
            stack: BinaryHeap::new(),
            delayed: Vec::new(),
            executed_delayed: HashSet::new(),
            next_id: 0,
            sequence: 0,
            priority: starting_player,
            passes: 0,
            replacement_effects: Vec::new(),
            continuous_effects: Vec::new(),
        }
    }

    /// Cast a spell: put it on the stack with lowest priority and reset pass count.
    pub fn cast_spell(&mut self, card: Card, controller: Player) {
        info!("{:?} casts {}", controller, card.name);
        self.push(StackEntry::Spell { card, controller }, 0);
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
    /// and whose dependencies have been met, in scheduled order.
    pub fn dispatch_delayed(&mut self, current_phase: GamePhase) {
        let mut still = Vec::new();
        let mut ready: Vec<_> = self.delayed.drain(..)
            .filter(|d| {
                d.execute_phase == current_phase
                    && d.depends_on.iter().all(|dep| self.executed_delayed.contains(dep))
            })
            .collect();
        // Those not ready remain
        for d in ready.iter().chain(self.delayed.iter()) {
            if d.execute_phase != current_phase
                || !d.depends_on.iter().all(|dep| self.executed_delayed.contains(dep))
            {
                still.push(d.clone());
            }
        }
        self.delayed = still;
        // Queue ready effects in creation order
        ready.sort_by_key(|d| d.id);
        for d in ready {
            info!("Dispatching delayed effect {} at {:?}", d.id, current_phase);
            self.executed_delayed.insert(d.id);
            self.handle_effect(d.effect.clone());
        }
    }

    /// Trigger abilities in response to a game event, grouping multiple triggers atomically.
    pub fn trigger_event(&mut self, event: GameEvent, battlefield: &mut Vec<Card>, controller: Player) {
        info!("Firing event: {:?}", event);
        let mut batch = Vec::new();
        for card in battlefield.iter_mut() {
            let effects = match &event {
                GameEvent::SpellResolved(name) => card.trigger_by(&Trigger::Custom(format!("OnCastResolved:{}", name))),
                GameEvent::CreatureDied(_)    => card.trigger_by(&Trigger::OnDeath),
                GameEvent::TurnEnded          => card.trigger_by(&Trigger::EndOfTurn),
                GameEvent::Custom(s)          => card.trigger_by(&Trigger::Custom(s.clone())),
                GameEvent::PhaseChange(p)     => card.trigger_by(&Trigger::Custom(format!("PhaseChange:{:?}", p))),
            };
            for eff in effects {
                batch.push((card.clone(), eff));
            }
        }
        self.reset_priority();
        for (source, eff) in batch {
            match &eff {
                Effect::Delayed{ effect, phase, deps } => {
                    let id = self.schedule_delayed(*effect.clone(), *phase, deps.clone());
                    info!("Scheduled delayed effect id {} from trigger", id);
                }
                _ => {
                    let prio = match eff {
                        Effect::SelfAttributeChange(_) | Effect::Poliferate {..} => 2,
                        _ => 1,
                    };
                    self.push(StackEntry::TriggeredAbility { source: Some(source), effect: eff, controller }, prio);
                }
            }
        }
    }

    /// Player passes priority. After two passes, resolve the top of the stack.
    pub fn pass_priority(&mut self) {
        self.passes += 1;
        if self.passes >= 2 {
            self.resolve_stack();
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

    /// Resolve all entries on the stack respecting priority.
    pub fn resolve_stack(&mut self) {
        while let Some(pe) = self.stack.pop() {
            info!("Resolving {:?}", pe.entry);
            match pe.entry {
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
        let mut to_exec = if let Some(repl) = self.replacement_effects.iter().find_map(|r| r(&effect)) {
            repl
        } else {
            vec![effect]
        };
        for cont in &self.continuous_effects {
            for e in &mut to_exec { cont(e); }
        }
        for e in to_exec { self.execute(e); }
    }

    /// Register a replacement effect (e.g., replace destroy with exile).
    pub fn add_replacement_effect<F>(&mut self, effect: F)
    where F: 'static + Fn(&Effect) -> Option<Vec<Effect>> {
        self.replacement_effects.push(Box::new(effect));
    }

    /// Register a continuous effect (e.g., damage modification).
    pub fn add_continuous_effect<F>(&mut self, effect: F)
    where F: 'static + Fn(&mut Effect) {
        self.continuous_effects.push(Box::new(effect));
    }

    /// Execute or queue an effect immediately (for delayed dispatch).
    pub fn execute(&self, effect: Effect) {
        info!("Executing effect: {:?}", effect);
        // TODO: Concrete state mutation logic
    }

    /// Push a new entry onto the stack with given priority.
    fn push(&mut self, entry: StackEntry, priority: u8) {
        let seq = self.sequence;
        self.sequence = self.sequence.wrapping_add(1);
        self.stack.push(PriorityEntry { priority, sequence: seq, entry });
    }

    /// Add an entry to the stack as an activated ability, resetting priority.
    pub fn push_to_stack(&mut self, entry: StackEntry) {
        let prio = if matches!(entry, StackEntry::ActivatedAbility { .. }) { 3 } else { 1 };
        self.push(entry, prio);
        self.reset_priority();
    }
}
