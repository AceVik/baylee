#[allow(clippy::wildcard_imports)]
use super::*;
use crate::engine::card_tests::legends_kit as lk;

fn viashino_outrider() -> CardIndex {
    card_index("3e81c22e-52ec-4142-9a98-88681d04ce60")
}

#[test]
fn viashino_outrider_sacrifices_on_upkeep_if_echo_unpaid() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, mountain())
        .hand(0, &[viashino_outrider()])
        .battlefield(0, &[mountain(), mountain(), mountain()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    cast_from_hand(&mut engine, p0, viashino_outrider());
    pass_until(&mut engine, stack_is_empty);
    assert!(on_battlefield(&engine, p0, viashino_outrider()).is_some());

    reach_their_main_phase(&mut engine, p1);
    reach_their_main_phase(&mut engine, p0);

    assert!(on_battlefield(&engine, p0, viashino_outrider()).is_none());
    assert!(in_graveyard(&engine, p0, viashino_outrider()).is_some());
}

#[test]
fn viashino_outrider_echo_can_be_paid_and_is_not_due_twice() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = lk::game(SEED, &[], &[], &[viashino_outrider()], &[]);
    lk::float(&mut engine, p0, &[(ManaColor::Red, 3)]);
    lk::cast_at(&mut engine, p0, viashino_outrider(), &[], &[]);
    reach_their_main_phase(&mut engine, p1);
    assert!(
        on_battlefield(&engine, p0, viashino_outrider()).is_some(),
        "no echo on opponent upkeep"
    );
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
    assert!(on_battlefield(&engine, p0, viashino_outrider()).is_some());
    assert_eq!(lk::pool_total(&engine, p0), 0);
    reach_their_main_phase(&mut engine, p1);
    reach_their_main_phase(&mut engine, p0);
    assert!(
        on_battlefield(&engine, p0, viashino_outrider()).is_some(),
        "echo was paid already"
    );
}
