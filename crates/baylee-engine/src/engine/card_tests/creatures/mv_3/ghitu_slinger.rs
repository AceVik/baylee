#[allow(clippy::wildcard_imports)]
use super::*;

fn ghitu_slinger() -> CardIndex {
    card_index("8e2b5f90-e405-4b24-a71a-0988c37cf67a")
}

#[test]
fn ghitu_slinger_deals_two_damage_to_any_target_on_etb() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, mountain())
        .hand(0, &[ghitu_slinger()])
        .battlefield(0, &[mountain(), mountain(), mountain()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    cast_from_hand(&mut engine, p0, ghitu_slinger());
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseTargets { .. })
    });
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![],
                players: vec![p1],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(engine.state().players[1].life, 18);
}
