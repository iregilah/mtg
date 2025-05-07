// src/app/gre.rs

use std::collections::{BinaryHeap, HashSet};
use crate::app::game_state::{GameEvent, GamePhase, Player};
use crate::app::card_attribute::{Effect, Trigger, TargetFilter, PlayerSelector, Duration, Condition};
use crate::app::card_library::{Card, ManaCost};
use tracing::info;

/// Wrapper for prioritized stack entries, supports custom ordering.
#[derive(Debug, Clone, Eq)]
pub struct PriorityEntry {
    pub priority: u8,
    pub sequence: usize,
    pub entry: StackEntry,
}

impl PriorityEntry {
    pub fn entry(&self) -> &StackEntry {
        &self.entry
    }
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

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ActivatedAbility {
    pub cost: ManaCost,
    pub condition: Condition,
    pub effect: Effect,
    pub activated_this_turn: bool,
}

/// An entry on the stack: spells, triggered or activated abilities.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StackEntry {
    Spell { card: Card, controller: Player },
    TriggeredAbility { source: Option<Card>, effect: Effect, controller: Player },
    ActivatedAbility { source: Card, ability: ActivatedAbility, controller: Player },
}


/// A delayed effect scheduled for a future phase.
#[derive(Debug, Clone)]
pub struct DelayedEffect {
    pub effect: Effect,
    pub execute_phase: GamePhase,
    pub id: usize,
    pub depends_on: Vec<usize>,
}

/// A replacement effect with priority.
struct ReplacementEffect {
    priority: u8,
    f: Box<dyn Fn(&Effect) -> Option<Vec<Effect>>>,
}

/// Game Rules Engine managing stack, replacement and continuous effects.
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
    /// Replacement effects, sorted by priority descending.
    replacement_effects: Vec<ReplacementEffect>,
    /// Continuous effects.
    pub continuous_effects: Vec<Box<dyn Fn(&mut Effect)>>,
    /// Követte, hogy az ellenfél veszített-e életet ebben a körben.
    pub opponent_lost_life_this_turn: bool,
    /// Követte, hogy mi veszítettünk-e életet ebben a körben.
    pub us_lost_life_this_turn: bool,
    /// Megakadályozott életnyerés állapotjelzők.
    pub prevent_life_gain_opponent: bool,
    pub prevent_life_gain_us: bool,
}

impl Default for Gre {
    fn default() -> Self {
        Gre::new(Player::Us)
    }
}

impl Gre {
    /// Initialize the GRE, starting with the given player on priority.
    pub fn new(starting_player: Player) -> Self {
        let mut gre = Self {
            stack: BinaryHeap::new(),
            delayed: Vec::new(),
            executed_delayed: HashSet::new(),
            next_id: 0,
            sequence: 0,
            priority: starting_player,
            passes: 0,
            replacement_effects: Vec::new(),
            continuous_effects: Vec::new(),
            opponent_lost_life_this_turn: false,
            us_lost_life_this_turn: false,
            prevent_life_gain_opponent: false,
            prevent_life_gain_us: false,
        };
        gre
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
    // Dispatch TurnEnded eseménykor
    pub fn on_turn_end(&mut self, battlefield: &mut Vec<Card>) {
        // reseteltetjük a „life lost this turn” jelzőket
        self.opponent_lost_life_this_turn = false;
        self.us_lost_life_this_turn = false;
        for card in battlefield.iter_mut() {
            for abil in card.activated_abilities.iter_mut() {
                abil.activated_this_turn = false;
            }
        }
    }

    // Able to activate check
    pub fn can_activate(&self, ability: &ActivatedAbility) -> bool {
        !ability.activated_this_turn && match ability.condition {
            Condition::OpponentLostLifeThisTurn => self.opponent_lost_life_this_turn,
            Condition::FirstTimeThisTurn => !ability.activated_this_turn,
            _ => false,
        }
    }
    /// Aktivál egy képességet: beleteszi a GRE stackbe, és flag-et állít
    pub fn activate_ability(&mut self, source: Card, ability: ActivatedAbility, controller: Player) {
        self.push_to_stack(StackEntry::ActivatedAbility { source, ability, controller });
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
        for d in self.delayed.drain(..) {
            still.push(d);
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
                GameEvent::SpellResolved(_) => card.trigger_by(&Trigger::OnCastResolved),
                GameEvent::CreatureDied(_) => card.trigger_by(&Trigger::OnDeath { filter: TargetFilter::SelfCard }),
                GameEvent::TurnEnded => card.trigger_by(&Trigger::AtPhase { phase: GamePhase::End, player: PlayerSelector::AnyPlayer }),
                GameEvent::PhaseChange(p) => card.trigger_by(&Trigger::AtPhase { phase: *p, player: PlayerSelector::AnyPlayer }),
                _ => Vec::new(),
            };
            for eff in effects {
                batch.push((card.clone(), eff));
            }
        }
        self.reset_priority();
        for (source, eff) in batch {
            match eff {
                Effect::Delayed { effect, phase, deps } => {
                    let id = self.schedule_delayed(*effect.clone(), phase, deps.clone());
                    info!("Scheduled delayed effect id {} from trigger", id);
                }
                eff => {
                    let prio = match &eff {
                        Effect::ModifyStats { .. } | Effect::Proliferate { .. } => 2,
                        _ => 1,
                    };
                    self.push(StackEntry::TriggeredAbility { source: Some(source), effect: eff, controller }, prio);
                }
            }
        }
    }

