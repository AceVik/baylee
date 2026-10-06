//! `cards/lands/tapland/sunlit_marsh.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Sunlit Marsh — land, `{T}`: Add {W} or {B}, enters tapped.
/// Both printed halves are the engine's to answer: the entry is a
/// replacement (`EnterModifier::Tapped`), so the land is already tapped the
/// moment it lands and its mana line is not even in the offer for a turn; and
/// the mana line is a *choice* of two colours, which is a `ChooseColor` and
/// not a fixed `Add {W}`. One turn cycle reads both — the Marsh untaps with
/// its controller, and the tapped-out board's one blue-less pool says the
/// mana really came off the land and not off a basic beside it. The Forest is
/// the control that makes the colour read exact: green is what the other
/// source makes, and the black the answer named can only be the Marsh's.
#[test]
fn sunlit_marsh_enters_tapped_and_taps_for_white_or_black() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(41, forest())
        .battlefield(0, &[forest()])
        .hand(0, &[sunlit_marsh()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    // Playing the land is a real `PlayLand`, so the entry replacement runs:
    // `starting_battlefield` would have placed it untapped and the tapped
    // half would be unread.
    let marsh = play_land(&mut engine, p0, sunlit_marsh());
    pass_until(&mut engine, stack_is_empty);
    assert!(is_tapped(&engine, marsh), "\"This land enters tapped\"");
    assert!(
        engine
            .state()
            .object(marsh)
            .is_some_and(|o| o.characteristics().types.contains(TypeSet::LAND)),
        "and it is the land it prints"
    );

    // A tapped land has no `{T}` left, so its line is not offered — and the
    // board is tapped out first so this is not merely "no mana was floating".
    tap_all_mana_but(&mut engine, p0, Some(sunlit_marsh()));
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending());
    };
    assert!(
        !legal.mana_abilities.contains(&marsh)
            && !legal.abilities.iter().any(|(src, _)| *src == marsh),
        "a tapped Marsh offers nothing, and the offer is read with mana already floating: {:?}",
        legal.mana_abilities
    );

    // Across the opponent's turn and back: the Marsh untaps with its
    // controller, which is what makes the tap below legal at all.
    reach_their_main_phase(&mut engine, p1);
    assert!(walk_to_own_main(&mut engine, p0), "p0 takes another turn");
    assert!(
        !is_tapped(&engine, marsh),
        "the untap step ran, so the printed {{T}} is payable now"
    );

    // The Forest is kept back: it is the board's other source, and green is
    // what it makes — the black that lands in the pool afterwards has no
    // other source on this table.
    tap_all_mana_but(&mut engine, p0, Some(sunlit_marsh()));
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Green),
        1,
        "the Forest is tapped and made green"
    );
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Black),
        0,
        "and green is not black: nothing has produced {{B}} yet"
    );

    activate(&mut engine, p0, sunlit_marsh(), 0);
    let Pending::ChooseColor { player, options } = engine.pending().clone() else {
        panic!(
            "`Add {{W}} or {{B}}` is a question, got {:?}",
            engine.pending()
        )
    };
    assert_eq!(player, p0, "the seat that tapped names the colour");
    assert_eq!(
        options,
        vec![ManaColor::White, ManaColor::Black],
        "the two colours the card prints, and green is not one of them"
    );

    engine
        .apply(p0, PlayerAction::ChooseColor(ManaColor::Black))
        .expect("black was one of the colours it offered");
    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(
        pool.available(ManaColor::Black),
        1,
        "the colour that was named, and not a default"
    );
    assert_eq!(
        pool.available(ManaColor::White),
        0,
        "one question, one colour: a land that added both would show two here"
    );
    assert_eq!(pool.total(), 2, "one green from the Forest and one black");
    assert!(
        stack_is_empty(&engine),
        "CR 605.3b: a mana ability uses no stack"
    );
    assert!(is_tapped(&engine, marsh), "the Marsh paid its own {{T}}");
}
