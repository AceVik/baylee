//! `cards/lands/battle/scorched_geyser.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Scorched Geyser is a `Land — Island Mountain` that "enters tapped unless
/// you control two or more basic lands" and taps for `{U}` or `{R}`.
///
/// The count is the whole card and it is off by one for free: the entering
/// land is itself a basic land with two basic land types, so a count that
/// read it would turn a lone Forest into "two or more" and this land into an
/// untapped one. So the same card is played twice — once beside a single
/// Forest, where it must arrive tapped, and once beside two, where it must
/// arrive standing — and the standing one is then really tapped, for a
/// colour the engine offered rather than one assumed.
#[test]
fn scorched_geyser_counts_the_basic_lands_it_is_not_and_taps_for_blue_or_red() {
    let p0 = PlayerId::new(0);

    // One basic land: the Geyser entering must not be the second one.
    let mut thin = Duel::new(SEED, forest())
        .battlefield(0, &[forest()])
        .hand(0, &[scorched_geyser()])
        .start();
    keep_mulligans(&mut thin);
    reach_main_phase(&mut thin, p0);
    let early = play_land(&mut thin, p0, scorched_geyser());
    assert!(
        entered_tapped(&thin, early),
        "one basic land is not \"two or more\", and a count that included the \
         entering Geyser would say it was"
    );

    // Two basic lands: the printed condition already holds, so the
    // replacement has nothing to do.
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[forest(), forest()])
        .hand(0, &[scorched_geyser()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let geyser = play_land(&mut engine, p0, scorched_geyser());
    assert!(
        !entered_tapped(&engine, geyser),
        "two basic lands are \"two or more\", so the land arrives standing"
    );

    // The Forests are tapped first and the Geyser is held back: `{T}` is the
    // whole price of its own mana ability, and the helper would have spent it.
    tap_all_mana_but(&mut engine, p0, Some(scorched_geyser()));
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        2,
        "two green from the Forests before the Geyser is pressed"
    );
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    let routes = deeds(&legal, &[geyser]);
    assert!(
        !routes.is_empty(),
        "an untapped Island Mountain has a mana ability to offer: {legal:?}"
    );
    engine.apply(p0, routes[0].1.action(geyser)).unwrap();

    let Pending::ChooseColor { player, options } = engine.pending().clone() else {
        panic!(
            "`Add {{U}} or {{R}}` is a question, got {:?}",
            engine.pending()
        )
    };
    assert_eq!(player, p0, "the activating seat names the colour");
    assert!(
        options.contains(&ManaColor::Blue) && options.contains(&ManaColor::Red),
        "the two colours the land prints: {options:?}"
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
    assert_eq!(
        pool.total(),
        3,
        "two Forests and one Island Mountain, off one tap each"
    );
    assert!(is_tapped(&engine, geyser), "the Geyser paid its own {{T}}");
    assert!(
        stack_is_empty(&engine),
        "CR 605.3b: a mana ability uses no stack, so nothing is waiting to resolve"
    );
}
