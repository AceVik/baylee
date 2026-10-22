//! `cards/creatures/artifacts/mv_3/serrated_biskelion.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index::{GRIZZLY_BEARS, SERRATED_BISKELION};

/// `Serrated Biskelion` ({3}, Artifact Creature — Construct 2/2) taps to put a
/// -1/-1 counter on itself and a -1/-1 counter on target creature.
#[test]
fn serrated_biskelion_puts_m1m1_counters_on_self_and_target() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, plains())
        .battlefield(0, &[SERRATED_BISKELION])
        .battlefield(1, &[GRIZZLY_BEARS])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let biskelion = on_battlefield(&engine, p0, SERRATED_BISKELION).unwrap();
    let bears = on_battlefield(&engine, p1, GRIZZLY_BEARS).unwrap();
    activate(&mut engine, p0, SERRATED_BISKELION, 0);
    aim_at(&mut engine, p0, bears);
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(pt(&engine, biskelion), (1, 1));
    assert_eq!(pt(&engine, bears), (1, 1));
}
