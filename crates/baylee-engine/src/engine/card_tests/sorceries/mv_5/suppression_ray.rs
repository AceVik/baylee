//! `cards/sorceries/mv_5/suppression_ray.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// `Suppression Ray` // `Orderly Plaza` (`Coverage::Partial`): "Tap all creatures target player
/// controls. You may pay any amount of {E}. If you do, choose up to that many creatures tapped
/// this way. Put a stun counter on each of them. // This land enters tapped. {T}: Add {W} or {U}."
///
/// Under `Coverage::Partial`, the front-face effect is omitted, but the back face (`Orderly Plaza`)
/// is fully implemented. The test plays the land face via `play_land_face`, verifies that it enters
/// tapped, advances two turns until it untaps, and activates its mana ability to choose `{W}`
/// from `Pending::ChooseColor`.
#[test]
fn orderly_plaza_enters_tapped_and_taps_for_chosen_mana() {
    let (mut engine, land) = play_land_face(suppression_ray(), 1).expect("plays as Orderly Plaza");
    assert!(is_tapped(&engine, land), "Orderly Plaza enters tapped");

    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    reach_their_main_phase(&mut engine, p1);
    reach_their_main_phase(&mut engine, p0);

    assert!(!is_tapped(&engine, land), "untaps on next turn");
    let white_before = engine.state().players[0]
        .mana_pool
        .available(ManaColor::White);

    activate(&mut engine, p0, suppression_ray(), 0);
    let Pending::ChooseColor { options, .. } = engine.pending().clone() else {
        panic!("expected color choice, got {:?}", engine.pending())
    };
    assert_eq!(
        options,
        vec![ManaColor::White, ManaColor::Blue],
        "offers White and Blue"
    );
    engine
        .apply(p0, PlayerAction::ChooseColor(ManaColor::White))
        .unwrap();

    assert!(is_tapped(&engine, land), "tapped to activate mana ability");
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::White),
        white_before + 1,
        "adds one white mana to the pool"
    );
}
