//! `cards/sorceries/mv_3/seeds_of_innocence.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use crate::engine::card_tests::partials_w2c_kit::bears as grizzly_bears;

fn seeds_of_innocence() -> CardIndex {
    card_index("f5ebad5c-9675-4a37-b76c-760416083939")
}

/// Seeds of Innocence (`{1}{G}{G}`, `Coverage::Partial`): destroys all
/// artifacts with no regeneration.
#[test]
fn seeds_of_innocence_destroys_all_artifacts() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[forest(), forest(), forest(), sol_ring()])
        .battlefield(1, &[sol_ring(), grizzly_bears()])
        .hand(0, &[seeds_of_innocence()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    cast_from_hand(&mut engine, p0, seeds_of_innocence());
    pass_until(&mut engine, stack_is_empty);

    assert!(on_battlefield(&engine, p0, sol_ring()).is_none());
    assert!(on_battlefield(&engine, p1, sol_ring()).is_none());
    assert!(on_battlefield(&engine, p1, grizzly_bears()).is_some());
}
