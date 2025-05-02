// tests/game_state_updater_tests.rs

use std::collections::BinaryHeap;
use MTGA_me::app::card_attribute::{Effect, Trigger};
use MTGA_me::app::game_state_updater::GameStateUpdater;
use MTGA_me::app::game_state::{GameState, Player, StackEntry, GamePhase, GameEvent};
use MTGA_me::app::card_library::{build_card_library, Card};
use MTGA_me::app::gre::PriorityEntry;

/// Segédfüggvény: alapállapot előkészítése
fn setup() -> (GameStateUpdater, GameState) {
    let updater = GameStateUpdater::new();
    let state = GameState::default();
    (updater, state)
}

#[test]
fn refresh_battlefield_adds_tokens_and_auras() {
    // game_state_updater.rs L45–L70: battlefield szinkronizálás OCR alapján
    let (mut updater, mut state) = setup();
    // Tegyük fel, hogy a bot battlefield_creatures mező tokennel bővült:
    let mock_token = Card {
        name: "SoldierToken".into(),
        card_type: MTGA_me::app::card_library::CardType::Creature(
            MTGA_me::app::card_library::Creature {
                name: "SoldierToken".into(),
                summoning_sickness: false,
                power: 1,
                toughness: 1,
            }
        ),
        mana_cost: Default::default(),
        attributes: vec![],
        triggers: vec![],
    };
    state.battlefield.push(mock_token.clone());
    updater.refresh_all(
        1920, 1080,
        &vec![],                // cards_texts
        &build_card_library(),
        0,                      // land_number
        false,                  // land_played_this_turn
        &vec![],                // stack_snapshot
    );
    assert!(
        updater.state.battlefield.iter().any(|c| c.name == "SoldierToken"),
        "A battlefield nem tartalmazza a token-t (game_state_updater.rs L53–L58)"
    );
}

#[test]
fn mana_and_land_update_across_turns() {
    // game_state_updater.rs L80–L95: land és mana reset új körben
    let (mut updater, mut state) = setup();
    state.land_played_this_turn = true;
    state.mana_available = 1;
    updater.state = state.clone();
    // Új kör jön, build_card_library() land_count=3, land_played reset
    updater.refresh_all(1920, 1080, &vec![], &build_card_library(), 3, false, &vec![]);
    assert_eq!(
        updater.state.mana_available, 3,
        "mana_available-nek 3-nak kell lennie új körben (game_state_updater.rs L86–L93)"
    );
    assert!(
        !updater.state.land_played_this_turn,
        "land_played_this_turn false-ra kell resetelni új körben (game_state_updater.rs L86–L93)"
    );
}

#[test]
fn stack_snapshot_is_refreshed() {
    // game_state_updater.rs L96–L110: stack frissítése OCR alapján
    let (mut updater, state) = setup();
    let fake_entry = PriorityEntry {
        priority: 1,
        sequence: 0,
        entry: StackEntry::Spell {
            card: build_card_library().get("Lightning Strike").unwrap().clone(),
            controller: Player::Us,
        },
    };
    let snapshot = vec![fake_entry.clone()];
    updater.refresh_all(1920, 1080, &vec![], &build_card_library(), 0, false, &snapshot);
    assert_eq!(
        updater.state.stack.len(), 1,
        "A stack_snapshot-nek 1 elemet kell tartalmaznia (game_state_updater.rs L100–L105)"
    );
    match &updater.state.stack[0] {
        StackEntry::Spell { card, controller } => {
            assert_eq!(card.name, "Lightning Strike");
            assert_eq!(*controller, Player::Us);
        }
        _ => panic!("Váratlan StackEntry típus"),
    }
}

#[test]
fn invalid_bot_state_gracefully_handled() {
    // game_state_updater.rs L108–L120: hibás OCR vagy hiányzó mezők kezelése
    let (mut updater, mut state) = setup();
    state.hand.clear();
    state.library_count = 5; // bot szerint van 5 kártya, de OCR nem lát semmit
    updater.state = state.clone();
    // Nem szabad pánikba esni:
    updater.refresh_all(1920, 1080, &vec![], &build_card_library(), 0, false, &vec![]);
    // Legalább üres, de valid struktúra:
    assert_eq!(
        updater.state.hand.len(), 0,
        "Üres kéz esetén is fusson le hibamentesen (game_state_updater.rs L113–L120)"
    );
}
#[test]
fn refresh_battlefield_multiple_simultaneous_cards_and_enchantments() {
    let (mut updater, mut state) = setup();
    // több token és aura egyszerre
    let creature = Card::new("TestCreature").with_base_stats(2,2);
    let aura = Card::new("TestAura").with_delayed_trigger(Trigger::OnDeath, Effect::DrawCard(Player::Us));
    state.battlefield.push(creature.clone());
    state.battlefield.push(aura.clone());
    updater.refresh_all(1920, 1080, &vec![], &build_card_library(), 0, false, &vec![]);
    // mindkettőt látja
    let names: Vec<_> = updater.state.battlefield.iter().map(|c| &c.name).collect();
    assert!(names.contains(&"TestCreature"));
    assert!(names.contains(&"TestAura"));
}

