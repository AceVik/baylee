//! `cards/lands/check/rootbound_crag.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Rootbound Crag prints two sentences and both are played here, because the
/// second is only reachable on the board the first leaves: "This land enters
/// tapped unless you control a Mountain or a Forest" and "{T}: Add {R} or
/// {G}." A Forest under the same seat is the whole of the condition, so the
/// land arrives untapped and its `{T}` is live the same turn — and the mana
/// line has to be a real choice, two colours on the menu with the named one
/// in the pool rather than a default green off a card that says "or". The
/// same board with an Island instead of the Forest is the counter-half: no
/// Mountain, no Forest, so the land enters tapped and offers no ability at
/// all in the turn it arrives.
#[test]
fn rootbound_crag_enters_untapped_beside_a_forest_and_taps_for_either_color() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[forest()])
        .hand(0, &[rootbound_crag()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let crag = play_land(&mut engine, p0, rootbound_crag());
    assert!(
        !entered_tapped(&engine, crag),
        "a Forest under the same seat is half the printed condition, so the \
         land arrives untapped"
    );

    // Ability 0 is the printed mana ability; the tap symbol is its whole
    // price, so nothing has to be floating for it to be offered.
    activate(&mut engine, p0, rootbound_crag(), 0);
    let Pending::ChooseColor { player, options } = engine.pending().clone() else {
        panic!(
            "\"Add {{R}} or {{G}}\" is a question, got {:?}",
            engine.pending()
        )
    };
    assert_eq!(player, p0, "the land's controller names the colour");
    assert!(
        options.contains(&ManaColor::Red) && options.contains(&ManaColor::Green),
        "both printed colours are on the menu: {options:?}"
    );
    assert_eq!(options.len(), 2, "and nothing else: {options:?}");

    engine
        .apply(p0, PlayerAction::ChooseColor(ManaColor::Green))
        .expect("green was one of the two it offered");
    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(
        pool.available(ManaColor::Green),
        1,
        "the colour that was named, and not a default"
    );
    assert_eq!(pool.total(), 1, "one mana, off one tap");
    assert!(
        stack_is_empty(&engine),
        "CR 605.3b: a mana ability uses no stack"
    );
    assert!(is_tapped(&engine, crag), "the land paid its own {{T}}");

    // The other half of the printed condition. Nothing on this board is a
    // Mountain or a Forest, so the land enters tapped — and an
    // enters-tapped land's {T} is not a cost it can pay this turn.
    let mut without = Duel::new(SEED + 1, forest())
        .battlefield(0, &[island()])
        .hand(0, &[rootbound_crag()])
        .start();
    keep_mulligans(&mut without);
    reach_main_phase(&mut without, p0);
    let tapped_crag = play_land(&mut without, p0, rootbound_crag());
    assert!(
        entered_tapped(&without, tapped_crag),
        "an Island is neither a Mountain nor a Forest, so the land enters tapped"
    );
    let Pending::Priority { legal, .. } = without.pending().clone() else {
        panic!("expected priority, got {:?}", without.pending())
    };
    assert!(
        !legal
            .abilities
            .iter()
            .any(|(source, _)| *source == tapped_crag),
        "an enters-tapped land offers no {{T}} in the turn it arrives: {:?}",
        legal.abilities
    );
}
