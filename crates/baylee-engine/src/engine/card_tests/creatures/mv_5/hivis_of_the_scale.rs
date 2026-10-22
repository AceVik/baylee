//! `cards/creatures/mv_5/hivis_of_the_scale.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index::{HIVIS_OF_THE_SCALE, SHIVAN_DRAGON};

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

/// Hivis of the Scale (c4ccc997-2886-4f97-90cb-b767f62c3b3d) — {3}{R}{R} 3/4:
/// "You may choose not to untap Hivis of the Scale during your untap step.
/// {T}: Gain control of target Dragon for as long as you control Hivis of the
/// Scale and Hivis of the Scale remains tapped."
#[test]
fn hivis_of_the_scale_gains_control_of_target_dragon_while_tapped() {
    let mut engine = Duel::new(SEED, mountain())
        .battlefield(0, &[HIVIS_OF_THE_SCALE])
        .battlefield(1, &[SHIVAN_DRAGON])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    let dragon = on_battlefield(&engine, P1, SHIVAN_DRAGON).unwrap();

    activate(&mut engine, P0, HIVIS_OF_THE_SCALE, 1);
    aim_at(&mut engine, P0, dragon);
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(engine.state().object(dragon).unwrap().controller, P0);
}