    /// Register a replacement effect with priority.
    pub fn add_replacement_effect<F>(&mut self, priority: u8, f: F)
    where
        F: 'static + Fn(&Effect) -> Option<Vec<Effect>>,
    {
        self.replacement_effects.push(ReplacementEffect { priority, f: Box::new(f) });
        self.replacement_effects.sort_by(|a, b| b.priority.cmp(&a.priority));
    }

    /// Register a continuous effect (e.g., damage modification).
    pub fn add_continuous_effect<F>(&mut self, effect: F)
    where
        F: 'static + Fn(&mut Effect),
    {
        self.continuous_effects.push(Box::new(effect));
    }

    /// Handle an effect: apply replacement chaining, continuous effects, then execute.
    pub fn handle_effect(&mut self, effect: Effect) {
        let replaced = if self.replacement_effects.is_empty() {
            vec![effect]
        } else {
            self.apply_replacement(&effect, 0)
        };
        let mut final_effects = Vec::new();
        for mut e in replaced {
            for cont in &self.continuous_effects {
                cont(&mut e);
            }
            final_effects.push(e);
        }
        for e in final_effects {
            self.execute(e);
        }
    }

    /// Recursively apply replacement effects in priority order.
    fn apply_replacement(&self, effect: &Effect, idx: usize) -> Vec<Effect> {
        if idx >= self.replacement_effects.len() {
            return vec![effect.clone()];
        }
        let replacer = &self.replacement_effects[idx];
        if let Some(repls) = (replacer.f)(effect) {
            repls.into_iter()
                .flat_map(|eff| self.apply_replacement(&eff, idx + 1))
                .collect()
        } else {
            self.apply_replacement(effect, idx + 1)
        }
    }

