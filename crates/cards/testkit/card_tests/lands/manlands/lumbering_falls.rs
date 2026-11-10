//! `cards/lands/manlands/lumbering_falls.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Lumbering Falls enters tapped, taps for {G} or {U}, and for {2}{G}{U}
/// becomes a 3/3 green and blue Elemental creature with hexproof until end
/// of turn — while staying a land. That last clause is the load-bearing one:
/// the animation *adds* the creature type to the land rather than replacing
/// it, so the permanent has to read as LAND and CREATURE at once, and the
/// hexproof is a keyword only the layer system can project. The board stops
/// at the moment the animation resolves, because a creature that arrived
/// this turn can neither attack (CR 302.6) nor tap for mana.
#[test]
fn lumbering_falls_enters_tapped_then_animates_into_a_hexproof_elemental_that_is_still_a_land() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[forest(), forest(), island(), island()])
        .hand(0, &[lumbering_falls()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    // The land arrives the way the card prints it: tapped, so the turn it is
    // played it offers no mana at all. Nothing else is asked on the way in,
    // which is why `play_land` is enough and no entry question is answered.
    let falls = play_land(&mut engine, p0, lumbering_falls());
    assert!(is_tapped(&engine, falls), "\"This land enters tapped\"");
    assert!(
        !types(&engine, falls).contains(TypeSet::CREATURE),
        "a land that has only just arrived is no creature"
    );

    // A whole turn cycle so the untap step has run and the animation can be
    // paid out of the four lands beside it. Lumbering Falls is named as the
    // source to keep back: the animation costs no {T}, and a land tapped for
    // mana is a land that cannot say so afterwards.
    reach_their_main_phase(&mut engine, p1);
    reach_their_main_phase(&mut engine, p0);
    assert!(!is_tapped(&engine, falls), "the untap step stood it up");

    tap_all_mana_but(&mut engine, p0, Some(lumbering_falls()));
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        4,
        "two Forests and two Islands pay the {{2}}{{G}}{{U}}"
    );

    // Ability 0 is the printed mana ability, ability 1 is the animation: a
    // `{2}{G}{U}` activation with no tap symbol, so the land itself stays
    // untapped on the way through.
    activate(&mut engine, p0, lumbering_falls(), 1);
    pass_until(&mut engine, stack_is_empty);

    let kinds = types(&engine, falls);
    assert!(
        kinds.contains(TypeSet::LAND),
        "\"It's still a land\" — the animation adds a type, it does not \
         replace one: {kinds:?}"
    );
    assert!(
        kinds.contains(TypeSet::CREATURE),
        "and it is a creature until end of turn: {kinds:?}"
    );
    assert_eq!(pt(&engine, falls), (3, 3), "a 3/3 body");
    assert!(
        keywords(&engine, falls).contains(KeywordSet::HEXPROOF),
        "with hexproof, which only the layer system can project"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "and the {{2}}{{G}}{{U}} was the price"
    );
}
