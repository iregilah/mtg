// src/app/gre/mod.rs

use crate::app::gre::gre_structs::{DelayedEffect, ReplacementEffect};
use std::collections::{BinaryHeap, HashMap, HashSet};
use tracing::{debug, info, warn};

use crate::app::card_library::{Card, CardType, Creature, ManaCost};
use crate::app::card_library::CardTypeFlags;
use crate::app::card_attribute::{Effect, Condition, Duration, PlayerSelector, CreatureType, TargetFilter, Trigger};
use crate::app::game_state::{GamePhase, GameEvent, Player};


// A többi saját mod
pub mod stack;
pub mod gre_structs;
pub mod trigger;
pub mod effect_resolution;

// Publikus újra-exportálás, hogy kívülről elérhető legyen
pub use stack::{StackEntry, PriorityEntry};
pub use gre_structs::ActivatedAbility;
use crate::app::gre::effect_resolution::replace_targeted_filter_with_exact;

/// Ez lesz a "Game Rules Engine" (GRE) maga
pub struct Gre {
    /// A stack
    pub stack: BinaryHeap<PriorityEntry>,
    /// Késleltetett effektek
    pub delayed: Vec<DelayedEffect>,
    pub executed_delayed: HashSet<usize>,

    pub next_id: usize,
    pub next_card_id: u64,
    pub sequence: usize,
    pub priority: Player,
    pub passes: u8,

    pub replacement_effects: Vec<ReplacementEffect>,
    pub continuous_effects: Vec<Box<dyn Fn(&mut Effect)>>,

    /// Életvesztés jelzései
    pub opponent_lost_life_this_turn: bool,
    pub us_lost_life_this_turn: bool,
    pub prevent_life_gain_opponent: bool,
    pub prevent_life_gain_us: bool,

    /// Itt tároljuk a belső "trackelt" lényeinket
    pub battlefield_creatures: HashMap<u64, Card>,

    pub death_triggers_this_turn: Vec<(Card, Effect)>,

    pub current_source_card: Option<Card>,
}

impl Gre {
    pub fn new(starting_player: Player) -> Self {
        Self {
            stack: BinaryHeap::new(),
            delayed: Vec::new(),
            executed_delayed: HashSet::new(),
            next_id: 0,
            next_card_id: 1,
            sequence: 0,
            priority: starting_player,
            passes: 0,
            replacement_effects: Vec::new(),
            continuous_effects: Vec::new(),
            opponent_lost_life_this_turn: false,
            us_lost_life_this_turn: false,
            prevent_life_gain_opponent: false,
            prevent_life_gain_us: false,
            battlefield_creatures: HashMap::new(),
            death_triggers_this_turn: Vec::new(),
            current_source_card: None,
        }
    }
}

impl Default for Gre {
    fn default() -> Self {
        Gre::new(Player::Us)
    }
}

// Metódusok, amiket itt hagyunk (például):
impl Gre {
    pub fn on_turn_end(&mut self) {
        info!("on_turn_end() -> turn is ending, reset life-lost flags & death_triggers.");
        self.opponent_lost_life_this_turn = false;
        self.us_lost_life_this_turn = false;
        self.death_triggers_this_turn.clear();
        // ...
        for (_id, card) in self.battlefield_creatures.iter_mut() {
            for abil in card.activated_abilities.iter_mut() {
                abil.activated_this_turn = false;
            }
        }
    }

    pub fn can_activate(&self, ability: &crate::app::gre::ActivatedAbility) -> bool {
        !ability.activated_this_turn && match ability.condition {
            Condition::OpponentLostLifeThisTurn => self.opponent_lost_life_this_turn,
            Condition::FirstTimeThisTurn => !ability.activated_this_turn,
            _ => false,
        }
    }

    pub fn reset_priority(&mut self) {
        debug!("reset_priority() -> passes=0");
        self.passes = 0;
    }

    pub fn push_to_stack(&mut self, entry: StackEntry) {
        let prio = if matches!(entry, StackEntry::ActivatedAbility { .. }) { 3 } else { 1 };
        self.push(entry, prio);
        self.reset_priority();
    }

    fn push(&mut self, entry: StackEntry, priority: u8) {
        let seq = self.sequence;
        self.sequence = self.sequence.wrapping_add(1);
        debug!("push() -> pushing to stack: {:?}, prio={}, seq={}", entry, priority, seq);
        self.stack.push(PriorityEntry { priority, sequence: seq, entry });
    }

    pub fn cast_spell_with_target(&mut self, card: Card, controller: Player, target: Card) {
        // Először mentsük ki a card_id–t (és ha kell, a nevet is).
        let target_id = target.card_id;
        let target_name = target.name.clone();  // ha a nevét is ki akarod írni

        info!("{:?} casts '{}', target='{}'", controller, card.name, target_name);

        // Ezután konstruáljuk a StackEntry::Spell‐t, ezzel "belemovoljuk" a 'target'‐et.
        let entry = StackEntry::Spell {
            card,
            controller,
            target_creature: Some(target),
        };
        self.push(entry, 0);
        self.reset_priority();

        // Végül meghívjuk a “Targeted” eseményt a kimentett 'target_id' alapján
        self.trigger_event(GameEvent::Targeted(target_id), &mut Vec::new(), controller);
    }

