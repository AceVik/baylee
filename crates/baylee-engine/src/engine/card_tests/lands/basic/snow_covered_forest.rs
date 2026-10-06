//! `cards/lands/basic/snow_covered_forest.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Snow-Covered Forest is `Coverage::Implemented` and prints nothing but its
/// own intrinsic line — `({T}: Add {G}.)` — so the whole of the card is the
/// two supertypes behind that subtype. The board is built to read them apart:
/// the land is played from hand so it arrives as a real entry rather than a
/// setup placement, the SNOW and BASIC supertypes are read off the same object
/// as the LAND type, and one tap off an otherwise empty board has to put
/// exactly one green in the pool. A test that only saw the Forest subtype
/// would have the same mana from a plain Forest and say nothing about snow.
#[test]
fn snow_covered_forest_arrives_untapped_as_a_basic_snow_land_that_taps_for_green() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(7, basic_forest())
        .hand(0, &[snow_covered_forest()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let land = play_land(&mut engine, p0, snow_covered_forest());
    assert!(!is_tapped(&engine, land), "a basic land enters untapped");

    let chars = engine
        .state()
        .object(land)
        .expect("the land is on the battlefield")
        .characteristics();
    assert!(
        chars.supertypes.contains(SupertypeSet::SNOW),
        "the printing's whole difference is the supertype: {:?}",
        chars.supertypes
    );
    assert!(
        chars.supertypes.contains(SupertypeSet::BASIC),
        "and it is basic"
    );
    assert!(
        chars.types.contains(TypeSet::LAND),
        "a land, and nothing else"
    );

    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "nothing floats before the land is tapped"
    );
    let taken = tap_all_mana(&mut engine, p0);
    assert_eq!(taken, 1, "one land on the battlefield, one route");
    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(
        pool.available(ManaColor::Green),
        1,
        "the {{G}} it prints, and no other colour"
    );
    assert_eq!(pool.total(), 1, "one mana, off one tap");
    assert!(is_tapped(&engine, land), "the land paid its own {{T}}");
}
