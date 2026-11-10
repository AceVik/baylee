//! `cards/artifacts/mv_5/mercadian_atlas.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

// oracle_id = "5bae9ffb-99b3-4feb-8aed-265d874fd68b"
fn mercadian_atlas() -> CardIndex {
    card_index("5bae9ffb-99b3-4feb-8aed-265d874fd68b")
}

/// `Mercadian Atlas` (`Coverage::Implemented`):
/// "At the beginning of your end step, if you didn't play a land this turn, you may draw a card."
///
/// Verifies that `Mercadian Atlas` triggers and draws a card on your end step when
/// no land was played this turn, and does not trigger on a turn you played a land.
#[test]
fn mercadian_atlas_draws_at_end_step_only_when_no_land_played() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(9403, forest())
        .battlefield(0, &[mercadian_atlas()])
        .hand(0, &[forest()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0));

    let hand_before = engine.state().zones.list(ZoneLocation::Hand(p0)).len();
    // Advance to P0's end step without playing a land.
    pass_until(&mut engine, |e| {
        matches!(
            e.pending(),
            Pending::YesNo {
                prompt: YesNoPrompt::MayDo,
                ..
            }
        )
    });
    engine.apply(p0, PlayerAction::YesNo(true)).unwrap();
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(p0)).len(),
        hand_before + 1,
        "Mercadian Atlas draws a card when no land was played this turn"
    );

    // Pass through P1's turn to P0's next main phase, then play a land.
    assert!(walk_to_own_main(&mut engine, p1));
    assert!(walk_to_own_main(&mut engine, p0));
    let land = in_hand(&engine, p0, forest()).expect("Forest in hand");
    engine
        .apply(p0, PlayerAction::PlayLand { card: land })
        .unwrap();
    let hand_after_land = engine.state().zones.list(ZoneLocation::Hand(p0)).len();

    // Advance into P1's turn: Mercadian Atlas must not trigger on P0's end step.
    assert!(walk_to_own_main(&mut engine, p1));
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(p0)).len(),
        hand_after_land,
        "Mercadian Atlas does not trigger when a land was played this turn"
    );
}
