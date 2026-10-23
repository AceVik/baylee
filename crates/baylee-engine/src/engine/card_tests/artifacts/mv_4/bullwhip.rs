//! `cards/artifacts/mv_4/bullwhip.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index;

const P0: PlayerId = PlayerId::new(0);

fn bullwhip() -> CardIndex {
    card_index("d4e7a9f5-5228-4e41-8f58-6e7453fe68dc")
}

/// Bullwhip (`d4e7a9f5-5228-4e41-8f58-6e7453fe68dc`) — {4} Artifact (Implemented):
/// "{2}, {T}: This artifact deals 1 damage to target creature. That creature attacks this turn if able."
#[test]
fn bullwhip_pings_target_creature_and_forces_attack() {
    let mut engine = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[
                forest(),
                forest(),
                forest(),
                forest(),
                forest(),
                forest(),
                index::GRIZZLY_BEARS,
            ],
        )
        .hand(0, &[bullwhip()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    cast_from_hand(&mut engine, P0, bullwhip());
    pass_until(&mut engine, stack_is_empty);

    let bears = on_battlefield(&engine, P0, index::GRIZZLY_BEARS).unwrap();
    activate(&mut engine, P0, bullwhip(), 0);
    aim_at(&mut engine, P0, bears);
    pass_until(&mut engine, stack_is_empty);

    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    let Pending::ChooseAttackers { required, .. } = engine.pending().clone() else {
        panic!("expected ChooseAttackers");
    };
    assert!(required.contains(&bears));
}
