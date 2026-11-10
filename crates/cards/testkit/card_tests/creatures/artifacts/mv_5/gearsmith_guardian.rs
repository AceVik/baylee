//! `cards/creatures/artifacts/mv_5/gearsmith_guardian.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

fn gearsmith_guardian() -> CardIndex {
    card_index("0c0f7092-3fe6-4a6e-8b4e-803810b43e50")
}

/// Gearsmith Guardian — "This creature gets +2/+0 as long as you control a
/// blue creature." Mine counts, the opponent's does not.
#[test]
fn gearsmith_guardian_grows_only_beside_my_own_blue_creature() {
    let p0 = PlayerId::new(0);
    for (mine, theirs, expected) in [
        (vec![], vec![], (3, 5)),
        (vec![aven_envoy()], vec![], (5, 5)),
        (vec![], vec![aven_envoy()], (3, 5)),
        (vec![gray_ogre()], vec![], (3, 5)),
    ] {
        let mut board = vec![gearsmith_guardian()];
        board.extend(mine);
        let mut engine = Duel::new(SEED, forest())
            .battlefield(0, &board)
            .battlefield(1, &theirs)
            .start();
        keep_mulligans(&mut engine);
        reach_main_phase(&mut engine, p0);
        let guardian = on_battlefield(&engine, p0, gearsmith_guardian()).expect("seated");
        assert_eq!(pt(&engine, guardian), expected);
    }
}

/// The condition is tracked live: the blue creature dying takes the +2/+0 away.
#[test]
fn gearsmith_guardian_loses_the_bonus_when_the_blue_creature_leaves() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[gearsmith_guardian(), aven_envoy()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let guardian = on_battlefield(&engine, p0, gearsmith_guardian()).expect("seated");
    let envoy = on_battlefield(&engine, p0, aven_envoy()).expect("seated");
    assert_eq!(pt(&engine, guardian), (5, 5));
    kill(&mut engine, envoy);
    assert_eq!(pt(&engine, guardian), (3, 5));
}
