//! `cards/artifacts/mv_3/bone_flute.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

fn bone_flute() -> CardIndex {
    card_index("8664a004-d786-4357-8856-2f8f5a9ae24d")
}

fn grizzly_bears() -> CardIndex {
    card_index("14c8f55d-d177-4c25-a931-ebeb9e6062a0")
}

fn hill_giant() -> CardIndex {
    card_index("342199e0-15b6-4824-83da-25caef2592b3")
}

fn wall_of_wood() -> CardIndex {
    card_index("aa5e6894-8c97-46f8-a0fd-07c6db51e9a7")
}

fn a_flute_over_three_creatures() -> (Engine<RegistryLookup>, [ObjectId; 3]) {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[bone_flute(), grizzly_bears(), forest(), forest()])
        .battlefield(1, &[hill_giant(), wall_of_wood()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let bears = on_battlefield(&engine, p0, grizzly_bears()).expect("Bears");
    let giant = on_battlefield(&engine, p1, hill_giant()).expect("Giant");
    let wall = on_battlefield(&engine, p1, wall_of_wood()).expect("Wall");
    assert_eq!(pt(&engine, bears), (2, 2));
    assert_eq!(pt(&engine, giant), (3, 3));
    assert_eq!(pt(&engine, wall), (0, 3));
    (engine, [bears, giant, wall])
}

/// Bone Flute (8664a004-d786-4357-8856-2f8f5a9ae24d) — "{2}, {T}: All
/// creatures get -1/-0 until end of turn."
///
/// Every creature on the table, both sides', loses 1 power and no toughness;
/// the price is {2} and the Flute's own tap.
#[test]
fn bone_flute_gives_every_creature_minus_one_power() {
    let p0 = PlayerId::new(0);
    let (mut engine, [bears, giant, wall]) = a_flute_over_three_creatures();
    let flute = on_battlefield(&engine, p0, bone_flute()).expect("the Flute");
    assert_eq!(tap_all_mana(&mut engine, p0), 2, "the two Forests");
    activate(&mut engine, p0, bone_flute(), 0);
    assert!(is_tapped(&engine, flute), "{{T}} was paid");
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "{{2}} was paid"
    );
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(pt(&engine, bears), (1, 2), "mine");
    assert_eq!(pt(&engine, giant), (2, 3), "theirs");
    assert_eq!(pt(&engine, wall), (-1, 3), "a 0-power creature goes to -1");
}

/// "Until end of turn": the opponent's turn starts with everything restored.
#[test]
fn bone_flute_wears_off_at_end_of_turn() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let (mut engine, [bears, giant, wall]) = a_flute_over_three_creatures();
    tap_all_mana(&mut engine, p0);
    activate(&mut engine, p0, bone_flute(), 0);
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(pt(&engine, giant), (2, 3));

    reach_their_main_phase(&mut engine, p1);
    assert_eq!(pt(&engine, bears), (2, 2));
    assert_eq!(pt(&engine, giant), (3, 3));
    assert_eq!(pt(&engine, wall), (0, 3));
}
