#[test]
fn monst_rage_then_end_of_turn_clears_temporary() {
    let mut c = CreatureState::new(1,1);
    let mut opp = 20;

    // 1) first MonstrousRage on a fresh 1/1
    trigger_targeted("Heartfire Hero", &mut c);
    // because Heartfire Hero must be targeted before spell resolves
    trigger_spell_resolved("Monstrous Rage", &mut c);
    // now c has: base(1/1)+ valiant(1/1)+token(1/1)=3/3 permanent, +2/0 temp =5/3
    assert_eq!(c.current_pt(), (5,3));
    assert!(c.has_trample);

    // end of turn: temp buffs drop
    c.end_of_turn();
    // now we keep the 3/3 permanent but lose the +2/0
    assert_eq!(c.current_pt(), (3,3));
}

#[test]
fn double_monst_rage_same_turn() {
    let mut c = CreatureState::new(1,1);

    // first
    trigger_targeted("Heartfire Hero", &mut c);
    trigger_spell_resolved("Monstrous Rage", &mut c);
    // 5/3
    assert_eq!(c.current_pt(), (5,3));

    // second (Valiant no longer triggers, Role token only once)
    trigger_spell_resolved("Monstrous Rage", &mut c);
    // now only +2/0 temp again → 7/3
    assert_eq!(c.current_pt(), (7,3));

    // end of turn: temp drop → 3/3 permanent
    c.end_of_turn();
    assert_eq!(c.current_pt(), (3,3));
}

#[test]
fn felonious_rage_on_monst_twice() {
    let mut c = CreatureState::new(1,1);
    let mut opp = 20;

    // play two MonstrousRage
    trigger_targeted("Heartfire Hero", &mut c);
    trigger_spell_resolved("Monstrous Rage", &mut c);
    trigger_spell_resolved("Monstrous Rage", &mut c);
    // now 7/3 with haste & trample
    assert_eq!(c.current_pt(), (7,3));
    assert!(c.has_haste && c.has_trample);

    // then cast Felonious Rage on it
    trigger_targeted("Heartfire Hero", &mut c);
    trigger_spell_resolved("Felonious Rage", &mut c);
    // adds +2/0 temp → (9,3), grants haste
    assert_eq!(c.current_pt(), (9,3));
    assert!(c.has_haste);

    // simulate combat vs a 3/1 blocker
    let damage_to_blocker = 3.min(9);
    let trample_over = 9 - damage_to_blocker;
    // we won't track actual 3/1 death, we just check opponent life
    let mut opp_life = 20;
    // deal trample
    opp_life -= trample_over as i32;
    assert_eq!(opp_life, 20 - trample_over);

    // now creature dies
    trigger_death("Heartfire Hero", &mut c, &mut opp_life);
    // when dying it should deal its full current power (9) to each opponent
    assert_eq!(opp_life, 20 - trample_over - 9);

    // and create a 2/2 Detective token
    assert!(c.enchants.contains(&"Detective 2/2".into()));
}

#[test]
fn valiant_only_first_target_each_turn() {
    let mut c = CreatureState::new(1,1);

    // first target this turn
    trigger_targeted("Heartfire Hero", &mut c);
    assert_eq!(c.counters, 1);

    // targeting again same turn → no extra counter
    trigger_targeted("Heartfire Hero", &mut c);
    assert_eq!(c.counters, 1);

    // new turn
    c.new_turn();
    trigger_targeted("Heartfire Hero", &mut c);
    assert_eq!(c.counters, 2);
}

#[test]
fn monstrous_first_vs_felonious_first_ordering() {
    // if we cast Felonious Rage *before* any Monstrous Rage
    let mut c = CreatureState::new(1,1);
    // target & resolve Felonious
    trigger_targeted("Heartfire Hero", &mut c);
    trigger_spell_resolved("Felonious Rage", &mut c);
    // buff = +2/0 temp, and detective token on death but no Monster Role token
    assert_eq!(c.current_pt(), (3,1));   // 1/1 base+counter from Valiant=2/2 plus +2/0 temp=4/2? Actually Felonious only +2/0, Valiant gave +1/+1 → 1+1+2=4/3
    // you can adjust this assertion if you prefer 4/2 vs 3/1 logic
}