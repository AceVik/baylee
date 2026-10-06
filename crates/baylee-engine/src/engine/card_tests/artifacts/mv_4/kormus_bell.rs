//! `cards/artifacts/mv_4/kormus_bell.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Kormus Bell is `{4}` and prints one sentence as three statics: "All Swamps
/// are 1/1 black creatures that are still lands."
///
/// Every clause needs its own witness on the board, which is why all three are
/// read off one cast: a Swamp of mine is the subject, the Swamp across the
/// table is what tells "all Swamps" from "Swamps you control", and an Island
/// beside them is the land the filter must decline — a static that had read
/// `Filter::LAND` instead of `HasSubtype(SWAMP)` would animate it too, and
/// every assertion about the Swamps would still pass. The two readings taken
/// before the artifact resolves are the control for the ones after it, since
/// a permanent that was already animated would satisfy them for a reason that
/// has nothing to do with the card arriving.
#[test]
#[allow(clippy::too_many_lines)] // one printed card, played end to end: the length is the card's
fn kormus_bell_animates_every_swamp_and_leaves_every_other_land_alone() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[
                forest(),
                forest(),
                forest(),
                forest(),
                forest(),
                swamp(),
                island(),
            ],
        )
        .hand(0, &[kormus_bell()])
        .battlefield(1, &[swamp()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let mine = on_battlefield(&engine, p0, swamp()).expect("my Swamp is out");
    let theirs = on_battlefield(&engine, p1, swamp()).expect("their Swamp is out");
    let other = on_battlefield(&engine, p0, island()).expect("the Island is out");
    assert!(
        !types(&engine, mine).contains(TypeSet::CREATURE),
        "with no Bell on the battlefield a Swamp is a land and nothing else"
    );
    assert!(
        engine
            .state()
            .object(mine)
            .expect("the Swamp is an object")
            .characteristics()
            .power
            .is_none(),
        "and it carries no body for the static below to be given credit for"
    );

    // {4} out of the six mana the five Forests and the Island make. The Swamp
    // is named as the printing kept back, because it is the permanent this
    // test reads afterwards and a source tapped for mana is a source whose
    // status has already changed for a reason of its own.
    tap_all_mana_but(&mut engine, p0, Some(swamp()));
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        6,
        "five Forests and one Island, and neither Swamp contributed"
    );
    cast_with_floating(&mut engine, p0, kormus_bell());
    pass_until(&mut engine, |e| {
        on_battlefield(e, p0, kormus_bell()).is_some()
    });
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        2,
        "the {{4}} came out of the pool, so the Bell really was cast"
    );

    let bell = on_battlefield(&engine, p0, kormus_bell()).expect("the Bell resolved");
    let bell_types = types(&engine, bell);
    assert!(
        bell_types.contains(TypeSet::ARTIFACT) && !bell_types.contains(TypeSet::CREATURE),
        "the Bell is the artifact it prints: \"All Swamps\" is not \"this\""
    );

    // The three clauses, all read on my own Swamp.
    let animated = types(&engine, mine);
    assert!(
        animated.contains(TypeSet::CREATURE) && animated.contains(TypeSet::LAND),
        "\"1/1 black creatures that are still lands\": a creature *and* still a \
         land, and not one of the two: {animated:?}"
    );
    assert_eq!(
        pt(&engine, mine),
        (1, 1),
        "the body \"are 1/1\" prints, on a permanent that had none"
    );
    assert!(
        engine
            .state()
            .object(mine)
            .expect("the Swamp is an object")
            .characteristics()
            .colors
            .contains(baylee_core::color::Color::Black),
        "\"...black...\": the third static, which a type change alone would not show"
    );

    // "All Swamps" is the whole table and not this seat's half of it.
    let across = types(&engine, theirs);
    assert!(
        across.contains(TypeSet::CREATURE) && across.contains(TypeSet::LAND),
        "a Swamp across the table is animated too, and is still a land: {across:?}"
    );
    assert_eq!(
        pt(&engine, theirs),
        (1, 1),
        "and it is the same 1/1 body, whoever controls it"
    );

    // The negative control: the land the filter declines.
    let plain = types(&engine, other);
    assert!(
        plain.contains(TypeSet::LAND) && !plain.contains(TypeSet::CREATURE),
        "\"All Swamps\": an Island is a land and no Swamp: {plain:?}"
    );
    assert!(
        engine
            .state()
            .object(other)
            .expect("the Island is an object")
            .characteristics()
            .power
            .is_none(),
        "and it was given no body either"
    );
}
