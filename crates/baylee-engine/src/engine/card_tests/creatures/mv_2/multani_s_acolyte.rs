#[allow(clippy::wildcard_imports)]
use super::*;
use crate::engine::card_tests::legends_kit as lk;

fn multani_s_acolyte() -> CardIndex {
    card_index("d08725e6-ed9a-4c82-89d4-0dd037c376f6")
}

#[test]
fn multani_s_acolyte_draws_a_card_on_etb() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[forest(), forest()])
        .hand(0, &[multani_s_acolyte()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let hand_before = engine.state().zones.list(ZoneLocation::Hand(p0)).len();
    cast_from_hand(&mut engine, p0, multani_s_acolyte());
    pass_until(&mut engine, stack_is_empty);

    assert!(on_battlefield(&engine, p0, multani_s_acolyte()).is_some());
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(p0)).len(),
        hand_before
    );
}

#[test]
fn multani_s_acolyte_echo_can_be_paid_and_is_not_due_twice() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = lk::game(SEED, &[], &[], &[multani_s_acolyte()], &[]);
    lk::float(&mut engine, p0, &[(ManaColor::Green, 2)]);
    lk::cast_at(&mut engine, p0, multani_s_acolyte(), &[], &[]);
    reach_their_main_phase(&mut engine, p1);
    assert!(
        on_battlefield(&engine, p0, multani_s_acolyte()).is_some(),
        "no echo on opponent upkeep"
    );
    pass_until(&mut engine, |e| {
        e.state().turn.active == p0
            && e.state().turn.step == crate::turn::Step::Upkeep
            && matches!(e.pending(), Pending::Priority { .. })
    });
    lk::float(&mut engine, p0, &[(ManaColor::Green, 2)]);
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::YesNo { .. })
    });
    lk::yes_no(&mut engine, true);
    reach_main_phase(&mut engine, p0);
    assert!(on_battlefield(&engine, p0, multani_s_acolyte()).is_some());
    assert_eq!(lk::pool_total(&engine, p0), 0);
    reach_their_main_phase(&mut engine, p1);
    reach_their_main_phase(&mut engine, p0);
    assert!(
        on_battlefield(&engine, p0, multani_s_acolyte()).is_some(),
        "echo was paid already"
    );
}
