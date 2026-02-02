// src/app/gre/effect_resolution.rs

use crate::app::card_attribute::{
    Amount, Condition, CounterType, Duration, Effect, PlayerSelector, TargetFilter, Trigger,
    TriggeredEffectAttribute,
};
use crate::app::card_library::CardTypeFlags;
use crate::app::card_library::{Card, CardType, Creature, ManaCost};
use crate::app::game_state::{GameEvent, GamePhase, Player};
use crate::app::gre::Gre;
use crate::app::gre::gre_structs::DelayedEffect;
use crate::app::gre::gre_structs::ReplacementEffect;
use crate::app::gre::stack::StackEntry;
use tracing::{debug, info, warn};

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
            repls
                .into_iter()
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
                                info!(
                                    "  -> Ideiglenes buff: +({}/{}) a kör végéig '{}' (id={}) lénnyel.",
                                    power_delta, toughness_delta, card.name, cid
                                );
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
                                info!(
                                    "  -> Permanent stat change: +({}/{}) '{}'(id={})",
                                    power_delta, toughness_delta, card.name, cid
                                );
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
                info!(
                    "CreateEnchantmentToken effect detected: name='{}', buff=({}/{}) ability={:?}",
                    name, power_buff, toughness_buff, ability
                );

                // Első lépés: van-e target a stack tetején?
                if let Some(target_card) = Gre::current_stack_target(self) {
                    info!(
                        "  Target megtalálva: '{}' (id={})",
                        target_card.name, target_card.card_id
                    );

                    // Létrehozunk egy token card-ot
                    let mut aura_card = Card::new(&name, CardType::Enchantment, ManaCost::free());
                    aura_card.type_flags |= CardTypeFlags::TOKEN;

                    // Rácsatoljuk a megcélzott creature-re
                    aura_card.attached_to = Some(target_card.card_id);

                    // Hozzáadjuk az OnEnterBattlefield és OnDeath triggert a buff eltávolításához
                    debug!(
                        "  Létrehozott 'aura_card' token, csatoljuk a target_card-hoz, \
                             beállítjuk a TriggeredEffectAttribute-kat + buff-hatásokat."
                    );

                    // 1) OnEnterBattlefield
                    aura_card.triggers.push(Trigger::OnEnterBattlefield {
                        filter: TargetFilter::SelfCard,
                    });
                    aura_card
                        .attributes
                        .push(Box::new(TriggeredEffectAttribute {
                            trigger: Trigger::OnEnterBattlefield {
                                filter: TargetFilter::SelfCard,
                            },
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
                        }));

                    // 2) OnDeath => -1/-1, RemoveAbility(Trample)
                    aura_card.triggers.push(Trigger::OnDeath {
                        filter: TargetFilter::SelfCard,
                    });
                    aura_card
                        .attributes
                        .push(Box::new(TriggeredEffectAttribute {
                            trigger: Trigger::OnDeath {
                                filter: TargetFilter::SelfCard,
                            },
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
                        }));

                    debug!("  Token attribute-ok hozzáadva, mielőtt belép a battlefieldre.");

                    // Végül berakjuk a battlefieldre
                    self.enter_battlefield(&mut aura_card);
                    info!(
                        "'{}' enchantment token létrehozva és a(z) '{}' lényhez csatolva.",
                        name, target_card.name
                    );
                } else {
                    warn!("  Nincs target a CreateEnchantmentToken-höz, kihagyjuk.");
                }
            }

            Effect::AddCounter {
                counter,
                amount,
                target,
            } => {
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
                                        info!(
                                            "'{}' kap {} db +1/+1 countert => most base {}/{}",
                                            card.name, amount, cr.power, cr.toughness
                                        );
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
                info!(
                    "RemoveAbility effect: ability={:?}, target={:?}",
                    ability, target
                );
                match target {
                    TargetFilter::ExactCardID(id) => {
                        info!(
                            "  RemoveAbility – megpróbáljuk kikeresni a battlefielden card_id={}",
                            id
                        );
                        if let Some(mut c) = self.battlefield_creatures.get_mut(&id) {
                            info!(
                                "  Megtaláltuk a kártyát ('{}', id={}), abilities törlése.",
                                c.name, c.card_id
                            );
                            if let CardType::Creature(ref mut cr) = c.card_type {
                                let before_len = cr.abilities.len();
                                cr.abilities.retain(|&a| a != ability);
                                let after_len = cr.abilities.len();
                                debug!(
                                    "  {} -> {} ability maradt ({} törölve).",
                                    before_len,
                                    after_len,
                                    before_len - after_len
                                );
                            }
                        } else {
                            warn!(
                                "  Nem található creature az id={} értéken, effect sikertelen.",
                                id
                            );
                        }
                    }
                    _ => {
                        warn!("RemoveAbility target nem ExactCardID, átugorjuk.");
                    }
                }
            }
            Effect::AddMana {
                colorless,
                red,
                blue,
                green,
                black,
                white,
            } => {
                info!(
                    "AddMana effect: adding mana -> +({}, {}, {}, {}, {}, {})",
                    colorless, red, blue, green, black, white
                );
                if let Some(src) = &self.current_source_card {
                    self.trigger_event(
                        GameEvent::ManaAdded(src.card_id),
                        &mut Vec::new(),
                        Player::Us,
                    );
                } else {
                    warn!("AddMana: no source card for mana effect");
                }
            }
            Effect::Damage { amount, target } => {
                let damage_value = match amount {
                    Amount::Fixed(v) => v,
                    Amount::SourcePower => {
                        if let Some(ref src) = self.current_source_card {
                            if let CardType::Creature(cr) = &src.card_type {
                                cr.power + cr.ephemeral_power
                            } else {
                                0
                            }
                        } else {
                            0
                        }
                    }
                    Amount::SourceToughness => {
                        if let Some(ref src) = self.current_source_card {
                            if let CardType::Creature(cr) = &src.card_type {
                                cr.toughness + cr.ephemeral_toughness
                            } else {
                                0
                            }
                        } else {
                            0
                        }
                    }
                };
                info!("Damage effect: amount={} target={:?}", damage_value, target);
                match target {
                    TargetFilter::ExactCardID(cid) => {
                        if let Some(card) = self.battlefield_creatures.get_mut(&cid) {
                            if let CardType::Creature(ref mut cr) = card.card_type {
                                info!(
                                    "  Dealing {} damage to creature '{}' (id={})",
                                    damage_value, card.name, cid
                                );
                                if damage_value >= cr.toughness + cr.ephemeral_toughness {
                                    info!("  -> Lethal damage, '{}'(id={}) dies", card.name, cid);
                                    let dead_card = card.clone();
                                    self.battlefield_creatures.remove(&cid);
                                    self.trigger_event(
                                        GameEvent::CreatureDied(dead_card),
                                        &mut Vec::new(),
                                        Player::Us,
                                    );
                                }
                            }
                        }
                    }
                    TargetFilter::Player => {
                        info!("  Damage to player: {}", damage_value);
                        self.opponent_lost_life_this_turn = true;
                    }
                    _ => {
                        info!("  Damage effect with unsupported target: {:?}", target);
                    }
                }
            }
            Effect::DamageByTargetPower { source, target } => {
                info!(
                    "DamageByTargetPower effect: source={:?}, target={:?}",
                    source, target
                );
                let mut dmg_amount = 0;
                if let TargetFilter::ExactCardID(src_id) = source {
                    if let Some(src_card) = self.battlefield_creatures.get(&src_id) {
                        if let CardType::Creature(cr) = &src_card.card_type {
                            dmg_amount = cr.power + cr.ephemeral_power;
                            info!(
                                "  Source creature '{}' (id={}) power={} -> damage {}",
                                src_card.name, src_id, cr.power, dmg_amount
                            );
                        }
                    }
                }
                if dmg_amount == 0 {
                    if let Some(src_card) = self
                        .battlefield_creatures
                        .values()
                        .find(|c| matches!(c.card_type, CardType::Creature(_)))
                    {
                        if let CardType::Creature(cr) = &src_card.card_type {
                            dmg_amount = cr.power + cr.ephemeral_power;
                            info!(
                                "  Using '{}' as source, damage={}",
                                src_card.name, dmg_amount
                            );
                        }
                    }
                }
                if dmg_amount > 0 {
                    if let TargetFilter::ExactCardID(tid) = target {
                        if let Some(tgt_card) = self.battlefield_creatures.get_mut(&tid) {
                            if let CardType::Creature(ref mut tgt_cr) = tgt_card.card_type {
                                info!(
                                    "  Dealing {} damage to '{}' (id={})",
                                    dmg_amount, tgt_card.name, tid
                                );
                                if dmg_amount >= tgt_cr.toughness + tgt_cr.ephemeral_toughness {
                                    info!("  -> Lethal damage, target dies");
                                    let dead_card = tgt_card.clone();
                                    self.battlefield_creatures.remove(&tid);
                                    self.trigger_event(
                                        GameEvent::CreatureDied(dead_card),
                                        &mut Vec::new(),
                                        Player::Us,
                                    );
                                }
                            }
                        }
                    } else {
                        info!("  No target creature specified, skipping damage");
                    }
                } else {
                    info!("  No source creature power determined, skipping");
                }
            }
            Effect::TapTarget { target } => {
                info!("TapTarget effect: target={:?}", target);
                match target {
                    TargetFilter::ExactCardID(tid) => {
                        if let Some(card) = self.battlefield_creatures.get_mut(&tid) {
                            info!("  Tapping card '{}' (id={})", card.name, tid);
                            for abil in card.activated_abilities.iter_mut() {
                                abil.activated_this_turn = true;
                            }
                        }
                    }
                    TargetFilter::SelfCard => {
                        if let Some(ref src) = self.current_source_card {
                            let sid = src.card_id;
                            if let Some(card) = self.battlefield_creatures.get_mut(&sid) {
                                info!("  Tapping source card '{}' (id={})", card.name, sid);
                                for abil in card.activated_abilities.iter_mut() {
                                    abil.activated_this_turn = true;
                                }
                            }
                        }
                    }
                    TargetFilter::ControllerCreature => {
                        if let Some((_cid, card)) = self
                            .battlefield_creatures
                            .iter_mut()
                            .find(|(_, c)| matches!(c.card_type, CardType::Creature(_)))
                        {
                            info!("  Tapping card '{}' (id={})", card.name, card.card_id);
                            for abil in card.activated_abilities.iter_mut() {
                                abil.activated_this_turn = true;
                            }
                        } else {
                            warn!("  TapTarget: no ControllerCreature found to tap");
                        }
                    }
                    _ => {
                        warn!("  TapTarget: unsupported target {:?}", target);
                    }
                }
            }
            Effect::BuffAllByMaxPower { filter, duration } => {
                info!(
                    "BuffAllByMaxPower: filter={:?}, duration={:?}",
                    filter, duration
                );
                let mut max_power_val = 0;
                if let TargetFilter::ControllerCreature = filter {
                    for card in self.battlefield_creatures.values() {
                        if let CardType::Creature(cr) = &card.card_type {
                            max_power_val = max_power_val.max(cr.power + cr.ephemeral_power);
                        }
                    }
                } else {
                    warn!("  BuffAllByMaxPower: unsupported filter {:?}", filter);
                    return;
                }
                info!("  Max power among filtered creatures = {}", max_power_val);
                for (&cid, card) in self.battlefield_creatures.iter_mut() {
                    if let CardType::Creature(ref mut cr) = card.card_type {
                        match duration {
                            Duration::EndOfTurn => {
                                cr.ephemeral_power += max_power_val;
                                cr.ephemeral_toughness += max_power_val;
                                info!(
                                    "    '{}' gets +{}/+{} until end of turn",
                                    card.name, max_power_val, max_power_val
                                );
                                let revert_effect = Effect::ModifyStats {
                                    power_delta: -max_power_val,
                                    toughness_delta: -max_power_val,
                                    duration: Duration::Permanent,
                                    target: TargetFilter::ExactCardID(cid),
                                };
                                self.schedule_delayed(revert_effect, GamePhase::End, vec![]);
                            }
                            Duration::Permanent => {
                                cr.power += max_power_val;
                                cr.toughness += max_power_val;
                                info!(
                                    "    '{}' permanently gets +{}/+{}",
                                    card.name, max_power_val, max_power_val
                                );
                            }
                            _ => {
                                warn!("    BuffAllByMaxPower: unsupported duration {:?}", duration);
                            }
                        }
                    }
                }
            }
            Effect::AddCounterAll {
                counter,
                amount,
                filter,
            } => {
                info!(
                    "AddCounterAll: counter={:?}, amount={:?}, filter={:?}",
                    counter, amount, filter
                );
                let count_value = match amount {
                    Amount::Fixed(v) => v,
                    Amount::SourcePower => {
                        if let Some(ref src) = self.current_source_card {
                            if let CardType::Creature(cr) = &src.card_type {
                                cr.power + cr.ephemeral_power
                            } else {
                                0
                            }
                        } else {
                            0
                        }
                    }
                    Amount::SourceToughness => {
                        if let Some(ref src) = self.current_source_card {
                            if let CardType::Creature(cr) = &src.card_type {
                                cr.toughness + cr.ephemeral_toughness
                            } else {
                                0
                            }
                        } else {
                            0
                        }
                    }
                };
                if count_value == 0 {
                    info!("  AddCounterAll: amount resolved to 0, nothing to do");
                    return;
                }
                match filter {
                    TargetFilter::ControllerCreature => {
                        for (&cid, card) in self.battlefield_creatures.iter_mut() {
                            if let CardType::Creature(ref mut cr) = card.card_type {
                                if counter == CounterType::PlusOnePlusOne {
                                    cr.power += count_value;
                                    cr.toughness += count_value;
                                    info!(
                                        "    '{}' kap {} db +1/+1 countert => most base {}/{}",
                                        card.name, count_value, cr.power, cr.toughness
                                    );
                                } else if counter == CounterType::Loyalty {
                                    if let CardType::Planeswalker = card.card_type {
                                        info!(
                                            "    '{}' gets {} loyalty counter(s)",
                                            card.name, count_value
                                        );
                                    }
                                }
                                self.trigger_event(
                                    GameEvent::CounterAdded(cid, count_value as u32),
                                    &mut Vec::new(),
                                    Player::Us,
                                );
                            }
                        }
                    }
                    _ => {
                        warn!("AddCounterAll: unsupported filter {:?}", filter);
                    }
                }
            }
            Effect::Destroy { target } => {
                info!("Destroy effect: target={:?}", target);
                match target {
                    TargetFilter::ExactCardID(cid) => {
                        if let Some(card) = self.battlefield_creatures.remove(&cid) {
                            info!(
                                "  '{}' (id={}) destroyed and removed from battlefield",
                                card.name, cid
                            );
                            if matches!(card.card_type, CardType::Creature(_)) {
                                self.trigger_event(
                                    GameEvent::CreatureDied(card.clone()),
                                    &mut Vec::new(),
                                    Player::Us,
                                );
                            }
                        }
                    }
                    TargetFilter::Artifact => {
                        if let Some((&aid, _)) = self
                            .battlefield_creatures
                            .iter()
                            .find(|(_, c)| matches!(c.card_type, CardType::Artifact))
                        {
                            if let Some(card) = self.battlefield_creatures.remove(&aid) {
                                info!("  Artifact '{}' (id={}) destroyed", card.name, aid);
                            }
                        } else {
                            info!("  No artifact found to destroy");
                        }
                    }
                    _ => {
                        warn!("Destroy: unsupported target filter {:?}", target);
                    }
                }
            }
            Effect::Exile { target } => {
                info!("Exile effect: target={:?}", target);
                match target {
                    TargetFilter::ExactCardID(cid) => {
                        if let Some(card) = self.battlefield_creatures.remove(&cid) {
                            info!("  '{}' (id={}) exiled from battlefield", card.name, cid);
                            self.last_exiled_card_was_creature =
                                matches!(card.card_type, CardType::Creature(_));
                        }
                    }
                    TargetFilter::CardInGraveyard => {
                        info!("  Exiling a card from graveyard (assuming creature card)");
                        self.last_exiled_card_was_creature = true;
                    }
                    _ => {
                        warn!("Exile: unsupported target filter {:?}", target);
                    }
                }
            }
            Effect::Conditional {
                condition,
                effect_if_true,
                effect_if_false,
            } => {
                info!("Conditional effect: condition={:?}", condition);
                let cond_met = match condition {
                    Condition::OpponentLostLifeThisTurn => self.opponent_lost_life_this_turn,
                    Condition::FirstTimeThisTurn => true,
                    Condition::SpellWasNonCreature => false,
                    Condition::Tap => false,
                    Condition::SacrificeSelf => false,
                    Condition::Always => true,
                    Condition::SpellWasKicked => false,
                    Condition::HasCreaturePower4OrMore => {
                        self.battlefield_creatures.values().any(|c| {
                            if let CardType::Creature(cr) = &c.card_type {
                                cr.power + cr.ephemeral_power >= 4
                            } else {
                                false
                            }
                        })
                    }
                    Condition::ExiledCardWasCreature => self.last_exiled_card_was_creature,
                };
                if cond_met {
                    info!("  Condition met, executing effect_if_true");
                    self.handle_effect(*effect_if_true);
                } else {
                    info!("  Condition false");
                    if let Some(false_eff) = effect_if_false {
                        self.handle_effect(*false_eff);
                    }
                }
            }
            Effect::DrawCardsCounted => {
                info!("DrawCardsCounted effect: resolving as draw action (no direct state change)");
                // In this engine, actual card draw is not simulated beyond logging
                info!("  (DrawCardsCounted resolved)");
            }
            Effect::TargetedEffects { sub_effects } => {
                info!("TargetedEffects: sub_effects len={}", sub_effects.len());
                if let Some(target_card) = Gre::current_stack_target(self) {
                    info!(
                        "  Stack célpontja: '{}' (id={})",
                        target_card.name, target_card.card_id
                    );
                    for (i, subeff) in sub_effects.into_iter().enumerate() {
                        debug!(
                            "    Feldolgozzuk a(z) {}. sub_effectet: {:?}",
                            i + 1,
                            subeff
                        );
                        let replaced =
                            replace_targeted_filter_with_exact(self, subeff, &target_card);
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
                    info!(
                        "  Death-trigger regisztrálása a(z) '{}' kártyán.",
                        target_card.name
                    );
                    self.death_triggers_this_turn
                        .push((target_card.clone(), *effect));
                } else {
                    warn!("  Nincs target, a WhenTargetDiesThisTurn effectet nem regisztráljuk.");
                }
            }

            Effect::Offspring { cost } => {
                info!(
                    "Offspring effect: cost={}. Megnézzük a current_source_card-ot...",
                    cost
                );
                if let Some(ref src) = self.current_source_card {
                    debug!(
                        "  source_card='{}' (id={}). Készítünk belőle klónt tokenként.",
                        src.name, src.card_id
                    );
                    let cloned =
                        Card::clone_card(src, Some(1), Some(1), Some(CardTypeFlags::TOKEN));
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
                    info!(
                        "ChooseSome => a(z) {}. effectet hajtjuk végre: {:?}",
                        idx, chosen
                    );
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
                    info!(
                        "Life gain {} for {:?} meghiúsul (PreventLifeGain).",
                        amount, player
                    );
                } else {
                    info!(
                        "{:?} GAIN LIFE: +{} (state-ben még nem frissítjük).",
                        player, amount
                    );
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
        effect, tcard.name, tcard.card_id
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
                info!(
                    "    TargetFilter::Creature → lecseréljük ExactCardID({})-re",
                    tcard.card_id
                );
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
                info!(
                    "    TargetFilter::Creature → ExactCardID({})",
                    tcard.card_id
                );
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
            debug!(
                "  → TargetedEffects branch, sub_effects len={}",
                sub_effects.len()
            );

            let replaced_subs = sub_effects
                .into_iter()
                .map(|sub| {
                    debug!("    TargetedEffects - belső sub: {:?}", sub);
                    crate::app::gre::effect_resolution::replace_targeted_filter_with_exact(
                        gre, sub, tcard,
                    )
                })
                .collect();

            Effect::TargetedEffects {
                sub_effects: replaced_subs,
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

    debug!(
        "replace_targeted_filter_with_exact() - End: returning={:?}",
        result
    );
    result
}
