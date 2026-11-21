//! `cards/creatures/mv_2/arctic_merfolk.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_card_testkit::card_tests::legends_kit as lk;

fn arctic_merfolk() -> CardIndex {
    card_index("c56671e1-28a6-40c3-bad5-b5e617ddb7da")
}

/// Arctic Merfolk ({1}{U} 1/1 Merfolk):
/// "Kicker—Return a creature you control to its owner's hand.
/// If this creature was kicked, it enters with a +1/+1 counter on it."
#[test]
fn arctic_merfolk_enters_1_1_unkicked_and_2_2_when_kicked_by_returning_your_creature() {
    let mut e = lk::game(
        SEED,
        &[island(), island(), grizzly_bears()],
        &[pearled_unicorn()],
        &[arctic_merfolk()],
        &[],
    );
    let my_bear = lk::obj(&e, lk::P0, grizzly_bears());
    let opp_unicorn = lk::obj(&e, lk::P1, pearled_unicorn());

    cast_from_hand(&mut e, lk::P0, arctic_merfolk());
    lk::yes_no(&mut e, true);

    let Pending::ChooseCards { options, .. } = e.pending().clone() else {
        panic!("expected kicker bounce choice, got {:?}", e.pending());
    };
    assert_eq!(
        options,
        vec![my_bear],
        "only a creature you control may pay the kicker cost"
    );
    assert!(!options.contains(&opp_unicorn));
    e.apply(
        lk::P0,
        PlayerAction::ChooseObjects {
            objects: vec![my_bear],
        },
    )
    .unwrap();
    assert!(in_hand(&e, lk::P0, grizzly_bears()).is_some());
    lk::settle(&mut e);

    let merfolk = lk::obj(&e, lk::P0, arctic_merfolk());
    assert_eq!(
        e.state()
            .object(merfolk)
            .unwrap()
            .counters
            .get(CounterKind::P1P1),
        1
    );
    assert_eq!(pt(&e, merfolk), (2, 2));

    let mut e2 = lk::game(
        SEED,
        &[island(), island(), grizzly_bears()],
        &[],
        &[arctic_merfolk()],
        &[],
    );
    cast_from_hand(&mut e2, lk::P0, arctic_merfolk());
    lk::yes_no(&mut e2, false);
    lk::settle(&mut e2);
    let plain = lk::obj(&e2, lk::P0, arctic_merfolk());
    assert_eq!(
        e2.state()
            .object(plain)
            .unwrap()
            .counters
            .get(CounterKind::P1P1),
        0
    );
    assert_eq!(pt(&e2, plain), (1, 1));
    assert!(on_battlefield(&e2, lk::P0, grizzly_bears()).is_some());
}
