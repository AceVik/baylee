#[allow(clippy::wildcard_imports)]
use super::*;

fn ticking_gnomes() -> CardIndex {
    card_index("963de959-848d-48df-90fa-88a6d988b9fe")
}

#[test]
fn ticking_gnomes_sacrifices_to_deal_damage() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, plains())
        .battlefield(0, &[plains(), plains(), plains()])
        .hand(0, &[ticking_gnomes()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    cast_from_hand(&mut engine, p0, ticking_gnomes());
    pass_until(&mut engine, stack_is_empty);

    let gnomes = on_battlefield(&engine, p0, ticking_gnomes()).unwrap();
    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source: gnomes,
                ability_index: 1,
            },
        )
        .unwrap();
    if matches!(engine.pending(), Pending::ChooseTargets { .. }) {
        engine
            .apply(
                p0,
                PlayerAction::ChooseTargets {
                    objects: vec![],
                    players: vec![p1],
                },
            )
            .unwrap();
    }
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(engine.state().players[1].life, 19);
    assert!(in_graveyard(&engine, p0, ticking_gnomes()).is_some());
}
