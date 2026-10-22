//! `cards/creatures/mv_6/canopy_dragon.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index::CANOPY_DRAGON;

const P0: PlayerId = PlayerId::new(0);

/// Canopy Dragon (a9e59ee4-02a6-498c-82f0-f4f2ff334d65) — {4}{G}{G} 4/4:
/// "Trample; {1}{G}: This creature gains flying and loses trample until end of turn."
#[test]
fn canopy_dragon_swaps_trample_for_flying_until_end_of_turn() {
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[CANOPY_DRAGON, forest(), forest()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    let dragon = on_battlefield(&engine, P0, CANOPY_DRAGON).unwrap();
    let kw_before = keywords(&engine, dragon);
    assert!(kw_before.contains(KeywordSet::TRAMPLE));
    assert!(!kw_before.contains(KeywordSet::FLYING));

    tap_all_mana(&mut engine, P0);
    activate(&mut engine, P0, CANOPY_DRAGON, 0);
    pass_until(&mut engine, stack_is_empty);

    let kw_after = keywords(&engine, dragon);
    assert!(kw_after.contains(KeywordSet::FLYING));
    assert!(!kw_after.contains(KeywordSet::TRAMPLE));
}
