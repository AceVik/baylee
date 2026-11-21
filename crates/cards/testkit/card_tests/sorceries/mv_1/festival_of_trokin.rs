//! `cards/sorceries/mv_1/festival_of_trokin.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;
use baylee_card_testkit::card_tests::legends_kit as lk;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn festival_of_trokin() -> CardIndex {
    card_index("a5a3cd2c-20ae-49a8-88fa-e8e3e7cc50db")
}

fn grizzly_bears() -> CardIndex {
    card_index("14c8f55d-d177-4c25-a931-ebeb9e6062a0")
}

/// Festival of Trokin (a5a3cd2c-20ae-49a8-88fa-e8e3e7cc50db) — {W} Sorcery:
/// "You gain 2 life for each creature you control." Counts only creatures the
/// caster controls, ignoring noncreature permanents and opposing creatures.
#[test]
fn festival_of_trokin_gains_two_life_per_controlled_creature() {
    let mut engine = lk::game(
        SEED,
        &[
            plains(),
            sol_ring(),
            llanowar_elves(),
            grizzly_bears(),
            aurochs(),
        ],
        &[grizzly_bears(), aurochs()],
        &[festival_of_trokin()],
        &[],
    );

    cast_from_hand(&mut engine, P0, festival_of_trokin());
    lk::settle(&mut engine);

    assert_eq!(
        lk::life(&engine, P0),
        26,
        "3 controlled creatures -> +6 life"
    );
    assert_eq!(lk::life(&engine, P1), 20);
    assert!(in_graveyard(&engine, P0, festival_of_trokin()).is_some());
}

/// Festival of Trokin — with zero controlled creatures, gains 0 life.
#[test]
fn festival_of_trokin_gains_zero_life_with_no_controlled_creatures() {
    let mut engine = lk::game(
        SEED,
        &[plains(), sol_ring()],
        &[grizzly_bears()],
        &[festival_of_trokin()],
        &[],
    );

    cast_from_hand(&mut engine, P0, festival_of_trokin());
    lk::settle(&mut engine);

    assert_eq!(lk::life(&engine, P0), 20);
    assert_eq!(lk::life(&engine, P1), 20);
}
