//! `cards/creatures/mv_3/mindwhip_sliver.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use crate::engine::card_tests::partials_w1b_kit as kit;
use baylee_core::generated::index;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn mindwhip_sliver() -> CardIndex {
    card_index("e9e43839-7cc1-40ff-a8ec-a47316de6440")
}

/// Mindwhip Sliver (`e9e43839-7cc1-40ff-a8ec-a47316de6440`) — {2}{B} Creature — Sliver (Partial):
/// "All Slivers have '{2}, Sacrifice this permanent: Target player discards a card at random. Activate only as a sorcery.'"
#[test]
fn mindwhip_sliver_grants_sacrifice_random_discard_ability() {
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[swamp(), swamp(), swamp(), forest(), forest()])
        .hand(0, &[mindwhip_sliver()])
        .hand(1, &[index::GRIZZLY_BEARS])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    cast_from_hand(&mut engine, P0, mindwhip_sliver());
    pass_until(&mut engine, stack_is_empty);

    assert!(in_hand(&engine, P1, index::GRIZZLY_BEARS).is_some());
    let sliver = on_battlefield(&engine, P0, mindwhip_sliver()).unwrap();
    tap_all_mana(&mut engine, P0);
    let idx = kit::offered(&engine, sliver)[0];
    kit::activate_object(&mut engine, P0, sliver, idx);
    pass_until(&mut engine, stack_is_empty);

    assert!(in_graveyard(&engine, P0, mindwhip_sliver()).is_some());
    assert!(in_graveyard(&engine, P1, index::GRIZZLY_BEARS).is_some());
}
