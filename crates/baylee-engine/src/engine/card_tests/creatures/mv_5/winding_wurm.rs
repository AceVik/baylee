#[allow(clippy::wildcard_imports)]
use super::*;
use crate::engine::card_tests::legends_kit as lk;

fn winding_wurm() -> CardIndex {
    card_index("8f788188-d901-44f3-9fec-e2e5d0263140")
}

#[test]
fn winding_wurm_sacrifices_on_upkeep_if_echo_unpaid() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, forest())
        .hand(0, &[winding_wurm()])
        .battlefield(0, &[forest(), forest(), forest(), forest(), forest()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    cast_from_hand(&mut engine, p0, winding_wurm());
    pass_until(&mut engine, stack_is_empty);
    assert!(on_battlefield(&engine, p0, winding_wurm()).is_some());

    reach_their_main_phase(&mut engine, p1);
    reach_their_main_phase(&mut engine, p0);

    assert!(on_battlefield(&engine, p0, winding_wurm()).is_none());
}

#[test]
fn winding_wurm_echo_can_be_paid_and_is_not_due_twice() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = lk::game(SEED, &[], &[], &[winding_wurm()], &[]);
    lk::float(&mut engine, p0, &[(ManaColor::Green, 5)]);
    lk::cast_at(&mut engine, p0, winding_wurm(), &[], &[]);
    reach_their_main_phase(&mut engine, p1);
    assert!(
        on_battlefield(&engine, p0, winding_wurm()).is_some(),
        "no echo on opponent upkeep"
    );
    pass_until(&mut engine, |e| {
        e.state().turn.active == p0
            && e.state().turn.step == crate::turn::Step::Upkeep
            && matches!(e.pending(), Pending::Priority { .. })
    });
    lk::float(&mut engine, p0, &[(ManaColor::Green, 5)]);
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::YesNo { .. })
    });
    lk::yes_no(&mut engine, true);
    reach_main_phase(&mut engine, p0);
    assert!(on_battlefield(&engine, p0, winding_wurm()).is_some());
    assert_eq!(lk::pool_total(&engine, p0), 0);
    reach_their_main_phase(&mut engine, p1);
    reach_their_main_phase(&mut engine, p0);
    assert!(
        on_battlefield(&engine, p0, winding_wurm()).is_some(),
        "echo was paid already"
    );
}
