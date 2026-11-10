//! `cards/lands/tapland/tresserhorn_sinks.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Tresserhorn Sinks is a snow land with two lines: it enters tapped, and it
/// taps for black or red. The entry has to be a real land drop rather than a
/// `starting_battlefield` placement — a placement is not an entry, so no
/// modifier ever looks at it and the land would arrive untapped whatever the
/// card says. The mana line is then read on the turn *after* it untaps, since
/// a land that arrives tapped is a permanent with no `{T}` to pay with, and
/// the "or" is the colour question: a check of one colour would pass on a land
/// that only printed that one.
#[test]
fn tresserhorn_sinks_enters_tapped_and_taps_for_black_or_red() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .hand(0, &[tresserhorn_sinks()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let land = play_land(&mut engine, p0, tresserhorn_sinks());
    assert!(
        is_tapped(&engine, land),
        "\"This land enters tapped\" — and it was played, so the modifier \
         saw the entry at all"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "a tapped land made no mana on the way in"
    );

    // A tapped land is a permanent with no `{T}` to spend, so the mana line is
    // read on its controller's next turn. The walk goes out through seat 1's
    // main phase and back, which is what puts the untap step in between.
    reach_their_main_phase(&mut engine, PlayerId::new(1));
    reach_their_main_phase(&mut engine, p0);
    assert!(
        !is_tapped(&engine, land),
        "the untap step ran, so the land is able to pay its own tap symbol"
    );

    // Ability 0 is the printed "{T}: Add {B} or {R}", an ordinary entry in
    // `legal.abilities` because a printed mana ability has an index to name.
    activate(&mut engine, p0, tresserhorn_sinks(), 0);
    let Pending::ChooseColor { player, options } = engine.pending().clone() else {
        panic!("`{{B}} or {{R}}` is a question, got {:?}", engine.pending())
    };
    assert_eq!(player, p0, "the activating seat is the one that names it");
    assert_eq!(
        options,
        vec![ManaColor::Black, ManaColor::Red],
        "the two colours the card prints, and no third one"
    );

    engine
        .apply(p0, PlayerAction::ChooseColor(ManaColor::Black))
        .expect("black was one of the colours it offered");

    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(
        pool.available(ManaColor::Black),
        1,
        "the colour that was named, and not the other one"
    );
    assert_eq!(
        pool.available(ManaColor::Red),
        0,
        "one question, one colour"
    );
    assert_eq!(
        pool.total(),
        1,
        "one mana off one tap — the whole price was the tap symbol"
    );
    assert!(
        stack_is_empty(&engine),
        "CR 605.3b: a mana ability uses no stack, so nothing is waiting to resolve"
    );
    assert!(is_tapped(&engine, land), "the land paid its own {{T}}");
}
