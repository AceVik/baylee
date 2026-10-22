//! `cards/creatures/mv_4/ekundu_cyclops.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index::EKUNDU_CYCLOPS;

const P0: PlayerId = PlayerId::new(0);

/// Ekundu Cyclops (6c89e4e6-c4d7-416c-b985-41739c6f5b50) — {3}{R} 3/4:
/// "If a creature you control attacks, this creature also attacks if able."
#[test]
fn ekundu_cyclops_casts_and_enters_as_three_four() {
    let mut engine = Duel::new(SEED, mountain())
        .battlefield(0, &[mountain(), mountain(), mountain(), mountain()])
        .hand(0, &[EKUNDU_CYCLOPS])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    tap_all_mana(&mut engine, P0);
    cast_with_floating(&mut engine, P0, EKUNDU_CYCLOPS);
    pass_until(&mut engine, stack_is_empty);

    let cyclops = on_battlefield(&engine, P0, EKUNDU_CYCLOPS).unwrap();
    assert_eq!(pt(&engine, cyclops), (3, 4));
}
