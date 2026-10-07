//! `cards/lands/towns/windurst_federation_center.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Windurst, Federation Center is a Town land whose entire printed text is
/// "This land enters tapped" and "{T}: Add {G} or {W}". Neither sentence can be
/// read off the card file: `starting_battlefield` *places* a permanent without
/// an entry, so the land has to be played out of a hand for the tapped clause
/// to be read at all, and "or" is a claim about the shape of a choice the
/// engine makes rather than about a pool total. The mana line is taken a turn
/// later, once the untap step has stood the land back up — which is also the
/// control for the offer the tapped land does not have.
#[test]
fn windurst_enters_tapped_and_taps_for_green_or_white() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, forest())
        .hand(0, &[windurst_federation_center()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    // A real land drop: a permanent seeded with `starting_battlefield` never
    // enters, so it would stand untapped whatever the card says.
    let land = play_land(&mut engine, p0, windurst_federation_center());
    assert!(is_tapped(&engine, land), "\"This land enters tapped\"");
    assert!(
        lands_of(&engine, p0).contains(&land),
        "and the drop landed as a land this seat controls: {:?}",
        lands_of(&engine, p0)
    );

    // A tapped permanent has no {T} to pay with, so the printed line is not
    // even offered on the turn the land arrives. The same line is offered and
    // used below, so this is the tapped clause and not a missing ability.
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        !legal.abilities.iter().any(|(source, _)| *source == land),
        "a tapped land has no {{T}} left to pay with: {:?}",
        legal.abilities
    );

    // The untap step of its controller's next turn is what makes the line
    // payable at all.
    reach_their_main_phase(&mut engine, p1);
    reach_their_main_phase(&mut engine, p0);
    assert!(!is_tapped(&engine, land), "the untap step stood it back up");
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "and the pool emptied with the step that ended (CR 500.5)"
    );

    // Ability 0 is the printed "{T}: Add {G} or {W}" — a mana ability a card
    // prints, so an ordinary `(source, index)` entry and not the CR 305.6
    // shortcut, and its whole price is its own tap.
    activate(&mut engine, p0, windurst_federation_center(), 0);
    let Pending::ChooseColor { player, options } = engine.pending().clone() else {
        panic!(
            "\"Add {{G}} or {{W}}\" is a question, got {:?}",
            engine.pending()
        )
    };
    assert_eq!(player, p0, "the activating seat is the one that names it");
    assert_eq!(
        options.len(),
        2,
        "the two colours the card prints and no third: {options:?}"
    );
    assert!(
        options.contains(&ManaColor::Green),
        "green is one half of the choice: {options:?}"
    );
    assert!(
        options.contains(&ManaColor::White),
        "and white the other: {options:?}"
    );

    engine
        .apply(p0, PlayerAction::ChooseColor(ManaColor::White))
        .expect("white was one of the colours it offered");

    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(
        pool.available(ManaColor::White),
        1,
        "the colour that was named, and not a default"
    );
    assert_eq!(
        pool.available(ManaColor::Green),
        0,
        "\"or\" is one mana of one colour: a card that added both would leave \
         green beside the white"
    );
    assert_eq!(pool.total(), 1, "one mana, off one tap");
    assert!(
        stack_is_empty(&engine),
        "CR 605.3b: a mana ability uses no stack, so the mana is here at once"
    );
    assert!(is_tapped(&engine, land), "the land paid its own {{T}}");
}
