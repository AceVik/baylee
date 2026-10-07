//! `cards/lands/restricted/ghost_town.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Ghost Town: "{T}: Add {C}." / "{0}: Return this land to its owner's hand. Activate only if it's not your turn."
/// Under `Coverage::Partial`, the not-your-turn timing restriction is omitted.
/// Activating ability 1 for {0} returns Ghost Town from the battlefield to its owner's hand.
#[test]
fn ghost_town_returns_to_hand_on_activation() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(113, forest())
        .battlefield(0, &[ghost_town()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    assert!(on_battlefield(&engine, p0, ghost_town()).is_some());
    assert!(in_hand(&engine, p0, ghost_town()).is_none());

    activate(&mut engine, p0, ghost_town(), 1);
    pass_until(&mut engine, stack_is_empty);

    assert!(on_battlefield(&engine, p0, ghost_town()).is_none());
    assert!(in_hand(&engine, p0, ghost_town()).is_some());
}
