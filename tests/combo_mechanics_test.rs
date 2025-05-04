// tests/combo_mechanics_tests.rs
/*
use MTGA_me::app::card_library::{build_card_library, Card, CardType, ManaCost};
use MTGA_me::app::card_attribute::{Trigger, Effect, Damage, TargetFilter, Enchantment, AttributeChange};
use MTGA_me::app::game_state::{GameState, GamePhase, Player};
use MTGA_me::app::gre::{Gre, StackEntry, PriorityEntry};

// 1. Heartfire Hero + Monstrous Rage buff → permanent +2/+0 + Monster Role enchant
#[test]
fn heartfire_hero_monstrous_rage_triggers() {
    // 1) Kijátszod a Heartfire Hero-t (1/1).
    let mut lib = build_card_library();
    let mut hero = lib.get("Heartfire Hero").unwrap().clone();

    // 2) Kijátszol egy Monstrous Rage-et targetelve a Heartfire Hero-dra → OnCastResolved trigger
    let effects = hero.trigger_by(&Trigger::Custom("OnCastResolved".into()));

    // 3) Várjuk, hogy legyen +2/+0 buff (AttributeChange) és AttachEnchantment Monster Role
    assert!(effects.iter().any(|e| matches!(
        e, Effect::SelfAttributeChange(AttributeChange { power: 2, toughness: 0 })
    )), "Nem kapta meg a +2/+0 buffot");
    assert!(effects.iter().any(|e| matches!(
        e, Effect::AttachEnchantment { enchantment } if enchantment.name == "Monster Role"
    )), "Nem jött létre a Monster Role enchantment");
}
// A teszt folyamata:
// - Monstrous Rage OnCastResolved eseménye buffolja a Heartfire Hero-t +2/+0-ás ideiglenes változással.
// - Ugyanezen trigger létrehoz egy “Monster Role” enchantmentet is, ami +1/+1-et és trample-t ad.

// 2. Heartfire Hero halála → DamageTarget equal to current power
#[test]
fn heartfire_hero_death_deals_equal_damage() {
    // 1) Heartfire Hero base 1/1, rákap egy +2/+0 buffot → ideiglenes 3/1
    let mut lib = build_card_library();
    let mut hero = lib.get("Heartfire Hero").unwrap().clone();
    // manuálisan adjunk +2/+0 buffot (szimulálva a „Monstrous Rage” hatását)
    hero.card_type = CardType::Creature(
        MTGA_me::app::card_library::Creature { name: hero.name.clone(), summoning_sickness: false, power: 3, toughness: 1 }
    );

    // 2) Meghal (OnDeath trigger)
    let effects = hero.trigger_by(&Trigger::OnDeath);

    // 3) Várjuk, hogy DamageTarget legyen amount == 3
    assert!(effects.iter().any(|e| matches!(
        e, Effect::DamageTarget { damage: Damage { amount: 3, .. }, .. }
    )), "Nem okozott a halálakor a megfelelő mennyiségű sebzést");
    // A teszt folyamata:
    // - A Hero-t ideiglenesen 3/1-be buffoljuk.
    // - OnDeath triggerként létrejön egy DamageTarget effekt, damage.amount == aktuális power (3).
}

// 3. Cacophony Scamp + Felonious Rage → +2/+0, haste, OnCombatDamage proliferate + OnDeath detective token
#[test]
fn cacophony_scamp_felonious_rage_combo() {
    let mut lib = build_card_library();
    let mut scamp = lib.get("Cacophony Scamp").unwrap().clone();

    // 1) Felonious Rage OnTargeted + Custom("OnCastResolved") trigger a scamp-on
    let rage_card = lib.get("Felonious Rage").unwrap().clone();
    let mut effects = scamp.trigger_by(&Trigger::OnTargeted);
    effects.extend(scamp.trigger_by(&Trigger::Custom("OnCastResolved".into())));

    // 2) Várjuk a +2/+0 buffot és a Haste-et
    assert!(effects.iter().any(|e| matches!(
        e, Effect::SelfAttributeChange(AttributeChange { power: 2, toughness: 0 })
    )), "Scamp nem kapott +2/+0 buffot");
    assert!(effects.iter().any(|e| matches!(
        e, Effect::Haste
    )), "Scamp nem kapott haste-et");

    // 3) OnCombatDamage → Poliferate (a kodban SpawnCounterOnDamage nem szerepel, de PoliferateOnDamage igen)
    let proliferate = scamp.trigger_by(&Trigger::OnCombatDamage);
    assert!(proliferate.iter().any(|e| matches!(
        e, Effect::Poliferate { .. }
    )), "Scamp OnCombatDamage nem hívja meg a proliferate-et");

    // 4) OnDeath → DamageTarget + SpawnTokenOnDeath
    let death_effects = scamp.trigger_by(&Trigger::OnDeath);
    assert!(death_effects.iter().any(|e| matches!(
        e, Effect::DamageTarget { .. }
    )), "Scamp OnDeath nem okoz DamageTarget-et");
    assert!(death_effects.iter().any(|e| matches!(
        e, Effect::SpawnNewCreature
    )), "Scamp OnDeath nem spawnol új token-t");
    // A teszt folyamata:
    // - Felonious Rage buffolja (+2/+0, haste) → scamp 3/1 haste lesz.
    // - Scamp OnCombatDamage eseményénél proliferate.
    // - Scamp OnDeath eseményénél damage equal to its power (3) + spawn token.
}

// 4. Burst Lightning kicker nélkül és kickelve
#[test]
fn burst_lightning_kicker_variants() {
    let mut lib = build_card_library();
    let mut bl_no_kick = lib.get("Burst Lightning").unwrap().clone();
    let mut bl_kick = bl_no_kick.clone();
    // A kodi implementáció szerint a kicked flag az attribute-ban tárolódik
    bl_kick.attributes = vec![Box::new(MTGA_me::app::card_attribute::BurstLightningAttribute { kicked: true })];

    // 1) No kick OnCastResolved → DamageTarget amount == 2
    let eff1 = bl_no_kick.trigger_by(&Trigger::Custom("OnCastResolved".into())).pop().unwrap();
    assert!(matches!(eff1, Effect::DamageTarget { damage: Damage { amount: 2, .. }, .. }),
            "Burst Lightning without kicker nem okoz 2 damage");

    // 2) Kick OnCastResolved → DamageTarget amount == 4
    let eff2 = bl_kick.trigger_by(&Trigger::Custom("OnCastResolved".into())).pop().unwrap();
    assert!(matches!(eff2, Effect::DamageTarget { damage: Damage { amount: 4, .. }, .. }),
            "Burst Lightning with kicker nem okoz 4 damage");
    // A teszt folyamata:
    // - Kicker nélkül 2 sebzés.
    // - Kickerrel (kicked=true) 4 sebzés.
}

// 5. Lightning Strike alap működés
#[test]
fn lightning_strike_base_effect() {
    let mut lib = build_card_library();
    let mut ls = lib.get("Lightning Strike").unwrap().clone();

    // OnCastResolved trigger → DamageTarget amount == 3
    let mut eff = ls.trigger_by(&Trigger::Custom("OnCastResolved".into())).pop().unwrap();
    assert!(matches!(eff, Effect::DamageTarget { damage: Damage { amount: 3, .. }, .. }),
            "Lightning Strike nem okoz 3 damage");
    // A teszt folyamata:
    // - Lightning Strike OnCastResolved eseményénél mindig 3 sebzés.
}

fn apply_monstrous_rage(card: &mut Card) {
    // trigger Monstrous Rage
    let mut effects = card.trigger_by(&Trigger::Custom("OnCastResolved".into()));
    // apply any SelfAttributeChange buffs
    for eff in effects.drain(..) {
        if let Effect::SelfAttributeChange(AttributeChange { power, toughness }) = eff {
            if let CardType::Creature(mut c) = card.card_type.clone() {
                c.power += power;
                c.toughness += toughness;
                card.card_type = CardType::Creature(c);
            }
        }
    }
}

fn apply_felonious_rage(card: &mut Card) {
    let mut effects = card.trigger_by(&Trigger::OnTargeted);
    effects.extend(card.trigger_by(&Trigger::Custom("OnCastResolved".into())));
    for eff in effects.drain(..) {
        match eff {
            Effect::SelfAttributeChange(AttributeChange { power, toughness }) => {
                if let CardType::Creature(mut c) = card.card_type.clone() {
                    c.power += power;
                    c.toughness += toughness;
                    card.card_type = CardType::Creature(c);
                }
            }
            Effect::Haste => {
                if let CardType::Creature(mut c) = card.card_type.clone() {
                    c.summoning_sickness = false;
                    card.card_type = CardType::Creature(c);
                }
            }
            _ => {}
        }
    }
}

#[test]
fn valiant_only_once_and_monster_role_once() {
    let mut lib = build_card_library();
    let mut hero = lib.get("Heartfire Hero").unwrap().clone();

    // initial state
    if let CardType::Creature(ref mut c) = hero.card_type {
        c.power = 1; c.toughness = 1; c.summoning_sickness = false;
    }

    // first Monstrous Rage
    apply_monstrous_rage(&mut hero);
    // Valiant should trigger once → +1/+1 counter permanently
    assert_eq!(match &hero.card_type {
        CardType::Creature(c) => c.power - 1, // buff removed after turn
        _ => 0,
    }, 1);
    // Monster Role enchantment gives another +1/+1
    // total permanent power now 3

    // second Monstrous Rage same turn
    apply_monstrous_rage(&mut hero);
    // Valiant must NOT trigger again → only +2/+0 this time temporary
    assert_eq!(match &hero.card_type {
        CardType::Creature(c) => c.power,
        _ => 0,
    }, 5); // 3 permanent + 2 temporary

    // Felonious Rage same turn
    apply_felonious_rage(&mut hero);
    // +2/+0 again
    assert_eq!(match &hero.card_type {
        CardType::Creature(c) => c.power,
        _ => 0,
    }, 7);
    // Haste cleared summoning sickness
    assert_eq!(match &hero.card_type {
        CardType::Creature(c) => c.summoning_sickness,
        _ => true,
    }, false);
}

#[test]
fn death_triggers_and_trample_damage() {
    let mut lib = build_card_library();
    let mut hero = lib.get("Heartfire Hero").unwrap().clone();

    // bring hero to 9/3 via 2x Monstrous Rage + Felonious Rage
    if let CardType::Creature(ref mut c) = hero.card_type {
        c.power = 9; c.toughness = 3; c.summoning_sickness = false;
    }

    // simulate blocking by 3/1 → hero dies
    // calculate trample excess = hero.power - blocker.power
    let excess = match &hero.card_type {
        CardType::Creature(c) => c.power - 3,
        _ => 0,
    };
    assert!(excess > 0);

    // OnDeath triggers: damage equal to power (9) and spawn detective
    let death_effects = hero.trigger_by(&Trigger::OnDeath);
    assert!(death_effects.iter().any(|e|
    matches!(e, Effect::DamageTarget { damage, .. } if damage.amount == 9)
    ), "Death damage must equal hero's current power");
    assert!(death_effects.iter().any(|e|
    matches!(e, Effect::SpawnNewCreature)
    ), "Felonious Rage detective token must be created");

    // trample damage flows: excess (6) plus death damage (9) = 15 total
    assert_eq!(excess + 9, 15);
}

 */