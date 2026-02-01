// src/app/gre/trigger.rs

use crate::app::card_attribute::TargetFilter;
use crate::app::card_attribute::{Duration, PlayerSelector};
use crate::app::card_attribute::{Effect, Trigger};
use crate::app::card_library::Card;
use crate::app::card_library::CardType::Creature;
use crate::app::card_library::CardTypeFlags;
use crate::app::game_state::{GameEvent, GamePhase, Player};
use crate::app::gre::Gre; // hivatkozunk a Gre struktúrára
use crate::app::gre::gre_structs::DelayedEffect;
use crate::app::gre::gre_structs::ReplacementEffect;
use crate::app::gre::stack::StackEntry;
use tracing::{debug, info, warn};

impl Gre {
    /// Események (pl. OnCastResolved) kiváltása a battlefielden lévő kártyákra
    pub fn trigger_event(
        &mut self,
        event: GameEvent,
        battlefield: &mut Vec<Card>,
        controller: Player,
    ) {
        info!(
            "trigger_event() -> event={:?}, controller={:?}, bf_len={}",
            event,
            controller,
            battlefield.len()
        );

        // Ha CreatureDied, nézzük meg a death_triggers_this_turn listát
        if let GameEvent::CreatureDied(ref died_card) = event {
            info!(
                "  Checking death_triggers_this_turn, died_card='{}'(id={})",
                died_card.name, died_card.card_id
            );
            let mut to_trigger = Vec::new();
            let mut indices_to_remove = Vec::new();

            for (i, (tracked_creature, eff)) in self.death_triggers_this_turn.iter().enumerate() {
                if tracked_creature == died_card {
                    debug!("    -> Found a death-trigger effect: {:?}", eff);
                    to_trigger.push(eff.clone());
                    indices_to_remove.push(i);
                }
            }
            // Lefuttatjuk
            for eff in to_trigger {
                debug!("    handle_effect from death_triggers_this_turn: {:?}", eff);
                self.handle_effect(eff);
            }
            // Töröljük
            for &i in indices_to_remove.iter().rev() {
                self.death_triggers_this_turn.remove(i);
            }
        }
        if let GameEvent::Targeted(tid) = event {
            let mut batch = Vec::new();
            // Végigmegyünk a GRE saját 'battlefield_creatures' mapjén
            // (ez a te belső nyilvántartásod a lényekről).
            for (_id, c) in self.battlefield_creatures.iter_mut() {
                if c.card_id == tid {
                    // Ha a kártyában van OnTargetedFirstTimeEachTurn { filter: SelfCard },
                    // akkor ez a "trigger_by(...)" hívás lekéri az effect(ek)et.
                    let triggered_effects = c.trigger_by(&Trigger::OnTargetedFirstTimeEachTurn {
                        filter: TargetFilter::SelfCard,
                    });
                    for eff in triggered_effects {
                        batch.push((c.clone(), eff));
                    }
                }
            }
            // A begyűjtött effecteket stackre tesszük TriggeredAbility formájában:
            for (source_card, eff) in batch {
                self.push_to_stack(StackEntry::TriggeredAbility {
                    source: Some(source_card),
                    effect: eff,
                    controller,
                });
            }
        }
        if let GameEvent::ManaAdded(id) = event {
            let mut batch = Vec::new();
            for (_cid, c) in self.battlefield_creatures.iter_mut() {
                if c.card_id == id {
                    let effects = c.trigger_by(&Trigger::OnAddMana {
                        filter: TargetFilter::SelfCard,
                    });
                    for eff in effects {
                        batch.push((c.clone(), eff));
                    }
                }
                if c.card_id != id {
                    let effects = c.trigger_by(&Trigger::OnAddMana {
                        filter: TargetFilter::ControllerCreature,
                    });
                    for eff in effects {
                        batch.push((c.clone(), eff));
                    }
                }
            }
            for (source_card, eff) in batch {
                match eff {
                    Effect::Delayed {
                        effect,
                        phase,
                        deps,
                    } => {
                        let id = self.schedule_delayed(*effect.clone(), phase, deps.clone());
                        info!(
                            "    -> Scheduled delayed effect #{} from OnAddMana trigger",
                            id
                        );
                    }
                    e => {
                        let prio = match &e {
                            Effect::ModifyStats { .. } | Effect::Proliferate { .. } => 2,
                            _ => 1,
                        };
                        info!(
                            "    -> Pushing TriggeredAbility to stack (prio={}), effect={:?}",
                            prio, e
                        );
                        self.push(
                            StackEntry::TriggeredAbility {
                                source: Some(source_card.clone()),
                                effect: e,
                                controller,
                            },
                            prio,
                        );
                    }
                }
            }
        }
        if let GameEvent::CounterAdded(id, count) = event {
            let mut batch = Vec::new();
            for (_cid, c) in self.battlefield_creatures.iter_mut() {
                if c.card_id == id {
                    let effects = c.trigger_by(&Trigger::OnCounterAdded {
                        filter: TargetFilter::SelfCard,
                    });
                    for eff in effects {
                        batch.push((c.clone(), eff));
                    }
                }
                if self.battlefield_creatures.contains_key(&id) {
                    let effects = c.trigger_by(&Trigger::OnCounterAdded {
                        filter: TargetFilter::ControllerCreature,
                    });
                    for eff in effects {
                        batch.push((c.clone(), eff));
                    }
                }
            }
            for (source_card, eff) in batch {
                let mut effect_to_push = eff.clone();
                if let Effect::DrawCardsCounted = eff {
                    let n = count;
                    info!(
                        "    Converting DrawCardsCounted to DrawCards({}) for '{}'",
                        n, source_card.name
                    );
                    effect_to_push = Effect::DrawCards {
                        count: n,
                        player: PlayerSelector::Controller,
                    };
                }
                match effect_to_push {
                    Effect::Delayed {
                        effect,
                        phase,
                        deps,
                    } => {
                        let id = self.schedule_delayed(*effect.clone(), phase, deps.clone());
                        info!(
                            "    -> Scheduled delayed effect #{} from OnCounterAdded trigger",
                            id
                        );
                    }
                    e => {
                        let prio = match &e {
                            Effect::ModifyStats { .. } | Effect::Proliferate { .. } => 2,
                            _ => 1,
                        };
                        info!(
                            "    -> Pushing TriggeredAbility to stack (prio={}), effect={:?}",
                            prio, e
                        );
                        self.push(
                            StackEntry::TriggeredAbility {
                                source: Some(source_card.clone()),
                                effect: e,
                                controller,
                            },
                            prio,
                        );
                    }
                }
            }
        }

        // A battlefield kártyáin végigmegyünk
        let mut batch = Vec::new();
        for card in battlefield.iter_mut() {
            let effects = match &event {
                GameEvent::SpellResolved(_spell_name) => {
                    card.trigger_by(&crate::app::card_attribute::Trigger::OnCastResolved)
                }
                GameEvent::CreatureDied(_) => {
                    card.trigger_by(&crate::app::card_attribute::Trigger::OnDeath {
                        filter: TargetFilter::SelfCard,
                    })
                }
                GameEvent::TurnEnded => {
                    card.trigger_by(&crate::app::card_attribute::Trigger::AtPhase {
                        phase: GamePhase::End,
                        player: PlayerSelector::AnyPlayer,
                    })
                }
                GameEvent::PhaseChange(p) => {
                    card.trigger_by(&crate::app::card_attribute::Trigger::AtPhase {
                        phase: *p,
                        player: PlayerSelector::AnyPlayer,
                    })
                }
                _ => Vec::new(),
            };
            if !effects.is_empty() {
                debug!(
                    "  Card '{}': {} trigger-effect(s)",
                    card.name,
                    effects.len()
                );
            }
            for eff in effects {
                debug!("    effect => {:?}", eff);
                batch.push((card.clone(), eff));
            }
        }

        self.reset_priority();

        // A begyűjtött effectek stackre rakása / delayed schedule
        for (source_card, eff) in batch {
            match eff {
                Effect::Delayed {
                    effect,
                    phase,
                    deps,
                } => {
                    let id = self.schedule_delayed(*effect.clone(), phase, deps.clone());
                    info!(
                        "    -> Scheduled delayed effect #{} from normal trigger",
                        id
                    );
                }
                e => {
                    let prio = match &e {
                        Effect::ModifyStats { .. } | Effect::Proliferate { .. } => 2,
                        _ => 1,
                    };
                    info!(
                        "    -> Pushing TriggeredAbility to stack (prio={}), effect={:?}",
                        prio, e
                    );
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

    /// BFS/DFS jellegű trigger-lánc bejárás a belső `battlefield_creatures` map-en
    pub fn trigger_event_tree(&mut self, event: GameEvent, controller: Player) {
        info!(
            "trigger_event_tree() -> event={:?}, BFS/DFS-based. Searching root permanents...",
            event
        );

        let root_ids: Vec<u64> = self
            .battlefield_creatures
            .values()
            .filter(|c| c.attached_to.is_none())
            .map(|c| c.card_id)
            .collect();

        debug!("  Found {} root(s): {:?}", root_ids.len(), root_ids);

        for rid in root_ids {
            self.traverse_trigger_tree(rid, &event, controller);
        }

        self.reset_priority();
    }

    pub fn traverse_trigger_tree(&mut self, card_id: u64, event: &GameEvent, controller: Player) {
        debug!("    traverse_trigger_tree() -> card_id={}", card_id);

        let mut card = if let Some(card) = self.battlefield_creatures.remove(&card_id) {
            card
        } else {
            debug!("      -> Card not found in battlefield_creatures, returning.");
            return;
        };

        let triggered_effects = self.event_to_triggers(event, &mut card);

        if !triggered_effects.is_empty() {
            debug!(
                "      -> card '{}' triggered {} effect(s)",
                card.name,
                triggered_effects.len()
            );
        }

        for eff in triggered_effects {
            match eff {
                Effect::Delayed {
                    effect,
                    phase,
                    deps,
                } => {
                    let id = self.schedule_delayed(*effect.clone(), phase, deps);
                    info!(
                        "        Scheduled delayed effect #{} from traverse_trigger_tree",
                        id
                    );
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

        debug!(
            "      -> inserting card '{}' (id={}) back to battlefield_creatures",
            card.name, card_id
        );
        self.battlefield_creatures.insert(card_id, card);

        let child_ids: Vec<u64> = self
            .battlefield_creatures
            .values()
            .filter(|c2| c2.attached_to == Some(card_id))
            .map(|c2| c2.card_id)
            .collect();

        debug!(
            "      -> found {} child(ren): {:?}",
            child_ids.len(),
            child_ids
        );
        for cid in child_ids {
            self.traverse_trigger_tree(cid, event, controller);
        }
    }

    fn event_to_triggers(&mut self, event: &GameEvent, card: &mut Card) -> Vec<Effect> {
        debug!(
            "        event_to_triggers(): event={:?}, card='{}'",
            event, card.name
        );
        let res = match event {
            GameEvent::SpellResolved(_spell_name) => {
                card.trigger_by(&crate::app::card_attribute::Trigger::OnCastResolved)
            }
            GameEvent::CreatureDied(died_card) => {
                if died_card.card_id == card.card_id {
                    card.trigger_by(&crate::app::card_attribute::Trigger::OnDeath {
                        filter: TargetFilter::SelfCard,
                    })
                } else {
                    Vec::new()
                }
            }
            GameEvent::TurnEnded => {
                card.trigger_by(&crate::app::card_attribute::Trigger::AtPhase {
                    phase: GamePhase::End,
                    player: PlayerSelector::AnyPlayer,
                })
            }
            GameEvent::PhaseChange(p) => {
                card.trigger_by(&crate::app::card_attribute::Trigger::AtPhase {
                    phase: *p,
                    player: PlayerSelector::AnyPlayer,
                })
            }
            _ => Vec::new(),
        };

        if !res.is_empty() {
            debug!(
                "          -> card '{}' returned {} effect(s)",
                card.name,
                res.len()
            );
        }
        res
    }
}
