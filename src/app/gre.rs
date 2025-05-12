use std::collections::{BinaryHeap, HashMap, HashSet};
use crate::app::game_state::{GameEvent, GamePhase, Player};
use crate::app::card_attribute::{Effect, Trigger, TargetFilter, PlayerSelector, Duration, Condition, Amount, OffspringAttribute, CreatureType, TriggeredEffectAttribute};
use crate::app::card_library::{Card, CardType, Creature, ManaCost};
use crate::app::card_library::CardTypeFlags;
use tracing::{debug, info, warn};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ActivatedAbility {
    pub cost: ManaCost,
    pub condition: Condition,
    pub effect: Effect,
    pub activated_this_turn: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StackEntry {
    Spell {
        card: Card,
        controller: Player,
        target_creature: Option<Card>,
    },
    TriggeredAbility { source: Option<Card>, effect: Effect, controller: Player },
    ActivatedAbility { source: Card, ability: ActivatedAbility, controller: Player },
}

/// A stackbeli tételek prioritással
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

/// A késleltetett effectek
#[derive(Debug, Clone)]
pub struct DelayedEffect {
    pub effect: Effect,
    pub execute_phase: GamePhase,
    pub id: usize,
    pub depends_on: Vec<usize>,
}

struct ReplacementEffect {
    priority: u8,
    f: Box<dyn Fn(&Effect) -> Option<Vec<Effect>>>,
}

// A Debug és Clone implementációk a closure miatt speciálisak
impl std::fmt::Debug for ReplacementEffect {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ReplacementEffect")
            .field("priority", &self.priority)
            .finish()
    }
}
// Hasonlóan, a Clone sem megy automatikusan. Ha tényleg klónozni kell:
impl Clone for ReplacementEffect {
    fn clone(&self) -> Self {
        panic!("ReplacementEffect is not clonable in a simple way!");
    }
}

/// Game Rules Engine
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
    replacement_effects: Vec<ReplacementEffect>,
    pub continuous_effects: Vec<Box<dyn Fn(&mut Effect)>>,

    /// Életvesztés jelzései
    pub opponent_lost_life_this_turn: bool,
    pub us_lost_life_this_turn: bool,
    pub prevent_life_gain_opponent: bool,
    pub prevent_life_gain_us: bool,

    /// Itt tároljuk a belső "trackelt" lényeinket
    pub battlefield_creatures: HashMap<u64, Card>,

    pub death_triggers_this_turn: Vec<(Card, Effect)>,

    /// A TriggeredAbility-k végrehajtásakor az épp "source" card
    current_source_card: Option<Card>,
}

impl Default for Gre {
    fn default() -> Self {
        Gre::new(Player::Us)
    }
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

    pub fn add_replacement_effect<F>(&mut self, priority: u8, f: F)
    where
        F: 'static + Fn(&Effect) -> Option<Vec<Effect>>,
    {
        self.replacement_effects.push(ReplacementEffect {
            priority,
            f: Box::new(f),
        });
        // priority alapján csökkenő sorrend
        self.replacement_effects.sort_by(|a, b| b.priority.cmp(&a.priority));
    }

    pub fn add_continuous_effect<F>(&mut self, effect: F)
    where
        F: 'static + Fn(&mut Effect),
    {
        self.continuous_effects.push(Box::new(effect));
    }


    /// Spell a stackre
    /*
   pub fn cast_spell(&mut self, card: Card, controller: Player) {
       info!("{:?} casts {}", controller, card.name);
       self.push(StackEntry::Spell { card, controller, target_creature }, 0);
       self.reset_priority();
   }*/
    pub fn cast_spell_with_target(&mut self, card: Card, controller: Player, target: Card) {
        info!("{:?} casts {}", controller, card.name);
        let entry = StackEntry::Spell {
            card,
            controller,
            target_creature: Some(target),
        };
        self.push(entry, 0);
        self.reset_priority();
    }
    /// Késleltetett effect feljegyzése
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

    /// Turn end event

