//! `cards/creatures/mv_5/water_elemental.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Clone — "You may have this creature enter as a copy of any creature on
/// the battlefield." Answered yes, it becomes the Water Elemental's body;
/// answered no, it is a 0/0 Shapeshifter and dies to state-based actions on
/// the spot (CR 704.5f applies to whatever a 0/0 creature turns out to be).
#[test]
fn clone_may_enter_as_a_copy_of_a_creature_on_the_battlefield() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, island())
        .battlefield(0, &[island(), island(), island(), island()])
        .battlefield(1, &[water_elemental()])
        .hand(0, &[clone()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let elemental = on_battlefield(&engine, p1, water_elemental()).expect("their Elemental");
    cast_from_hand(&mut engine, p0, clone());
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseTargets { .. })
    });
    let Pending::ChooseTargets {
        options, min, max, ..
    } = engine.pending().clone()
    else {
        panic!("expected the copy question, got {:?}", engine.pending())
    };
    assert_eq!((min, max), (0, 1), "\"you may\": zero is a legal answer");
    assert!(
        options.contains(&elemental),
        "any creature on the battlefield"
    );
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![elemental],
                players: vec![],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);
    let copy = on_battlefield(&engine, p0, clone()).expect("it entered");
    assert_eq!(pt(&engine, copy), (5, 4), "the Elemental's printed body");
    assert!(
        engine
            .state()
            .object(copy)
            .expect("seated")
            .characteristics()
            .subtypes
            .contains(baylee_core::generated::subtypes::creature::ELEMENTAL),
        "copied the Elemental's subtype, not its own Shapeshifter"
    );
}

/// Clone declining the copy: a 0/0 Shapeshifter with lethal toughness of
/// zero, so it never stands on the battlefield at all.
#[test]
fn clone_declining_the_copy_is_a_zero_zero_that_dies_at_once() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, island())
        .battlefield(0, &[island(), island(), island(), island()])
        .battlefield(1, &[water_elemental()])
        .hand(0, &[clone()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    cast_from_hand(&mut engine, p0, clone());
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseTargets { .. })
    });
    let Pending::ChooseTargets { min, max, .. } = engine.pending().clone() else {
        panic!("expected the copy question, got {:?}", engine.pending())
    };
    assert_eq!((min, max), (0, 1));
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
    assert!(
        on_battlefield(&engine, p0, clone()).is_none(),
        "a 0/0 does not survive state-based actions"
    );
    assert!(
        in_graveyard(&engine, p0, clone()).is_some(),
        "it dies to its own zero toughness the instant it arrives"
    );
}

/// Water Elemental — vanilla `{3}{U}{U}` 5/4 Elemental.
#[test]
fn water_elemental_is_a_five_four_elemental_for_3uu() {
    let p0 = PlayerId::new(0);
    assert_eq!(
        cast_saying_nothing(water_elemental(), island(), 5),
        Zone::Battlefield
    );
    let mut engine = Duel::new(SEED, island())
        .battlefield(0, &[water_elemental()])
        .start();
    keep_mulligans(&mut engine);
    let id = on_battlefield(&engine, p0, water_elemental()).expect("seated");
    assert_eq!(pt(&engine, id), (5, 4));
}
