//! `cards/lands/battle/smoldering_marsh.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Smoldering Marsh — Land — Swamp Mountain: "This land enters tapped unless
/// you control two or more basic lands" and "({T}: Add {B} or {R})."
///
/// The two boards straddle the printed threshold by exactly one: a single
/// Forest is one basic land and the Marsh arrives tapped, two Forests are two
/// and it arrives untapped. The mana line is read on that untapped board with
/// an empty pool, so the two colours on the menu and the single mana that
/// lands are this land's own tap and its Swamp Mountain type line — a
/// nonbasic land whose *types* are basic does not count itself toward the
/// arrival it is deciding.
#[test]
fn smoldering_marsh_enters_tapped_below_two_basic_lands_and_taps_for_black_or_red() {
    let p0 = PlayerId::new(0);

    // One basic land is one short of the printed "two or more".
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[forest()])
        .hand(0, &[smoldering_marsh()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");
    let short = play_land(&mut engine, p0, smoldering_marsh());
    assert!(
        entered_tapped(&engine, short),
        "one Forest is one basic land, and the Marsh asks for two"
    );

    // Two basic lands, and the same land arrives ready.
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[forest(), forest()])
        .hand(0, &[smoldering_marsh()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");
    let marsh = play_land(&mut engine, p0, smoldering_marsh());
    assert!(
        !entered_tapped(&engine, marsh),
        "two Forests are the two basic lands the printing asks for"
    );

    // `{T}: Add {B} or {R}`, off an empty pool: whatever lands in it came off
    // this one tap and from nothing else.
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "nothing is floating before the land is tapped"
    );
    activate(&mut engine, p0, smoldering_marsh(), 0);
    let Pending::ChooseColor { player, options } = engine.pending().clone() else {
        panic!(
            "`{{B}} or {{R}}` is a question the land asks, got {:?}",
            engine.pending()
        )
    };
    assert_eq!(player, p0, "the land's controller names the colour");
    assert_eq!(
        options.len(),
        2,
        "a Swamp Mountain is two colours and never five: {options:?}"
    );
    assert!(
        options.contains(&ManaColor::Black) && options.contains(&ManaColor::Red),
        "the two colours its printed type line grants: {options:?}"
    );
    engine
        .apply(p0, PlayerAction::ChooseColor(ManaColor::Red))
        .expect("red was one of the colours it offered");

    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(
        pool.available(ManaColor::Red),
        1,
        "the colour that was named, and not a default"
    );
    assert_eq!(pool.available(ManaColor::Black), 0, "and not the other one");
    assert_eq!(pool.total(), 1, "one tap, one mana");
    assert!(is_tapped(&engine, marsh), "the tap was the whole price");
}
