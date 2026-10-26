//! `cards/enchantments/mv_3/saproling_cluster.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

#[test]
fn saproling_cluster_allows_any_player_to_discard_and_create_saproling() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[card_index("3b64051e-d320-4991-bc9e-6e1435a23c17"), forest()],
        )
        .battlefield(1, &[forest()])
        .hand(0, &[forest()])
        .hand(1, &[forest()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let card0 = in_hand(&engine, p0, forest()).expect("card in p0 hand");
    tap_all_mana(&mut engine, p0);
    activate(
        &mut engine,
        p0,
        card_index("3b64051e-d320-4991-bc9e-6e1435a23c17"),
        0,
    );
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![card0],
            },
        )
        .expect("p0 discards");
    pass_until(&mut engine, stack_is_empty);

    let p0_creatures = engine
        .state()
        .zones
        .list(ZoneLocation::Battlefield)
        .iter()
        .filter(|&&id| {
            let obj = engine.state().object(id).unwrap();
            obj.controller == p0 && obj.characteristics().types.contains(TypeSet::CREATURE)
        })
        .count();
    assert_eq!(p0_creatures, 1);

    // Opponent (p1) can also activate Saproling Cluster.
    let cluster = on_battlefield(
        &engine,
        p0,
        card_index("3b64051e-d320-4991-bc9e-6e1435a23c17"),
    )
    .expect("cluster");
    let card1 = in_hand(&engine, p1, forest()).expect("card in p1 hand");
    pass_until(
        &mut engine,
        |e| matches!(e.pending(), Pending::Priority { player, .. } if *player == p1),
    );
    tap_all_mana(&mut engine, p1);
    engine
        .apply(
            p1,
            PlayerAction::ActivateAbility {
                source: cluster,
                ability_index: 0,
            },
        )
        .expect("p1 activates Saproling Cluster");
    engine
        .apply(
            p1,
            PlayerAction::ChooseObjects {
                objects: vec![card1],
            },
        )
        .expect("p1 discards");
    pass_until(&mut engine, stack_is_empty);

    let p1_creatures = engine
        .state()
        .zones
        .list(ZoneLocation::Battlefield)
        .iter()
        .filter(|&&id| {
            let obj = engine.state().object(id).unwrap();
            obj.controller == p1 && obj.characteristics().types.contains(TypeSet::CREATURE)
        })
        .count();
    assert_eq!(p1_creatures, 1);
}
