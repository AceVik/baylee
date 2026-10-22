//! `cards/artifacts/mv_4/amber_prison.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index::{AMBER_PRISON, GRIZZLY_BEARS};

#[test]
fn amber_prison_taps_and_locks_down_while_tapped() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, plains())
        .battlefield(
            0,
            &[AMBER_PRISON, plains(), plains(), plains(), plains()],
        )
        .battlefield(1, &[GRIZZLY_BEARS])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let target = on_battlefield(&engine, p1, GRIZZLY_BEARS).unwrap();

    tap_all_mana(&mut engine, p0);
    activate(&mut engine, p0, AMBER_PRISON, 1);
    aim_at(&mut engine, p0, target);
    pass_until(&mut engine, stack_is_empty);

    assert!(
        engine
            .state()
            .object(target)
            .unwrap()
            .status
            .contains(Status::TAPPED),
        "Amber Prison should tap the target permanent"
    );

    // Advance to p1's first main phase; target should remain tapped.
    reach_their_main_phase(&mut engine, p1);
    assert!(
        engine
            .state()
            .object(target)
            .unwrap()
            .status
            .contains(Status::TAPPED),
        "target should not untap while Amber Prison remains tapped"
    );
}
