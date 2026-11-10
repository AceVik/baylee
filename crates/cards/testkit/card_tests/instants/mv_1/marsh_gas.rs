//! `cards/instants/mv_1/marsh_gas.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

fn marsh_gas() -> CardIndex {
    card_index("08ae9f91-48c8-4721-a82d-063defc1b727")
}

fn grizzly_bears() -> CardIndex {
    card_index("14c8f55d-d177-4c25-a931-ebeb9e6062a0")
}

fn savannah_lions() -> CardIndex {
    card_index("60ba93eb-39e6-4af2-9c66-cd38f72daff2")
}

fn hill_giant() -> CardIndex {
    card_index("342199e0-15b6-4824-83da-25caef2592b3")
}

fn elves() -> CardIndex {
    card_index("68954295-54e3-4303-a6bc-fc4547a4e3a3")
}

/// Marsh Gas (08ae9f91-48c8-4721-a82d-063defc1b727) — "All creatures get
/// -2/-0 until end of turn."
///
/// Every creature on both sides loses two power and nothing else, and the
/// effect is gone when the turn is.
#[test]
fn marsh_gas_gives_every_creature_minus_two_power_until_end_of_turn() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, swamp())
        .battlefield(0, &[swamp(), grizzly_bears(), savannah_lions()])
        .hand(0, &[marsh_gas()])
        .battlefield(1, &[hill_giant()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");
    let bears = on_battlefield(&engine, p0, grizzly_bears()).expect("the Bears are out");
    let lions = on_battlefield(&engine, p0, savannah_lions()).expect("the Lions are out");
    let giant = on_battlefield(&engine, p1, hill_giant()).expect("the Giant is out");
    assert_eq!(pt(&engine, bears), (2, 2));
    assert_eq!(pt(&engine, lions), (2, 1));
    assert_eq!(pt(&engine, giant), (3, 3));

    cast_from_hand(&mut engine, p0, marsh_gas());
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(pt(&engine, bears), (0, 2), "my Bears lose power only");
    assert_eq!(pt(&engine, lions), (0, 1), "so do my Lions");
    assert_eq!(pt(&engine, giant), (1, 3), "and the opponent's Giant");

    pass_until(&mut engine, |e| {
        e.state().turn.active == p1 && matches!(e.state().turn.phase, Phase::FirstMain)
    });
    assert_eq!(pt(&engine, bears), (2, 2), "until end of turn");
    assert_eq!(pt(&engine, lions), (2, 1));
    assert_eq!(pt(&engine, giant), (3, 3));
}

/// "All creatures" is the set of creatures when the spell resolves: a
/// creature that enters afterwards is not shrunk.
#[test]
fn marsh_gas_does_not_reach_a_creature_that_enters_afterwards() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, swamp())
        .battlefield(0, &[swamp(), forest(), grizzly_bears()])
        .hand(0, &[marsh_gas(), elves()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");
    let bears = on_battlefield(&engine, p0, grizzly_bears()).expect("the Bears are out");

    tap_all_mana(&mut engine, p0);
    cast_with_floating(&mut engine, p0, marsh_gas());
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(pt(&engine, bears), (0, 2), "the Bears were there to be hit");

    cast_with_floating(&mut engine, p0, elves());
    pass_until(&mut engine, stack_is_empty);
    let elf = on_battlefield(&engine, p0, elves()).expect("the Elves resolved");
    assert_eq!(
        pt(&engine, elf),
        (1, 1),
        "the Elves came too late to be hit"
    );
    assert_eq!(pt(&engine, bears), (0, 2), "the Bears are still shrunk");
}
