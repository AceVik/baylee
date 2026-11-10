//! `cards/artifacts/mv_0/mox_emerald.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Mox Emerald prints `{T}: Add {G}` on a `{0}` artifact, so the whole card is
/// one unconditional mana line: it costs nothing to arrive and its tap *names*
/// a color where Mox Diamond would have to ask for one. The Forest beside it is
/// the control that makes the green legible — it is a green source too, and it
/// must still be standing untapped when the pool holds exactly one green, so
/// the mana can only have come off the Mox. Casting it for `{0}` is read off an
/// empty pool, which is what says no land paid for it either.
#[test]
fn mox_emerald_taps_for_one_green_and_asks_no_question() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(23, forest())
        .battlefield(0, &[forest()])
        .hand(0, &[mox_emerald()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    // {0} spends nothing, so whatever is in the pool afterwards came off the
    // Mox and not off a land.
    let card = in_hand(&engine, p0, mox_emerald()).expect("the Mox is in hand");
    engine
        .apply(p0, PlayerAction::CastSpell { card })
        .expect("{0} is affordable on an empty board");
    pass_until(&mut engine, |e| at_rest(e, p0));
    let mox = on_battlefield(&engine, p0, mox_emerald()).expect("the Mox resolved onto the table");
    let land = on_battlefield(&engine, p0, forest()).expect("the Forest is still out");
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "no land was tapped to pay for a zero-cost artifact"
    );

    // Ability 0 is the printed "{T}: Add {G}", and it is an ordinary entry in
    // `legal.abilities` — a mana ability a card prints has an index to name,
    // unlike the CR 305.6 shortcut a basic land uses.
    activate(&mut engine, p0, mox_emerald(), 0);

    // One color and no question: the card names its mana where "Add one mana of
    // any color" would have to ask. A `ChooseColor` here would mean the engine
    // read a printed {G} as a choice it does not have.
    assert!(
        !matches!(engine.pending(), Pending::ChooseColor { .. }),
        "the card names its color, so there is nothing to choose: {:?}",
        engine.pending()
    );
    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(
        pool.available(ManaColor::Green),
        1,
        "{{G}} — the one color the card prints"
    );
    assert_eq!(
        pool.total(),
        1,
        "one mana, off one tap, and nothing beside it"
    );
    assert!(
        stack_is_empty(&engine),
        "CR 605.3b: a mana ability uses no stack, so nothing is waiting to resolve"
    );
    assert!(is_tapped(&engine, mox), "the Mox paid its own {{T}}");
    assert!(
        !is_tapped(&engine, land),
        "and the Forest beside it never moved, so the green has no other source on this board"
    );

    // The other half of "its whole price is its own tap": the tap is spent, so
    // the line is no longer one the seat may take.
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("the seat holds priority again, got {:?}", engine.pending())
    };
    assert!(
        !legal.abilities.contains(&(mox, 0)),
        "a tapped Mox has no {{T}} left to pay with: {:?}",
        legal.abilities
    );
}
