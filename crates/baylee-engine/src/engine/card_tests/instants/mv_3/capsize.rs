//! `cards/instants/mv_3/capsize.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn capsize() -> CardIndex {
    card_index("77637eff-2963-4402-88f3-ca346f762fc8")
}

/// Capsize (77637eff-2963-4402-88f3-ca346f762fc8) — {1}{U}{U} Instant (Partial):
/// "Return target permanent to its owner's hand."
#[test]
fn capsize_returns_target_permanent_to_owners_hand() {
    let mut engine = Duel::new(SEED, island())
        .battlefield(0, &[island(), island(), island()])
        .battlefield(1, &[index::GRIZZLY_BEARS])
        .hand(0, &[capsize()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    let bears = on_battlefield(&engine, P1, index::GRIZZLY_BEARS).unwrap();
    cast_from_hand(&mut engine, P0, capsize());
    aim_at(&mut engine, P0, bears);
    pass_until(&mut engine, stack_is_empty);

    assert!(on_battlefield(&engine, P1, index::GRIZZLY_BEARS).is_none());
    assert!(in_hand(&engine, P1, index::GRIZZLY_BEARS).is_some());
}
