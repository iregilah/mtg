// tests/gre_tests.rs
/*
use MTGA_me::app::gre::*;
use MTGA_me::app::card_library::*;
use MTGA_me::app::game_state::*;
use MTGA_me::app::card_attribute::*;

#[test]
fn test_gre_new_initializes_properly() {
    let gre = Gre::new(Player::Us);
    assert_eq!(gre.priority, Player::Us);
    assert!(gre.stack.is_empty());
    assert!(gre.delayed.is_empty());
}

#[test]
fn test_cast_spell_pushes_spell_on_stack() {
    let mut gre = Gre::default();
    let card = build_card_library().get("Burst Lightning").unwrap().clone();
    gre.cast_spell(card.clone(), Player::Us);
    assert_eq!(gre.stack.len(), 1);
    match gre.stack.peek().unwrap().entry() {
        StackEntry::Spell { card: c, controller } => {
            assert_eq!(c.name, "Burst Lightning");
            assert_eq!(*controller, Player::Us);
        },
        _ => panic!("Expected Spell entry on stack"),
    }
}

#[test]
fn test_schedule_and_dispatch_delayed_effect() {
    let mut gre = Gre::default();
    let effect = Effect::SpawnNewCreature;
    gre.schedule_delayed(effect.clone(), GamePhase::End, vec![]);
    assert_eq!(gre.delayed.len(), 1);

    gre.dispatch_delayed(GamePhase::End);
    assert_eq!(gre.delayed.len(), 0);
    assert!(gre.executed_delayed.contains(&0));
}

#[test]
fn test_trigger_event_creates_triggered_ability() {
    let mut gre = Gre::default();
    let mut battlefield = vec![build_card_library().get("Cacophony Scamp").unwrap().clone()];
    gre.trigger_event(GameEvent::CreatureDied("Cacophony Scamp".into()), &mut battlefield, Player::Us);

    assert_eq!(gre.stack.len(), 2);
}

#[test]
fn test_pass_priority_resolves_stack_entry() {
    let mut gre = Gre::default();
    let card = build_card_library().get("Lightning Strike").unwrap().clone();
    gre.cast_spell(card.clone(), Player::Us);
    assert_eq!(gre.stack.len(), 1);

    gre.pass_priority();
    assert_eq!(gre.priority, Player::Opponent);

    gre.pass_priority();
    assert_eq!(gre.stack.len(), 0);
}

#[test]
fn test_replacement_effect() {
    let mut gre = Gre::default();
    gre.add_replacement_effect(10, |eff| {
        if let Effect::DestroyTarget { target_filter } = eff {
            Some(vec![Effect::ExileTarget { target_filter: target_filter.clone() }])
        } else {
            None
        }
    });

    let effect = Effect::DestroyTarget { target_filter: TargetFilter { filter: 0 } };
    gre.handle_effect(effect.clone());
}

#[test]
fn test_continuous_effect_modifies_damage() {
    let mut gre = Gre::default();
    gre.add_continuous_effect(|eff| {
        if let Effect::DamageTarget { damage, .. } = eff {
            damage.amount += 1;
        }
    });

    let mut damage_effect = Effect::DamageTarget {
        damage: Damage { amount: 2, special: None },
        target_filter: TargetFilter { filter: 0 }
    };

    gre.handle_effect(damage_effect.clone());
}

#[test]
fn test_resolve_stack_executes_effect() {
    let mut gre = Gre::default();
    let card = build_card_library().get("Burst Lightning").unwrap().clone();
    gre.cast_spell(card.clone(), Player::Us);

    gre.resolve_stack();
    assert!(gre.stack.is_empty());
}

#[test]
fn test_apply_replacement_chaining() {
    let mut gre = Gre::default();

    gre.add_replacement_effect(10, |eff| {
        if let Effect::DestroyTarget { target_filter } = eff {
            Some(vec![Effect::ExileTarget { target_filter: target_filter.clone() }])
        } else {
            None
        }
    });

    gre.add_replacement_effect(5, |eff| {
        if let Effect::ExileTarget { target_filter } = eff {
            Some(vec![Effect::DamageTarget {
                damage: Damage { amount: 3, special: None },
                target_filter: target_filter.clone(),
            }])
        } else {
            None
        }
    });

    let original_effect = Effect::DestroyTarget { target_filter: TargetFilter { filter: 1 } };
    let replaced_effects = gre.apply_replacement(&original_effect, 0);

    assert_eq!(replaced_effects.len(), 1);
    if let Effect::DamageTarget { damage, .. } = &replaced_effects[0] {
        assert_eq!(damage.amount, 3);
    } else {
        panic!("Expected DamageTarget effect");
    }
}

#[test]
fn test_priority_entry_ordering() {
    let entry1 = PriorityEntry { priority: 1, sequence: 1, entry: StackEntry::Spell { card: build_card_library()["Lightning Strike"].clone(), controller: Player::Us } };
    let entry2 = PriorityEntry { priority: 2, sequence: 2, entry: StackEntry::Spell { card: build_card_library()["Burst Lightning"].clone(), controller: Player::Opponent } };

    assert!(entry2 > entry1);
}
#[test]
fn test_schedule_delayed_unique_ids() {
    let mut gre = Gre::default();
    let effect = Effect::SelfAttributeChange(AttributeChange { power: 0, toughness: 0 });
    let id1 = gre.schedule_delayed(effect.clone(), GamePhase::End, vec![]);
    let id2 = gre.schedule_delayed(effect.clone(), GamePhase::End, vec![]);
    assert_eq!(id2, id1 + 1, "Each scheduled delayed effect should get a unique, incrementing ID");
}

#[test]
fn test_dispatch_delayed_wrong_phase() {
    let mut gre = Gre::default();
    let effect = Effect::SelfAttributeChange(AttributeChange { power: 1, toughness: 1 });
    let id = gre.schedule_delayed(effect.clone(), GamePhase::Combat, vec![]);
    gre.dispatch_delayed(GamePhase::End);
    // Should not dispatch when phase doesn't match
    assert_eq!(gre.delayed.len(), 1);
    assert!(!gre.executed_delayed.contains(&id));
}

#[test]
fn test_dispatch_delayed_with_dependencies() {
    let mut gre = Gre::default();
    let e1 = Effect::SelfAttributeChange(AttributeChange { power: 1, toughness: 1 });
    let e2 = Effect::SelfAttributeChange(AttributeChange { power: 2, toughness: 2 });
    let id1 = gre.schedule_delayed(e1.clone(), GamePhase::PostCombatMain, vec![]);
    let id2 = gre.schedule_delayed(e2.clone(), GamePhase::PostCombatMain, vec![id1]);
    // First dispatch should only execute id1
    gre.dispatch_delayed(GamePhase::PostCombatMain);
    assert!(gre.executed_delayed.contains(&id1));
    assert_eq!(gre.delayed.len(), 1, "Only the dependent effect should remain queued");
    // Second dispatch should now execute id2
    gre.dispatch_delayed(GamePhase::PostCombatMain);
    assert!(gre.executed_delayed.contains(&id2));
    assert!(gre.delayed.is_empty());
}

#[test]
fn test_trigger_event_schedules_delayed_counter() {
    let mut gre = Gre::default();
    let mut battlefield = vec![build_card_library().get("Temporal Distortion").unwrap().clone()];
    let before = gre.delayed.len();
    gre.trigger_event(GameEvent::Custom("OnCastResolved".into()), &mut battlefield, Player::Us);
    assert_eq!(gre.delayed.len(), before + 1, "Temporal Distortion should schedule a delayed counter");
    let d = &gre.delayed.last().unwrap();
    assert_eq!(d.execute_phase, GamePhase::PostCombatMain);
    match &*d.effect {
        Effect::SelfAttributeChange(attr) => assert_eq!(attr.toughness, 1),
        _ => panic!("Expected a SelfAttributeChange delayed effect"),
    }
}

#[test]
fn test_apply_replacement_no_match() {
    let gre = Gre::default();
    let original = Effect::DamageTarget {
        damage: Damage { amount: 5, special: None },
        target_filter: TargetFilter { filter: 0 },
    };
    let replaced = gre.apply_replacement(&original, 0);
    assert_eq!(replaced, vec![original.clone()], "Unmatched effects should remain unchanged");
}

#[test]
fn test_pass_priority_resets_pass_count() {
    let mut gre = Gre::default();
    gre.passes = 42;
    gre.pass_priority();
    gre.pass_priority(); // two passes => resolve + reset
    assert_eq!(gre.passes, 0, "Pass count should reset after two passes and resolution");
}

#[test]
fn test_cast_spell_resets_pass_count() {
    let mut gre = Gre::default();
    let card = build_card_library().get("Burst Lightning").unwrap().clone();
    gre.passes = 7;
    gre.cast_spell(card, Player::Us);
    assert_eq!(gre.passes, 0, "Casting a spell should reset pass count");
}

#[test]
fn test_push_to_stack_sets_correct_priority() {
    let mut gre = Gre::default();
    let card = build_card_library().get("Lightning Strike").unwrap().clone();
    let entry = StackEntry::ActivatedAbility {
        source: card.clone(),
        effect: Effect::Haste,
        controller: Player::Us,
    };
    gre.push_to_stack(entry);
    // Activated abilities should get priority 3
    let top = gre.stack.peek().unwrap();
    assert_eq!(top.priority, 3);
}

#[test]
fn multiple_simultaneous_triggers() {
    // Előkészítés: létrehozzuk a játékállapotot két játékossal.
    let mut game = GameState::new();
    let p1 = game.add_player("Player1", /*élet:*/ 20);
    let p2 = game.add_player("Player2", /*élet:*/ 20);

    // Mindkét játékos kap egy lapot, ami "minden kör kezdetén sebzi az ellenfelet".
    let card1 = Card::with_trigger("UpkeepBurn", Trigger::BeginUpkeep, Effect::Damage(3, p2));
    let card2 = Card::with_trigger("UpkeepBurn", Trigger::BeginUpkeep, Effect::Damage(3, p1));
    game.play_card(p1, card1);
    game.play_card(p2, card2);

    // Lépjünk a körfázisban előre a Player1 upkeepjéig, ahol mindkét kártya triggerel.
    game.start_turn(p1);
    game.goto_phase(Phase::Upkeep);
    // Ekkor keletkeznie kell 2 triggernek egyszerre.
    assert_eq!(game.stack.len(), 2);
    // Ellenőrizzük, hogy a stacken a triggerek sorrendje megfelel az elvártnak (Player1 trigger, majd Player2 trigger).
    let top_effect = game.stack.top();
    assert!(top_effect.origin == p2 && game.stack[1].origin == p1, "APNAP sorrend sérült");

    // Feloldjuk a stacket és ellenőrizzük a hatásokat: először Player2 trigger (Player2 sebzi Player1-et), majd Player1 trigger.
    game.resolve_stack();
    assert_eq!(game.get_player_life(p1), 17);  // Player1 kapott 3 sebzést Player2 triggerétől.
    assert_eq!(game.get_player_life(p2), 17);  // Player2 kapott 3 sebzést Player1 triggerétől (oldódott fel utoljára).
}

