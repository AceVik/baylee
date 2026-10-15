//! `cards/creatures/artifacts/mv_2/a_i_m_synthoids.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

fn a_i_m_synthoids() -> CardIndex {
    card_index("10916f33-aaa6-43a1-9835-b60fd061161f")
}

/// A.I.M. Synthoids: "When this creature enters, surveil 2." A 1/3 artifact
/// creature; the trigger asks a surveil of two, and what goes to the
/// graveyard leaves the library while the rest stays in it.
#[test]
fn a_i_m_synthoids_surveils_two_when_it_enters() {
    let p0 = PlayerId::new(0);
    let mut engine = table(&[a_i_m_synthoids()], &[forest(), forest()], &[]);
    let synthoids = cast_and_land(&mut engine, a_i_m_synthoids());
    assert_eq!(pt(&engine, synthoids), (1, 3), "the printed body");
    let t = types(&engine, synthoids);
    assert!(t.contains(TypeSet::ARTIFACT) && t.contains(TypeSet::CREATURE));

    let library = library_size(&engine, p0);
    let graveyard = graveyard_count(&engine, 0);
    let cards = surveil_asked(&mut engine, 2);
    engine
        .apply(p0, look_answer(&cards, &[cards[0]]))
        .expect("one of the two goes to the graveyard");
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(library_size(&engine, p0), library - 1);
    assert_eq!(graveyard_count(&engine, 0), graveyard + 1);
    assert!(
        engine
            .state()
            .zones
            .list(crate::zone::ZoneLocation::Graveyard(p0))
            .contains(&cards[0]),
        "the card sent away is in the graveyard"
    );
    assert!(
        engine
            .state()
            .zones
            .list(crate::zone::ZoneLocation::Library(p0))
            .contains(&cards[1]),
        "the other stays in the library"
    );
}

/// Surveil may keep both: nothing moves when nothing is sent away.
#[test]
fn a_i_m_synthoids_may_keep_both_cards() {
    let p0 = PlayerId::new(0);
    let mut engine = table(&[a_i_m_synthoids()], &[forest(), forest()], &[]);
    cast_and_land(&mut engine, a_i_m_synthoids());
    let library = library_size(&engine, p0);
    let graveyard = graveyard_count(&engine, 0);
    let cards = surveil_asked(&mut engine, 2);
    engine.apply(p0, look_answer(&cards, &[])).unwrap();
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(library_size(&engine, p0), library);
    assert_eq!(graveyard_count(&engine, 0), graveyard);
}
