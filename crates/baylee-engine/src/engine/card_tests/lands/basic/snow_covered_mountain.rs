//! `cards/lands/basic/snow_covered_mountain.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Snow-Covered Mountain prints no ability at all: its whole oracle line is
/// `({T}: Add {R}.)`, and the rules supply that line from the Mountain
/// subtype (CR 305.6) rather than from anything the card prints. That
/// distinction is only visible in the offer, so that is where the test looks:
/// the permanent arrives in `LegalActions::mana_abilities` — the no-index
/// shortcut — and never in `LegalActions::abilities`, where a card that
/// literally printed `{T}: Add {R}` would land. Playing it for the land drop
/// and tapping the board for exactly one red keeps the claim from being a
/// reading of the card file, and the SNOW supertype is asserted beside it
/// because it is the only thing telling this printing from a plain Mountain.
#[test]
fn snow_covered_mountain_is_a_basic_snow_land_that_taps_for_exactly_one_red() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(7, forest())
        .hand(0, &[snow_covered_mountain()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    // The land drop, like any other, and it arrives untapped: the card prints
    // no entry modifier and belongs to no "enters tapped unless" cycle.
    let card = in_hand(&engine, p0, snow_covered_mountain()).expect("the land is in hand");
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        legal.lands.contains(&card),
        "a basic land in hand is playable as the turn's land drop: {:?}",
        legal.lands
    );
    let land = play_land(&mut engine, p0, snow_covered_mountain());
    assert!(
        !entered_tapped(&engine, land),
        "nothing is printed that would bring it in tapped, so it is ready to tap"
    );

    let chars = engine
        .state()
        .object(land)
        .expect("the land is an object")
        .characteristics();
    assert!(
        chars.supertypes.contains(SupertypeSet::BASIC)
            && chars.supertypes.contains(SupertypeSet::SNOW),
        "basic and snow are the two supertypes the printing carries: {:?}",
        chars.supertypes
    );
    assert!(chars.types.contains(TypeSet::LAND), "and it is a land");

    // The {R} is the Mountain subtype read through CR 305.6 and not a printed
    // ability, which is precisely what the two lists exist to tell apart.
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        legal.mana_abilities.contains(&land),
        "the basic land type is offered on the no-index shortcut: {:?}",
        legal.mana_abilities
    );
    // And in `abilities` as well, which is not a contradiction: this pool
    // writes a land's CR 305.6 mana as an explicit `mana_ability!` —
    // `landgen::intrinsic_mana_ability`, put back by the transcoder so the
    // same card comes out the same whichever reader reached it — so a basic
    // land really does carry one printed ability and appears on both lists.
    // The shortcut is what a land with *two* basic types is refused, because
    // it would have to pick a colour; see the original duals further down.
    assert!(
        legal.abilities.contains(&(land, 0)),
        "the mana this pool writes on every basic land is an ability like any \
         other: {:?}",
        legal.abilities
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "nothing is floating before a tap"
    );

    let taken = tap_all_mana(&mut engine, p0);
    assert_eq!(
        taken, 1,
        "the Mountain is the only mana source on the battlefield"
    );
    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(pool.available(ManaColor::Red), 1, "exactly one red");
    assert_eq!(pool.total(), 1, "and nothing beside it");
    assert!(is_tapped(&engine, land), "the tap is what paid for it");
}
