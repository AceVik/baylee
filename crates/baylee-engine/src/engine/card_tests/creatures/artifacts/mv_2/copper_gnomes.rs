#[allow(clippy::wildcard_imports)]
use super::*;

fn copper_gnomes() -> CardIndex {
    card_index("4fd6d9af-4d53-4e10-bc23-672454803099")
}

#[test]
fn copper_gnomes_sacrifices_to_put_artifact_from_hand_onto_battlefield() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, plains())
        .hand(0, &[sol_ring(), grizzly_bears()])
        .battlefield(
            0,
            &[copper_gnomes(), plains(), plains(), plains(), plains()],
        )
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let ring = in_hand(&engine, p0, sol_ring()).expect("in hand");
    tap_all_mana(&mut engine, p0);
    activate(&mut engine, p0, copper_gnomes(), 0);
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseCards { .. }) || stack_is_empty(e)
    });
    if let Pending::ChooseCards { options, min, .. } = engine.pending().clone() {
        assert_eq!(min, 0, "putting an artifact is optional");
        assert!(options.contains(&ring));
        assert!(!options.contains(&in_hand(&engine, p0, grizzly_bears()).unwrap()));
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

    assert!(on_battlefield(&engine, p0, copper_gnomes()).is_none());
    assert!(on_battlefield(&engine, p0, sol_ring()).is_some());
}