    pub fn activate_ability(&mut self, source: Card, ability: crate::app::gre::ActivatedAbility, controller: Player) {
        info!("activate_ability() -> source='{}', condition={:?}, effect={:?}",
              source.name, ability.condition, ability.effect);
        self.push_to_stack(StackEntry::ActivatedAbility { source, ability, controller });
    }

    pub fn resolve_stack(&mut self) {
        info!("resolve_stack() -> start resolving all stack entries...");
        while let Some(pe) = self.stack.pop() {
            info!("  popped top: {:?}", pe.entry);
            match pe.entry {
                StackEntry::Spell { card, controller, target_creature } => {
                    // Itt mentsük el lokálisan a célpontot
                    let local_target = target_creature.clone();

                    info!("  -> Resolving Spell '{}'", card.name);
                    let mut c = card.clone();
                    self.enter_battlefield(&mut c);

                    // Ha instant/sorcery, OnCastResolved triggereket futtatunk
                    let effects = c.trigger_by(&Trigger::OnCastResolved);
                    for eff in effects {
                        match eff {
                            Effect::TargetedEffects { sub_effects } => {
                                if let Some(ref actual_target) = local_target {
                                    // itt a sub_effects mindegyikét local_target szerint dolgozzuk fel:
                                    for subeff in sub_effects {
                                        match subeff {
                                            // Ha a WhenTargetDiesThisTurn effektre futunk,
                                            // ne a current_stack_target()–et kérdezzük,
                                            // hanem használjuk az actual_target–et:
                                            Effect::WhenTargetDiesThisTurn { effect } => {
                                                // regisztráljuk a death_triggers_this_turn listába
                                                self.death_triggers_this_turn.push((
                                                    actual_target.clone(),
                                                    *effect, // pl. a CreateCreatureToken effect
                                                ));
                                            }
                                            Effect::CreateEnchantmentToken { name, power_buff, toughness_buff, ability } => {
                                                // Pl.:
                                                info!("Lokális 'actual_target' van => Létrehozzuk a token aura-t a '{}'-hez", actual_target.name);

                                                // Lényegében ugyanaz a kód, mint a `execute()`–beli CreateEnchantmentToken ága,
                                                // de a 'target_card' helyett 'actual_target'–et használunk:
                                                let mut aura_card = Card::new(
                                                    &name,
                                                    CardType::Enchantment,
                                                    ManaCost::free(),
                                                )
                                                    .with_added_type(CardTypeFlags::TOKEN);

                                                aura_card.attached_to = Some(actual_target.card_id);

                                                // OnEnterBattlefield: +X/+Y, GrantAbility(Trample) ...
                                                // OnDeath: -X/-Y, RemoveAbility(Trample)
                                                // ...
                                                // Végül
                                                self.enter_battlefield(&mut aura_card);
                                                info!("'{}' enchantment token létrehozva és a(z) '{}' lényhez csatolva.",
                                                     name, actual_target.name);
                                            }
                                            // egyéb sub–effektek (ModifyStats, GrantAbility, stb.)
                                            other => {
                                                let replaced =
                                                    replace_targeted_filter_with_exact(self, other, actual_target);
                                                self.handle_effect(replaced);
                                            }
                                        }
                                    }
                                } else {
                                    warn!("Nincs local_target, átugorjuk a sub_effects végrehajtást.");
                                }
                            }
                            _ => {
                                // Minden más effectet a handle_effect‐tel futtatunk
                                self.handle_effect(eff);
                            }
                        }
                    }

                    // További események pl. SpellResolved...
                    self.trigger_event(
                        GameEvent::SpellResolved(c.name.clone()),
                        &mut Vec::new(),
                        controller,
                    );
                }

                StackEntry::TriggeredAbility { source, effect, controller } => {
                    info!("  -> Resolving TriggeredAbility: effect={:?}", effect);
                    self.current_source_card = source;
                    self.handle_effect(effect);
                    self.current_source_card = None;
                }

                StackEntry::ActivatedAbility { source, ability, controller } => {
                    info!("  -> Resolving ActivatedAbility: effect={:?}", ability.effect);
                    self.current_source_card = Some(source);
                    self.handle_effect(ability.effect.clone());
                    self.current_source_card = None;
                }
            }
        }
        info!("resolve_stack() -> stack is now empty.");
    }

