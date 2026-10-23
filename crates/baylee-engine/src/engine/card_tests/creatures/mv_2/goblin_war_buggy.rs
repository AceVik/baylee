#[allow(clippy::wildcard_imports)]
use super::*;

fn goblin_war_buggy() -> CardIndex {
    card_index("59fb1640-d8ed-476a-a78d-91d60b6faac5")
}

#[test]
fn goblin_war_buggy_enters_with_haste_and_echo() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, mountain())
        .hand(0, &[goblin_war_buggy()])
        .battlefield(0, &[mountain(), mountain()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    cast_from_hand(&mut engine, p0, goblin_war_buggy());
    pass_until(&mut engine, stack_is_empty);

    let perm = on_battlefield(&engine, p0, goblin_war_buggy()).expect("seated");
    assert_eq!(pt(&engine, perm), (2, 2));
    let attackers = crate::engine::card_tests::legends_kit::attackers_offered(&mut engine);
    assert!(attackers.contains(&perm));
    engine
        .apply(
            p0,
            PlayerAction::DeclareAttackers {
                attackers: vec![(perm, baylee_core::ids::Defender::Player(PlayerId::new(1)))],
            },
        )
        .unwrap();
}

#[test]
fn goblin_war_buggy_echo_can_be_paid_and_is_not_due_again() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, mountain())
        .hand(0, &[goblin_war_buggy()])
        .battlefield(0, &[mountain(), mountain()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    cast_from_hand(&mut engine, p0, goblin_war_buggy());
    pass_until(&mut engine, stack_is_empty);
    reach_their_main_phase(&mut engine, p1);
    assert!(
        on_battlefield(&engine, p0, goblin_war_buggy()).is_some(),
        "not due on the opponent's upkeep"
    );
    pass_until(&mut engine, |e| {
        e.state().turn.active == p0
            && e.state().turn.step == crate::turn::Step::Upkeep
            && matches!(e.pending(), Pending::Priority { .. })
    });
    tap_all_mana(&mut engine, p0);
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::YesNo { .. })
    });
    let Pending::YesNo { player, .. } = engine.pending() else {
        unreachable!()
    };
    assert_eq!(*player, p0);
    engine.apply(p0, PlayerAction::YesNo(true)).unwrap();
    reach_main_phase(&mut engine, p0);
    assert!(on_battlefield(&engine, p0, goblin_war_buggy()).is_some());
    reach_their_main_phase(&mut engine, p1);
    reach_their_main_phase(&mut engine, p0);
    assert!(
        on_battlefield(&engine, p0, goblin_war_buggy()).is_some(),
        "echo was already paid"
    );
}

#[test]
fn goblin_war_buggy_is_sacrificed_when_echo_is_unpaid() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, mountain())
        .hand(0, &[goblin_war_buggy()])
        .battlefield(0, &[mountain(), mountain()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    cast_from_hand(&mut engine, p0, goblin_war_buggy());
    pass_until(&mut engine, stack_is_empty);
    reach_their_main_phase(&mut engine, p1);
    reach_their_main_phase(&mut engine, p0);
    assert!(on_battlefield(&engine, p0, goblin_war_buggy()).is_none());
    assert!(in_graveyard(&engine, p0, goblin_war_buggy()).is_some());
}
