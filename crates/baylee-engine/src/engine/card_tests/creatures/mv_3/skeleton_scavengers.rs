//! `cards/creatures/mv_3/skeleton_scavengers.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use crate::engine::card_tests::partials_w2c_kit::power_toughness;
use baylee_core::generated::index;

const P0: PlayerId = PlayerId::new(0);

fn skeleton_scavengers() -> CardIndex {
    card_index("88782913-bfba-4300-b26c-405e6a45537c")
}

/// Skeleton Scavengers (`88782913-bfba-4300-b26c-405e6a45537c`) — {2}{B} 0/0 Creature — Skeleton (Partial):
/// "This creature enters with a +1/+1 counter on it. {1}: Regenerate this creature and put a +1/+1 counter on it."
#[test]
fn skeleton_scavengers_enters_with_counter_and_regenerates_with_another_counter() {
    let mut engine = Duel::new(SEED, swamp())
        .battlefield(0, &[swamp(), swamp(), swamp(), swamp(), mountain()])
        .hand(0, &[skeleton_scavengers(), index::LIGHTNING_BOLT])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    tap_all_mana(&mut engine, P0);
    cast_with_floating(&mut engine, P0, skeleton_scavengers());
    pass_until(&mut engine, stack_is_empty);

    let skel = on_battlefield(&engine, P0, skeleton_scavengers()).unwrap();
    assert_eq!(power_toughness(&engine, skel), (1, 1));

    activate(&mut engine, P0, skeleton_scavengers(), 0);
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(power_toughness(&engine, skel), (2, 2));

    cast_with_floating(&mut engine, P0, index::LIGHTNING_BOLT);
    aim_at(&mut engine, P0, skel);
    pass_until(&mut engine, stack_is_empty);

    assert!(
        on_battlefield(&engine, P0, skeleton_scavengers()).is_some(),
        "Skeleton Scavengers should regenerate from Lightning Bolt"
    );
}
