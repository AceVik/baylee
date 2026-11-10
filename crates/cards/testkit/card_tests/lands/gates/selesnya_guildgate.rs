//! `cards/lands/gates/selesnya_guildgate.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Selesnya Guildgate — Land — Gate: "This land enters tapped." and "{T}: Add
/// {G} or {W}."
///
/// The two printed sentences are one scenario because the first is what
/// delays the second: the Gate is played in p0's own main phase and stands
/// tapped, and a `{T}` ability is not merely worse while its permanent is
/// tapped — it is not offered at all, which is the half of "enters tapped"
/// that reading the card cannot show. A whole turn around the table is the
/// control for that reading: same board, same hand, and the only thing that
/// changed is that an untap step ran. The colour question is then the card's
/// own "or" — both colours on the menu — and the pool afterwards holds
/// exactly the one that was named and nothing else.
#[test]
fn selesnya_guildgate_enters_tapped_and_taps_for_green_or_white() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, forest())
        .hand(0, &[selesnya_guildgate()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let gate = play_land(&mut engine, p0, selesnya_guildgate());
    assert!(entered_tapped(&engine, gate), "\"This land enters tapped\"");

    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!(
            "expected priority after the land drop, got {:?}",
            engine.pending()
        )
    };
    assert!(
        !legal.mana_abilities.contains(&gate),
        "a Gate taps for no basic land type, so the CR 305.6 shortcut never \
         names it: {:?}",
        legal.mana_abilities
    );
    assert!(
        !legal.abilities.iter().any(|(source, _)| *source == gate),
        "and its printed {{T}}: Add is withheld while it stands tapped, \
         because there is no untapped permanent to pay for it: {:?}",
        legal.abilities
    );

    // A whole turn around the table. `reach_their_main_phase` reads the
    // *phase*, so p1's main is the door out of p0's turn and p0's own main
    // afterwards is the turn whose untap step stands the Gate back up.
    reach_their_main_phase(&mut engine, p1);
    reach_their_main_phase(&mut engine, p0);
    assert!(
        !is_tapped(&engine, gate),
        "the untap step ran, and that is the only thing this half of the \
         test changed"
    );

    // Ability 0 is the only line the card prints, and nothing has to be
    // floated first: the whole price is the tap symbol.
    activate(&mut engine, p0, selesnya_guildgate(), 0);
    let Pending::ChooseColor { player, options } = engine.pending().clone() else {
        panic!(
            "\"Add {{G}} or {{W}}\" is a question, got {:?}",
            engine.pending()
        )
    };
    assert_eq!(player, p0, "the activating seat names it");
    assert!(
        options.contains(&ManaColor::Green) && options.contains(&ManaColor::White),
        "\"or\" offers both, and neither is a default: {options:?}"
    );
    assert_eq!(
        options.len(),
        2,
        "a Gate makes green or white and nothing else"
    );

    engine
        .apply(p0, PlayerAction::ChooseColor(ManaColor::Green))
        .expect("green was one of the colours it offered");
    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(
        pool.available(ManaColor::Green),
        1,
        "the colour that was named, and not a default"
    );
    assert_eq!(
        pool.available(ManaColor::White),
        0,
        "the other half of the \"or\" was not made as well"
    );
    assert_eq!(pool.total(), 1, "one mana, off one tap");
    assert!(is_tapped(&engine, gate), "the Gate paid its own {{T}}");
    assert!(
        stack_is_empty(&engine),
        "CR 605.3b: a mana ability uses no stack, so nothing waits to resolve"
    );
}
