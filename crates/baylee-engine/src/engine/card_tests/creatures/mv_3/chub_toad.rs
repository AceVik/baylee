//! `cards/creatures/mv_3/chub_toad.rs`, played.
//!
//! Oracle:
//! - "Whenever this creature blocks or becomes blocked, it gets +2/+2 until end of turn."

#[allow(clippy::wildcard_imports)]
use super::*;
use crate::engine::card_tests::legends_kit as lk;
use baylee_core::generated::index;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

// oracle_id = "a0daace2-92fc-4c7d-b497-94b4ba775559"
fn chub_toad() -> CardIndex {
    card_index("a0daace2-92fc-4c7d-b497-94b4ba775559")
}

#[test]
fn chub_toad_gets_plus_two_plus_two_when_it_becomes_blocked() {
    let mut engine = lk::game(SEED, &[chub_toad()], &[index::GRIZZLY_BEARS], &[], &[]);
    let toad = lk::obj(&engine, P0, chub_toad());
    let bears = lk::obj(&engine, P1, index::GRIZZLY_BEARS);

    assert_eq!(pt(&engine, toad), (1, 1));

    let _ = lk::attack(&mut engine, &[toad]);
    lk::block(&mut engine, &[(bears, toad)]);

    pass_until(&mut engine, stack_is_empty);
    assert_eq!(
        pt(&engine, toad),
        (3, 3),
        "Chub Toad gets +2/+2 until end of turn when it becomes blocked"
    );

    // Advance past combat damage: 3/3 Chub Toad kills 2/2 Grizzly Bears and survives.
    lk::through_combat(&mut engine);
    assert!(on_battlefield(&engine, P0, chub_toad()).is_some());
    assert!(on_battlefield(&engine, P1, index::GRIZZLY_BEARS).is_none());
}
