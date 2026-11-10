//! `cards/creatures/mv_2/mawloc.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// "Ravenous (This creature enters with X +1/+1 counters on it. If X is 5
/// or more, draw a card when it enters.) Terror from the Deep — When this
/// creature enters, it fights up to one target creature an opponent
/// controls. If that creature would die this turn, exile it instead."
///
/// X = 2: two counters, a 4/4, no card. It fights the Guard, which dies and
/// is exiled instead of going to the graveyard.
#[test]
fn mawloc_fights_and_what_would_die_is_exiled() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let hand_before = 0;
    let mut engine = mawloc_for(2, 4);
    let guard = on_battlefield(&engine, p1, steadfast_guard()).unwrap();
    let giant = on_battlefield(&engine, p1, thundering_giant()).unwrap();
    let Pending::ChooseTargets { options, max, .. } = engine.pending().clone() else {
        panic!("the fight's target, got {:?}", engine.pending())
    };
    assert_eq!(max, 1, "up to one");
    assert!(options.contains(&guard) && options.contains(&giant));
    assert!(
        on_battlefield(&engine, p0, mawloc()).is_none_or(|m| !options.contains(&m)),
        "an opponent's creature"
    );
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![guard],
                players: vec![],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);
    let mawloc = on_battlefield(&engine, p0, mawloc()).expect("it survives the Guard");
    assert_eq!(counters_on(&engine, mawloc, CounterKind::P1P1), 2, "X = 2");
    assert_eq!(pt(&engine, mawloc), (4, 4));
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(p0)).len(),
        hand_before,
        "X is less than 5"
    );
    assert!(
        in_graveyard(&engine, p1, steadfast_guard()).is_none(),
        "exiled instead"
    );
    assert!(
        engine
            .state()
            .zones
            .list(ZoneLocation::Exile(p1))
            .contains(&guard)
    );
}

/// X = 1: a 3/3 against Thundering Giant. Both die in the fight; the Giant
/// is exiled and Mawloc, which the sentence is not about, goes to its
/// owner's graveyard.
#[test]
fn mawloc_is_not_the_creature_it_exiles() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = mawloc_for(1, 3);
    let giant = on_battlefield(&engine, p1, thundering_giant()).unwrap();
    let (power, toughness) = pt(&engine, giant);
    assert!(power >= 3 && toughness <= 3, "{power}/{toughness}");
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![giant],
                players: vec![],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);
    assert!(in_graveyard(&engine, p0, mawloc()).is_some(), "Mawloc died");
    assert!(in_graveyard(&engine, p1, thundering_giant()).is_none());
    assert!(
        engine
            .state()
            .zones
            .list(ZoneLocation::Exile(p1))
            .contains(&giant)
    );
}

/// X = 5: five counters and a card, and "up to one" lets it fight nothing.
#[test]
fn mawloc_for_five_draws_a_card() {
    let p0 = PlayerId::new(0);
    let mut engine = mawloc_for(5, 7);
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseTargets { .. })
    });
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![],
                players: vec![],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);
    let mawloc = on_battlefield(&engine, p0, mawloc()).unwrap();
    assert_eq!(pt(&engine, mawloc), (7, 7));
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(p0)).len(),
        1,
        "X is 5 or more"
    );
}
