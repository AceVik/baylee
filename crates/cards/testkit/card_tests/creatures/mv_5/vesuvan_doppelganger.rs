//! `cards/creatures/mv_5/vesuvan_doppelganger.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Vesuvan Doppelganger, its written half: it enters as a copy of a
/// creature on the battlefield, except that it keeps its own colour.
#[test]
fn vesuvan_doppelganger_enters_as_a_blue_copy() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let doppelganger = card_index("aeaccab9-3e2c-4a40-a483-52c4972b2014");
    let mut engine = Duel::new(SEED, island())
        .battlefield(0, &[island(); 5])
        .battlefield(1, &[llanowar_elves()])
        .hand(0, &[doppelganger])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let elf = on_battlefield(&engine, p1, llanowar_elves()).expect("their Elves");
    cast_from_hand(&mut engine, p0, doppelganger);
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseTargets { .. })
    });
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![elf],
                players: vec![],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);
    let copy = on_battlefield(&engine, p0, doppelganger).expect("it entered");
    let c = engine
        .state()
        .object(copy)
        .expect("seated")
        .characteristics();
    assert_eq!(
        (c.power, c.toughness),
        (Some(1), Some(1)),
        "the Elves' body"
    );
    assert_eq!(
        c.colors,
        baylee_core::color::ColorSet::from_slice(&[baylee_core::color::Color::Blue]),
        "blue, not the Elves' green"
    );
}

/// The "may": answered with no creature, the Doppelganger stays what it prints,
/// a blue 0/0 that is put into the graveyard as a state-based action; it did not
/// copy the Elves and so carries none of the copy's upkeep ability either.
#[test]
fn vesuvan_doppelganger_declined_enters_as_a_zero_zero_and_dies() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let doppelganger = card_index("aeaccab9-3e2c-4a40-a483-52c4972b2014");
    let mut engine = Duel::new(SEED, island())
        .battlefield(0, &[island(); 5])
        .battlefield(1, &[llanowar_elves()])
        .hand(0, &[doppelganger])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    cast_from_hand(&mut engine, p0, doppelganger);
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseTargets { .. })
    });
    let Pending::ChooseTargets { min, .. } = engine.pending().clone() else {
        unreachable!("pass_until stopped on the copy question")
    };
    assert_eq!(min, 0, "\"you may have this creature enter as a copy\"");
    engine
        .apply(p0, PlayerAction::ChooseObjects { objects: vec![] })
        .expect("declining the copy is a legal answer");
    pass_until(&mut engine, stack_is_empty);

    assert!(
        on_battlefield(&engine, p0, doppelganger).is_none(),
        "a 0/0 does not survive"
    );
    assert!(in_graveyard(&engine, p0, doppelganger).is_some());
    assert!(on_battlefield(&engine, p1, llanowar_elves()).is_some());
}
