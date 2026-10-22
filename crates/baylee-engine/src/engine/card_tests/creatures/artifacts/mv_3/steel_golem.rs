//! `cards/creatures/artifacts/mv_3/steel_golem.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index::{GRIZZLY_BEARS, STEEL_GOLEM};

/// `Steel Golem` ({3}, Artifact Creature — Golem 3/4) prevents its controller
/// from casting creature spells.
#[test]
fn steel_golem_prevents_controller_from_casting_creature_spells() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[STEEL_GOLEM, forest(), forest()])
        .hand(0, &[GRIZZLY_BEARS])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let bears = in_hand(&engine, p0, GRIZZLY_BEARS).unwrap();
    tap_all_mana(&mut engine, p0);
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority");
    };
    assert!(
        !legal.castable.contains(&bears),
        "Steel Golem forbids its controller from casting creature spells"
    );
}