    pub fn on_turn_end(&mut self) {
        info!("on_turn_end() -> turn is ending, reset life-lost flags & death_triggers.");
        self.opponent_lost_life_this_turn = false;
        self.us_lost_life_this_turn = false;
        self.death_triggers_this_turn.clear();
        // A lényeink activated_ability jelzőit is visszaállítjuk
        for (_id, card) in self.battlefield_creatures.iter_mut() {
            for abil in card.activated_abilities.iter_mut() {
                abil.activated_this_turn = false;
            }
        }
    }

    pub fn can_activate(&self, ability: &ActivatedAbility) -> bool {
        !ability.activated_this_turn && match ability.condition {
            Condition::OpponentLostLifeThisTurn => self.opponent_lost_life_this_turn,
            Condition::FirstTimeThisTurn => !ability.activated_this_turn,
            _ => false,
        }
    }

    pub fn activate_ability(&mut self, source: Card, ability: ActivatedAbility, controller: Player) {
        info!("activate_ability() -> source='{}', condition={:?}, effect={:?}",
              source.name, ability.condition, ability.effect);
        self.push_to_stack(StackEntry::ActivatedAbility { source, ability, controller });
    }

    /// Delayed effectek futtatása a megfelelő fázisban
    pub fn dispatch_delayed(&mut self, current_phase: GamePhase) {
        info!("dispatch_delayed() -> current_phase={:?}", current_phase);
        let mut still = Vec::new();
        let mut ready: Vec<_> = self.delayed.drain(..)
            .filter(|d| {
                d.execute_phase == current_phase &&
                    d.depends_on.iter().all(|dep| self.executed_delayed.contains(dep))
            })
            .collect();
        // a maradékot visszatesszük
        for d in self.delayed.drain(..) {
            still.push(d);
        }
        self.delayed = still;

        // Sorba tesszük id szerint
        ready.sort_by_key(|d| d.id);

        // Lekezeljük
        for d in ready {
            info!("Dispatching delayed effect {} at {:?}", d.id, current_phase);
            self.executed_delayed.insert(d.id);
            self.handle_effect(d.effect.clone());
        }
    }
    // -------------------------------------------------------------------------
    //                         TRIGGER HANDLING
    // -------------------------------------------------------------------------

