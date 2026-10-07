//! `cards/lands/tapland/urborg_volcano.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Urborg Volcano prints exactly two sentences: "This land enters tapped"
/// and "{T}: Add {B} or {R}". Both are the engine's answer rather than the
/// card's, and each needs its own kind of board. The entry modifier is only
/// visible through a real `PlayLand` — `starting_battlefield` moves a card
/// with `Cause::Setup`, which no replacement looks at — so the land is
/// played on turn one and has to arrive tapped. The mana line is then read
/// on the *next* turn, because a land that came in tapped has no `{T}` to
/// pay with until its controller's untap step; the empty pool there makes
/// "one black and nothing else" a claim about the land rather than about a
/// board that happened to have another source on it.
#[test]
fn urborg_volcano_enters_tapped_and_taps_for_black_or_red() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, forest())
        .hand(0, &[urborg_volcano()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let volcano = play_land(&mut engine, p0, urborg_volcano());
    assert!(
        entered_tapped(&engine, volcano),
        "\"This land enters tapped\" — an entry modifier is only consulted by \
         a real `PlayLand`, so a board seated through `starting_battlefield` \
         would never have shown this"
    );

    // With the land down and nothing untapped on the table, its `{T}` is not
    // a price anything can pay, so the printed line is withheld rather than
    // offered-and-refused (`can_afford`, CR 601.2h).
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!(
            "playing a land hands priority back, got {:?}",
            engine.pending()
        )
    };
    assert!(
        !legal.mana_abilities.contains(&volcano),
        "a tapped land has no {{T}} to pay with: {:?}",
        legal.mana_abilities
    );
    assert!(
        !legal.abilities.iter().any(|(src, _)| *src == volcano),
        "and the mana ability it prints is withheld for the same reason: {:?}",
        legal.abilities
    );

    // Across p1's turn and back: the untap step is what makes the second half
    // of the card reachable, and the pool is read at zero so that the mana
    // claimed below has no source other than this land.
    reach_their_main_phase(&mut engine, p1);
    reach_their_main_phase(&mut engine, p0);
    assert!(
        !is_tapped(&engine, volcano),
        "the untap step brought it back"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "and nothing is floating before the tap, so one black can only be the \
         land's own"
    );
    assert_eq!(
        lands_of(&engine, p0).len(),
        1,
        "one land on the battlefield, and it is the Volcano"
    );

    // Ability 0 is the printed "{T}: Add {B} or {R}", and two colors is a
    // question (CR 106.1b) rather than a fixed line.
    activate(&mut engine, p0, urborg_volcano(), 0);
    let Pending::ChooseColor { player, options } = engine.pending().clone() else {
        panic!(
            "\"Add {{B}} or {{R}}\" is a question, got {:?}",
            engine.pending()
        )
    };
    assert_eq!(player, p0, "the land's controller names the color");
    assert_eq!(options.len(), 2, "two colors and no third: {options:?}");
    assert!(
        options.contains(&ManaColor::Black) && options.contains(&ManaColor::Red),
        "black and red, the two the card prints: {options:?}"
    );

    engine
        .apply(p0, PlayerAction::ChooseColor(ManaColor::Black))
        .expect("black was one of the colors it offered");
    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(
        pool.available(ManaColor::Black),
        1,
        "the color that was named, and not a default"
    );
    assert_eq!(
        pool.available(ManaColor::Red),
        0,
        "and not the other half of the choice, which was declined"
    );
    assert_eq!(
        pool.total(),
        1,
        "one mana, off one tap, with an empty pool before it"
    );
    assert!(
        stack_is_empty(&engine),
        "CR 605.3b: a mana ability uses no stack, so nothing is waiting to \
         resolve"
    );
    assert!(is_tapped(&engine, volcano), "the land paid its own {{T}}");
}
