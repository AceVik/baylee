#[allow(clippy::wildcard_imports)]
use super::*;

fn herald_of_serra() -> CardIndex {
    card_index("bc42f32d-e4e1-4174-904c-6058e487f963")
}

#[test]
fn herald_of_serra_enters_with_flying_and_vigilance() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, plains())
        .hand(0, &[herald_of_serra()])
        .battlefield(0, &[plains(), plains(), plains(), plains()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    cast_from_hand(&mut engine, p0, herald_of_serra());
    pass_until(&mut engine, stack_is_empty);

    let perm = on_battlefield(&engine, p0, herald_of_serra()).expect("seated");
    assert_eq!(pt(&engine, perm), (3, 4));
}

#[test]
fn herald_of_serra_echo_can_be_paid_and_is_not_due_again() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, plains())
        .hand(0, &[herald_of_serra()])
        .battlefield(0, &[plains(), plains(), plains(), plains()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    cast_from_hand(&mut engine, p0, herald_of_serra());
    pass_until(&mut engine, stack_is_empty);
    reach_their_main_phase(&mut engine, p1);
    assert!(
        on_battlefield(&engine, p0, herald_of_serra()).is_some(),
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
    assert!(on_battlefield(&engine, p0, herald_of_serra()).is_some());
    reach_their_main_phase(&mut engine, p1);
    reach_their_main_phase(&mut engine, p0);
    assert!(
        on_battlefield(&engine, p0, herald_of_serra()).is_some(),
        "echo was already paid"
    );
}

#[test]
fn herald_of_serra_is_sacrificed_when_echo_is_unpaid() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, plains())
        .hand(0, &[herald_of_serra()])
        .battlefield(0, &[plains(), plains(), plains(), plains()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    cast_from_hand(&mut engine, p0, herald_of_serra());
    pass_until(&mut engine, stack_is_empty);
    reach_their_main_phase(&mut engine, p1);
    reach_their_main_phase(&mut engine, p0);
    assert!(on_battlefield(&engine, p0, herald_of_serra()).is_none());
    assert!(in_graveyard(&engine, p0, herald_of_serra()).is_some());
}

#[test]
fn herald_of_serra_attacks_without_tapping_and_requires_a_flying_blocker() {
    use crate::engine::card_tests::legends_kit as lk;
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, plains())
        .battlefield(
            0,
            &[herald_of_serra(), plains(), plains(), plains(), plains()],
        )
        .battlefield(1, &[grizzly_bears(), ornithopter()])
        .start();
    keep_mulligans(&mut engine);
    pass_until(&mut engine, |e| {
        e.state().turn.active == p0
            && e.state().turn.step == crate::turn::Step::Upkeep
            && matches!(e.pending(), Pending::Priority { .. })
    });
    tap_all_mana(&mut engine, p0);
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::YesNo { .. })
    });
    engine.apply(p0, PlayerAction::YesNo(true)).unwrap();
    reach_main_phase(&mut engine, p0);
    let angel = on_battlefield(&engine, p0, herald_of_serra()).unwrap();
    let bear = on_battlefield(&engine, p1, grizzly_bears()).unwrap();
    let flyer = on_battlefield(&engine, p1, ornithopter()).unwrap();
    let blockers = lk::attack(&mut engine, &[angel]);
    assert!(!lk::tapped(&engine, angel));
    assert!(!lk::may_block(&blockers, bear, angel));
    assert!(lk::may_block(&blockers, flyer, angel));
    lk::block(&mut engine, &[(flyer, angel)]);
}