    /// Események (pl. OnCastResolved) kiváltása a battlefielden lévő kártyákra
    /// Események (pl. OnCastResolved) kiváltása a battlefielden lévő kártyákra
    pub fn trigger_event(&mut self, event: GameEvent, battlefield: &mut Vec<Card>, controller: Player) {
        info!("trigger_event() -> event={:?}, controller={:?}, bf_len={}", event, controller, battlefield.len());

        // Kifejezetten a `GameEvent::CreatureDied`-hez: megnézzük a death_triggers_this_turn listát
        if let GameEvent::CreatureDied(ref died_card) = event {
            info!("  Checking death_triggers_this_turn, died_card='{}' (id={})", died_card.name, died_card.card_id);
            let mut to_trigger = Vec::new();
            let mut indices_to_remove = Vec::new();

            for (i, (tracked_creature, eff)) in self.death_triggers_this_turn.iter().enumerate() {
                if tracked_creature == died_card {
                    debug!("    -> Found a death-trigger effect: {:?}", eff);
                    to_trigger.push(eff.clone());
                    indices_to_remove.push(i);
                }
            }
            // Lefuttatjuk őket
            for eff in to_trigger {
                debug!("    handle_effect from death_triggers_this_turn: {:?}", eff);
                self.handle_effect(eff);
            }
            // Töröljük a már felhasznált trigger-bejegyzéseket
            for &i in indices_to_remove.iter().rev() {
                self.death_triggers_this_turn.remove(i);
            }
        }

        // Végigmegyünk a battlefield kártyáin, megnézzük, milyen effecteket adnak vissza a trigger_by() hívások
        let mut batch = Vec::new();
        for card in battlefield.iter_mut() {
            let effects = match &event {
                GameEvent::SpellResolved(_spell_name) => {
                    card.trigger_by(&Trigger::OnCastResolved)
                }
                GameEvent::CreatureDied(_name) => {
                    card.trigger_by(&Trigger::OnDeath { filter: TargetFilter::SelfCard })
                }
                GameEvent::TurnEnded => {
                    card.trigger_by(&Trigger::AtPhase { phase: GamePhase::End, player: PlayerSelector::AnyPlayer })
                }
                GameEvent::PhaseChange(p) => {
                    card.trigger_by(&Trigger::AtPhase { phase: *p, player: PlayerSelector::AnyPlayer })
                }
                _ => Vec::new(),
            };
            if !effects.is_empty() {
                debug!("  Card '{}': {} trigger-effect(s) found", card.name, effects.len());
            }
            for eff in effects {
                debug!("    effect => {:?}", eff);
                batch.push((card.clone(), eff));
            }
        }

        // Priority visszaállítása, hogy a stacken megint legyen kié a kör
        self.reset_priority();

        // A begyűjtött effecteket a stackre tesszük, vagy delayedet ütemezünk
        for (source_card, eff) in batch {
            match eff {
                Effect::Delayed { effect, phase, deps } => {
                    let id = self.schedule_delayed(*effect.clone(), phase, deps.clone());
                    info!("    -> Scheduled delayed effect #{} from normal trigger", id);
                }
                e => {
                    let prio = match &e {
                        Effect::ModifyStats { .. } | Effect::Proliferate { .. } => 2,
                        _ => 1,
                    };
                    info!("    -> Pushing TriggeredAbility to stack (prio={}), effect={:?}", prio, e);
                    self.push(
                        StackEntry::TriggeredAbility {
                            source: Some(source_card),
                            effect: e,
                            controller,
                        },
                        prio,
                    );
                }
            }
        }
    }

    /// DFS / BFS jellegű triggerezés, ami a `battlefield_creatures` belső táblát járja be
    pub fn trigger_event_tree(&mut self, event: GameEvent, controller: Player) {
        info!("trigger_event_tree() -> event={:?}, BFS/DFS-based. Searching root permanents...", event);

        let root_ids: Vec<u64> = self.battlefield_creatures
            .values()
            .filter(|c| c.attached_to.is_none())
            .map(|c| c.card_id)
            .collect();

        debug!("  Found {} root(s): {:?}", root_ids.len(), root_ids);

        for rid in root_ids {
            self.traverse_trigger_tree(rid, &event, controller);
        }

        // Rendszerint ilyenkor is reseteljük a priority-t
        self.reset_priority();
    }

    fn traverse_trigger_tree(&mut self, card_id: u64, event: &GameEvent, controller: Player) {
        debug!("    traverse_trigger_tree() -> card_id={}", card_id);

        // 1) kivesszük a kártyát a belső HashMap-ből
        let mut card = if let Some(card) = self.battlefield_creatures.remove(&card_id) {
            card
        } else {
            debug!("      -> Card not found in battlefield_creatures, returning.");
            return;
        };

        // 2) a kártyán lekérdezzük az adott eventhez tartozó effekteket
        let triggered_effects = self.event_to_triggers(event, &mut card);

        if !triggered_effects.is_empty() {
            debug!("      -> card '{}' triggered {} effect(s)", card.name, triggered_effects.len());
        }

        // 3) ezeket a stackre tesszük, vagy delayedet schedule-ölünk
        for eff in triggered_effects {
            match eff {
                Effect::Delayed { effect, phase, deps } => {
                    let id = self.schedule_delayed(*effect.clone(), phase, deps);
                    info!("        Scheduled delayed effect #{} from traverse_trigger_tree", id);
                }
                e => {
                    info!("        Push TriggeredAbility on stack. effect={:?}", e);
                    self.push(
                        StackEntry::TriggeredAbility {
                            source: Some(card.clone()),
                            effect: e,
                            controller,
                        },
                        1,
                    );
                }
            }
        }

        // Visszarakjuk a kártyát a táblába
        debug!("      -> inserting card '{}' (id={}) back to battlefield_creatures", card.name, card_id);
        self.battlefield_creatures.insert(card_id, card);

        // 4) a gyermek-lapok (akik 'attached_to == Some(card_id)') bejárása
        let child_ids: Vec<u64> = self.battlefield_creatures
            .values()
            .filter(|c2| c2.attached_to == Some(card_id))
            .map(|c2| c2.card_id)
            .collect();

        debug!("      -> found {} child(ren): {:?}", child_ids.len(), child_ids);
        for cid in child_ids {
            self.traverse_trigger_tree(cid, event, controller);
        }
    }