    /// Execute or queue an effect immediately (for delayed dispatch).
    pub fn execute(&mut self, effect: Effect) {
        match effect {
            Effect::ChooseSome { choose, options } => {
                // At the beginning of combat on your turn, target Mouse you control
                // gains your choice of double strike or trample until end of turn.
                //
                // (ide lehet majd bekérni a felhasználótól a választást, most demo-ként
                // automatikusan az első `choose` opciót alkalmazzuk)
                for opt in options.into_iter().take(choose) {
                    self.handle_effect(opt);
                }
            }
            Effect::Offspring { template } => {
                // Build token card: clone template, set P/T to 1, add Token type
                let mut token = template.clone();
                // reset power/toughness to 1/1
                for ct in token.card_types.iter_mut() {
                    if let super::card_library::CardType::Creature(ref mut cr) = ct {
                        cr.power = 1;
                        cr.toughness = 1;
                    }
                }
                // tag as a token
                token.card_types.push(super::card_library::CardType::Token);
                // Immediately put token onto battlefield by emitting CreateToken effect
                let create = Effect::CreateToken { token: crate::app::card_attribute::Token { name: token.name.clone() }, player: PlayerSelector::Controller };
                self.handle_effect(create);
            }
            // === CreateToken: actually place the token on the battlefield ===
            Effect::CreateToken { token, player } => {
                info!("Creating token {} for {:?}", token.name, player);
                // Here we'd insert `token.name` into the appropriate battlefield collection
                // e.g., call a callback or update GameStateUpdater when refreshing.
                // For now, log and assume Bot/GAME_STATE_UPDATER will pick up this code.
            }
            Effect::PreventLifeGain { player, duration } => {
                // Ha ideiglenes, kapcsoljuk be és ütemezzük a visszaállítást
                let flag = match player {
                    PlayerSelector::Opponent => &mut self.prevent_life_gain_opponent,
                    PlayerSelector::Controller => &mut self.prevent_life_gain_us,
                    _ => return,
                };
                if duration != Duration::Permanent {
                    *flag = true;
                    // Kör végén kapcsoljuk ki
                    self.schedule_delayed(
                        Effect::PreventLifeGain { player, duration: Duration::Permanent },
                        GamePhase::End,
                        vec![],
                    );
                } else {
                    // Permanent jelenti a kikapcsolást
                    *flag = false;
                }
            }
            Effect::GainLife { amount, player } => {
                // Ha prevent flag aktív, ne engedélyezzük az élet nyerését
                let prevented = match player {
                    PlayerSelector::Opponent => self.prevent_life_gain_opponent,
                    PlayerSelector::Controller => self.prevent_life_gain_us,
                    _ => false,
                };
                if prevented {
                    info!("Életnyerés ({:?}, {}) megakadályozva", player, amount);
                } else {
                    // TODO: implement life gain handling (GameState frissítés vagy trigger-event)
                    info!("GainLife effektus alkalmazva: {} life to {:?}", amount, player);
                }
            }
            _ => {
                // Egyéb effektusok meglévő logikája
            }
        }
    }

    /// Példa a feltétel kiértékelésére
    fn evaluate_condition(&self, cond: &Condition) -> bool {
        match cond {
            Condition::Always => true,
            Condition::FirstTimeThisTurn => {
                // implementáld, hogy csak egyszer fusson le
                true
            }
            _ => false
        }
    }

    /// Resolve all entries on the stack respecting priority.
    pub fn resolve_stack(&mut self) {
        while let Some(pe) = self.stack.pop() {
            info!("Resolving {:?}", pe.entry);
            match pe.entry {
                StackEntry::Spell { card, controller } => {
                    info!("Resolving spell: {}", card.name);
                    // 1) Trigger OnCastResolved
                    let mut battlefield = Vec::new();
                    // 2) Immediately trigger OnEnterBattlefield for "enters" triggers
                    self.trigger_event(
                        GameEvent::PhaseChange(GamePhase::Beginning),
                        &mut battlefield,
                        controller,
                    );
                    self.trigger_event(GameEvent::SpellResolved(card.name.clone()), &mut battlefield, controller);
                }
                StackEntry::TriggeredAbility { effect, .. } => {
                    self.handle_effect(effect);
                }
                StackEntry::ActivatedAbility { ability, .. } => {
                    // pull the effect out of the ActivatedAbility
                    self.handle_effect(ability.effect.clone());
                }
            }
        }
    }

    fn reset_priority(&mut self) {
        self.passes = 0;
    }

    fn push(&mut self, entry: StackEntry, priority: u8) {
        let seq = self.sequence;
        self.sequence = self.sequence.wrapping_add(1);
        self.stack.push(PriorityEntry { priority, sequence: seq, entry });
    }

    pub fn push_to_stack(&mut self, entry: StackEntry) {
        let prio = if matches!(entry, StackEntry::ActivatedAbility { .. }) { 3 } else { 1 };
        self.push(entry, prio);
        self.reset_priority();
    }

    pub fn resolve_top_of_stack(&mut self) {
        if let Some(pe) = self.stack.pop() {
            match pe.entry {
                StackEntry::TriggeredAbility { effect, .. } => {
                    self.handle_effect(effect);
                }
                StackEntry::ActivatedAbility { ability, .. } => {
                    self.handle_effect(ability.effect.clone());
                }
                _ => {}
            }
        }
    }
}
