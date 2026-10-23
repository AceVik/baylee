#[allow(clippy::wildcard_imports)]
use super::*;

fn peace_and_quiet() -> CardIndex {
    card_index("0517f736-e6eb-4fdb-8288-e0c8a9a44379")
}

#[test]
fn peace_and_quiet_destroys_two_target_enchantments() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, plains())
        .battlefield(0, &[plains(), plains()])
        .battlefield(1, &[wizard_class(), wizard_class()])
        .hand(0, &[peace_and_quiet()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let targets: Vec<ObjectId> = engine
        .state()
        .battlefield_seen()
        .filter(|&id| {
            engine
                .state()
                .object(id)
                .is_some_and(|o| o.controller == p1)
        })
        .collect();
    assert_eq!(targets.len(), 2);

    let wrong = on_battlefield(&engine, p0, plains()).unwrap();
    cast_from_hand(&mut engine, p0, peace_and_quiet());
    let Pending::ChooseTargets { options, .. } = engine.pending() else {
        unreachable!()
    };
    assert!(!options.contains(&wrong), "a land is not a legal target");
    for objects in [vec![targets[0]], vec![targets[0], targets[0]]] {
        assert!(
            engine
                .apply(
                    p0,
                    PlayerAction::ChooseTargets {
                        objects,
                        players: vec![]
                    }
                )
                .is_err()
        );
    }
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: targets,
                players: vec![],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert!(on_battlefield(&engine, p1, wizard_class()).is_none());
}
