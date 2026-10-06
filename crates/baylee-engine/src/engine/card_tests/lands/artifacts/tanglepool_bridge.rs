//! `cards/lands/artifacts/tanglepool_bridge.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Tanglepool Bridge is an artifact land that enters tapped, is
/// indestructible, and taps for {G} or {U}. All three are engine behaviour
/// rather than card text: the tap is an entry replacement that a
/// `starting_battlefield` placement would never run, indestructible is only
/// worth anything read by a destroy, and "or" is a colour question with
/// exactly two answers. So the land is played for real, named by a
/// Vindicate from across the table, and read again in its controller's next
/// untap step — a land that arrives tapped offers nothing on the turn it
/// arrives (CR 502.3).
#[test]
#[allow(clippy::too_many_lines)] // one printed card, played end to end: the length is the card's
fn tanglepool_bridge_enters_tapped_survives_destruction_and_taps_for_green_or_blue() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, basic_forest())
        .hand(0, &[tanglepool_bridge()])
        .battlefield(1, &[plains(), swamp(), plains()])
        .hand(1, &[vindicate()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let bridge = play_land(&mut engine, p0, tanglepool_bridge());
    assert!(
        entered_tapped(&engine, bridge),
        "the printed `EnterModifier::Tapped` runs on a real entry: it is \
         tapped the moment it lands"
    );
    let kinds = types(&engine, bridge);
    assert!(
        kinds.contains(TypeSet::ARTIFACT) && kinds.contains(TypeSet::LAND),
        "a land and an artifact at once: {kinds:?}"
    );
    assert!(
        keywords(&engine, bridge).contains(KeywordSet::INDESTRUCTIBLE),
        "the printed keyword reaches the permanent"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "and a land that came in tapped makes nothing on its arrival turn"
    );

    // Across the table, a destroy effect names it. Vindicate's target is a
    // permanent, and a tapped artifact land is one.
    reach_their_main_phase(&mut engine, p1);
    cast_from_hand(&mut engine, p1, vindicate());
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseTargets { .. })
    });
    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        unreachable!("the predicate just matched")
    };
    assert!(
        options.contains(&bridge),
        "\"target permanent\" reaches the Bridge: {options:?}"
    );
    engine
        .apply(
            p1,
            PlayerAction::ChooseObjects {
                objects: vec![bridge],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert!(
        on_battlefield(&engine, p0, tanglepool_bridge()).is_some(),
        "indestructible is read by the destroy itself, not merely projected"
    );
    assert!(
        in_graveyard(&engine, p0, tanglepool_bridge()).is_none(),
        "and a permanent that was not destroyed is in no graveyard"
    );

    // Back around: the untap step stands it up, which is also the control
    // that the destroy above met a real board rather than an absent one.
    reach_their_main_phase(&mut engine, p0);
    assert!(!is_tapped(&engine, bridge), "the untap step ran");
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "nothing floats on this board before the tap"
    );

    activate(&mut engine, p0, tanglepool_bridge(), 0);
    let Pending::ChooseColor { player, options } = engine.pending().clone() else {
        panic!(
            "`Add {{G}} or {{U}}` is a colour choice, got {:?}",
            engine.pending()
        )
    };
    assert_eq!(player, p0, "the seat that tapped names the colour");
    assert!(
        options.contains(&ManaColor::Green) && options.contains(&ManaColor::Blue),
        "both halves of the printed sentence are offered: {options:?}"
    );
    assert_eq!(
        options.len(),
        2,
        "an `or` is exactly two colours: {options:?}"
    );

    engine
        .apply(p0, PlayerAction::ChooseColor(ManaColor::Blue))
        .expect("blue was one of the colours it offered");
    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(
        pool.available(ManaColor::Blue),
        1,
        "the colour that was named"
    );
    assert_eq!(pool.available(ManaColor::Green), 0, "and not the other one");
    assert_eq!(
        pool.total(),
        1,
        "one mana off one tap, and nothing beside it"
    );
    assert!(
        stack_is_empty(&engine),
        "CR 605.3b: a mana ability uses no stack"
    );
    assert!(is_tapped(&engine, bridge), "the Bridge paid its own {{T}}");
}
