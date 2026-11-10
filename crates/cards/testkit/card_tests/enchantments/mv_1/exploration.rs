//! `cards/enchantments/mv_1/exploration.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Exploration: "You may play an additional land on each of your turns."
/// With Exploration on the battlefield, the active player is permitted two land plays in a turn.
/// After playing the first land from hand, a second land drop remains legal, but playing a third is refused.
#[test]
fn exploration_allows_playing_additional_land() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(102, forest())
        .battlefield(0, &[exploration()])
        .hand(0, &[forest(), forest()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let f1 = play_land(&mut engine, p0, forest());
    assert!(on_battlefield(&engine, p0, forest()).is_some());

    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority after first land drop");
    };
    assert!(!legal.lands.is_empty(), "second land drop offered");

    let f2 = play_land(&mut engine, p0, forest());
    assert_ne!(f1, f2);

    let Pending::Priority {
        legal: legal_after, ..
    } = engine.pending().clone()
    else {
        panic!("expected priority after second land drop");
    };
    assert!(legal_after.lands.is_empty(), "no third land drop offered");
}