#[test]
fn multiple_spells_interaction_stack_order() {
    let mut game = GameState::new();
    let p1 = game.add_player("Attacker", 20);
    let p2 = game.add_player("Defender", 20);

    // Varázslat A: 5 sebzést okoz a védőnek.
    let spell_a = Spell::new("Fireball", Effect::Damage(5, p2));
    // Varázslat B: gyógyít 5 életet a védőnek (pl. reakcióként a sebzésre).
    let spell_b = Spell::new("Heal", Effect::Heal(5, p2));
    // Varázslat C: semlegesíti (countereli) a Fireball-t.
    let spell_c = Spell::new("Counterspell", Effect::Counter("Fireball"));

    // A sorrend: Player1 kijátssza Fireball-t, Player2 reagál Heal-lal, Player1 reagál Counterspell-lel a Heal előtt Fireballra.
    game.cast_spell(p1, spell_a);
    game.cast_spell(p2, spell_b);
    game.cast_spell(p1, spell_c);

    // Most a stacknek 3 varázslatot kell tartalmaznia LIFO sorrendben: [Fireball, Heal, Counterspell].
    assert_eq!(game.stack.len(), 3);
    assert_eq!(game.stack.top().name, "Counterspell");
    // Feloldjuk a stacket lépésenként.
    game.resolve_top_of_stack();  // Counterspell felold, megcélozza a Fireball-t.
    assert!(game.is_countered("Fireball"), "Fireball-t semlegesíteni kellett");
    game.resolve_top_of_stack();  // Heal feloldódik (növeli Player2 életét).
    assert_eq!(game.get_player_life(p2), 25);  // Player2 életének 25-re kellett nőnie.
    game.resolve_top_of_stack();  // Fireball megpróbálna oldódni, de semlegesítve lett, nem okoz sebzést.
    assert_eq!(game.get_player_life(p2), 25, "Fireball nem sebezhetett, élet változatlan marad");
    // Végül ellenőrizzük, hogy a stack üres.
    assert!(game.stack.is_empty());
}
#[test]
fn spell_fizzles_when_target_invalid() {
    let mut game = GameState::new();
    let p1 = game.add_player("Mage", 20);
    let p2 = game.add_player("Target", 20);

    // Player1 kijátszik egy varázslatot Player2 egyik lénye ellen (pl. Lightning Bolt 3 sebzéssel).
    let creature = Card::new("VictimCreature").with_base_stats(3, 3);
    game.play_card(p2, creature);
    let spell = Spell::new("Lightning Bolt", Effect::Damage(3, creature.id()));
    game.cast_spell(p1, spell);

    // Mielőtt a varázslat feloldódna, távolítsuk el a célpont lényt (pl. Player2 feláldozza, vagy destroy hatás).
    game.remove_card(p2, creature.id());  // a lény kikerül a játékból
    // Most feloldjuk a varázslatot.
    game.resolve_stack();
    // Ellenőrizzük, hogy Player2 élete nem csökkent (a sebző varázslat hatástalan maradt).
    assert_eq!(game.get_player_life(p2), 20);
    // A varázslatnak a graveyard-ba kellett kerülnie (feloldódás nélkül).
    assert!(game.graveyard(p1).contains("Lightning Bolt"), "A varázslatnak a gyűjtőbe kellett kerülnie érvénytelen célpont miatt.");
}
#[test]
fn infinite_loop_prevention() {
    let mut game = GameState::new();
    let player = game.add_player("Looper", 20);

    // Létrehozunk egy lényt, amelynek két hatása okozhat ciklust:
    // 1. Ha sebzést kap -> gyógyul 1-et.
    // 2. Ha gyógyul -> sebződik 1-et.
    let looper = Card::new("LooperCreature").with_abilities(vec![
        Ability::Trigger(Trigger::DamageReceived, Effect::Heal(1, SelfCard)),    // ha sebződik, gyógyul
        Ability::Trigger(Trigger::LifeGain, Effect::Damage(1, SelfCard)),        // ha gyógyul, sebződik
    ]);
    game.play_card(player, looper);

    // Okozunk 1 sebzést a lénynek, ami azonnal beindíthatná a ciklust.
    game.direct_damage(looper.id(), 1);
    // Most megpróbáljuk feloldani a keletkezett triggerek sorozatát biztonságosan.
    let result = game.resolve_stack_safe();
    // Az engine-nek fel kell ismernie a potenciális végtelen hurkot, és kezelnie (pl. megszakítani) kell.
    assert!(result.is_ok(), "A végtelen ciklust felismerte és kezelt a motor");
    assert!(game.get_card_by_id(looper.id()).is_some(), "A lénynek még játékban kell maradnia (nem hal meg a ciklustól)");
    // Ellenőrizzük, hogy a lény élete nem változott meg anomálisan (pl. nem végtelen gyógyult vagy halt meg).
    assert_eq!(game.get_card_life(looper.id()), game.get_card_max_life(looper.id()), "A lény élete normalizálódott a ciklus után");
}
#[test]
fn replacement_modifies_delayed_effects() {
    let mut gre = Gre::new(Player::Us);
    // eredeti delayed: SelfAttributeChange
    let orig = Effect::Delayed {
        effect: Box::new(Effect::SelfAttributeChange(AttributeChange { power:1, toughness:1 })),
        phase: GamePhase::End,
        deps: vec![],
    };
    // replacement, ami a belső efektust átalakítja
    gre.add_replacement_effect(100, |e| {
        if let Effect::SelfAttributeChange(_) = e {
            Some(vec![Effect::DamageTarget {
                damage: Damage { amount: 7, special: None },
                target_filter: TargetFilter { filter: 0 },
            }])
        } else {
            None
        }
    });
    let replaced = gre.apply_replacement(&orig, 0);
    assert_eq!(replaced.len(), 1);
    assert!(matches!(replaced[0], Effect::DamageTarget { damage, .. } if damage.amount == 7));
}

