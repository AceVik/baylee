//! `cards/instants/mv_3/valakut_awakening.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Valakut Awakening // Valakut Stoneforge (`Coverage::Partial`): "Put any number
/// of cards from your hand on the bottom of your library, then draw that many
/// cards plus one. // This land enters tapped. {T}: Add {R}."
///
/// Under `Coverage::Partial`, the front-face hand cycling is not expressible
/// in the DSL, but the back face (Valakut Stoneforge) is built in full. The test
/// plays the back face as a land, confirms it enters tapped, advances to the
/// next turn so it untaps, and activates its mana ability to add `{R}`.
#[test]
fn valakut_stoneforge_enters_tapped_and_taps_for_red_mana() {
    let (mut engine, stoneforge) =
        play_land_face(valakut_awakening(), 1).expect("plays as Valakut Stoneforge");
    assert!(
        is_tapped(&engine, stoneforge),
        "Valakut Stoneforge enters tapped"
    );

    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    reach_their_main_phase(&mut engine, p1);
    reach_their_main_phase(&mut engine, p0);

    assert!(!is_tapped(&engine, stoneforge), "untaps on next turn");
    let red_before = engine.state().players[0]
        .mana_pool
        .available(ManaColor::Red);

    activate(&mut engine, p0, valakut_awakening(), 0);

    assert!(
        is_tapped(&engine, stoneforge),
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
