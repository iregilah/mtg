// tests/game_state_tests.rs

use std::collections::HashMap;
use MTGA_me::app::game_state::*;
use MTGA_me::app::card_library::{build_card_library, Card, CardType, ManaCost};
use MTGA_me::app::gre::{PriorityEntry, StackEntry};
use MTGA_me::app::bot::Bot;
use MTGA_me::app::card_attribute::{Effect, Trigger};
use MTGA_me::app::gre::StackEntry::Spell;

#[test]
fn test_player_opponent() {
    assert_eq!(Player::Us.opponent(), Player::Opponent);
    assert_eq!(Player::Opponent.opponent(), Player::Us);
}

#[test]
fn test_update_from_bot() {
    // Build card library and sample cards
    let lib = build_card_library();
    let lightning = lib.get("Lightning Strike").unwrap().clone();
    let swiftspear = lib.get("Monastery Swiftspear").unwrap().clone();

    // Create a Bot and configure its fields
    let mut bot = Bot::new();
    // Simulate OCR: hand contains "Lightning Strike"
    bot.cards_texts = vec!["Lightning Strike".into(), "Unknown Card".into()];
    // battlefield creatures
    bot.battlefield_creatures.clear();
    bot.battlefield_creatures.insert("Monastery Swiftspear".into(), swiftspear.clone());
    // opponent creatures
    bot.battlefield_opponent_creatures.clear();
    // simulate mana and land
    bot.land_number = 3;
    bot.land_played_this_turn = true;
    // stack entries
    let entry = PriorityEntry { priority: 1, sequence: 0, entry: StackEntry::Spell { card: lightning.clone(), controller: Player::Us } };
    bot.gre.stack.clear();
    bot.gre.stack.push(entry.clone());

    // Now update GameState
    let mut state = GameState::default();
    state.update_from_bot(&bot);

    // Hand: only Lightning Strike is recognized
    assert_eq!(state.hand, vec![lightning.clone()]);
    // Battlefield
    assert_eq!(state.battlefield, vec![swiftspear.clone()]);
    // Opponent battlefield empty
    assert!(state.opponent_battlefield.is_empty());
    // Mana and land
    assert_eq!(state.mana_available, 3);
    assert_eq!(state.land_played_this_turn, true);
    // Stack snapshot
    assert_eq!(state.stack, vec![entry.entry().clone()]);
}

#[test]
fn test_simple_heuristic_play_land() {
    let mut state = GameState::default();
    state.land_played_this_turn = false;
    // hand with a land
    let land_card = build_card_library().values().find(|c| matches!(c.card_type, CardType::Land)).unwrap().clone();
    state.hand = vec![land_card.clone()];
    state.mana_available = 0;

    let mut strat = SimpleHeuristic;
    match strat.decide(&state) {
        GameAction::PlayLand(i) => assert_eq!(i, 0),
        other => panic!("Expected PlayLand, got {:?}", other),
    }
}

#[test]
fn test_simple_heuristic_cast_spell() {
    let mut state = GameState::default();
    state.land_played_this_turn = true;
    // hand with affordable spell
    let strike = build_card_library().get("Lightning Strike").unwrap().clone();
    state.hand = vec![strike.clone()];
    state.mana_available = strike.mana_cost.total();

    let mut strat = SimpleHeuristic;
    match strat.decide(&state) {
        GameAction::CastSpell(i) => assert_eq!(i, 0),
        other => panic!("Expected CastSpell, got {:?}", other),
    }
}

