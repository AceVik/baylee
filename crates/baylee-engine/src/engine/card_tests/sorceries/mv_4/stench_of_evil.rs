//! `cards/sorceries/mv_4/stench_of_evil.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn stench_of_evil() -> CardIndex {
    card_index("14062abe-14df-4dbe-ad7b-530e5f6c4988")
}

/// Stench of Evil (14062abe-14df-4dbe-ad7b-530e5f6c4988) — {2}{B}{B} Sorcery:
/// "Destroy all Plains."
/// (Coverage::Partial — per-Plains damage unless controller pays {2} not supported.)
#[test]
fn stench_of_evil_destroys_all_plains_and_leaves_non_plains_alone() {
    let mut engine = Duel::new(SEED, swamp())
        .battlefield(0, &[swamp(), swamp(), swamp(), swamp(), plains()])
        .battlefield(1, &[plains(), island()])
        .hand(0, &[stench_of_evil()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    cast_from_hand(&mut engine, P0, stench_of_evil());
    pass_until(&mut engine, stack_is_empty);

    assert!(
        on_battlefield(&engine, P0, plains()).is_none()
            && in_graveyard(&engine, P0, plains()).is_some(),
        "P0's Plains was destroyed"
    );
    assert!(
        on_battlefield(&engine, P1, plains()).is_none()
            && in_graveyard(&engine, P1, plains()).is_some(),
        "P1's Plains was destroyed"
    );
    assert!(
        on_battlefield(&engine, P0, swamp()).is_some()
            && on_battlefield(&engine, P1, island()).is_some(),
        "non-Plains lands are untouched"
    );
}
