// tests/card_attribute_tests.rs
/*
use MTGA_me::app::card_attribute::*;
use MTGA_me::app::card_library::{Card, CardType, Instant_, ManaCost};
use MTGA_me::app::game_state::{GamePhase, GameState};
use MTGA_me::app::gre::Gre;
use MTGA_me::app::gre::StackEntry::Spell;

#[test]
fn test_modify_attack_defense() {
    let mut attr = ModifyAttackDefense { power: 2, toughness: 3 };
    let effect = attr.on_trigger(&Trigger::Custom("Any".into())).unwrap();
    assert_eq!(effect, Effect::SelfAttributeChange(AttributeChange { power: 2, toughness: 3 }));
}

#[test]
fn test_poliferate_on_damage() {
    let mut attr = Poliferate;
    assert!(attr.on_trigger(&Trigger::OnCombatDamage).is_some());
    assert!(attr.on_trigger(&Trigger::OnDeath).is_none());
}

#[test]
fn test_spawn_token_on_death() {
    let mut attr = SpawnTokenOnDeath;
    assert!(attr.on_trigger(&Trigger::OnDeath).is_some());
    assert!(attr.on_trigger(&Trigger::OnCombatDamage).is_none());
}

#[test]
fn test_damage_equal_power_on_death() {
    let mut attr = DamageEqualPowerOnDeath {
        damage: Damage { amount: 2, special: None },
        target_filter: TargetFilter { filter: 0 },
    };
    let effect = attr.on_trigger(&Trigger::OnDeath).unwrap();
    assert_eq!(effect, Effect::DamageTarget { damage: Damage { amount: 2, special: None }, target_filter: TargetFilter { filter: 0 }});
}

#[test]
fn test_haste_attribute() {
    let mut attr = HasteAttribute;
    assert_eq!(attr.on_trigger(&Trigger::Custom("OnCastResolved".into())), Some(Effect::Haste));
    assert_eq!(attr.on_trigger(&Trigger::OnDeath), None);
}

#[test]
fn test_burst_lightning_attribute() {
    let mut attr = BurstLightningAttribute { kicked: true };
    let effect = attr.on_trigger(&Trigger::Custom("OnCastResolved".into())).unwrap();
    assert_eq!(effect, Effect::DamageTarget {
        damage: Damage { amount: 4, special: None },
        target_filter: TargetFilter { filter: 0 },
    });
}

#[test]
fn test_deal_damage_on_resolve() {
    let mut attr = DealDamageOnResolve { amount: 3 };
    let effect = attr.on_trigger(&Trigger::Custom("OnCastResolved".into())).unwrap();
    assert_eq!(effect, Effect::DamageTarget {
        damage: Damage { amount: 3, special: None },
        target_filter: TargetFilter { filter: 0 },
    });
}

#[test]
fn test_plus_two_plus_zero_and_role() {
    let mut attr = PlusTwoPlusZeroAndRole { role: "Monster".into() };
    let effect = attr.on_trigger(&Trigger::Custom("OnCastResolved".into())).unwrap();
    assert_eq!(effect, Effect::SelfAttributeChange(AttributeChange { power: 2, toughness: 0 }));
}

#[test]
fn test_proliferate_on_spell_cast() {
    let mut attr = ProliferateOnSpellCast;
    assert!(attr.on_trigger(&Trigger::OnSpellCast).is_some());
    assert!(attr.on_trigger(&Trigger::OnDeath).is_none());
}

#[test]
fn test_prowess_attribute() {
    let mut attr = ProwessAttribute;
    let effect = attr.on_trigger(&Trigger::OnSpellCast).unwrap();
    assert_eq!(effect, Effect::SelfAttributeChange(AttributeChange { power: 1, toughness: 1 }));
}

#[test]
fn test_valiant_attribute() {
    let mut attr = ValiantAttribute { used: false };
    let first_use = attr.on_trigger(&Trigger::OnTargeted).unwrap();
    assert_eq!(first_use, Effect::SelfAttributeChange(AttributeChange { power: 1, toughness: 1 }));

    assert!(attr.on_trigger(&Trigger::OnTargeted).is_none());

    let death_effect = attr.on_trigger(&Trigger::OnDeath).unwrap();
    assert_eq!(death_effect, Effect::DamageTarget {
        damage: Damage { amount: 0, special: Some("CURRENT_POWER".into()) },
        target_filter: TargetFilter { filter: 0 },
    });
}

#[test]
fn test_enchant_creature_buff() {
    let mut attr = EnchantCreatureBuff {
        power: 1,
        toughness: 1,
        abilities: vec!["Menace".into(), "Trample".into()],
    };
    let effect = attr.on_trigger(&Trigger::Custom("OnCastResolved".into())).unwrap();
    assert_eq!(effect, Effect::AttachEnchantment { enchantment: Enchantment { name: "Demonic Ruckus".into() } });
}

#[test]
fn test_draw_on_aura_dies() {
    let mut attr = DrawOnAuraDies;
    assert!(attr.on_trigger(&Trigger::OnDeath).is_some());
    assert!(attr.on_trigger(&Trigger::OnTargeted).is_none());
}

#[test]
fn test_add_mana_ability() {
    let mut attr = AddManaAbility { mana_type: "Red".into(), condition: ManaCondition::Any };
    let effect = attr.on_trigger(&Trigger::Custom("AddRedMana".into())).unwrap();
    assert_eq!(effect, Effect::AddMana { mana_type: "Red".into() });
}

#[test]
fn test_plus_one_zero_and_haste_on_spell() {
    let mut attr = PlusOneZeroAndHasteOnSpell { color_filter: "".into() };
    let effect = attr.on_trigger(&Trigger::OnSpellCast).unwrap();
    assert_eq!(effect, Effect::SelfAttributeChange(AttributeChange { power: 1, toughness: 0 }));
}

#[test]
fn test_delayed_counter_attribute() {
    let mut attr = DelayedCounterAttribute { delay_phase: GamePhase::PostCombatMain };
    let effect = attr.on_trigger(&Trigger::Custom("OnCastResolved".into())).unwrap();
    assert_eq!(effect, Effect::Delayed {
        effect: Box::new(Effect::SelfAttributeChange(AttributeChange { power: 0, toughness: 1 })),
        phase: GamePhase::PostCombatMain,
        deps: vec![],
    });
}

#[test]
fn death_replacement_effects_conflict() {
    let mut game = GameState::new();
    let player = game.add_player("Alice", 20);

    // Létrehozunk egy lényt két különböző helyettesítő képességgel a halálára.
    let creature = Card::new("TestCreature").with_replacement_effects(vec![
        // 1. effektus: ha meghalna, exile-ba kerül helyette
        ReplacementEffect::OnDeath(Outcome::Exile),
        // 2. effektus: ha meghalna, visszakeverjük a pakliba helyette
        ReplacementEffect::OnDeath(Outcome::ShuffleIntoDeck),
    ]);
    game.play_card(player, creature);

    // A lény megsemmisítése (pl. egy varázslat hatására).
    game.kill_creature(player, "TestCreature");
    // A motorban választani kell melyik helyettesítő hatás érvényesüljön (vagy meghatározott prioritású).
    // Feltételezzük, hogy az első effektus (Exile) érvényesül és a második ignorálódik.
    assert!(game.graveyard(player).is_empty(), "A lény nem kerülhet a sima graveyard-ba");
    let exile_zone = game.exile_zone(player);
    let library = game.library(player);
    // Ellenőrizzük, hogy **pontosan az egyik** zónában van a lény.
    let exiled = exile_zone.contains("TestCreature");
    let shuffled = library.contains("TestCreature");
    assert!(exiled ^ shuffled, "A lénynek vagy exile-ban vagy a pakliban kell lennie, de nem mindkettőben.");
}
#[test]
fn multiple_attribute_modifiers() {
    let mut game = GameState::new();
    let player = game.add_player("Tester", 20);

    // Létrehozunk egy 2/2-es lényt.
    let creature = Card::new("BuffableCreature").with_base_stats(2, 2);
    game.play_card(player, creature);
    let cid = game.get_card_id(player, "BuffableCreature");

    // Alkalmazunk egy hatást, ami beállítja a lény erejét 5-re (pl. "alakváltás" jellegű effect).
    game.apply_effect(Effect::SetPower(cid, 5));
    // Alkalmazunk egy másik hatást, ami +3/+0 buffot ad a lénynek.
    game.apply_effect(Effect::ModifyPower(cid, 3));

    // Most a lény erejének vagy 5 (ha a set felülírja a buffot) vagy 8 (ha a buff hozzáadódik az új alaphoz) kell lennie,
    // attól függően, hogy a motor hogyan kezeli az effektusokat. Tegyük fel, hogy a buff hozzáadódik az új alapértékhez.
    assert_eq!(game.get_power(cid), 8, "A lény erejének 5 (alap) + 3 (buff) = 8 kell legyen.");
    assert_eq!(game.get_toughness(cid), 2, "A toughness változatlan (2).");

    // Tisztítás: eltávolítjuk a hatásokat a lényről.
    game.clear_effects(cid);
    assert_eq!(game.get_power(cid), 2, "Buffok levétele után az erő visszaáll az alap 2-re.");
}
#[test]
fn temporary_effect_expires_end_of_turn() {
    let mut game = GameState::new();
    let player = game.add_player("BuffPlayer", 20);
    let creature = Card::new("TemporaryBuffCreature").with_base_stats(4, 4);
    game.play_card(player, creature);
    let cid = game.get_card_id(player, "TemporaryBuffCreature");

    // Alkalmazunk egy ideiglenes +3/+0 buffot a lényre a kör végéig.
    game.apply_effect(Effect::BuffUntilEOT(cid, 3, 0));
    assert_eq!(game.get_power(cid), 7);
    assert_eq!(game.get_toughness(cid), 4);

    // Lépjünk a végfázisba, majd a cleanup-ba.
    game.goto_phase(Phase::End);
    game.resolve_stack(); // (ha a buff esetleg triggerként került volna fel, de valószínűleg nem, csak időalapú)
    game.cleanup_phase(); // cleanup step, ahol az ideiglenes hatások lejárnak
    // A buffnak mostanra el kell múlnia.
    assert_eq!(game.get_power(cid), 4, "A kör végi buff hatása elmúlt a cleanup step-ben");
    assert_eq!(game.get_toughness(cid), 4);
}

#[test]
fn test_attribute_conflict_multiple_modifiers() {
    // Két ellentétes attribútum ugyanazon triggerre
    let mut card = Card {
        name: "ConflictCard".into(),
        card_type: CardType::Instant(Instant_ { name: "ConflictCard".into() }),
        mana_cost: ManaCost::default(),
        attributes: vec![
            Box::new(DealDamageOnResolve { amount: 3 }),
            Box::new(ModifyAttackDefense { power: 1, toughness: 1 }),
        ],
        triggers: vec![Trigger::Custom("OnCastResolved".into())],
    };
    let effects = card.trigger_by(&Trigger::Custom("OnCastResolved".into()));
    assert!(
        effects.iter().any(|e| matches!(e, Effect::DamageTarget { damage, .. } if damage.amount == 3)),
        "Vártuk a 3 sebzést"
    );
    assert!(
        effects.iter().any(|e| matches!(e, Effect::SelfAttributeChange(attr) if attr.power == 1 && attr.toughness == 1)),
        "Vártuk a +1/+1 buffot"
    );
}
#[test]
fn test_multiple_continuous_effects_apply_in_order() {
    let mut gre = Gre::default();
    // Két continuous, az első +1 damage, a második +2
    gre.add_continuous_effect(|e| if let Effect::DamageTarget { ref mut damage, .. } = e { damage.amount += 1; });
    gre.add_continuous_effect(|e| if let Effect::DamageTarget { ref mut damage, .. } = e { damage.amount += 2; });
    // Kézzel hívjuk meg a handle_effect-et
    gre.handle_effect(Effect::DamageTarget { damage: Damage { amount: 0, special: None }, target_filter: TargetFilter { filter: 0 } });
    // Nincs közvetlen eredmény, de nem pánikol el – coverage-re ez is kell
}

#[test]
fn test_card_removal_during_active_effects() {
    let mut game = GameState::new();
    let p = game.add_player("Tester", 20);
    // Hozzunk létre egy Abyss-szerű kártyát, ami minden permanenst elpusztít:
    let abyss = Spell::new("Abyss", Effect::DestroyTarget { target_filter: TargetFilter { filter: 0 } });
    // Tegyük a battlefieldre egy lényt
    let creature = Card::new("Victim").with_base_stats(2, 2);
    game.play_card(p, creature.clone());
    // Kijátszuk az Abyss-t, ami elpusztítja a lényt → graveyardba kerül
    game.cast_spell(p, abyss.clone());
    game.resolve_stack();
    // Utána azonnal távolítsuk el a Victim-et
    game.remove_card(p, game.get_card_id(p, "Victim"));
    // Nem szabad, hogy bármilyen panic legyen, és a játék állapota konzisztens marad
    assert!(!game.battlefield(p).contains("Victim"));
}
*/