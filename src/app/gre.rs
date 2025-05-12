use std::collections::{BinaryHeap, HashMap, HashSet};
use crate::app::game_state::{GameEvent, GamePhase, Player};
use crate::app::card_attribute::{Effect, Trigger, TargetFilter, PlayerSelector, Duration, Condition, Amount, OffspringAttribute, CreatureType};
use crate::app::card_library::{Card, CardType, Creature, ManaCost};
use crate::app::card_library::CardTypeFlags;
use tracing::info;

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

#[derive(Debug, Clone)]
struct ReplacementEffect {
    priority: u8,
    f: Box<dyn Fn(&Effect) -> Option<Vec<Effect>>>,
}

/// Game Rules Engine
pub struct Gre {
    /// A stack
    pub stack: BinaryHeap<PriorityEntry>,
    /// Késleltetett effektek
    pub delayed: Vec<DelayedEffect>,
    pub executed_delayed: HashSet<usize>,
    pub next_id: usize,
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
    pub battlefield_creatures: HashMap<String, Card>,

    pub death_triggers_this_turn: Vec<(Card, Effect)>,

    /// Amikor `handle_effect`‐et hívjuk pl. egy TriggeredAbility-ből,
    /// ebbe a mezőbe tesszük be ideiglenesen a `source` kártyát,
    /// hogy az Offspring / CreateToken tudjon rá hivatkozni.
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
        // pl. nullázás
        self.opponent_lost_life_this_turn = false;
        self.us_lost_life_this_turn = false;
        self.death_triggers_this_turn.clear();
        // Lepasszolhatnánk a "battlefield_creatures" kártyáknak,
        // hogy (activated_this_turn = false)
        for (_name, card) in self.battlefield_creatures.iter_mut() {
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
        self.push_to_stack(StackEntry::ActivatedAbility { source, ability, controller });
    }

