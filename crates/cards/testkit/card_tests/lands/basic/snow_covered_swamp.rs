//! `cards/lands/basic/snow_covered_swamp.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Snow-Covered Swamp prints one line — the intrinsic `{T}: Add {B}` it has for
/// being a Swamp (CR 305.6) — plus the one word above its type line, "Snow".
/// The scenario is the played land and nothing else: no other permanent on the
/// board, so the pool it leaves behind is entirely this card's. Playing it for
/// real rather than seeding a `starting_battlefield` is the point, since that
/// path is a placement (`Cause::Setup`) that no replacement effect ever looks
/// at, and the black mana that comes off it is the subtype's because a snow
/// basic prints no ability of its own to press. The projected supertypes are
/// the other half, read off the permanent because nothing in the pool keys on
/// "Snow" yet — the type line is the only place the word can be shown to have
/// arrived at all.
#[test]
fn snow_covered_swamp_is_played_as_a_snow_basic_swamp_and_taps_for_one_black() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(7, forest())
        .hand(0, &[snow_covered_swamp()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let swamp = play_land(&mut engine, p0, snow_covered_swamp());
    let landed = engine
        .state()
        .object(swamp)
        .expect("the land that was just played is on the battlefield");
    assert!(
        !is_tapped(&engine, swamp),
        "a basic land prints no enter modifier, so it arrives standing"
    );
    let chars = landed.characteristics();
    assert!(
        chars.types.contains(TypeSet::LAND) && !chars.types.contains(TypeSet::CREATURE),
        "a land and nothing else: {:?}",
        chars.types
    );
    assert!(
        chars.supertypes.contains(SupertypeSet::BASIC)
            && chars.supertypes.contains(SupertypeSet::SNOW),
        "the printed supertypes, Snow among them: {:?}",
        chars.supertypes
    );

    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "the land drop spends nothing"
    );
    let taken = tap_all_mana(&mut engine, p0);
    assert_eq!(taken, 1, "one permanent on the table, one route to mana");
    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(
        pool.available(ManaColor::Black),
        1,
        "the black the Swamp subtype grants (CR 305.6)"
    );
    assert_eq!(pool.total(), 1, "one mana, and none of another colour");
    assert!(is_tapped(&engine, swamp), "and the land paid its own tap");
}
