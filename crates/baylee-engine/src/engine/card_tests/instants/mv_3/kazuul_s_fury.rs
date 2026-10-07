//! `cards/instants/mv_3/kazuul_s_fury.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Kazuul's Fury // Kazuul's Cliffs (`Coverage::Partial`): "As an additional
/// cost to cast this spell, sacrifice a creature. Kazuul's Fury deals damage
/// equal to the sacrificed creature's power to any target. // This land enters
/// tapped. {T}: Add {R}."
///
/// Under `Coverage::Partial`, the front-face sacrifice cost and damage are
/// not implemented, but the back face (Kazuul's Cliffs) is built in full.
/// The test plays the back face as a land, confirms it enters tapped, advances
/// to the next turn so it untaps, and activates its mana ability to add `{R}`.
#[test]
fn kazuuls_cliffs_enters_tapped_and_taps_for_red_mana() {
    let (mut engine, cliffs) =
        play_land_face(kazuul_s_fury(), 1).expect("plays as Kazuul's Cliffs");
    assert!(is_tapped(&engine, cliffs), "Kazuul's Cliffs enters tapped");

    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    reach_their_main_phase(&mut engine, p1);
    reach_their_main_phase(&mut engine, p0);

    assert!(!is_tapped(&engine, cliffs), "untaps on next turn");
    let red_before = engine.state().players[0]
        .mana_pool
        .available(ManaColor::Red);

    activate(&mut engine, p0, kazuul_s_fury(), 0);

    assert!(
        is_tapped(&engine, cliffs),
        "tapped to activate mana ability"
    );
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Red),
        red_before + 1,
        "adds one red mana to the pool"
    );
}
