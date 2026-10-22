//! `cards/enchantments/mv_5/cadaverous_bloom.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index::CADAVEROUS_BLOOM;

const P0: PlayerId = PlayerId::new(0);

/// Cadaverous Bloom (fbb0f73b-5e30-4632-99c1-e49582e41f8d) — {3}{B}{G} Enchantment
/// (Partial: exile-from-hand mana ability not yet supported by `pay_cost`).
#[test]
fn cadaverous_bloom_casts_and_enters_battlefield() {
    let mut engine = Duel::new(SEED, swamp())
        .battlefield(0, &[swamp(), swamp(), swamp(), forest(), forest()])
        .hand(0, &[CADAVEROUS_BLOOM])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    tap_all_mana(&mut engine, P0);
    cast_with_floating(&mut engine, P0, CADAVEROUS_BLOOM);
    pass_until(&mut engine, stack_is_empty);

    assert!(on_battlefield(&engine, P0, CADAVEROUS_BLOOM).is_some());
}