    /// Delayed effectek futtatása a megfelelő fázisban
    pub fn dispatch_delayed(&mut self, current_phase: GamePhase) {
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

    /// Események (pl. OnCastResolved) kiváltása a battlefielden lévő kártyákra
    pub fn trigger_event(&mut self, event: GameEvent, battlefield: &mut Vec<Card>, controller: Player) {
        info!("Firing event: {:?}", event);
        if let GameEvent::CreatureDied(died_card) = event {
            // Kikeressük, mely death-triggereink vonatkoznak erre a kártyára
            let mut indices_to_remove = Vec::new();

            for (i, (tracked_creature, eff)) in self.death_triggers_this_turn.iter().enumerate() {

                // most teljes eq: a died_card == tracked_creature
                if *tracked_creature == died_card {
                    // lefuttatjuk a tárolt effectet
                    self.handle_effect(eff.clone());
                    indices_to_remove.push(i);
                }
            }
            // most töröljük hátulról
            for &i in indices_to_remove.iter().rev() {
                self.death_triggers_this_turn.remove(i);
            }
        }
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
            for eff in effects {
                batch.push((card.clone(), eff));
            }
        }

        self.reset_priority();

        // Minden effectet TriggeredAbility formában tolunk a stackre,
        // elmentve a forrást "Some(card)".
        for (source_card, eff) in batch {
            // delayed effect?
            match eff {
                Effect::Delayed { effect, phase, deps } => {
                    let id = self.schedule_delayed(*effect.clone(), phase, deps.clone());
                    info!("Scheduled delayed effect id {} from trigger", id);
                }
                e => {
                    let prio = match &e {
                        Effect::ModifyStats { .. } | Effect::Proliferate { .. } => 2,
                        _ => 1,
                    };
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

    /// **Beléptetünk** egy kártyát a battlefieldre (a GRE belső track-jébe),
    /// majd lefuttatjuk rajta az OnEnterBattlefield triggert.
    pub fn enter_battlefield(&mut self, card: &mut Card) {
        let card_name = card.name.clone();

        // 1) betesszük a belső táblába
        self.battlefield_creatures.insert(card_name.clone(), card.clone());

        // 2) OnEnterBattlefield triggerek futtatása
        let effects = card.trigger_by(&Trigger::OnEnterBattlefield { filter: TargetFilter::SelfCard });
        for eff in effects {
            self.handle_effect(eff);
        }
    }

    /// A "fő" effectkezelő, replacement + continuous effektekkel
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
        // 1) új Card, Creature + paraméteres power/toughness
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
            // és a bitflag: CREATURE | TOKEN
            .with_added_type(CardTypeFlags::CREATURE)
            .with_added_type(CardTypeFlags::TOKEN);

        // 2) Hozzáadjuk a battlefieldhez (GRE belső track), OnEnterBattlefield
        self.enter_battlefield(&mut new_card);
    }

    /// A klónozott (vagy egyéb) kártyát beteszi a battlefieldre,
    /// lefuttatva az OnEnterBattlefield triggert is.
    pub fn create_clone_card(&mut self, mut cloned: Card) {
        self.enter_battlefield(&mut cloned);
    }
    /// Példa: megnézzük a stack tetején lévő bejegyzést (vagy a “current_source_card”-ot),
    /// s ha Spell { target_creature: Some(x),..}, azt visszaadjuk.
    fn current_stack_target(&self) -> Option<Card> {
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
    fn replace_targeted_filter_with_exact(&self, effect: Effect, tcard: &Card) -> Effect {
        match effect {
            Effect::ModifyStats { power_delta, toughness_delta, duration, target } => {
                // Ha valamilyen target = Creature, akkor cseréljük ExactCard-ra
                let new_t = if target == TargetFilter::Creature {
                    TargetFilter::ExactCard(tcard.clone())
                } else {
                    target
                };
                Effect::ModifyStats { power_delta, toughness_delta, duration, target: new_t }
            }
            Effect::GrantAbility { ability, duration, target } => {
                let new_t = if target == TargetFilter::Creature {
                    TargetFilter::ExactCard(tcard.clone())
                } else {
                    target
                };
                Effect::GrantAbility { ability, duration, target: new_t }
            }
            Effect::TargetedEffects { sub_effects } => {
                // Maradhat a rekurzív feldolgozás is, ha kell
                let replaced_subs = sub_effects.into_iter()
                    .map(|sub| self.replace_targeted_filter_with_exact(sub, tcard))
                    .collect();
                Effect::TargetedEffects { sub_effects: replaced_subs }
            }
            Effect::WhenTargetDiesThisTurn { effect } => {
                // marad ugyanaz - a “Which creature?” is a stack->target, nem a filter mezőben
                Effect::WhenTargetDiesThisTurn { effect }
            }
            // minden más effect mehet simán vissza
            other => other,
        }
    }


    /// A tényleges "egy effect" végrehajtása
    pub fn execute(&mut self, effect: Effect) {
        match effect {
            Effect::TargetedEffects { sub_effects } => {
                // 1) Kikeressük a “most resolving” stack entry-ből a targetet
                let maybe_target = self.current_stack_target();
                // (lásd lentebb “fn current_stack_target() -> Option<Card>”)
                if let Some(target_card) = maybe_target {
                    // 2) A sub_effects mindegyikén lecseréljük a `TargetFilter::TargetedCreature`
                    //    -> `TargetFilter::ExactCard(target_card.clone())`
                    for subeff in sub_effects {
                        let replaced = self.replace_targeted_filter_with_exact(subeff, &target_card);
                        self.handle_effect(replaced);
                    }
                } else {
                    info!("No target_creature found for TargetedEffects, skipping.");
                }
            }

            Effect::WhenTargetDiesThisTurn { effect } => {
                let maybe_target = self.current_stack_target();
                if let Some(target_card) = maybe_target {
                    // Ezt eltároljuk a halál-figyelők közé
                    self.death_triggers_this_turn.push((target_card.clone(), *effect));
                    info!("Registered a death-trigger for target '{}'", target_card.name);
                } else {
                    info!("No target found for WhenTargetDiesThisTurn, skipping.");
                }
            }

            Effect::Offspring { cost } => {
                info!("** Offspring effect resolved, cost = {} **", cost);
                if let Some(ref src) = self.current_source_card {
                    // 1/1-es klónozás, TOKEN bitflag hozzáadva
                    let cloned = self.clone_card(
                        src,
                        Some(1),
                        Some(1),
                        Some(CardTypeFlags::TOKEN),
                    );
                    self.create_clone_card(cloned);
                } else {
                    info!("No current_source_card - Offspring had no source to clone.");
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

    /// A stacket teljesen feloldjuk
    pub fn resolve_stack(&mut self) {
        while let Some(pe) = self.stack.pop() {
            info!("Resolving {:?}", pe.entry);
            match pe.entry {
                StackEntry::Spell { card, controller } => {
                    info!("Resolving spell: '{}'", card.name);

                    // 1) A kijátszott lény "valójában" a battlefieldre kerülne:
                    //    Ehhez pl. enter_battlefield(&mut card).
                    //    Ha OnEnterBattlefield van, az lefut.
                    let mut c = card.clone();
                    self.enter_battlefield(&mut c);

                    // 2) OnCastResolved triggerek is
                    self.trigger_event(
                        GameEvent::SpellResolved(card.name.clone()),
                        &mut Vec::new(), // ide pl. a stacken kívüli permanenseket is beírhatnánk
                        controller,
                    );
                }

                StackEntry::TriggeredAbility { source, effect, .. } => {
                    // Forrás-lapot elmentjük a current_source_card-ba,
                    // hogy Offspring / CreateToken is tudja, mihez klónozza a tokent
                    self.current_source_card = source;
                    self.handle_effect(effect);
                    // Takarítás
                    self.current_source_card = None;
                }

                StackEntry::ActivatedAbility { source, ability, .. } => {
                    self.current_source_card = Some(source);
                    self.handle_effect(ability.effect.clone());
                    self.current_source_card = None;
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
                StackEntry::TriggeredAbility { source, effect, .. } => {
                    self.current_source_card = source;
                    self.handle_effect(effect);
                    self.current_source_card = None;
                }
                StackEntry::ActivatedAbility { source, ability, .. } => {
                    self.current_source_card = Some(source);
                    self.handle_effect(ability.effect.clone());
                    self.current_source_card = None;
                }
                StackEntry::Spell { card, .. } => {
                    info!("Spell popped from stack: '{}'", card.name);
                    // akár ide is tehetünk logikát
                }
            }
        }
    }
}
