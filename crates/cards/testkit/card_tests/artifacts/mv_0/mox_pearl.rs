//! `cards/artifacts/mv_0/mox_pearl.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Mox Pearl is `{0}` for a `{T}: Add {W}`, and both halves are the engine's
/// answer rather than the card's. `{0}` is what lets it arrive on a board with
/// nothing to pay with: the pool is read *after* the cast, so the white mana
/// seen below can have come off nothing but the Mox itself. Its mana ability
/// is printed and therefore carries an index, so it lives in
/// `LegalActions::abilities` rather than in the CR 305.6 shortcut — pressing
/// it by index is that claim, and CR 605.3b is the stack that stays empty.
/// The lone Forest is the control: it never moves, so the white in the pool
/// has no other source on this board.
#[test]
fn mox_pearl_arrives_for_nothing_and_taps_for_white() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(17, forest())
        .battlefield(0, &[forest()])
        .hand(0, &[mox_pearl()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    let card = in_hand(&engine, p0, mox_pearl()).expect("the Mox is in hand");
    engine
        .apply(p0, PlayerAction::CastSpell { card })
        .expect("{0} is affordable on an empty board");
    pass_until(&mut engine, |e| at_rest(e, p0));
    let mox = on_battlefield(&engine, p0, mox_pearl()).expect("the Mox resolved onto the table");
    let land = on_battlefield(&engine, p0, forest()).expect("the Forest is still out");
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "no land was tapped to pay for a zero-cost artifact"
    );
    assert!(
        types(&engine, mox).contains(TypeSet::ARTIFACT),
        "and what arrived is an artifact"
    );

    // Ability 0 is the printed "{T}: Add {W}." — one mana of one named colour,
    // so nothing is asked on the way and there is no `ChooseColor` here.
    activate(&mut engine, p0, mox_pearl(), 0);
    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(pool.available(ManaColor::White), 1, "{{T}}: Add {{W}}");
    assert_eq!(pool.total(), 1, "one mana, off one tap");
    assert!(
        stack_is_empty(&engine),
        "CR 605.3b: a mana ability uses no stack, so nothing is waiting to resolve"
    );
    assert!(is_tapped(&engine, mox), "the Mox paid its own {{T}}");
    assert!(
        !is_tapped(&engine, land),
        "and the Forest beside it never moved, so the white mana has no \
         other source on this board"
    );
}
