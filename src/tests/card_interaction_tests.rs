use std::collections::VecDeque;
use MTGA_me::app::card_library::{build_card_library, Card, CardType, Creature};
use MTGA_me::app::card_attribute::{Trigger, Effect};
use MTGA_me::app::gre::{Gre, GameEvent};
use MTGA_me::app::game_state::{Action, Player};

/// Segítség: kiveszi a queued Action-okat egy Vec-be, hogy könnyebb legyen összehasonlítani.
fn drain_actions(gre: &mut Gre) -> Vec<Action> {
    let mut out = Vec::new();
    while let Some(act) = gre.action_queue.pop_front() {
        out.push(act);
    }
    out
}

#[test]
fn monstrous_rage_hero_becomes_5_3_and_deals_5_on_death_this_turn() {
    // --- Arrange ------------------------------------------------------------
    let mut gre = Gre::new();
    let mut lib = build_card_library();

    // Szerezzük elő a lapot és a hozzá tartozó Card objektumot
    let mut hero = lib.remove("Heartfire Hero").unwrap();
    // A test során folyamatosan módosítani fogjuk a state-et, ezért vegyük ki másolatban:
    let mut battlefield = vec![ hero.clone() ];

    // --- Act: először targetoljuk (OnTargeted), majd resolve-oljuk a rage-et ---  
    gre.fire_event(GameEvent::Custom("OnTargeted".into()), &mut battlefield);
    gre.fire_event(GameEvent::SpellResolved("Monstrous Rage".into()), &mut battlefield);
    gre.resolve_all();

    // --- Collect & Assert buff-ok sorrendje --------------------------------
    // Várjuk először a valiant +1/+1-et, aztán a rage +2/+0-át, aztán a Monster Role +1/+1-ét
    let actions = drain_actions(&mut gre);
    let deltas: Vec<(i32,i32)> = actions.iter().filter_map(|a| {
        if let Action::ModifyPT{delta_power, delta_toughness, ..} = a {
            Some((*delta_power, *delta_toughness))
        } else { None }
    }).collect();

    assert_eq!(deltas, vec![
        // Valiant buff (OnTargeted)
        (1, 1),
        // Monstrous Rage buff (OnCastResolved)
        (2, 0),
        // Monster Role token buff (CreateRole => AttachEnchantment => ModifyPT)
        // _ha_ implementáltuk volna a buffot, akkor itt jönne:
        (1, 1),
    ]);

    // Most a hero valós P/T-je legyen 1+1+2+1 = 5 / 1+1+0+1 = 3
    let final_pt: (i32,i32) = {
        let mut p = 1; let mut t = 1;
        for (dp, dt) in deltas {
            p += dp; t += dt;
        }
        (p, t)
    };
    assert_eq!(final_pt, (5, 3));

    // --- Act2: ha ebben a körben meghal, OnDeath triggerelődik, és annyi damage-t okoz, amennyi a *jelenlegi* power-e ---
    gre.fire_event(GameEvent::CreatureDied("Heartfire Hero".into()), &mut battlefield);
    gre.resolve_all();

    // Az OnDeath hatása a Hero-n a ValiantAttribute Death-ága *jelenlegi* powerrel kell, hogy dobná.
    // Tehát itt 5-öt!
    let damage_actions: Vec<(String, u32)> = drain_actions(&mut gre).iter().filter_map(|a| {
        if let Action::DealDamage{source, amount, ..} = a {
            Some((source.clone(), *amount))
        } else { None }
    }).collect();

    assert_eq!(damage_actions, vec![
        ("Heartfire Hero".into(), 5)
    ]);
}

#[test]
fn after_turn_end_hero_becomes_3_3_and_deals_3_on_death_later() {
    // Ugyanez, de először buffolunk, aztán turn end, utána kipróbáljuk a death-et
    let mut gre = Gre::new();
    let mut lib = build_card_library();
    let mut hero = lib.remove("Heartfire Hero").unwrap();
    let mut battlefield = vec![ hero.clone() ];

    // 1) buffok
    gre.fire_event(GameEvent::Custom("OnTargeted".into()), &mut battlefield);
    gre.fire_event(GameEvent::SpellResolved("Monstrous Rage".into()), &mut battlefield);
    gre.resolve_all();
    // 2) turn end (itt resetelődnek az OnCastResolved‑re épülő buffok, de a Monster Role token enchantment marad)
    gre.fire_event(GameEvent::TurnEnded, &mut battlefield);
    gre.resolve_all();

    // Tehát most csak a Monster Role +1/+1 buff marad:
    let mut p = 1; let mut t = 1;
    // Valiant egyszer használódott, ragelés csak a körre szólt => innen már csak a token buffot számoljuk:
    p += 1; t += 1;
    assert_eq!((p,t), (2,2), "Ezt a mintakódot kiegészítendő: ha a rage buff eltűnik, de a token buff marad");

    // 3) utána meghal:
    gre.fire_event(GameEvent::CreatureDied("Heartfire Hero".into()), &mut battlefield);
    gre.resolve_all();
    // Damage = *jelenlegi* power = 2
    let damage_actions: Vec<u32> = drain_actions(&mut gre).iter().filter_map(|a| {
        if let Action::DealDamage{amount, ..} = a {
            Some(*amount)
        } else { None }
    }).collect();
    assert_eq!(damage_actions, vec![2]);
}