//! `cards/sorceries/mv_1/peach_garden_oath.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;
use baylee_card_testkit::card_tests::legends_kit as lk;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn peach_garden_oath() -> CardIndex {
    card_index("ad66a99b-c7e7-4b2c-8525-299d3609d1df")
}

/// Peach Garden Oath (ad66a99b-c7e7-4b2c-8525-299d3609d1df) — {W} Sorcery:
/// "You gain 2 life for each creature you control."
#[test]
fn peach_garden_oath_gains_two_life_per_controlled_creature() {
    let mut engine = lk::game(
        SEED,
        &[
            plains(),
            sol_ring(),
            llanowar_elves(),
            aurochs(),
            fangren_hunter(),
        ],
        &[aurochs(), fangren_hunter()],
        &[peach_garden_oath()],
        &[],
    );

    cast_from_hand(&mut engine, P0, peach_garden_oath());
    lk::settle(&mut engine);

    assert_eq!(lk::life(&engine, P0), 26, "3 controlled creatures -> +6");
    assert_eq!(lk::life(&engine, P1), 20);
    assert!(in_graveyard(&engine, P0, peach_garden_oath()).is_some());

    let mut none = lk::game(
        SEED,
        &[plains(), sol_ring()],
        &[aurochs()],
        &[peach_garden_oath()],
        &[],
    );
    cast_from_hand(&mut none, P0, peach_garden_oath());
    lk::settle(&mut none);
    assert_eq!(lk::life(&none, P0), 20);
}
