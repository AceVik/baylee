//! `cards/creatures/mv_3/tracker.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

fn tracker() -> CardIndex {
    card_index("86260d53-a68e-43cb-88ba-6a97c0b2f4f4")
}

fn hill_giant() -> CardIndex {
    card_index("342199e0-15b6-4824-83da-25caef2592b3")
}

fn float(engine: &mut Engine<RegistryLookup>, seat: usize, mana: &[(ManaColor, u32)]) {
    let state = engine
        .dev_state_mut(PlayerId::new(0))
        .expect("the harness may set boards up");
    for (color, n) in mana {
        state.players[seat].mana_pool.add(*color, *n);
    }
    engine.refresh_offer();
}

fn board(theirs: CardIndex) -> Engine<RegistryLookup> {
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[tracker()])
        .hand(0, &[giant_growth()])
        .battlefield(1, &[theirs])
        .start();
    keep_mulligans(&mut engine);
    assert!(
        walk_to_own_main(&mut engine, PlayerId::new(0)),
        "p0 reaches its own main"
    );
    engine
}

/// Tracker (86260d53-a68e-43cb-88ba-6a97c0b2f4f4) — "{G}{G}, {T}: This
/// creature deals damage equal to its power to target creature. That
/// creature deals damage equal to its power to this creature."
#[test]
fn tracker_kills_a_smaller_creature_and_takes_its_power_back() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = board(llanowar_elves());
    let ours = on_battlefield(&engine, p0, tracker()).expect("the Tracker is out");
    let elves = on_battlefield(&engine, p1, llanowar_elves()).expect("their Elves are out");
    float(&mut engine, 0, &[(ManaColor::Green, 2)]);
    activate(&mut engine, p0, tracker(), 0);
    let options = aim_at(&mut engine, p0, elves);
    assert!(options.contains(&elves), "any creature is a target");
    pass_until(&mut engine, stack_is_empty);

    assert!(
        in_graveyard(&engine, p1, llanowar_elves()).is_some(),
        "2 damage kills the 1/1"
    );
    assert_eq!(
        engine
            .state()
            .object(ours)
            .expect("the Tracker lives")
            .damage,
        1,
        "the Elves' power of 1 comes back"
    );
    assert!(is_tapped(&engine, ours), "{{T}} is part of the cost");
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "{{G}}{{G}} paid"
    );
}

/// A bigger creature survives the 2 damage and kills the Tracker.
#[test]
fn tracker_dies_to_a_bigger_creature_that_survives() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = board(hill_giant());
    let giant = on_battlefield(&engine, p1, hill_giant()).expect("their Giant is out");
    float(&mut engine, 0, &[(ManaColor::Green, 2)]);
    activate(&mut engine, p0, tracker(), 0);
    aim_at(&mut engine, p0, giant);
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        engine
            .state()
            .object(giant)
            .expect("the Giant lives")
            .damage,
        2,
        "the Tracker's power of 2"
    );
    assert!(
        in_graveyard(&engine, p0, tracker()).is_some(),
        "3 damage kills the 2/2 Tracker"
    );
}

/// "Equal to its power": pumped to 5/5, the Tracker kills the Giant and
/// takes the Giant's 3 without dying.
#[test]
fn tracker_deals_damage_equal_to_its_current_power() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = board(hill_giant());
    let ours = on_battlefield(&engine, p0, tracker()).expect("the Tracker is out");
    let giant = on_battlefield(&engine, p1, hill_giant()).expect("their Giant is out");
    float(&mut engine, 0, &[(ManaColor::Green, 3)]);
    cast_with_floating(&mut engine, p0, giant_growth());
    aim_at(&mut engine, p0, ours);
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(pt(&engine, ours), (5, 5));

    activate(&mut engine, p0, tracker(), 0);
    aim_at(&mut engine, p0, giant);
    pass_until(&mut engine, stack_is_empty);
    assert!(
        in_graveyard(&engine, p1, hill_giant()).is_some(),
        "5 damage kills the Giant"
    );
    assert_eq!(
        engine
            .state()
            .object(ours)
            .expect("the Tracker lives")
            .damage,
        3,
        "and the Giant's 3 comes back"
    );
}
