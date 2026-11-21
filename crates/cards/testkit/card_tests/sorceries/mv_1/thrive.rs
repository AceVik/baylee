//! `cards/sorceries/mv_1/thrive.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;
use baylee_card_testkit::card_tests::legends_kit as lk;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn thrive() -> CardIndex {
    card_index("fe3c524e-10b7-485a-9285-973092eeb214")
}

/// Thrive (fe3c524e-10b7-485a-9285-973092eeb214) — {X}{G} Sorcery:
/// "Put a +1/+1 counter on each of X target creatures."
#[test]
fn thrive_puts_a_plus_one_counter_on_each_of_x_target_creatures() {
    let mut engine = lk::game(
        SEED,
        &[forest(), forest(), forest(), aurochs(), fangren_hunter()],
        &[llanowar_elves()],
        &[thrive()],
        &[],
    );
    let beast = lk::obj(&engine, P0, aurochs());
    let hunter = lk::obj(&engine, P0, fangren_hunter());
    let their_elf = lk::obj(&engine, P1, llanowar_elves());

    tap_all_mana(&mut engine, P0);
    lk::announce(&mut engine, P0, thrive(), 0, 2, &[beast, hunter], &[]);
    lk::settle(&mut engine);

    assert_eq!(counters_on(&engine, beast, CounterKind::P1P1), 1);
    assert_eq!(pt(&engine, beast), (3, 4));
    assert_eq!(counters_on(&engine, hunter, CounterKind::P1P1), 1);
    assert_eq!(pt(&engine, hunter), (5, 5));
    assert_eq!(counters_on(&engine, their_elf, CounterKind::P1P1), 0);

    // X = 0 casts for {G} with zero targets.
    let mut zero = lk::game(SEED, &[forest()], &[aurochs()], &[thrive()], &[]);
    let untouched = lk::obj(&zero, P1, aurochs());
    tap_all_mana(&mut zero, P0);
    lk::announce(&mut zero, P0, thrive(), 0, 0, &[], &[]);
    lk::settle(&mut zero);
    assert_eq!(counters_on(&zero, untouched, CounterKind::P1P1), 0);
    assert!(in_graveyard(&zero, P0, thrive()).is_some());
}
