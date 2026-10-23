#[allow(clippy::wildcard_imports)]
use super::*;

fn avalanche_riders() -> CardIndex {
    card_index("c9048d35-aff7-45ae-ae97-db89397b3e74")
}

#[test]
fn avalanche_riders_destroys_target_land_on_etb_and_has_haste_and_echo() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, mountain())
        .hand(0, &[avalanche_riders()])
        .battlefield(0, &[mountain(), mountain(), mountain(), mountain()])
        .battlefield(1, &[island()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let opp_land = on_battlefield(&engine, p1, island()).unwrap();
    cast_from_hand(&mut engine, p0, avalanche_riders());
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseTargets { .. })
    });
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![opp_land],
                players: vec![],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert!(on_battlefield(&engine, p1, island()).is_none());
    assert!(in_graveyard(&engine, p1, island()).is_some());

    let riders = on_battlefield(&engine, p0, avalanche_riders()).unwrap();
    let _ = attack_and_collect_blocks(&mut engine, riders, p1);
    engine
        .apply(p1, PlayerAction::DeclareBlockers { blockers: vec![] })
        .unwrap();
    pass_until(&mut engine, |e| e.state().turn.phase == Phase::SecondMain);
    assert_eq!(
        engine.state().players[1].life,
        18,
        "haste permits attacking the turn it entered"
    );
    reach_their_main_phase(&mut engine, p1);
    reach_their_main_phase(&mut engine, p0);
    assert!(on_battlefield(&engine, p0, avalanche_riders()).is_none());
}
