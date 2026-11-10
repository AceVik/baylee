//! `cards/creatures/artifacts/mv_3/etched_familiar.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

fn etched_familiar() -> CardIndex {
    card_index("e2bc064e-4739-4eb1-ba44-1fd92b2829d7")
}

/// Etched Familiar — "When this creature dies, each opponent loses 2 life and
/// you gain 2 life."
#[test]
fn etched_familiar_drains_two_when_it_dies() {
    let p0 = PlayerId::new(0);
    let mut engine = main_with(&[], &[etched_familiar()]);
    let familiar = on_battlefield(&engine, p0, etched_familiar()).expect("seated");
    assert_eq!(pt(&engine, familiar), (3, 2));

    kill(&mut engine, familiar);

    assert!(in_graveyard(&engine, p0, etched_familiar()).is_some());
    assert_eq!(life_of(&engine, 0), 22, "its controller gained 2");
    assert_eq!(life_of(&engine, 1), 18, "the opponent lost 2");
}
