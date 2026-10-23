#[allow(clippy::wildcard_imports)]
use super::*;

fn extruder() -> CardIndex {
    card_index("93cc6425-a13b-4b32-bb8f-fdc9bfa0ef9b")
}

#[test]
fn extruder_sacrifices_artifact_to_put_plus_one_counter_on_target_creature() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, mountain())
        .hand(0, &[extruder()])
        .battlefield(
            0,
            &[
                sol_ring(),
                gray_ogre(),
                mountain(),
                mountain(),
                mountain(),
                mountain(),
            ],
        )
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let ring = on_battlefield(&engine, p0, sol_ring()).unwrap();
    let ogre = on_battlefield(&engine, p0, gray_ogre()).unwrap();

    tap_mana_except(&mut engine, p0, ring);
    cast_with_floating(&mut engine, p0, extruder());
    pass_until(&mut engine, stack_is_empty);

    activate(&mut engine, p0, extruder(), 1);
    if matches!(engine.pending(), Pending::ChooseCards { .. }) {
        engine
            .apply(
                p0,
                PlayerAction::ChooseObjects {
                    objects: vec![ring],
                },
            )
            .unwrap();
    }
    if matches!(engine.pending(), Pending::ChooseTargets { .. }) {
        engine
            .apply(
                p0,
                PlayerAction::ChooseTargets {
                    objects: vec![ogre],
                    players: vec![],
                },
            )
            .unwrap();
    }
    if matches!(engine.pending(), Pending::ChooseCards { .. }) {
        engine
            .apply(
                p0,
                PlayerAction::ChooseObjects {
                    objects: vec![ring],
                },
            )
            .unwrap();
    }
    pass_until(&mut engine, stack_is_empty);

    assert!(in_graveyard(&engine, p0, sol_ring()).is_some());
    assert_eq!(pt(&engine, ogre), (3, 3));
}