    /// event -> card.trigger_by(...) segédfüggvény
    fn event_to_triggers(&mut self, event: &GameEvent, card: &mut Card) -> Vec<Effect> {
        debug!("        event_to_triggers(): event={:?}, card='{}'", event, card.name);
        let res = match event {
            GameEvent::SpellResolved(_spell_name) => {
                card.trigger_by(&Trigger::OnCastResolved)
            }
            GameEvent::CreatureDied(died_card) => {
                if died_card.card_id == card.card_id {
                    card.trigger_by(&Trigger::OnDeath { filter: TargetFilter::SelfCard })
                } else {
                    Vec::new()
                }
            }
            GameEvent::TurnEnded => {
                card.trigger_by(
                    &Trigger::AtPhase {
                        phase: GamePhase::End,
                        player: PlayerSelector::AnyPlayer,
                    }
                )
            }
            GameEvent::PhaseChange(p) => {
                card.trigger_by(
                    &Trigger::AtPhase {
                        phase: *p,
                        player: PlayerSelector::AnyPlayer,
                    }
                )
            }
            // További game eventek implementálhatók...
            _ => Vec::new(),
        };

        if !res.is_empty() {
            debug!("          -> card '{}' returned {} effect(s) from event_to_triggers", card.name, res.len());
        }
        res
    }
    // -------------------------------------------------------------------------
    // A "fő" effectkezelő + execute
    // -------------------------------------------------------------------------

    pub fn handle_effect(&mut self, effect: Effect) {
        // Replacement
        let replaced = if self.replacement_effects.is_empty() {
            vec![effect]
        } else {
            self.apply_replacement(&effect, 0)
        };

        // Continuous effectek
        let mut final_effects = Vec::new();
        for mut e in replaced {
            for cont in &self.continuous_effects {
                cont(&mut e);
            }
            final_effects.push(e);
        }

        // Végrehajtás
        for e in final_effects {
            self.execute(e);
        }
    }

    /// Rekurzív replacement-chaining
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


