//! `cards/instants/mv_3/holy_light.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

fn holy_light() -> CardIndex {
    card_index("8f7dd328-b367-4546-ba79-1efc1e97f2ab")
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

fn jasmine_boreal() -> CardIndex {
    card_index("942407bf-862a-4a0b-b19d-b9f8ee833026")
}

fn elves() -> CardIndex {
    card_index("68954295-54e3-4303-a6bc-fc4547a4e3a3")
}

/// Holy Light (8f7dd328-b367-4546-ba79-1efc1e97f2ab) — "Nonwhite creatures
/// get -1/-1 until end of turn."
///
/// Green and red creatures shrink on both sides of the table, a 1/1 dies, a
/// white creature and a green-white one (which is white) do not shrink, and
/// the effect ends with the turn.
#[test]
fn holy_light_shrinks_nonwhite_creatures_only_until_end_of_turn() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, plains())
        .battlefield(
            0,
            &[
                plains(),
                plains(),
                plains(),
                grizzly_bears(),
                savannah_lions(),
            ],
        )
        .hand(0, &[holy_light()])
        .battlefield(1, &[hill_giant(), elves(), jasmine_boreal()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");
    let bears = on_battlefield(&engine, p0, grizzly_bears()).expect("the Bears are out");
    let lions = on_battlefield(&engine, p0, savannah_lions()).expect("the Lions are out");
    let giant = on_battlefield(&engine, p1, hill_giant()).expect("the Giant is out");
    let jasmine = on_battlefield(&engine, p1, jasmine_boreal()).expect("Jasmine is out");

    cast_from_hand(&mut engine, p0, holy_light());
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(pt(&engine, bears), (1, 1), "green Bears shrink");
    assert_eq!(pt(&engine, giant), (2, 2), "the red Giant shrinks");
    assert_eq!(pt(&engine, lions), (2, 1), "white Lions do not");
    assert_eq!(pt(&engine, jasmine), (4, 5), "green-white Jasmine is white");
    assert!(
        in_graveyard(&engine, p1, elves()).is_some(),
        "a green 1/1 gets -1/-1 and dies"
    );

    pass_until(&mut engine, |e| {
        e.state().turn.active == p1 && matches!(e.state().turn.phase, Phase::FirstMain)
    });
    assert_eq!(pt(&engine, bears), (2, 2), "until end of turn");
    assert_eq!(pt(&engine, giant), (3, 3));
}