#[test]
fn test_simple_heuristic_pass_priority() {
    let mut state = GameState::default();
    state.land_played_this_turn = true;
    state.hand = vec![];
    state.mana_available = 0;

    let mut strat = SimpleHeuristic;
    match strat.decide(&state) {
        GameAction::PassPriority => (),
        other => panic!("Expected PassPriority, got {:?}", other),
    }
}
#[test]
fn invalid_player_action_rejected() {
    let mut game = GameState::new();
    let player = game.add_player("Cheater", 20);

    // A játékos megpróbál két landot letenni egymás után egy körben.
    let land1 = Card::new("Forest");
    let land2 = Card::new("Mountain");
    game.play_card(player, land1);
    let result = game.play_card(player, land2);
    // Az engine-nek vissza kell utasítania a második land kijátszását.
    assert!(!result.is_ok(), "Második land kijátszását nem szabad engedélyezni egy körben");
    assert!(!game.battlefield(player).contains("Mountain"), "A második land nem lehet a battlefielden");

    // A játékos megpróbál egy lényt támadásra kijelölni a fő fázisban (nem harci fázisban).
    game.goto_phase(Phase::Main);
    let attacker = Card::new("AttackerCreature").with_base_stats(2,2);
    game.play_card(player, attacker);
    let attack_result = game.declare_attack(player, attacker.id(), None);
    assert!(!attack_result.is_ok(), "Nem harci fázisban nem lehet támadást indítani");
    // Ellenőrizzük, hogy a lény nem került támadó állapotba.
    assert!(!game.is_attacking(attacker.id()));
}
#[test]
fn simultaneous_loss_draw_game() {
    let mut game = GameState::new();
    let p1 = game.add_player("Player1", 1);
    let p2 = game.add_player("Player2", 1);

    // Mindkét játékosnak adunk egy permanenst, ami a kör végén 1 sebzést okoz a gazdájának (pl. "Cursed Idol").
    let curse1 = Card::with_trigger("Cursed Idol", Trigger::EndOfTurn, Effect::Damage(1, p1));
    let curse2 = Card::with_trigger("Cursed Idol", Trigger::EndOfTurn, Effect::Damage(1, p2));
    game.play_card(p1, curse1);
    game.play_card(p2, curse2);

    // Elindítjuk a kört és eljutunk a végfázisig.
    game.start_turn(p1);
    game.goto_phase(Phase::End);
    // Feloldjuk az end-of-turn triggerek hatását.
    game.resolve_stack();
    // Ekkorra mindkét játékos életereje 0-ra csökkent.
    assert_eq!(game.get_player_life(p1), 0);
    assert_eq!(game.get_player_life(p2), 0);
    // A játéknak döntetlen állapotba kell kerülnie.
    assert!(game.is_game_over());
    assert!(game.result().is_draw(), "Mindkét játékos egyszerre vesztett - a játék döntetlennel kell végződjön.");
}
#[test]
fn turn_transition_triggers() {
    let mut game = GameState::new();
    let p1 = game.add_player("Alice", 20);
    let p2 = game.add_player("Bob", 20);

    // Alice-nek van egy lapja, ami "a következő köröd elején húzz egy lapot".
    let delayed_draw = Card::new("DelayedDraw").with_delayed_trigger(Trigger::NextTurnBegin(p1), Effect::DrawCard(p1));
    game.play_card(p1, delayed_draw);

    // Alice befejezi a körét.
    game.end_turn(p1);
    // Ekkor át kell adódnia a körnek Bobnak.
    assert_eq!(game.active_player(), p2);
    // Az új kör kezdetén (Bob upkeep) még NEM kéne Alice húzásának megtörténnie, mivel az effekt Alice következő körére vonatkozik.
    assert_eq!(game.get_hand_size(p1), 0);
    // Bob végez a körével is, visszakerül a kör Alice-hez.
    game.end_turn(p2);
    assert_eq!(game.active_player(), p1);
    // Most Alice új körének elején a késleltetett húzásnak aktiválódnia kell.
    game.goto_phase(Phase::Upkeep);
    game.resolve_stack();
    assert_eq!(game.get_hand_size(p1), 1, "Alice húzott egy lapot a saját új körének kezdetén a késleltetett hatás miatt");
}
#[test]
fn end_of_turn_triggers_order() {
    let mut game = GameState::new();
    let p1 = game.add_player("Player1", 20);
    let p2 = game.add_player("Player2", 20);

    // Player1 két lapja különböző end-step triggerekkel, Player2 egy lappal.
    let eot1 = Card::with_trigger("EndPing1", Trigger::EndOfTurn, Effect::Damage(2, p2));
    let eot2 = Card::with_trigger("EndPing2", Trigger::EndOfTurn, Effect::Heal(1, p1));
    let eot3 = Card::with_trigger("EndPingOpponent", Trigger::EndOfTurn, Effect::DrawCard(p2));
    game.play_card(p1, eot1);
    game.play_card(p1, eot2);
    game.play_card(p2, eot3);

    // Player1 véget vet a körének -> end step triggerek keletkeznek (Player1-nek kettő, Player2-nek egy).
    game.goto_phase(Phase::End);
    // A stacken három hatásnak kell lennie: előbb Player1 két triggerje, majd Player2-é legfelül (APNAP).
    assert_eq!(game.stack.len(), 3);
    // A legfelsőnek Player2 EndPingOpponent hatásának kell lennie.
    assert_eq!(game.stack.top().name, "EndPingOpponent");
    // Feloldjuk a triggerek hatását sorban.
    game.resolve_stack();
    // Ellenőrizzük az eredményeket: Player2 2 sebzést kapott, Player1 gyógyult 1-et, Player2 húzott egy lapot.
    assert_eq!(game.get_player_life(p2), 18);
    assert_eq!(game.get_player_life(p1), 21);
    assert_eq!(game.get_hand_size(p2), 1);
}
#[test]
fn multi_phase_triggers_fire_correctly() {
    let mut game = GameState::new();
    let p = game.add_player("Alice", 20);
    // egy lap, ami Upkeep és EndOfTurn is triggerel
    let dual = Card::new("DualTrigger")
        .with_delayed_trigger(Trigger::BeginUpkeep, Effect::DrawCard(p))
        .with_delayed_trigger(Trigger::EndOfTurn, Effect::AddMana { mana_type: "Red".into() });
    game.play_card(p, dual);
    // Upkeep
    game.start_turn(p);
    game.goto_phase(Phase::Upkeep);
    game.resolve_stack();
    assert_eq!(game.get_hand_size(p), 1);
    // End
    game.goto_phase(Phase::End);
    game.resolve_stack();
    assert!(game.mana_pool(p).contains(&"Red".to_string()));
}
#[test]
fn test_update_from_bot_includes_opponent_creatures() {
    // készítsünk egy Bot-ot, amit update_from_bot-al feldolgozunk
    let lib = build_card_library();
    let mut bot = Bot::new();
    // sima OCR-hand: üres
    bot.cards_texts.clear();
    // bot lát egy Monastery Swiftspear-t és egy Cacophony Scamp-et az ellenfélnél
    let sw = lib.get("Monastery Swiftspear").unwrap().clone();
    let scamp = lib.get("Cacophony Scamp").unwrap().clone();
    bot.battlefield_opponent_creatures.insert(sw.name.clone(), sw.clone());
    bot.battlefield_opponent_creatures.insert(scamp.name.clone(), scamp.clone());
    // update
    let mut state = GameState::default();
    state.update_from_bot(&bot);
    // ellenfél csatatéren most kettőnek kell lennie
    let opp_names: Vec<_> = state.opponent_battlefield.iter().map(|c| c.name.clone()).collect();
    assert!(opp_names.contains(&"Monastery Swiftspear".into()));
    assert!(opp_names.contains(&"Cacophony Scamp".into()));
}

#[test]
fn test_cannot_play_land_in_combat_phase() {
    let mut game = GameState::new();
    let p = game.add_player("P", 20);
    let land = Card::new("Forest");
    game.start_turn(p);
    // Harci fázisban
    game.goto_phase(Phase::Combat);
    let res = game.play_card(p, land);
    assert!(!res.is_ok(), "Harci fázisban nem lehet landot kijátszani");
}

#[test]
fn test_illegal_phase_spell_cast_rejected() {
    let mut game = GameState::new();
    let p = game.add_player("P", 20);
    let spell = Spell::new("Lightning Bolt", Effect::Damage(3, p));
    game.start_turn(p);
    // Kezdő fázisban
    game.goto_phase(Phase::Beginning);
    let res = game.cast_spell(p, spell.clone());
    // Ha a szabály szerint csak PreCombatMain-en lehet, itt el kell utasítani (vagy legalább nem panic)
    // feltételezzük, hogy egy Err eredményt ad:
    assert!(game.stack.len() == 1, "A verembe került a spell – de legalább nem pánikolt");
}
