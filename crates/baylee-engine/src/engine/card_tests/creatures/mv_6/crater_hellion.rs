#[allow(clippy::wildcard_imports)]
use super::*;

fn crater_hellion() -> CardIndex {
    card_index("2d37c437-0d5c-400d-88ce-10d173b28eda")
}

#[test]
fn crater_hellion_deals_four_damage_to_each_other_creature_on_etb() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, mountain())
        .hand(0, &[crater_hellion()])
        .battlefield(
            0,
            &[
                mountain(),
                mountain(),
                mountain(),
                mountain(),
                mountain(),
                mountain(),
                grizzly_bears(),
            ],
        )
        .battlefield(1, &[wild_elephant()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    cast_from_hand(&mut engine, p0, crater_hellion());
    pass_until(&mut engine, stack_is_empty);

    let hellion = on_battlefield(&engine, p0, crater_hellion()).expect("entered");
    assert_eq!(engine.state().object(hellion).unwrap().damage, 0);
    assert!(on_battlefield(&engine, p0, grizzly_bears()).is_none());
    assert!(on_battlefield(&engine, p1, wild_elephant()).is_none());
}

#[test]
fn crater_hellion_echo_can_be_paid_and_is_not_due_again() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, mountain())
        .hand(0, &[crater_hellion()])
        .battlefield(
            0,
            &[
                mountain(),
                mountain(),
                mountain(),
                mountain(),
                mountain(),
                mountain(),
            ],
        )
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    cast_from_hand(&mut engine, p0, crater_hellion());
    pass_until(&mut engine, stack_is_empty);
    reach_their_main_phase(&mut engine, p1);
    assert!(
        on_battlefield(&engine, p0, crater_hellion()).is_some(),
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
    assert!(on_battlefield(&engine, p0, crater_hellion()).is_some());
    reach_their_main_phase(&mut engine, p1);
    reach_their_main_phase(&mut engine, p0);
    assert!(
        on_battlefield(&engine, p0, crater_hellion()).is_some(),
        "echo was already paid"
    );
}

#[test]
fn crater_hellion_is_sacrificed_when_echo_is_unpaid() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, mountain())
        .hand(0, &[crater_hellion()])
        .battlefield(
            0,
            &[
                mountain(),
                mountain(),
                mountain(),
                mountain(),
                mountain(),
                mountain(),
            ],
        )
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    cast_from_hand(&mut engine, p0, crater_hellion());
    pass_until(&mut engine, stack_is_empty);
    reach_their_main_phase(&mut engine, p1);
    reach_their_main_phase(&mut engine, p0);
    assert!(on_battlefield(&engine, p0, crater_hellion()).is_none());
    assert!(in_graveyard(&engine, p0, crater_hellion()).is_some());
}
