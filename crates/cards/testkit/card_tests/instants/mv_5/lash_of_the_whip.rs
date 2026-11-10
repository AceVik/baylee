//! `cards/instants/mv_5/lash_of_the_whip.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

// oracle_id = "e2ac369a-f6d4-4b58-8695-a7c5b906e999"
fn lash_of_the_whip() -> CardIndex {
    card_index("e2ac369a-f6d4-4b58-8695-a7c5b906e999")
}

/// Lash of the Whip: "Target creature gets -4/-4 until end of turn."
///
/// A 4/5 survives as a 0/1 and is whole again when the turn is over, and a 2/2
/// beside it dies of toughness zero.
#[test]
fn lash_of_the_whip_shrinks_until_end_of_turn() {
    let mut engine = table(
        923,
        &[lash_of_the_whip()],
        &[swamp(); 5],
        &[],
        &[earth_elemental()],
    );
    let big = on_battlefield(&engine, them(), earth_elemental()).expect("their Elemental");
    assert_eq!(pt(&engine, big), (4, 5));
    cast_at(&mut engine, lash_of_the_whip(), big);
    assert_eq!(pt(&engine, big), (0, 1), "-4/-4");
    assert!(
        on_battlefield(&engine, them(), earth_elemental()).is_some(),
        "it lives"
    );

    pass_until(&mut engine, |e| e.state().turn.active == them());
    assert_eq!(pt(&engine, big), (4, 5), "the turn is over");
}

/// A creature the shrink takes to toughness zero dies without being destroyed.
#[test]
fn lash_of_the_whip_kills_a_small_creature() {
    let mut engine = table(
        924,
        &[lash_of_the_whip()],
        &[swamp(); 5],
        &[],
        &[gray_ogre()],
    );
    let ogre = on_battlefield(&engine, them(), gray_ogre()).expect("their Ogre");
    cast_at(&mut engine, lash_of_the_whip(), ogre);
    assert!(in_graveyard(&engine, them(), gray_ogre()).is_some());
}
