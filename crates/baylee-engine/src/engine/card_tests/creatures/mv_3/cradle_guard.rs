#[allow(clippy::wildcard_imports)]
use super::*;

fn cradle_guard() -> CardIndex {
    card_index("846c0867-6f0e-4e44-8f45-8f9720b62065")
}

#[test]
fn cradle_guard_casts_with_trample() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .hand(0, &[cradle_guard()])
        .battlefield(0, &[forest(), forest(), forest()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    cast_from_hand(&mut engine, p0, cradle_guard());
    pass_until(&mut engine, stack_is_empty);

    let guard = on_battlefield(&engine, p0, cradle_guard()).expect("seated");
    assert!(keywords(&engine, guard).contains(KeywordSet::TRAMPLE));
}

#[test]
fn cradle_guard_echo_can_be_paid_and_is_not_due_again() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, forest())
        .hand(0, &[cradle_guard()])
        .battlefield(0, &[forest(), forest(), forest()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    cast_from_hand(&mut engine, p0, cradle_guard());
    pass_until(&mut engine, stack_is_empty);
    reach_their_main_phase(&mut engine, p1);
    assert!(
        on_battlefield(&engine, p0, cradle_guard()).is_some(),
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
    assert!(on_battlefield(&engine, p0, cradle_guard()).is_some());
    reach_their_main_phase(&mut engine, p1);
    reach_their_main_phase(&mut engine, p0);
    assert!(
        on_battlefield(&engine, p0, cradle_guard()).is_some(),
        "echo was already paid"
    );
}

#[test]
fn cradle_guard_is_sacrificed_when_echo_is_unpaid() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, forest())
        .hand(0, &[cradle_guard()])
        .battlefield(0, &[forest(), forest(), forest()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    cast_from_hand(&mut engine, p0, cradle_guard());
    pass_until(&mut engine, stack_is_empty);
    reach_their_main_phase(&mut engine, p1);
    reach_their_main_phase(&mut engine, p0);
    assert!(on_battlefield(&engine, p0, cradle_guard()).is_none());
    assert!(in_graveyard(&engine, p0, cradle_guard()).is_some());
}

#[test]
fn cradle_guard_tramples_over_a_small_blocker() {
    use crate::engine::card_tests::legends_kit as lk;
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[cradle_guard(), forest(), forest(), forest()])
        .battlefield(1, &[llanowar_elves()])
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
    let guard = on_battlefield(&engine, p0, cradle_guard()).unwrap();
    let elf = on_battlefield(&engine, p1, llanowar_elves()).unwrap();
    lk::attack(&mut engine, &[guard]);
    lk::block(&mut engine, &[(elf, guard)]);
    pass_until(&mut engine, |e| {
        e.state().turn.step == crate::turn::Step::End
    });
    assert_eq!(engine.state().players[1].life, 17);
    assert!(in_graveyard(&engine, p1, llanowar_elves()).is_some());
    assert!(on_battlefield(&engine, p0, cradle_guard()).is_some());
}
