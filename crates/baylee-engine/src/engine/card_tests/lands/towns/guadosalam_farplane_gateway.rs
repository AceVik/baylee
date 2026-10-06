//! `cards/lands/towns/guadosalam_farplane_gateway.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

// oracle_id = "849c97e4-df15-4ecb-bdf8-283bb497d90c"

/// Guadosalam, Farplane Gateway prints two lines and neither is readable off
/// the card file: "This land enters tapped" and "{T}: Add {G} or {U}". The
/// entry is played the way a land is played — a real land drop, so the
/// replacement effect that turns it over is looked at — and the state right
/// afterwards is the whole of the first claim: tapped, and offering nothing,
/// because a `{T}` ability of a tapped permanent is not on the menu at all.
/// The untap step is the only thing that changes before the ability is read,
/// so the pair is what tells the entry from a board that simply had no mana.
/// The colour is then named twice, a turn apart, so that `or` is shown to be
/// one colour and never both: green arrives with no blue in the pool, and blue
/// arrives with no green.
#[test]
#[allow(clippy::too_many_lines)]
fn guadosalam_farplane_gateway_enters_tapped_and_taps_for_green_or_blue() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, forest())
        .hand(0, &[guadosalam_farplane_gateway()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let land = play_land(&mut engine, p0, guadosalam_farplane_gateway());
    assert!(
        entered_tapped(&engine, land),
        "\"This land enters tapped\" — a real land drop, so the entry it prints \
         is the one that applies"
    );

    // The tap symbol is the whole price of the card's only ability, and a
    // permanent lying tapped has none to pay with: the line is absent from the
    // offer rather than refused.
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!(
            "a land drop hands priority back, got {:?}",
            engine.pending()
        )
    };
    assert!(
        !legal.abilities.iter().any(|(source, _)| *source == land),
        "a tapped land has no {{T}} left to pay with: {:?}",
        legal.abilities
    );

    // The untap step is the only difference between the two readings, which is
    // why the ability is claimed again on the far side of it.
    reach_their_main_phase(&mut engine, p1);
    reach_their_main_phase(&mut engine, p0);
    assert!(
        !is_tapped(&engine, land),
        "the untap step stood it up again"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "and the pool emptied with the step that ended (CR 500.5): the land is \
         the only mana source on this board"
    );

    // Ability 0 is the printed "{T}: Add {G} or {U}". A mana ability a card
    // prints has an index to name, so it is an ordinary entry in `abilities`
    // and never the CR 305.6 shortcut a basic land type would use — this land
    // has no basic land type at all.
    activate(&mut engine, p0, guadosalam_farplane_gateway(), 0);
    let Pending::ChooseColor { player, options } = engine.pending().clone() else {
        panic!(
            "\"Add {{G}} or {{U}}\" is a question, got {:?}",
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
        options.contains(&ManaColor::Green) && options.contains(&ManaColor::Blue),
        "both halves of the printed choice are offered: {options:?}"
    );

    engine
        .apply(p0, PlayerAction::ChooseColor(ManaColor::Green))
        .expect("green was one of the colours it offered");
    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(
        pool.available(ManaColor::Green),
        1,
        "the colour that was named, in the pool the moment the tap resolved"
    );
    assert_eq!(
        pool.available(ManaColor::Blue),
        0,
        "\"or\" is one mana of one colour: a second blue would mean both halves \
         were added"
    );
    assert_eq!(pool.total(), 1, "one mana, off one tap");
    assert!(
        stack_is_empty(&engine),
        "CR 605.3b: a mana ability uses no stack, so nothing is waiting to resolve"
    );
    assert!(is_tapped(&engine, land), "the land paid its own {{T}}");

    // The other half of the pair, on the next turn: the pool is gone with the
    // step it was made in and the land is standing again, so the same ability
    // is asked the same question and the other answer is read off the pool.
    reach_their_main_phase(&mut engine, p1);
    reach_their_main_phase(&mut engine, p0);
    assert!(
        !is_tapped(&engine, land),
        "the untap step stood it up again"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "and the green from the turn before is gone (CR 500.5)"
    );

    activate(&mut engine, p0, guadosalam_farplane_gateway(), 0);
    let Pending::ChooseColor { options, .. } = engine.pending().clone() else {
        panic!(
            "the other half of \"or\" is the same question, got {:?}",
            engine.pending()
        )
    };
    assert!(
        options.contains(&ManaColor::Blue),
        "blue is the half this activation is about: {options:?}"
    );
    engine
        .apply(p0, PlayerAction::ChooseColor(ManaColor::Blue))
        .expect("blue was one of the colours it offered");

    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(
        pool.available(ManaColor::Blue),
        1,
        "the second colour the card prints, off the same tap symbol"
    );
    assert_eq!(
        pool.available(ManaColor::Green),
        0,
        "and not a green as well: one activation makes one mana of one colour"
    );
    assert_eq!(pool.total(), 1, "one mana, off one tap");
    assert!(
        stack_is_empty(&engine),
        "CR 605.3b: a mana ability uses no stack, so nothing is waiting to resolve"
    );
    assert!(
        is_tapped(&engine, land),
        "the land paid its own {{T}} again"
    );
}
