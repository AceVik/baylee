//! `cards/lands/pain/salt_flats.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Salt Flats — Land. "This land enters tapped. {T}: Add {C}. {T}: Add {W} or
/// {B}. This land deals 1 damage to you."
///
/// All three printed sentences are played. The entry clause is read off the
/// permanent a real `PlayLand` left, not off `starting_battlefield` — a setup
/// placement runs no `EnterModifier`, so a land built that way arrives untapped
/// and proves nothing — and while it is down it offers neither `{T}`, which is
/// the whole cost the clause buys. Each tap is then taken on its own turn
/// against an empty pool, so the mana in it can only have come off this land:
/// the colourless line adds one and charges nobody, and the coloured one asks
/// for exactly `{W}` or `{B}` — two colours, not the five of "any colour" —
/// and takes the life point it prints from its own controller, which is what
/// the opponent's untouched total measures.
#[test]
fn salt_flats_enters_tapped_then_taps_for_a_colourless_and_a_colour_that_costs_one_life() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let (mut engine, flats) =
        play_land_face(salt_flats(), 0).expect("a land in hand over an empty board is playable");
    assert!(
        entered_tapped(&engine, flats),
        "\"This land enters tapped\" — an `EnterModifier` a setup placement skips"
    );

    // A tapped land is no source at all, so the printed clause costs a whole
    // turn: `{T}` cannot be paid for the rest of this one.
    let Pending::Priority { player, legal } = engine.pending().clone() else {
        panic!(
            "the land drop hands priority back, got {:?}",
            engine.pending()
        )
    };
    assert_eq!(player, p0, "the seat that played the land keeps priority");
    assert!(
        !legal.abilities.iter().any(|(source, _)| *source == flats),
        "tapped, it offers neither of its mana abilities: {:?}",
        legal.abilities
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "and playing it made no mana either"
    );

    // One turn has to pass before either line can be read at all.
    reach_their_main_phase(&mut engine, p1);
    reach_their_main_phase(&mut engine, p0);
    assert!(
        !is_tapped(&engine, flats),
        "its controller's untap step is what stands it back up"
    );

    // `{T}: Add {C}` — index 0, and the half of the card that costs nothing.
    activate(&mut engine, p0, salt_flats(), 0);
    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(pool.available(ManaColor::Colorless), 1, "{{T}}: Add {{C}}");
    assert_eq!(pool.total(), 1, "and nothing else came with it");
    assert_eq!(
        engine.state().players[0].life,
        20,
        "the colourless line takes no life from anybody"
    );
    assert!(is_tapped(&engine, flats), "it paid its own {{T}}");
    assert!(
        stack_is_empty(&engine),
        "CR 605.3b: a mana ability never uses the stack"
    );

    // Round again, which the second line needs for the same reason the first
    // did — one `{T}`, one mana, one turn.
    reach_their_main_phase(&mut engine, p1);
    reach_their_main_phase(&mut engine, p0);
    assert!(!is_tapped(&engine, flats), "untapped for the second tap");
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "that mana did not survive the turn, so the next number is this card's"
    );

    // `{T}: Add {W} or {B}. This land deals 1 damage to you.`
    activate(&mut engine, p0, salt_flats(), 1);
    let Pending::ChooseColor { player, options } = engine.pending().clone() else {
        panic!(
            "\"Add {{W}} or {{B}}\" is a question, got {:?}",
            engine.pending()
        )
    };
    assert_eq!(player, p0, "the seat that tapped the land names the colour");
    assert!(
        options.contains(&ManaColor::White) && options.contains(&ManaColor::Black),
        "the two colours the card prints: {options:?}"
    );
    assert_eq!(
        options.len(),
        2,
        "and not the five colours of \"any colour\": {options:?}"
    );
    engine
        .apply(p0, PlayerAction::ChooseColor(ManaColor::Black))
        .expect("black was one of the colours it offered");

    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(
        pool.available(ManaColor::Black),
        1,
        "the colour that was named, and no default"
    );
    assert_eq!(pool.total(), 1, "one mana, off one tap");
    assert_eq!(
        engine.state().players[0].life,
        19,
        "\"This land deals 1 damage to you\" — the price is the controller's"
    );
    assert_eq!(
        engine.state().players[1].life,
        20,
        "and the damage never crosses the table"
    );
    assert!(is_tapped(&engine, flats), "one tap, one mana, one life");
    assert!(
        stack_is_empty(&engine),
        "still a mana ability: nothing was put on a stack"
    );
}
