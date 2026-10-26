//! `cards/creatures/mv_2/darting_merfolk.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

const P0: PlayerId = PlayerId::new(0);

fn subject_card() -> CardIndex {
    card_index("2f33e684-a4e4-4528-9a20-3df902bc2cde")
}

/// Darting Merfolk — {1}{U} 1/1 Creature — Merfolk:
/// "{U}: Return this creature to its owner's hand."
#[test]
fn darting_merfolk_returns_itself_to_owners_hand() {
    let mut engine = Duel::new(SEED, island())
        .battlefield(0, &[island(), island(), island()])
        .hand(0, &[subject_card()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    cast_from_hand(&mut engine, P0, subject_card());
    pass_until(&mut engine, stack_is_empty);
    assert!(on_battlefield(&engine, P0, subject_card()).is_some());

    activate(&mut engine, P0, subject_card(), 0);
    pass_until(&mut engine, stack_is_empty);

    assert!(on_battlefield(&engine, P0, subject_card()).is_none());
    assert!(in_hand(&engine, P0, subject_card()).is_some());
}
