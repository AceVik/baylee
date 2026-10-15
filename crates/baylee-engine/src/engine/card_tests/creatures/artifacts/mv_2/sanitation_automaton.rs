//! `cards/creatures/artifacts/mv_2/sanitation_automaton.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

fn sanitation_automaton() -> CardIndex {
    card_index("be2ab552-df82-49a0-8f0b-ee66de34e1a7")
}

/// Sanitation Automaton: "When this creature enters, surveil 1." A 2/1; the
/// one card looked at may go to the graveyard.
#[test]
fn sanitation_automaton_surveils_one_when_it_enters() {
    let p0 = PlayerId::new(0);
    let mut engine = table(&[sanitation_automaton()], &[forest(), forest()], &[]);
    let automaton = cast_and_land(&mut engine, sanitation_automaton());
    assert_eq!(pt(&engine, automaton), (2, 1), "the printed body");

    let library = library_size(&engine, p0);
    let graveyard = graveyard_count(&engine, 0);
    let cards = surveil_asked(&mut engine, 1);
    engine
        .apply(p0, look_answer(&cards, &[cards[0]]))
        .expect("the looked-at card goes to the graveyard");
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(library_size(&engine, p0), library - 1);
    assert_eq!(graveyard_count(&engine, 0), graveyard + 1);
    assert!(
        engine
            .state()
            .zones
            .list(crate::zone::ZoneLocation::Graveyard(p0))
            .contains(&cards[0])
    );
}

/// Keeping the card on top moves nothing.
#[test]
fn sanitation_automaton_may_leave_the_card_on_top() {
    let p0 = PlayerId::new(0);
    let mut engine = table(&[sanitation_automaton()], &[forest(), forest()], &[]);
    cast_and_land(&mut engine, sanitation_automaton());
    let library = library_size(&engine, p0);
    let cards = surveil_asked(&mut engine, 1);
    engine.apply(p0, look_answer(&cards, &[])).unwrap();
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(library_size(&engine, p0), library);
    assert_eq!(graveyard_count(&engine, 0), 0);
}
