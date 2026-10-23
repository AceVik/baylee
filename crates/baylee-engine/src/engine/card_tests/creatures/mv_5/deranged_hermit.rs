#[allow(clippy::wildcard_imports)]
use super::*;
use crate::engine::card_tests::legends_kit as lk;

fn deranged_hermit() -> CardIndex {
    card_index("b9cd714b-2ad8-4fdb-a8aa-82b17730e071")
}

#[test]
fn deranged_hermit_creates_four_squirrels_and_pumps_them() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .hand(0, &[deranged_hermit()])
        .battlefield(0, &[forest(), forest(), forest(), forest(), forest()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    cast_from_hand(&mut engine, p0, deranged_hermit());
    pass_until(&mut engine, stack_is_empty);

    let squirrels: Vec<_> = engine
        .state()
        .zones
        .list(ZoneLocation::Battlefield)
        .iter()
        .copied()
        .filter(|&id| engine.state().object(id).unwrap().card.is_none())
        .collect();
    assert_eq!(squirrels.len(), 4);
    for sq in &squirrels {
        assert_eq!(pt(&engine, *sq), (2, 2));
    }
    reach_their_main_phase(&mut engine, PlayerId::new(1));
    reach_their_main_phase(&mut engine, p0);
    assert!(
        in_graveyard(&engine, p0, deranged_hermit()).is_some(),
        "unpaid echo sacrifices the Hermit"
    );
    for sq in squirrels {
        assert_eq!(
            pt(&engine, sq),
            (1, 1),
            "Squirrels lose the anthem when the Hermit leaves"
        );
    }
}

#[test]
fn deranged_hermit_echo_can_be_paid_and_is_not_due_twice() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = lk::game(SEED, &[], &[], &[deranged_hermit()], &[]);
    lk::float(&mut engine, p0, &[(ManaColor::Green, 5)]);
    lk::cast_at(&mut engine, p0, deranged_hermit(), &[], &[]);
    reach_their_main_phase(&mut engine, p1);
    assert!(
        on_battlefield(&engine, p0, deranged_hermit()).is_some(),
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
    assert!(on_battlefield(&engine, p0, deranged_hermit()).is_some());
    assert_eq!(lk::pool_total(&engine, p0), 0);
    reach_their_main_phase(&mut engine, p1);
    reach_their_main_phase(&mut engine, p0);
    assert!(
        on_battlefield(&engine, p0, deranged_hermit()).is_some(),
        "echo was paid already"
    );
}
