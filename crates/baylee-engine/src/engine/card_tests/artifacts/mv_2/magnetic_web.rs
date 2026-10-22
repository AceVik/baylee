//! `cards/artifacts/mv_2/magnetic_web.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use crate::engine::card_tests::partials_w2c_kit::glyph_counters;
use baylee_core::generated::index;

const P0: PlayerId = PlayerId::new(0);

fn magnetic_web() -> CardIndex {
    card_index("c62c8006-5650-43cb-8247-da097aa42376")
}

/// Magnetic Web (`c62c8006-5650-43cb-8247-da097aa42376`) — {2} Artifact (Partial):
/// "{1}, {T}: Put a magnet counter on target creature."
#[test]
fn magnetic_web_puts_magnet_counter_on_target_creature() {
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[forest(), forest(), forest(), index::GRIZZLY_BEARS])
        .hand(0, &[magnetic_web()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    cast_from_hand(&mut engine, P0, magnetic_web());
    pass_until(&mut engine, stack_is_empty);

    let bears = on_battlefield(&engine, P0, index::GRIZZLY_BEARS).unwrap();
    tap_all_mana(&mut engine, P0);
    activate(&mut engine, P0, magnetic_web(), 0);
    aim_at(&mut engine, P0, bears);
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(glyph_counters(&engine, bears), 1);
}
