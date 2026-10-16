//! `cards/creatures/artifacts/mv_3/filigree_familiar.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

fn filigree_familiar() -> CardIndex {
    card_index("b544f690-e4bf-4a5b-984d-9256518fd574")
}

/// Filigree Familiar — "When this creature enters, you gain 2 life" and
/// "When this creature dies, draw a card."
#[test]
fn filigree_familiar_gains_two_on_entering_and_draws_when_it_dies() {
    let p0 = PlayerId::new(0);
    let mut engine = main_with(&[filigree_familiar()], &[forest(), forest(), forest()]);

    cast_and_settle(&mut engine, filigree_familiar());
    assert_eq!(life_of(&engine, 0), 22, "the enters trigger");
    assert_eq!(life_of(&engine, 1), 20);
    let familiar = on_battlefield(&engine, p0, filigree_familiar()).expect("it resolved");
    assert_eq!(pt(&engine, familiar), (2, 2));
    let library_before = library_of(&engine, p0).len();
    let hand_before = engine.state().zones.list(ZoneLocation::Hand(p0)).len();

    kill(&mut engine, familiar);

    assert!(in_graveyard(&engine, p0, filigree_familiar()).is_some());
    assert_eq!(library_of(&engine, p0).len(), library_before - 1);
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(p0)).len(),
        hand_before + 1,
        "the dies trigger drew a card"
    );
    assert_eq!(life_of(&engine, 0), 22, "and gained no more life");
}
