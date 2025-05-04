// tests/card_library_tests.rs
/*
use MTGA_me::app::card_attribute::{Damage, DamageEqualPowerOnDeath, DelayedCounterAttribute, Effect, ModifyAttackDefense, SpawnTokenOnDeath, TargetFilter, Trigger};
use MTGA_me::app::card_library::{build_card_library, Card, CardType, Creature, Instant_, ManaCost};
use MTGA_me::app::game_state::{GameEvent, GamePhase, Player};
use MTGA_me::app::gre::{Gre, StackEntry};

#[test]
fn library_initialization_integrity() {
    // card_library.rs L10–L35: build_card_library definiálja a kártyákat
    let lib = build_card_library();
    assert!(
        lib.len() >= 10,
        "A kártyakönyvtár túl kicsi (card_library.rs L10–L35)"
    );
    for name in &["Lightning Strike", "Burst Lightning", "Felonious Rage"] {
        assert!(
            lib.contains_key(*name),
            "Hiányzik a(z) {} kártya a könyvtárból (card_library.rs L12–L30)",
            name
        );
    }
}

#[test]
fn card_lookup_by_name_behavior() {
    let lib = build_card_library();
    assert!(
        lib.get("Lightning Strike").is_some(),
        "Létező kártya nem található név szerint"
    );
    assert!(
        lib.get("NonExistentCard").is_none(),
        "Nem létező kártyánál None-t várok"
    );
}

#[test]
fn mana_cost_computation_colored_and_total() {
    let lib = build_card_library();
    let strike = lib.get("Lightning Strike").unwrap();
    let cost = &strike.mana_cost; // card_library.rs L40–L60
    assert_eq!(cost.colorless, 1, "Színtelen mana hibás");
    assert_eq!(cost.colored(), 1, "Színes mana hibás");
    assert_eq!(cost.total(), 2, "Total() metódus hibás");
}

#[test]
fn card_types_and_triggers_registered() {
    let lib = build_card_library();
    let scamp = lib.get("Cacophony Scamp").unwrap(); // card_library.rs L70–L100
    assert!(
        matches!(scamp.card_type, CardType::Creature(_)),
        "Cacophony Scamp-ként nem Creature típust regisztráltak"
    );
    let triggers = &scamp.triggers;
    use MTGA_me::app::card_attribute::Trigger;
    assert!(
        triggers.contains(&Trigger::OnDeath) && triggers.contains(&Trigger::OnCombatDamage),
        "OnDeath és OnCombatDamage triggereket várunk"
    );
}

#[test]
fn attribute_presence_for_key_cards() {
    let lib = build_card_library();
    let swiftspear = lib.get("Monastery Swiftspear").unwrap();
    let names: Vec<_> = swiftspear
        .attributes
        .iter()
        .map(|a| std::any::type_name_of_val(a.as_ref()))
        .collect();
    assert!(
        names.iter().any(|n| n.ends_with("HasteAttribute")),
        "Monastery Swiftspear-ről hiányzik a HasteAttribute"
    );
    assert!(
        names.iter().any(|n| n.ends_with("ProwessAttribute")),
        "Monastery Swiftspear-ről hiányzik a ProwessAttribute"
    );
}

#[test]
fn name_lookup_case_insensitive_and_partial() {
    let lib = build_card_library();
    // Kis-nagybetű és részleges keresés
    assert!(
        lib.get("lightning strike").is_some(),
        "Kisbetűs névvel is működnie kell"
    );
    assert!(
        lib.iter().any(|(name, _)| name.contains("Lightning")),
        "Részleges névkeresést is támogatni kell"
    );
}

#[test]
fn extreme_mana_cost_edge_cases() {
    // Feltételezzük, hogy build_card_library-ba idővel bekerül nagy manaköltségű kártya is
    let mut lib = build_card_library();
    lib.insert(
        "HugeSpell".into(),
        MTGA_me::app::card_library::Card {
            name: "HugeSpell".into(),
            mana_cost: ManaCost { colorless: 20, colored: vec![("Blue".into(), 5)] },
            ..Default::default()
        },
    );
    let huge = lib.get("HugeSpell").unwrap();
    assert_eq!(huge.mana_cost.colorless, 20);
    assert_eq!(huge.mana_cost.colored(), 5);
    assert_eq!(huge.mana_cost.total(), 25);
}
#[test]
fn library_remove_and_integrity() {
    let mut lib = build_card_library();
    // távolítsuk el a „Burst Lightning”-et
    assert!(lib.remove("Burst Lightning").is_some());
    // a többi kártya továbbra is ott van
    for name in &["Lightning Strike", "Felonious Rage", "Monastery Swiftspear"] {
        assert!(lib.contains_key(*name), "{} hiányzik a könyvtárból eltávolítás után", name);
    }
}

#[test]
fn dynamic_attribute_changes_through_library() {
    let mut lib = build_card_library();
    // adjunk ideiglenes buffot a Swiftspear-hez
    let card = lib.get_mut("Monastery Swiftspear").unwrap();
    card.attributes.push(Box::new(ModifyAttackDefense { power: 5, toughness: 5 }));
    // triggerek: OnCastResolved → 1/2 + új buff
    let effects: Vec<_> = card.trigger_by(&Trigger::Custom("OnCastResolved".into()));
    // kell lennie legalább egy Haste és egy +5/+5 change-nek
    assert!(effects.iter().any(|e| matches!(e, Effect::Haste)));
    assert!(effects.iter().any(|e| matches!(e, Effect::SelfAttributeChange(attr) if attr.power==5 && attr.toughness==5)));
}

#[test]
fn cards_trigger_each_other_complex_interaction() {
    let mut lib = build_card_library();
    // Két kártya, A átad egy countert B-nek, B meg vissza A-nak
    lib.insert("PingA".into(), Card {
        name: "PingA".into(),
        card_type: CardType::Instant(Instant_ { name: "PingA".into() }),
        mana_cost: ManaCost::default(),
        attributes: vec![Box::new(DelayedCounterAttribute { delay_phase: GamePhase::End })],
        triggers: vec![Trigger::Custom("OnCastResolved".into())],
    });
    lib.insert("PingB".into(), Card {
        name: "PingB".into(),
        card_type: CardType::Instant(Instant_ { name: "PingB".into() }),
        mana_cost: ManaCost::default(),
        attributes: vec![Box::new(DelayedCounterAttribute { delay_phase: GamePhase::End })],
        triggers: vec![Trigger::Custom("OnCastResolved".into())],
    });
    // A kijátszása → B buff késleltetett → End fázisban két delayed effekt
    let mut gre = Gre::default();
    gre.cast_spell(lib.get("PingA").unwrap().clone(), Player::Us);
    gre.cast_spell(lib.get("PingB").unwrap().clone(), Player::Us);
    // resolve now → schedule two delayed
    gre.resolve_stack();
    let ids: Vec<_> = gre.delayed.iter().map(|d| d.id).collect();
    assert_eq!(ids.len(), 2, "Két delayed effektet vártunk");
    // End fázisban mindkettőt kiküldi
    gre.dispatch_delayed(GamePhase::End);
    assert_eq!(gre.delayed.len(), 0);
    assert!(gre.executed_delayed.len() >= 2);
}

#[test]
fn partial_fuzzy_name_matching_returns_multiple() {
    let lib = build_card_library();
    // keressük az "fire" szót (kis-nagybetűtől függetlenül)
    let mut hits: Vec<_> = lib
        .keys()
        .filter(|name| name.to_lowercase().contains("fire"))
        .cloned()
        .collect();
    hits.sort();
    // biztosan szerepel benne Lightning Strike és Burst Lightning
    assert!(hits.contains(&"Lightning Strike".to_string()));
    assert!(hits.contains(&"Burst Lightning".to_string()));
}

#[test]
fn test_library_card_removal_handles_consistency() {
    let mut lib = build_card_library();
    // Távolítsuk el a Lightning Strike-et
    assert!(lib.remove("Lightning Strike").is_some());
    // Többé nem találjuk
    assert!(lib.get("Lightning Strike").is_none());
    // A többi kártya megmarad
    assert!(lib.get("Burst Lightning").is_some());
}

#[test]
fn test_complex_card_interaction_triggers_chain() {
    let mut lib = build_card_library();
    // Hozzáadunk egy egyszerű kártyát, ami OnDeath token-t spawnol
    lib.insert("DeathSpawn".into(), Card {
        name: "DeathSpawn".into(),
        card_type: CardType::Creature(Creature { name: "DeathSpawn".into(), summoning_sickness: false, power: 1, toughness: 1 }),
        mana_cost: ManaCost::default(),
        attributes: vec![Box::new(SpawnTokenOnDeath)],
        triggers: vec![Trigger::OnDeath],
    });
    // A token pedig halálkor sebez
    lib.insert("Token".into(), Card {
        name: "Token".into(),
        card_type: CardType::Creature(Creature { name: "Token".into(), summoning_sickness: false, power: 1, toughness: 1 }),
        mana_cost: ManaCost::default(),
        attributes: vec![Box::new(DamageEqualPowerOnDeath { damage: Damage { amount: 1, special: None }, target_filter: TargetFilter { filter: 0 } })],
        triggers: vec![Trigger::OnDeath],
    });

    // Simuláljuk a láncot a GRE-vel
    let mut gre = Gre::default();
    // Meghal a DeathSpawn → spawnol egy Token-t a battlefieldre:
    let mut battlefield = Vec::new();
    battlefield.push(lib.get("DeathSpawn").unwrap().clone());
    gre.trigger_event(GameEvent::CreatureDied("DeathSpawn".into()), &mut battlefield, Player::Us);
    // Legyen létrehozva egy Token a delayed vagy azonnali triggerben:
    assert!(gre.stack.iter().any(|pe| {
        matches!(pe.entry(), StackEntry::TriggeredAbility { source: Some(c), .. } if c.name=="DeathSpawn")
    }) || gre.delayed.iter().any(|d| matches!(d.effect, Effect::SpawnNewCreature)));
}
*/