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
