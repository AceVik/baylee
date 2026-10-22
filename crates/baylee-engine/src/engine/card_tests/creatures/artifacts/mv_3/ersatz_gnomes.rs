//! `cards/creatures/artifacts/mv_3/ersatz_gnomes.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index::{ERSATZ_GNOMES, GRIZZLY_BEARS};

const P0: PlayerId = PlayerId::new(0);

/// Ersatz Gnomes (ea016b84-7f77-42e7-bd14-2e62a1e35f3b) — {3} 1/1 Artifact Creature:
/// - "{T}: Target spell becomes colorless."
/// - "{T}: Target permanent becomes colorless until end of turn."
#[test]
fn ersatz_gnomes_makes_target_permanent_colorless_until_end_of_turn() {
    let mut engine = Duel::new(SEED, plains())
        .battlefield(0, &[ERSATZ_GNOMES, GRIZZLY_BEARS])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    let bears = on_battlefield(&engine, P0, GRIZZLY_BEARS).unwrap();
    assert!(
        !engine
            .state()
            .object(bears)
            .unwrap()
            .characteristics()
            .colors
            .is_empty()
    );

    activate(&mut engine, P0, ERSATZ_GNOMES, 1);
    aim_at(&mut engine, P0, bears);
    pass_until(&mut engine, stack_is_empty);

    assert!(
        engine
            .state()
            .object(bears)
            .unwrap()
            .characteristics()
            .colors
            .is_empty()
    );
}
