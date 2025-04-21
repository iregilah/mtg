use std::collections::HashMap;

use app::card_attribute::Effect;
use app::card_library::{build_card_library, Card, CardType, Trigger};
use app::card_library::Creature;
use app::card_library::ManaCost;

fn get_card_mut<'a>(lib: &'a mut HashMap<String, Card>, name: &str) -> &'a mut Card {
    lib.get_mut(name).expect(&format!("Card '{}' not found in library", name))
}

#[test]
fn cacophony_scamp_and_felonious_rage_combo() {
    // 1) Build a fresh library
    let mut library = build_card_library();

    // 2) Pull out our two cards
    let scamp = get_card_mut(&mut library, "Cacophony Scamp");
    let rage  = get_card_mut(&mut library, "Felonious Rage");

    // --- Step A: Cast Cacophony Scamp; no immediate effects expected on cast-resolved ---
    let effects = scamp.trigger_on_cast_resolved();
    assert!(effects.is_empty(), "Scamp should not get anything on cast-resolve");

    // --- Step B: Cast Felonious Rage targeting the Scamp ---
    let mut target_effects = rage.trigger_on_targeted();
    // We expect exactly two: +2/+0 and Haste
    assert!(target_effects.contains(&Effect::SelfAttributeChange(
        app::card_attribute::AttributeChange { power: 2, toughness: 0 }) ));
    assert!(target_effects.contains(&Effect::Haste),
            "Felonious Rage should grant Haste");

    // --- Apply those two effects to the scamp ourselves in test ---
    if let CardType::Creature(ref mut cr) = scamp.card_type {
        cr.power += 2;
        cr.summoning_sickness = false;  // Haste clears it
    } else {
        panic!("Scamp is not a Creature!");
    }

    // Now Scamp should be 3/1 and able to attack
    if let CardType::Creature(ref cr) = scamp.card_type {
        assert_eq!(cr.power, 3);
        assert_eq!(cr.toughness, 1);
        assert_eq!(cr.summoning_sickness, false);
    }

    // --- Step C: Simulate dealing combat damage to a player ---
    let mut combat_effects = scamp.trigger_on_combat_damage();
    // We expect two effects: sacrifice prompt (assume we sac) & proliferate
    assert!(combat_effects.iter().any(|e| matches!(e, Effect::DestroyTarget{..})),
            "Should prompt destroy/sacrifice itself");
    assert!(combat_effects.contains(&Effect::Poliferate { counter_type: app::card_attribute::CounterType::PlusOnePlusOne }),
            "Should proliferate");

    // --- Step D: Simulate that scamp dies this turn after we sacrifice it ---
    let death_effects = scamp.trigger_on_death();
    // We expect two: damage equal to power to any target, and spawn detective token
    assert!(death_effects.iter().any(|e| matches!(e, Effect::DamageTarget{..})),
            "Should deal damage equal to its power");
    assert!(death_effects.contains(&Effect::SpawnNewCreature),
            "Should spawn the 2/2 detective");
}