#[test]
fn invalid_phase_transition_handled_gracefully() {
    let (mut updater, mut state) = setup();
    // véletlenül rossz fázisadat jön OCR-ből
    // pl. negative mana vagy land_number túl nagy
    updater.state = state.clone();
    updater.refresh_all(1920, 1080, &vec![], &build_card_library(), u32::MAX, true, &vec![]);
    // ne essen össze, hanem clamp-olja a mana_available mezőt
    assert!(updater.state.mana_available < u32::MAX);
}
#[test]
fn test_update_stack_directly() {
    use std::collections::BinaryHeap;
    let mut updater = GameStateUpdater::new();
    let mut heap = BinaryHeap::new();
    // készítsünk egy GRE PriorityEntry-t
    let spell = build_card_library().get("Lightning Strike").unwrap().clone();
    let pe = PriorityEntry {
        priority: 7,
        sequence: 99,
        entry: StackEntry::Spell { card: spell.clone(), controller: Player::Opponent },
    };
    heap.push(pe.clone());

    updater.update_stack(&heap);
    // most a GameState.stack-ben ott kell lennie a pe.entry-nek
    assert_eq!(updater.state.stack.len(), 1);
    assert_eq!(updater.state.stack[0], pe.entry);
}

#[test]
fn test_refresh_all_complex_updates() {
    let mut updater = GameStateUpdater::new();
    // Előzetesen legyen valami a state-ben
    updater.state.hand.push(build_card_library().get("Lightning Strike").unwrap().clone());
    // GRE-stack két bejegyzéssel
    use std::collections::BinaryHeap;
    let mut heap = BinaryHeap::new();
    heap.push(PriorityEntry {
        priority: 1,
        sequence: 0,
        entry: StackEntry::Spell {
            card: build_card_library().get("Burst Lightning").unwrap().clone(),
            controller: Player::Us,
        },
    });
    heap.push(PriorityEntry {
        priority: 1,
        sequence: 1,
        entry: StackEntry::Spell {
            card: build_card_library().get("Lightning Strike").unwrap().clone(),
            controller: Player::Opponent,
        },
    });

    // Hívjuk meg refresh_all egyszerre kézzel megadott kézzel
    let texts = vec!["Lightning Strike".into(), "Burst Lightning".into()];
    updater.refresh_all(
        1024, 768,
        &texts,
        &build_card_library(),
        2,      // available_mana
        true,   // land_played
        &heap,
    );

    // Hand: mindkét kártya
    let names: Vec<_> = updater.state.hand.iter().map(|c| c.name.as_str()).collect();
    assert!(names.contains(&"Lightning Strike"));
    assert!(names.contains(&"Burst Lightning"));

    // Mana és land
    assert_eq!(updater.state.mana_available, 2);
    assert!(updater.state.land_played_this_turn);

    // Stack snapshot két elemmel
    assert_eq!(updater.state.stack.len(), 2);
}
#[test]
fn test_refresh_all_handles_inconsistent_ocr_texts() {
    let mut updater = GameStateUpdater::new();
    // Garbage OCR
    updater.refresh_all(
        800, 600,
        &["???".into(), "".into(), "NonCard".into()],
        &build_card_library(),
        0, false,
        &BinaryHeap::new(),
    );
    // Nem szabad pánikolni: üres kéz
    assert!(updater.state.hand.is_empty());
}

#[test]
fn test_refresh_all_simultaneous_battlefield_and_mana_updates() {
    let mut updater = GameStateUpdater::new();
    // A state-ben legyen korábban valami
    updater.state.land_played_this_turn = true;
    updater.state.mana_available = 5;
    // Hívjuk meg egyszerre
    updater.refresh_all(
        1024, 768,
        &vec![],
        &build_card_library(),
        2,  // új mana
        false,
        &BinaryHeap::new(),
    );
    // Győződjünk meg róla, hogy a mana felülíródott, de a korábbi land_played resetelve lett
    assert_eq!(updater.state.mana_available, 2);
    assert!(!updater.state.land_played_this_turn);
}
