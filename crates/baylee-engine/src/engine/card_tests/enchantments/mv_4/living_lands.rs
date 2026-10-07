//! `cards/enchantments/mv_4/living_lands.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

// oracle_id = "8f0179fe-6d7d-49cc-ab06-d3b402c6fc8d"

/// Living Lands — {3}{G} enchantment: "All Forests are 1/1 creatures that are
/// still lands."
///
/// Two printed words carry the card and each needs a different witness.
/// "Forests" is a *subtype*, so the Island across the table is the control: a
/// permanent that is a land and no Forest must stay a plain land, or the
/// static would be no more than "all lands are creatures". "All" is not "you
/// control", so the opponent's Forest is read beside my own — a filter that
/// had quietly grown a `ControlledByYou` would leave every one of my Forests
/// correct and only that one wrong. The body is the third claim, and `(1, 1)`
/// on a card that prints no power at all can only come off the layers.
#[test]
fn living_lands_turns_every_forest_into_a_one_one_creature_that_is_still_a_land() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(61, forest())
        .battlefield(0, &[forest(), forest(), forest(), forest()])
        .hand(0, &[living_lands()])
        .battlefield(1, &[forest(), island()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    // Four Forests and nothing else on this side of the table: the printed
    // {3}{G} is exactly the whole pool, so the enchantment is paid for rather
    // than merely announced.
    tap_all_mana(&mut engine, p0);
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        4,
        "four Forests, four green, and no other source under this seat"
    );
    cast_with_floating(&mut engine, p0, living_lands());
    pass_until(&mut engine, |e| at_rest(e, p0));

    assert!(
        on_battlefield(&engine, p0, living_lands()).is_some(),
        "the enchantment resolved onto the battlefield"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "the {{3}}{{G}} came out of the pool"
    );

    // Two types at once on a land that prints neither a body nor a type
    // line of its own: only the layer projection can produce this pair.
    let mine = all_on_battlefield(&engine, p0, forest());
    assert_eq!(mine.len(), 4, "every Forest this seat controls");
    for land in &mine {
        let kinds = types(&engine, *land);
        assert!(
            kinds.contains(TypeSet::LAND) && kinds.contains(TypeSet::CREATURE),
            "\"All Forests are 1/1 creatures that are still lands\": {kinds:?}"
        );
        assert_eq!(
            pt(&engine, *land),
            (1, 1),
            "and the body is the one the static sets, on a card that prints none"
        );
    }

    // "All" and not "you control": the same static reaches across the table.
    let theirs = on_battlefield(&engine, p1, forest()).expect("the opponent's Forest is out");
    let their_kinds = types(&engine, theirs);
    assert!(
        their_kinds.contains(TypeSet::LAND) && their_kinds.contains(TypeSet::CREATURE),
        "an opponent's Forest is a Forest too: {their_kinds:?}"
    );
    assert_eq!(
        pt(&engine, theirs),
        (1, 1),
        "and it is the same 1/1 as the ones on my side"
    );

    // The control: a land that is no Forest, which a static reading
    // `Filter::LAND` would have animated just the same.
    let other_land = on_battlefield(&engine, p1, island()).expect("the Island is out");
    let island_kinds = types(&engine, other_land);
    assert!(
        island_kinds.contains(TypeSet::LAND) && !island_kinds.contains(TypeSet::CREATURE),
        "an Island is a land and no Forest, so it stays a plain land: {island_kinds:?}"
    );
}