    /// A tényleges "egy effect" végrehajtása
    pub fn execute(&mut self, effect: Effect) {
        info!("GRE.execute() → Indul az effect végrehajtása: {:?}", effect);
        match effect {
            Effect::CreateEnchantmentToken {
                name,
                power_buff,
                toughness_buff,
                ability,
            } => {
                info!("CreateEnchantmentToken effect detected: name='{}', buff=({}/{}) ability={:?}",
                      name, power_buff, toughness_buff, ability);

                // Első lépés: van-e target a stack tetején?
                if let Some(target_card) = self.current_stack_target() {
                    info!("  Target megtalálva: '{}' (id={})", target_card.name, target_card.card_id);

                    // Létrehozunk egy token card-ot
                    let mut aura_card = Card::new(
                        &name,
                        CardType::Enchantment,
                        ManaCost::free(),
                    );
                    aura_card.type_flags |= CardTypeFlags::TOKEN;

                    // Rácsatoljuk a megcélzott creature-re
                    aura_card.attached_to = Some(target_card.card_id);

                    // Hozzáadjuk az OnEnterBattlefield és OnDeath triggert a buff eltávolításához
                    debug!("  Létrehozott 'aura_card' token, csatoljuk a target_card-hoz, \
                             beállítjuk a TriggeredEffectAttribute-kat + buff-hatásokat.");


                    // 1) OnEnterBattlefield
                    aura_card.triggers.push(
                        Trigger::OnEnterBattlefield { filter: TargetFilter::SelfCard }
                    );
                    aura_card.attributes.push(Box::new(
                        TriggeredEffectAttribute {
                            trigger: Trigger::OnEnterBattlefield { filter: TargetFilter::SelfCard },
                            effect: Effect::TargetedEffects {
                                sub_effects: vec![
                                    // +1/+1
                                    Effect::ModifyStats {
                                        power_delta: power_buff,
                                        toughness_delta: toughness_buff,
                                        duration: Duration::Permanent,
                                        // KONKRÉT ID
                                        target: TargetFilter::ExactCardID(target_card.card_id),
                                    },
                                    // GrantTrample
                                    Effect::GrantAbility {
                                        ability,
                                        duration: Duration::Permanent,
                                        target: TargetFilter::ExactCardID(target_card.card_id),
                                    },
                                ],
                            },
                        }
                    ));

                    // 2) OnDeath => -1/-1, RemoveAbility(Trample)
                    aura_card.triggers.push(
                        Trigger::OnDeath { filter: TargetFilter::SelfCard }
                    );
                    aura_card.attributes.push(Box::new(
                        TriggeredEffectAttribute {
                            trigger: Trigger::OnDeath { filter: TargetFilter::SelfCard },
                            effect: Effect::TargetedEffects {
                                sub_effects: vec![
                                    // visszavonjuk a buffot
                                    Effect::ModifyStats {
                                        power_delta: -power_buff,
                                        toughness_delta: -toughness_buff,
                                        duration: Duration::Permanent,
                                        target: TargetFilter::ExactCardID(target_card.card_id),
                                    },
                                    // visszavonjuk a képességet is
                                    Effect::RemoveAbility {
                                        ability,
                                        target: TargetFilter::ExactCardID(target_card.card_id),
                                    },
                                ],
                            },
                        }
                    ));

                    debug!("  Token attribute-ok hozzáadva, mielőtt belép a battlefieldre.");

                    // Végül berakjuk a battlefieldre
                    self.enter_battlefield(&mut aura_card);
                    info!("'{}' enchantment token létrehozva és a(z) '{}' lényhez csatolva.",
                          name, target_card.name);
                } else {
                    warn!("  Nincs target a CreateEnchantmentToken-höz, kihagyjuk.");
                }
            }

            Effect::RemoveAbility { ability, target } => {
                info!("RemoveAbility effect: ability={:?}, target={:?}", ability, target);
                match target {
                    TargetFilter::ExactCardID(id) => {
                        info!("  RemoveAbility – megpróbáljuk kikeresni a battlefielden card_id={}", id);
                        if let Some(mut c) = self.battlefield_creatures.get_mut(&id) {
                            info!("  Megtaláltuk a kártyát ('{}', id={}), abilities törlése.",
                                  c.name, c.card_id);
                            if let CardType::Creature(ref mut cr) = c.card_type {
                                let before_len = cr.abilities.len();
                                cr.abilities.retain(|&a| a != ability);
                                let after_len = cr.abilities.len();
                                debug!("  {} -> {} ability maradt ({} törölve).",
                                       before_len, after_len, before_len - after_len);
                            }
                        } else {
                            warn!("  Nem található creature az id={} értéken, effect sikertelen.", id);
                        }
                    }
                    _ => {
                        warn!("RemoveAbility target nem ExactCardID, átugorjuk.");
                    }
                }
            }

            Effect::TargetedEffects { sub_effects } => {
                info!("TargetedEffects: sub_effects len={}", sub_effects.len());
                if let Some(target_card) = self.current_stack_target() {
                    info!("  Stack célpontja: '{}' (id={})", target_card.name, target_card.card_id);
                    for (i, subeff) in sub_effects.into_iter().enumerate() {
                        debug!("    Feldolgozzuk a(z) {}. sub_effectet: {:?}", i + 1, subeff);
                        let replaced = self.replace_targeted_filter_with_exact(subeff, &target_card);
                        self.handle_effect(replaced);
                    }
                } else {
                    warn!("  Nincs target_creature, átugorjuk a sub_effects végrehajtást.");
                }
            }

            Effect::WhenTargetDiesThisTurn { effect } => {
                info!("WhenTargetDiesThisTurn effect: belső effect = {:?}", effect);
                // Megnézzük, van-e target
                if let Some(target_card) = self.current_stack_target() {
                    info!("  Death-trigger regisztrálása a(z) '{}' kártyán.", target_card.name);
                    self.death_triggers_this_turn.push((target_card.clone(), *effect));
                } else {
                    warn!("  Nincs target, a WhenTargetDiesThisTurn effectet nem regisztráljuk.");
                }
            }

            Effect::Offspring { cost } => {
                info!("Offspring effect: cost={}. Megnézzük a current_source_card-ot...", cost);
                if let Some(ref src) = self.current_source_card {
                    debug!("  source_card='{}' (id={}). Készítünk belőle klónt tokenként.", src.name, src.card_id);
                    let cloned = self.clone_card(
                        src,
                        Some(1),
                        Some(1),
                        Some(CardTypeFlags::TOKEN),
                    );
                    self.create_clone_card(cloned);
                    info!("  Offspring klón token sikeresen létrehozva.");
                } else {
                    warn!("  Nincs current_source_card, így nincs mit klónozni Offspring-gel.");
                }
            }


            Effect::CreateCreatureToken {
                name,
                power,
                toughness,
                creature_types,
            } => {
                self.create_creature_token(&name, power, toughness, creature_types);
            }

            // --- "Kettős" opció effektek (pl. Offspring vagy no-op)
            Effect::ChooseSome { choose, options } => {
                // Itt a prototípus-kódban mindig az 1. választást hívnánk,
                // de a valós UI-ban a user dönti el.
                // Hogy illusztráljuk, hívjuk az 'nth' effectet:
                if choose == 0 || options.is_empty() {
                    info!("ChooseSome => nincs választott effect (choose=0).");
                } else {
                    // tegyük fel, fixen az utolsó választást hívjuk:
                    let idx = choose.min(options.len());
                    let chosen = options[idx - 1].clone();
                    info!("ChooseSome => a(z) {}. effectet hajtjuk végre: {:?}", idx, chosen);
                    self.handle_effect(chosen);
                }
            }

            // --- A többi effect pl. "PreventLifeGain"
            Effect::PreventLifeGain { player, duration } => {
                let flag = match player {
                    PlayerSelector::Controller => &mut self.prevent_life_gain_us,
                    PlayerSelector::Opponent => &mut self.prevent_life_gain_opponent,
                    PlayerSelector::AnyPlayer => {
                        info!("PreventLifeGain(AnyPlayer) -> nem kezelt");
                        return;
                    }
                };
                match duration {
                    Duration::Permanent => {
                        // permanent = kikapcsolás
                        *flag = false;
                    }
                    _ => {
                        // bekapcsolás, és end phase-re kikapcs
                        *flag = true;
                        self.schedule_delayed(
                            Effect::PreventLifeGain {
                                player,
                                duration: Duration::Permanent,
                            },
                            GamePhase::End,
                            vec![],
                        );
                    }
                }
            }

            // Például life gain
            Effect::GainLife { amount, player } => {
                let prevented = match player {
                    PlayerSelector::Controller => self.prevent_life_gain_us,
                    PlayerSelector::Opponent => self.prevent_life_gain_opponent,
                    PlayerSelector::AnyPlayer => false,
                };
                if prevented {
                    info!("Life gain {} for {:?} meghiúsul (PreventLifeGain).", amount, player);
                } else {
                    info!("{:?} GAIN LIFE: +{} (state-ben még nem frissítjük).", player, amount);
                }
            }

            // Minden egyéb
            _ => {
                info!("Executing effect: {:?}", effect);
            }
        }
    }
    // -------------------------------------------------------------------------
    // További segéd-függvények (enter_battlefield, create_creature_token stb.)
    // -------------------------------------------------------------------------

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