    pub fn resolve_top_of_stack(&mut self) {
        info!("resolve_top_of_stack() -> attempting to pop 1 item from stack...");
        if let Some(pe) = self.stack.pop() {
            match pe.entry {
                StackEntry::TriggeredAbility { source, effect, controller } => {
                    info!("  -> top is TriggeredAbility, effect={:?}", effect);
                    self.current_source_card = source;
                    self.handle_effect(effect);
                    self.current_source_card = None;
                }
                StackEntry::ActivatedAbility { source, ability, controller } => {
                    info!("  -> top is ActivatedAbility, effect={:?}", ability.effect);
                    self.current_source_card = Some(source);
                    self.handle_effect(ability.effect.clone());
                    self.current_source_card = None;
                }
                StackEntry::Spell { card, controller, target_creature } => {
                    info!("  -> top is Spell '{}', (just popping, not auto-resolving).", card.name);
                    // ...
                }
            }
        } else {
            debug!("  -> stack is empty, nothing to pop.");
        }
    }

    /// Replacement effect hozzáadása
    pub fn add_replacement_effect<F>(&mut self, priority: u8, f: F)
    where
        F: 'static + Fn(&Effect) -> Option<Vec<Effect>>,
    {
        self.replacement_effects.push(ReplacementEffect {
            priority,
            f: Box::new(f),
        });
        // Legyen csökkenő sorrend a priority alapján
        self.replacement_effects.sort_by(|a, b| b.priority.cmp(&a.priority));
    }

    /// Continuous (folyamatos) effect hozzáadása
    pub fn add_continuous_effect<F>(&mut self, effect: F)
    where
        F: 'static + Fn(&mut Effect),
    {
        self.continuous_effects.push(Box::new(effect));
    }

    /// Delayed effect ütemezése egy adott fázisra
    pub fn schedule_delayed(&mut self, effect: Effect, phase: GamePhase, depends_on: Vec<usize>) -> usize {
        let id = self.next_id;
        self.next_id += 1;
        self.delayed.push(DelayedEffect {
            effect,
            execute_phase: phase,
            id,
            depends_on,
        });
        id
    }

    /// Delayed effectek futtatása az aktuális fázisban
    pub fn dispatch_delayed(&mut self, current_phase: GamePhase) {
        info!("dispatch_delayed() -> current_phase={:?}", current_phase);
        let mut still = Vec::new();

        // Kikeressük az éppen most lefuttatható delayed effecteket
        let mut ready: Vec<_> = self.delayed.drain(..)
            .filter(|d| {
                d.execute_phase == current_phase
                    && d.depends_on.iter().all(|dep| self.executed_delayed.contains(dep))
            })
            .collect();

        // A többit visszatesszük
        for d in self.delayed.drain(..) {
            still.push(d);
        }
        self.delayed = still;

        // Rendezés id alapján
        ready.sort_by_key(|d| d.id);

        // Lefuttatjuk
        for d in ready {
            info!("  Dispatching delayed effect #{} at {:?}", d.id, current_phase);
            self.executed_delayed.insert(d.id);
            self.handle_effect(d.effect.clone());
        }
    }

    pub fn current_stack_target(gre: &Gre) -> Option<Card> {
        if let Some(pe) = gre.stack.peek() {
            match &pe.entry {
                StackEntry::Spell { target_creature: Some(t), .. } => Some(t.clone()),
                _ => None,
            }
        } else {
            None
        }
    }
    /// Betesszük a kártyát a battlefieldre, automatikusan kiosztva neki az egyedi ID-t.
    pub fn enter_battlefield(&mut self, card: &mut Card) {
        if card.card_id == 0 {
            card.card_id = self.next_card_id;
            self.next_card_id += 1;
        }
        let new_id = card.card_id;
        info!("enter_battlefield() -> adding '{}' (id={}) to battlefield", card.name, new_id);

        self.battlefield_creatures.insert(new_id, card.clone());

        // OnEnterBattlefield triggerek
        let effects = card.trigger_by(&Trigger::OnEnterBattlefield {
            filter: TargetFilter::SelfCard,
        });
        debug!("  card '{}' -> OnEnterBattlefield returned {} effect(s)", card.name, effects.len());
        for eff in effects {
            self.handle_effect(eff);
        }
    }

    pub fn create_creature_token(
        &mut self,
        name: &str,
        power: i32,
        toughness: i32,
        creature_types: Vec<crate::app::card_attribute::CreatureType>,
    ) {
        info!("create_creature_token() -> name='{}', power={}, toughness={}, types={:?}",
          name, power, toughness, creature_types);

        let mut new_card = Card::new(
            name,
            CardType::Creature(Creature {
                power,
                toughness,
                summoning_sickness: true,
                abilities: Vec::new(),
                types: creature_types,
                ephemeral_power: 0,
                ephemeral_toughness: 0,
            }),
            ManaCost::free(),
        )
            .with_added_type(CardTypeFlags::CREATURE)
            .with_added_type(CardTypeFlags::TOKEN);

        self.enter_battlefield(&mut new_card);
        debug!("  creature_token létrehozva és battlefiedre került: '{}'", name);
    }

    pub fn create_clone_card(gre: &mut Gre, mut cloned: Card) {
        info!("create_clone_card() -> cloning card '{}' (id={}) and placing on battlefield", cloned.name, cloned.card_id);
        gre.enter_battlefield(&mut cloned);
    }
}