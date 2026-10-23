#[allow(clippy::wildcard_imports)]
use super::*;
use crate::engine::card_tests::legends_kit as lk;

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

#[test]
fn ghitu_slinger_can_pay_echo_once_and_survives() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = lk::game(SEED, &[], &[], &[ghitu_slinger()], &[]);
    lk::float(&mut engine, p0, &[(ManaColor::Red, 3)]);
    cast_with_floating(&mut engine, p0, ghitu_slinger());
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseTargets { .. })
    });
    lk::aim(&mut engine, &[], &[p1]);
    lk::settle(&mut engine);
    assert_eq!(lk::life(&engine, p1), 18);
    reach_their_main_phase(&mut engine, p1);
    pass_until(&mut engine, |e| {
        e.state().turn.active == p0
            && e.state().turn.step == crate::turn::Step::Upkeep
            && matches!(e.pending(), Pending::Priority { .. })
    });
    lk::float(&mut engine, p0, &[(ManaColor::Red, 3)]);
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::YesNo { .. })
    });
    lk::yes_no(&mut engine, true);
    reach_main_phase(&mut engine, p0);
    assert!(on_battlefield(&engine, p0, ghitu_slinger()).is_some());
    reach_their_main_phase(&mut engine, p1);
    reach_their_main_phase(&mut engine, p0);
    assert!(on_battlefield(&engine, p0, ghitu_slinger()).is_some());
}