    /// Általános kártya-klónozó: bemenet az eredeti Card, plusz opcionális power/toughness
    /// felülírás, plusz bitflag hozzáadás.
    pub fn clone_card(
        &self,
        original: &Card,
        new_power: Option<i32>,
        new_toughness: Option<i32>,
        added_flags: Option<CardTypeFlags>,
    ) -> Card {
        let mut cloned = original.clone();
        // Ha creature, power/toughness cseréje:
        if let CardType::Creature(ref mut cr) = cloned.card_type {
            if let Some(p) = new_power {
                cr.power = p;
            }
            if let Some(t) = new_toughness {
                cr.toughness = t;
            }
        }
        // plusz type_flags:
        if let Some(flags) = added_flags {
            cloned.type_flags |= flags;
        }
        cloned
    }

    /// Egy teljesen új “token creature” kártyát hoz létre (nem klón!),
    /// pl. Felonious Rage 2/2 Detective, stb.
    /// Summoning sickness = true, type = Creature + Token

    pub fn create_creature_token(
        &mut self,
        name: &str,
        power: i32,
        toughness: i32,
        creature_types: Vec<CreatureType>,
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
            }),
            ManaCost::free(),
        )
            .with_added_type(CardTypeFlags::CREATURE)
            .with_added_type(CardTypeFlags::TOKEN);

        self.enter_battlefield(&mut new_card);
        debug!("  creature_token létrehozva és battlefiedre került: '{}'", name);
    }

    pub fn create_clone_card(&mut self, mut cloned: Card) {
        info!("create_clone_card() -> cloning card '{}' (id={}) and placing on battlefield", cloned.name, cloned.card_id);
        self.enter_battlefield(&mut cloned);
    }

    pub fn current_stack_target(&self) -> Option<Card> {
        if let Some(pe) = self.stack.peek() {
            match &pe.entry {
                StackEntry::Spell { target_creature: Some(t), .. } => Some(t.clone()),
                _ => None,
            }
        } else {
            None
        }
    }

    /// Lecseréljük az effect-en belül a `TargetFilter::TargetedCreature` mezőt
    /// konkrét `ExactCard(target_card.clone())`-ra (rekurzív, ha kell).
    pub fn replace_targeted_filter_with_exact(&self, effect: Effect, tcard: &Card) -> Effect {
        // Függvény elején: milyen effectet kaptunk, és melyik target_card (id, name)
        debug!(
        "replace_targeted_filter_with_exact() - Start: effect={:?}, tcard='{}'(id={})",
        effect,
        tcard.name,
        tcard.card_id
    );

        // Megcsináljuk az eredeti match-elágazást
        let result = match effect {
            Effect::ModifyStats {
                power_delta,
                toughness_delta,
                duration,
                target,
            } => {
                debug!("  → ModifyStats branch. Eredeti target={:?}", target);

                // Ha a filter=Creature, ExactCardID-re cseréljük
                let new_t = if target == TargetFilter::Creature {
                    info!("    TargetFilter::Creature → lecseréljük ExactCardID({})-re", tcard.card_id);
                    TargetFilter::ExactCardID(tcard.card_id)
                } else {
                    debug!("    TargetFilter változatlan: {:?}", target);
                    target
                };

                Effect::ModifyStats {
                    power_delta,
                    toughness_delta,
                    duration,
                    target: new_t,
                }
            }

            Effect::GrantAbility {
                ability,
                duration,
                target,
            } => {
                debug!("  → GrantAbility branch. Eredeti target={:?}", target);

                let new_t = if target == TargetFilter::Creature {
                    info!("    TargetFilter::Creature → ExactCardID({})", tcard.card_id);
                    TargetFilter::ExactCardID(tcard.card_id)
                } else {
                    target
                };

                Effect::GrantAbility {
                    ability,
                    duration,
                    target: new_t,
                }
            }

            Effect::TargetedEffects { sub_effects } => {
                debug!("  → TargetedEffects branch, sub_effects len={}", sub_effects.len());

                // rekurzívan átalakítjuk a belső effekteket is
                let replaced_subs = sub_effects
                    .into_iter()
                    .map(|sub| {
                        debug!("    TargetedEffects - belső sub: {:?}", sub);
                        self.replace_targeted_filter_with_exact(sub, tcard)
                    })
                    .collect();

                Effect::TargetedEffects {
                    sub_effects: replaced_subs
                }
            }

            Effect::WhenTargetDiesThisTurn { effect } => {
                debug!("  → WhenTargetDiesThisTurn, effect={:?}", effect);
                // Itt tipikusan nem cseréljük a TargetFilter-t; ha akarnánk, hasonló logikával, mint feljebb
                Effect::WhenTargetDiesThisTurn { effect }
            }

            // Minden más effectet nem módosítunk, de logoljuk:
            other => {
                debug!("  → Egyéb effect, nincs célcserélés: {:?}", other);
                other
            }
        };

        // Függvény vége: visszaadott effect
        debug!("replace_targeted_filter_with_exact() - End: returning={:?}", result);
        result
    }

    // -------------------------------------------------------------------------
    // Stack kezelés: push, resolve
    // -------------------------------------------------------------------------

    fn reset_priority(&mut self) {
        debug!("reset_priority() -> passes=0");
        self.passes = 0;
    }

    fn push(&mut self, entry: StackEntry, priority: u8) {
        let seq = self.sequence;
        self.sequence = self.sequence.wrapping_add(1);
        debug!("push() -> pushing to stack: {:?}, prio={}, seq={}", entry, priority, seq);

        self.stack.push(PriorityEntry { priority, sequence: seq, entry });
    }

    pub fn push_to_stack(&mut self, entry: StackEntry) {
        // ActivatedAbility kapjon prio=3, egyébként prio=1
        let prio = if matches!(entry, StackEntry::ActivatedAbility { .. }) { 3 } else { 1 };
        self.push(entry, prio);
        self.reset_priority();
    }

    pub fn resolve_stack(&mut self) {
        info!("resolve_stack() -> start resolving all stack entries...");
        while let Some(pe) = self.stack.pop() {
            info!("  popped top: {:?}", pe.entry);
            match pe.entry {
                StackEntry::Spell { card, controller, target_creature } => {
                    info!("  -> Resolving Spell '{}'", card.name);
                    let mut c = card.clone();
                    self.enter_battlefield(&mut c);

                    self.trigger_event(
                        GameEvent::SpellResolved(card.name.clone()),
                        &mut Vec::new(),
                        controller,
                    );
                }

                StackEntry::TriggeredAbility { source, effect, .. } => {
                    info!("  -> Resolving TriggeredAbility: effect={:?}", effect);
                    self.current_source_card = source;
                    self.handle_effect(effect);
                    self.current_source_card = None;
                }

                StackEntry::ActivatedAbility { source, ability, .. } => {
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
                StackEntry::TriggeredAbility { source, effect, .. } => {
                    info!("  -> top is TriggeredAbility, effect={:?}", effect);
                    self.current_source_card = source;
                    self.handle_effect(effect);
                    self.current_source_card = None;
                }
                StackEntry::ActivatedAbility { source, ability, .. } => {
                    info!("  -> top is ActivatedAbility, effect={:?}", ability.effect);
                    self.current_source_card = Some(source);
                    self.handle_effect(ability.effect.clone());
                    self.current_source_card = None;
                }
                StackEntry::Spell { card, .. } => {
                    info!("  -> top is Spell '{}', (just popping, not auto-resolving).", card.name);
                }
            }
        } else {
            debug!("  -> stack is empty, nothing to pop.");
        }
    }

}