#[test]
fn gre_handles_invalid_stack_entry_type() {
    let mut gre = Gre::default();
    // push an impossible entry via manual hack
    gre.stack.push(PriorityEntry { priority: 1, sequence:0, entry: unsafe { std::mem::transmute([0u8;16]) }});
    // resolve_stack must not panic
    gre.resolve_stack();
    assert!(gre.stack.is_empty(), "Tisztázza a hibás entry-t is");
}
#[test]
fn test_push_to_stack_resets_passes_and_priority() {
    let mut gre = Gre::default();
    gre.passes = 5;
    gre.priority = Player::Opponent;
    let card = build_card_library().get("Lightning Strike").unwrap().clone();
    gre.push_to_stack(
        StackEntry::ActivatedAbility {
            source: card,
            effect: Effect::Haste,
            controller: Player::Us,
        }
    );
    // push_to_stack mindig reset-eli a pass count-et és priority-t a controller-re
    assert_eq!(gre.passes, 0);
    assert_eq!(gre.priority, Player::Us);
}
#[test]
fn test_gre_handles_stack_overflow_of_delayed_effects() {
    let mut gre = Gre::default();
    // Nagyszámú delayed effekt
    for _ in 0..2000 {
        gre.schedule_delayed(Effect::SpawnNewCreature, GamePhase::End, vec![]);
    }
    // Nem szabad, hogy memóriapánik legyen
    gre.dispatch_delayed(GamePhase::End);
    assert!(gre.delayed.is_empty());
}

#[test]
fn test_gre_resolves_invalid_stack_entry_gracefully() {
    let mut gre = Gre::default();
    // Unsafe: fabrikálunk egy érvénytelen StackEntry változatot
    let invalid: StackEntry = unsafe { std::mem::transmute([0u8; std::mem::size_of::<StackEntry>()]) };
    gre.stack.push(PriorityEntry { priority: 0, sequence: 0, entry: invalid });
    // Nem szabad, hogy panic legyen
    gre.resolve_stack();
    assert!(gre.stack.is_empty());
}
*/