#[allow(clippy::wildcard_imports)]
use super::*;

fn citanul_centaurs() -> CardIndex {
    card_index("831487cf-a377-4c9b-920d-c5bc68f17f24")
}

#[test]
fn citanul_centaurs_has_shroud_and_echo() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .hand(0, &[citanul_centaurs(), lightning_bolt()])
        .battlefield(0, &[forest(), forest(), forest(), forest()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    cast_from_hand(&mut engine, p0, citanul_centaurs());
    pass_until(&mut engine, stack_is_empty);

    let obj = on_battlefield(&engine, p0, citanul_centaurs()).expect("seated");
    assert_eq!(pt(&engine, obj), (6, 3));
    assert!(keywords(&engine, obj).contains(KeywordSet::SHROUD));
    crate::engine::card_tests::legends_kit::float(&mut engine, p0, &[(ManaColor::Red, 1)]);
    cast_with_floating(&mut engine, p0, lightning_bolt());
    let Pending::ChooseTargets { options, .. } = engine.pending() else {
        panic!("bolt target");
    };
    assert!(!options.contains(&obj));
    assert!(
        engine
            .apply(
                p0,
                PlayerAction::ChooseTargets {
                    objects: vec![obj],
                    players: vec![]
                }
            )
            .is_err()
    );
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![],
                players: vec![PlayerId::new(1)],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);
}

#[test]
fn citanul_centaurs_echo_can_be_paid_and_is_not_due_again() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, forest())
        .hand(0, &[citanul_centaurs(), lightning_bolt()])
        .battlefield(0, &[forest(), forest(), forest(), forest()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    cast_from_hand(&mut engine, p0, citanul_centaurs());
    pass_until(&mut engine, stack_is_empty);
    reach_their_main_phase(&mut engine, p1);
    assert!(
        on_battlefield(&engine, p0, citanul_centaurs()).is_some(),
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
    assert!(on_battlefield(&engine, p0, citanul_centaurs()).is_some());
    reach_their_main_phase(&mut engine, p1);
    reach_their_main_phase(&mut engine, p0);
    assert!(
        on_battlefield(&engine, p0, citanul_centaurs()).is_some(),
        "echo was already paid"
    );
}

#[test]
fn citanul_centaurs_is_sacrificed_when_echo_is_unpaid() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, forest())
        .hand(0, &[citanul_centaurs(), lightning_bolt()])
        .battlefield(0, &[forest(), forest(), forest(), forest()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    cast_from_hand(&mut engine, p0, citanul_centaurs());
    pass_until(&mut engine, stack_is_empty);
    reach_their_main_phase(&mut engine, p1);
    reach_their_main_phase(&mut engine, p0);
    assert!(on_battlefield(&engine, p0, citanul_centaurs()).is_none());
    assert!(in_graveyard(&engine, p0, citanul_centaurs()).is_some());
}
