//! `cards/instants/mv_5/throttle.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

// oracle_id = "01823ab7-b9f6-48bc-b45c-9b0fabc6f657"
fn throttle() -> CardIndex {
    card_index("01823ab7-b9f6-48bc-b45c-9b0fabc6f657")
}

/// Throttle: "Target creature gets -4/-4 until end of turn."
#[test]
fn throttle_shrinks_until_end_of_turn_and_kills_a_small_creature() {
    let mut engine = table(
        933,
        &[throttle()],
        &[swamp(); 5],
        &[],
        &[earth_elemental()],
    );
    let big = on_battlefield(&engine, them(), earth_elemental()).expect("their Elemental");
    cast_at(&mut engine, throttle(), big);
    assert_eq!(pt(&engine, big), (0, 1), "-4/-4 on a 4/5");
    pass_until(&mut engine, |e| e.state().turn.active == them());
    assert_eq!(pt(&engine, big), (4, 5), "until end of turn");
}

#[test]
fn throttle_kills_a_two_two() {
    let mut engine = table(
        934,
        &[throttle()],
        &[swamp(); 5],
        &[],
        &[gray_ogre()],
    );
    let ogre = on_battlefield(&engine, them(), gray_ogre()).expect("their Ogre");
    cast_at(&mut engine, throttle(), ogre);
    assert!(in_graveyard(&engine, them(), gray_ogre()).is_some());
}
