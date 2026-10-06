//! `cards/creatures/mv_4/disease_carriers.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// `Disease Carriers` is a `{2}{B}{B}` 2/2 Rat under `Coverage::Implemented`.
/// When it dies, its triggered ability targets a creature to give it -2/-2 until end of turn.
/// When destroyed by `heroes_downfall()`, the card moves to the graveyard and its `Trigger::Dies` ability
/// triggers, prompting `Pending::ChooseTargets` where targeting an opponent's 6/6 creature reduces its power
/// and toughness to 4/4 until end of turn.
#[test]
fn disease_carriers_dies_gives_target_creature_minus_two_minus_two() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, swamp())
        .battlefield(0, &[disease_carriers(), swamp(), swamp(), swamp()])
        .battlefield(1, &[rootbreaker_wurm()])
        .hand(0, &[heroes_downfall()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    let carriers =
        on_battlefield(&engine, p0, disease_carriers()).expect("Disease Carriers on battlefield");
    let wurm = on_battlefield(&engine, p1, rootbreaker_wurm())
        .expect("opponent Rootbreaker Wurm on battlefield");
    assert_eq!(pt(&engine, carriers), (2, 2));
    assert_eq!(pt(&engine, wurm), (6, 6));

    cast_from_hand(&mut engine, p0, heroes_downfall());

    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!(
            "expected ChooseTargets for downfall, got {:?}",
            engine.pending()
        );
    };
    assert!(options.contains(&carriers));
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![carriers],
                players: vec![],
            },
        )
        .unwrap();

    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseTargets { .. })
    });

    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!(
            "expected ChooseTargets for dies trigger, got {:?}",
            engine.pending()
        );
    };
    assert!(
        options.contains(&wurm),
        "opponent's creature is an available target for the dies trigger"
    );

    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![wurm],
                players: vec![],
            },
        )
        .unwrap();

    pass_until(&mut engine, stack_is_empty);

    assert!(
        in_graveyard(&engine, p0, disease_carriers()).is_some(),
        "Disease Carriers is in graveyard"
    );
    assert_eq!(
        pt(&engine, wurm),
        (4, 4),
        "target creature receives -2/-2 until end of turn"
    );
}
