//! `cards/lands/yavimaya_cradle_of_growth.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Yavimaya, Cradle of Growth is one continuous effect — "Each land is a
/// Forest in addition to its other land types" — and nothing to activate, so
/// the only place it can be read is on a land that was not a Forest. An Arid
/// Mesa has no basic land type and so no CR 305.6 mana ability at all, and the
/// granted Forest is what turns its `{T}` into green; a second Mesa across the
/// table is what tells the printed "each land" from a filter that had quietly
/// said "you control".
#[test]
fn yavimaya_makes_every_land_a_forest_that_taps_for_green_on_both_sides_of_the_table() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[yavimaya_cradle_of_growth(), arid_mesa()])
        .battlefield(1, &[arid_mesa()])
        .start();
    keep_mulligans(&mut engine);
    assert!(
        walk_to_own_main(&mut engine, p0),
        "p0 reaches its own main phase"
    );

    let mine = on_battlefield(&engine, p0, arid_mesa()).expect("my Arid Mesa is out");
    let theirs = on_battlefield(&engine, p1, arid_mesa()).expect("their Arid Mesa is out");
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "nothing floats: an Arid Mesa makes no mana of its own"
    );

    // A land with no basic land type has no CR 305.6 mana ability at all, so
    // the offer below can only be the Forest subtype the static grants.
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending());
    };
    assert!(
        legal.mana_abilities.contains(&mine),
        "the granted Forest prints `{{T}}: Add {{G}}`: {:?}",
        legal.mana_abilities
    );

    engine
        .apply(p0, PlayerAction::ActivateManaAbility { source: mine })
        .expect("the source the offer named is activated");
    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(
        pool.available(ManaColor::Green),
        1,
        "one green, and an Arid Mesa has no other colour to make"
    );
    assert_eq!(pool.total(), 1, "one mana, off one tap");
    assert!(
        stack_is_empty(&engine),
        "CR 605.3b: a mana ability uses no stack"
    );
    assert!(is_tapped(&engine, mine), "the land paid its own {{T}}");

    // "Each land" is not "each land you control", and the only seat that may
    // press the other Mesa's `{{T}}` is the seat that controls it.
    reach_their_main_phase(&mut engine, p1);
    let Pending::Priority { player, legal } = engine.pending().clone() else {
        panic!(
            "expected priority in their main phase, got {:?}",
            engine.pending()
        );
    };
    assert_eq!(player, p1, "the other seat holds the priority it is due");
    assert!(
        legal.mana_abilities.contains(&theirs),
        "their land is a Forest as much as mine: {:?}",
        legal.mana_abilities
    );
    engine
        .apply(p1, PlayerAction::ActivateManaAbility { source: theirs })
        .expect("the source the offer named is activated");
    assert_eq!(
        engine.state().players[1]
            .mana_pool
            .available(ManaColor::Green),
        1,
        "and it taps for the green the granted Forest prints"
    );
}
