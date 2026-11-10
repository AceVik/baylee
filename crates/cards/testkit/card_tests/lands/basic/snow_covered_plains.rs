//! `cards/lands/basic/snow_covered_plains.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Snow-Covered Plains is a *basic snow* land whose entire printed line is
/// `{T}: Add {W}` — and the subtype, not the line, is what the engine reads:
/// a basic land type is the CR 305.6 shortcut, so the offer has to name the
/// land in the index-free `LegalActions::mana_abilities` rather than in an
/// indexed ability. The white mana in the pool afterwards is the evidence
/// that the type it read was `Plains`, since nothing else on this board makes
/// mana. The supertypes are asserted beside it because `Snow` is the one word
/// this card prints that an ordinary Plains does not, and nothing else in the
/// scenario would go red if it were dropped.
#[test]
fn snow_covered_plains_is_a_basic_snow_plains_that_taps_for_white() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .hand(0, &[snow_covered_plains()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "nothing floats before the land is even on the table"
    );

    let card = in_hand(&engine, p0, snow_covered_plains()).expect("the Plains is in hand");
    engine
        .apply(p0, PlayerAction::PlayLand { card })
        .expect("a land in hand on an empty board is a legal land drop");
    let land =
        on_battlefield(&engine, p0, snow_covered_plains()).expect("it reached the battlefield");
    assert!(
        !entered_tapped(&engine, land),
        "it prints no enter modifiers, so the basic land arrives untapped"
    );
    assert_eq!(lands_of(&engine, p0), vec![land], "and it is the only land");

    let chars = engine
        .state()
        .object(land)
        .expect("the land is an object")
        .characteristics();
    assert!(
        chars.types.contains(TypeSet::LAND),
        "a land: {:?}",
        chars.types
    );
    assert!(
        chars.supertypes.contains(SupertypeSet::BASIC),
        "and a *basic* one — which is what the `Plains` line means"
    );
    assert!(
        chars.supertypes.contains(SupertypeSet::SNOW),
        "and snow-covered, the one word this printing adds to a Plains: {:?}",
        chars.supertypes
    );

    // The offer is read with the mana still floating in nothing: `can_afford`
    // reads the pool, so a mana ability has to be checked before it is taken.
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        legal.mana_abilities.contains(&land),
        "a land with a basic land type is named in the index-free list, since \
         there is no printed ability index to point at: {:?}",
        legal.mana_abilities
    );

    tap_all_mana(&mut engine, p0);
    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(
        pool.available(ManaColor::White),
        1,
        "{{T}}: Add {{W}}, and `Plains` is the type that says which colour"
    );
    assert_eq!(pool.total(), 1, "one land, one mana, and nothing else");
    for other in [
        ManaColor::Blue,
        ManaColor::Black,
        ManaColor::Red,
        ManaColor::Green,
    ] {
        assert_eq!(
            pool.available(other),
            0,
            "a Plains makes white and no other colour: {other:?}"
        );
    }
    assert!(is_tapped(&engine, land), "the tap was the price");
    assert!(
        stack_is_empty(&engine),
        "CR 605.3b: a mana ability uses no stack, so nothing is waiting to resolve"
    );
}
