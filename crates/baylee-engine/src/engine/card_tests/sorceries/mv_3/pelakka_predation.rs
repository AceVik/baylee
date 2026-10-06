//! `cards/sorceries/mv_3/pelakka_predation.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// `Pelakka Predation` // `Pelakka Caverns` (`Coverage::Partial`): "Target opponent reveals
/// their hand. You choose a card from it with mana value 3 or greater. That player discards
/// that card. // This land enters tapped. {T}: Add {B}."
///
/// Under `Coverage::Partial`, the front-face targeted discard clause is not expressible in
/// the DSL, but the back face (`Pelakka Caverns`) is fully implemented. The test plays the
/// land face, confirms it enters tapped, advances to the next turn so it untaps, and
/// activates its mana ability to add `{B}`.
#[test]
fn pelakka_caverns_enters_tapped_and_taps_for_black_mana() {
    let (mut engine, caverns) =
        play_land_face(pelakka_predation(), 1).expect("plays as Pelakka Caverns");
    assert!(is_tapped(&engine, caverns), "Pelakka Caverns enters tapped");

    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    reach_their_main_phase(&mut engine, p1);
    reach_their_main_phase(&mut engine, p0);

    assert!(!is_tapped(&engine, caverns), "untaps on next turn");
    let black_before = engine.state().players[0]
        .mana_pool
        .available(ManaColor::Black);

    activate(&mut engine, p0, pelakka_predation(), 0);

    assert!(
        is_tapped(&engine, caverns),
        "tapped to activate mana ability"
    );
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Black),
        black_before + 1,
        "adds one black mana to the pool"
    );
}
