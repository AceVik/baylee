//! `cards/sorceries/mv_2/word_of_binding.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;
use crate::engine::card_tests::legends_kit as lk;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn word_of_binding() -> CardIndex {
    card_index("8bbc9f21-2b4d-4d03-aef1-f5bf0b79257c")
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

/// Seat 0 holds Word of Binding and `swamps` Swamps, with a Grizzly Bears;
/// seat 1 has a Savannah Lions and a Hill Giant.
fn board(swamps: usize) -> Engine<RegistryLookup> {
    let mut mine = vec![swamp(); swamps];
    mine.push(grizzly_bears());
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &mine)
        .battlefield(1, &[savannah_lions(), hill_giant()])
        .hand(0, &[word_of_binding()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);
    engine
}

/// Word of Binding (8bbc9f21-2b4d-4d03-aef1-f5bf0b79257c) — "Tap X target
/// creatures." X = 2 ({2}{B}{B}) at two of the opponent's creatures; mine,
/// not named, stays untapped.
#[test]
fn word_of_binding_taps_exactly_x_target_creatures() {
    let mut engine = board(4);
    let mine = on_battlefield(&engine, P0, grizzly_bears()).expect("my Bears");
    let lions = on_battlefield(&engine, P1, savannah_lions()).expect("their Lions");
    let giant = on_battlefield(&engine, P1, hill_giant()).expect("their Giant");
    tap_all_mana(&mut engine, P0);
    lk::announce(
        &mut engine,
        P0,
        word_of_binding(),
        0,
        2,
        &[lions, giant],
        &[],
    );
    lk::settle(&mut engine);

    assert!(lk::tapped(&engine, lions), "target one is tapped");
    assert!(lk::tapped(&engine, giant), "target two is tapped");
    assert!(!lk::tapped(&engine, mine), "an unnamed creature is not");
    assert!(in_graveyard(&engine, P0, word_of_binding()).is_some());
}

/// Word of Binding — X = 1 taps one creature and leaves the other.
#[test]
fn word_of_binding_with_x_one_taps_only_one() {
    let mut engine = board(3);
    let lions = on_battlefield(&engine, P1, savannah_lions()).expect("their Lions");
    let giant = on_battlefield(&engine, P1, hill_giant()).expect("their Giant");
    tap_all_mana(&mut engine, P0);
    lk::announce(&mut engine, P0, word_of_binding(), 0, 1, &[giant], &[]);
    lk::settle(&mut engine);

    assert!(lk::tapped(&engine, giant));
    assert!(!lk::tapped(&engine, lions));
}

/// Word of Binding — "X target creatures" can be any creatures, mine
/// included, and X is what was paid: X = 3 taps all three.
#[test]
fn word_of_binding_may_tap_my_own_creature_too() {
    let mut engine = board(5);
    let mine = on_battlefield(&engine, P0, grizzly_bears()).expect("my Bears");
    let lions = on_battlefield(&engine, P1, savannah_lions()).expect("their Lions");
    let giant = on_battlefield(&engine, P1, hill_giant()).expect("their Giant");
    tap_all_mana(&mut engine, P0);
    lk::announce(
        &mut engine,
        P0,
        word_of_binding(),
        0,
        3,
        &[mine, lions, giant],
        &[],
    );
    lk::settle(&mut engine);

    for creature in [mine, lions, giant] {
        assert!(lk::tapped(&engine, creature));
    }
}

/// Word of Binding — X targets means X: naming one creature when X is 2 is
/// not a legal answer.
#[test]
fn word_of_binding_refuses_fewer_targets_than_x() {
    let mut engine = board(4);
    let lions = on_battlefield(&engine, P1, savannah_lions()).expect("their Lions");
    tap_all_mana(&mut engine, P0);
    cast_with_floating(&mut engine, P0, word_of_binding());
    engine
        .apply(P0, PlayerAction::ChooseNumber(2))
        .expect("X = 2 with {2}{B}{B} paid");
    let Pending::ChooseTargets { min, max, .. } = engine.pending().clone() else {
        panic!("a target choice was expected, got {:?}", engine.pending());
    };
    assert_eq!((min, max), (2, 2), "exactly X targets");
    assert!(
        engine
            .apply(
                P0,
                PlayerAction::ChooseTargets {
                    objects: vec![lions],
                    players: vec![]
                }
            )
            .is_err(),
        "one target is not X = 2 targets"
    );
}
