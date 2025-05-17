// src/app/gre/effect_resolution.rs

use tracing::{debug, info, warn};
use crate::app::card_attribute::{Effect, TargetFilter, Duration, Amount, PlayerSelector, TriggeredEffectAttribute, Trigger, CounterType};
use crate::app::card_library::{Card, CardType, Creature, ManaCost};
use crate::app::card_library::CardTypeFlags;
use crate::app::game_state::{GamePhase, GameEvent, Player};
use crate::app::gre::Gre;
use crate::app::gre::stack::StackEntry;
use crate::app::gre::gre_structs::DelayedEffect;
use crate::app::gre::gre_structs::ReplacementEffect;

/// Végső effectkezelő: Replacement + Continuous + Execute
impl Gre {
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

    pub fn apply_replacement(&self, effect: &Effect, idx: usize) -> Vec<Effect> {
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
            Effect::ModifyStats {
                power_delta,
                toughness_delta,
                duration,
                target,
            } => {
                match (duration, target) {
                    (Duration::EndOfTurn, TargetFilter::ExactCardID(cid)) => {
                        // 1) Kikeressük a battlefield_creatures mapből:
                        if let Some(card) = self.battlefield_creatures.get_mut(&cid) {
                            // Ha ez creature, hozzáadjuk az ephemeral buffot:
                            if let CardType::Creature(ref mut cr) = card.card_type {
                                // Ha még nincs ephemeral mező, bővítsd a Creature structot lásd lent
                                cr.ephemeral_power += power_delta;
                                cr.ephemeral_toughness += toughness_delta;
                                info!("  -> Ideiglenes buff: +({}/{}) a kör végéig '{}' (id={}) lénnyel.",
                                  power_delta, toughness_delta, card.name, cid);
                            }
                            // 2) Ütemezzük a visszavonást a kör végére:
                            let revert_effect = Effect::ModifyStats {
                                power_delta: -power_delta,
                                toughness_delta: -toughness_delta,
                                duration: Duration::Permanent, // fixen csökkentjük majd
                                target: TargetFilter::ExactCardID(cid),
                            };
                            self.schedule_delayed(revert_effect, GamePhase::End, vec![]);
                        }
                    }

                    (Duration::Permanent, TargetFilter::ExactCardID(cid)) => {
                        // Ez a "maradandó" stat-módosítás (pl. CreateEnchantmentToken)
                        if let Some(card) = self.battlefield_creatures.get_mut(&cid) {
                            if let CardType::Creature(ref mut cr) = card.card_type {
                                // Ráadásul itt közvetlenül a base power/toughness-t növeljük
                                cr.power += power_delta;
                                cr.toughness += toughness_delta;
                                info!("  -> Permanent stat change: +({}/{}) '{}'(id={})",
                                  power_delta, toughness_delta, card.name, cid);
                            }
                        }
                    }

                    (_, _) => {
                        // minden más esetet logolj, vagy hagyd üresen
                        info!("ModifyStats: ismeretlen target/duration, átugorjuk.");
                    }
                }
            }
            Effect::CreateEnchantmentToken {
                name,
                power_buff,
                toughness_buff,
                ability,
            } => {
                info!("CreateEnchantmentToken effect detected: name='{}', buff=({}/{}) ability={:?}",
                      name, power_buff, toughness_buff, ability);

                // Első lépés: van-e target a stack tetején?
                if let Some(target_card) = Gre::current_stack_target(self) {
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

            Effect::AddCounter { counter, amount, target } => {
                match target {
                    TargetFilter::ExactCardID(cid) => {
                        if let Some(card) = self.battlefield_creatures.get_mut(&cid) {
                            if let CardType::Creature(ref mut cr) = card.card_type {
                                match counter {
                                    CounterType::PlusOnePlusOne => {
                                        // pl. növeled a creature base stats–át
                                        // VAGY tárolsz egy plus_one_counters: i32 mezőt, stb.
                                        cr.power += amount as i32;
                                        cr.toughness += amount as i32;
                                        info!("'{}' kap {} db +1/+1 countert => most base {}/{}",
                                    card.name, amount, cr.power, cr.toughness);
                                    }
                                    _ => { /* loyalty counters, stb. ha akarsz */ }
                                }
                            }
                        }
                    }
                    _ => {
                        warn!("AddCounter: nem ExactCardID, átugorjuk");
                    }
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
                if let Some(target_card) = Gre::current_stack_target(self) {
                    info!("  Stack célpontja: '{}' (id={})", target_card.name, target_card.card_id);
                    for (i, subeff) in sub_effects.into_iter().enumerate() {
                        debug!("    Feldolgozzuk a(z) {}. sub_effectet: {:?}", i + 1, subeff);
                        let replaced = replace_targeted_filter_with_exact(self, subeff, &target_card);
                        self.handle_effect(replaced);
                    }
                } else {
                    warn!("  Nincs target_creature, átugorjuk a sub_effects végrehajtást.");
                }
            }

            Effect::WhenTargetDiesThisTurn { effect } => {
                info!("WhenTargetDiesThisTurn effect: belső effect = {:?}", effect);
                // Megnézzük, van-e target
                if let Some(target_card) = Gre::current_stack_target(self) {
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
                    let cloned = Card::clone_card(
                        src,
                        Some(1),
                        Some(1),
                        Some(CardTypeFlags::TOKEN),
                    );
                    Gre::create_clone_card(self, cloned);
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
}

pub fn replace_targeted_filter_with_exact(gre: &Gre, effect: Effect, tcard: &Card) -> Effect {
    debug!(
        "replace_targeted_filter_with_exact() - Start: effect={:?}, tcard='{}'(id={})",
        effect,
        tcard.name,
        tcard.card_id
    );

    let result = match effect {
        Effect::ModifyStats {
            power_delta,
            toughness_delta,
            duration,
            target,
        } => {
            debug!("  → ModifyStats branch. Eredeti target={:?}", target);

            let new_t = if target == TargetFilter::Creature {
                info!("    TargetFilter::Creature → lecseréljük ExactCardID({})-re", tcard.card_id);
                TargetFilter::ExactCardID(tcard.card_id)
            } else {
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

            let replaced_subs = sub_effects
                .into_iter()
                .map(|sub| {
                    debug!("    TargetedEffects - belső sub: {:?}", sub);
                    crate::app::gre::effect_resolution::replace_targeted_filter_with_exact(gre, sub, tcard)
                })
                .collect();

            Effect::TargetedEffects {
                sub_effects: replaced_subs
            }
        }

        Effect::WhenTargetDiesThisTurn { effect } => {
            debug!("  → WhenTargetDiesThisTurn, effect={:?}", effect);
            Effect::WhenTargetDiesThisTurn { effect }
        }

        other => {
            debug!("  → Egyéb effect, nincs célcserélés: {:?}", other);
            other
        }
    };

    debug!("replace_targeted_filter_with_exact() - End: returning={:?}", result);
    result
}