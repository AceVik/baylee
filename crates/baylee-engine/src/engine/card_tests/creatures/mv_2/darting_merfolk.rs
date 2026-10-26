//! `cards/creatures/mv_2/darting_merfolk.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index;

const P0: PlayerId = PlayerId::new(0);

/// Darting Merfolk — {1}{U} 1/1 Creature — Merfolk:
/// "{U}: Return this creature to its owner's hand."
#[test]
fn darting_merfolk_returns_itself_to_owners_hand() {
    let mut engine = Duel::new(SEED, island())
        .battlefield(0, &[island(), island(), island()])
        .hand(0, &[index::DARTING_MERFOLK])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    cast_from_hand(&mut engine, P0, index::DARTING_MERFOLK);
    pass_until(&mut engine, stack_is_empty);
    assert!(on_battlefield(&engine, P0, index::DARTING_MERFOLK).is_some());

    activate(&mut engine, P0, index::DARTING_MERFOLK, 0);
    pass_until(&mut engine, stack_is_empty);

    assert!(on_battlefield(&engine, P0, index::DARTING_MERFOLK).is_none());
    assert!(in_hand(&engine, P0, index::DARTING_MERFOLK).is_some());
}
