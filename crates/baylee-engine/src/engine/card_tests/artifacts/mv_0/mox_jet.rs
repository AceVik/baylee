//! `cards/artifacts/mv_0/mox_jet.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Mox Jet — {0} artifact: "{T}: Add {B}."
///
/// Both of its numbers are read on one board where nothing else could be
/// responsible for either. `{0}` is the cast on an *empty* pool, so the Mox
/// arriving costs no land a tap; and `Add {B}` is the black in the pool
/// afterwards, which the Forest beside it cannot have made — a Forest makes
/// green and stays green, so the counter-half is read before the tap
/// (`available(Black)` is zero with a Forest already spent).
///
/// A *fixed* colour is the other half of a mana rock worth playing, and it is
/// where Mox Jet parts company with Mox Diamond: there is nothing to name, so
/// the mana is in the pool the instant the tap resolves and no `ChooseColor`
/// was ever asked — the assertion on `Pending::Priority` immediately after the
/// activation is what says so. The second half spends it, because a colour
/// that cannot pay for a spell of that colour is a label and not mana: Dark
/// Ritual costs `{B}` and nothing else, and the green floating beside it pays
/// none of it.
#[test]
fn mox_jet_lands_for_free_and_taps_for_black_that_pays_a_black_spell() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(1171, forest())
        .battlefield(0, &[forest()])
        .hand(0, &[mox_jet(), dark_ritual()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    // {0} spends nothing: the pool is empty before the cast and empty after.
    let card = in_hand(&engine, p0, mox_jet()).expect("the Mox is in hand");
    engine
        .apply(p0, PlayerAction::CastSpell { card })
        .expect("{0} is affordable on an empty board");
    pass_until(&mut engine, |e| at_rest(e, p0));
    let mox = on_battlefield(&engine, p0, mox_jet()).expect("the Mox resolved onto the table");
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "no land was tapped to pay for a zero-cost artifact"
    );

    // The Forest is the board's only other source, and the Mox is kept back:
    // it prints its own `{T}: Add {B}`, so `tap_all_mana` would have spent the
    // very permanent this test activates by hand (#159).
    tap_all_mana_but(&mut engine, p0, Some(mox_jet()));
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
        "and green is not black: nothing on this board has produced {{B}} yet"
    );

    // Ability 0 is the printed "{T}: Add {B}", and there is no colour to name.
    activate(&mut engine, p0, mox_jet(), 0);
    assert!(
        matches!(engine.pending(), Pending::Priority { player, .. } if *player == p0),
        "a mana ability asks nothing on the way (CR 605.1), got {:?}",
        engine.pending()
    );
    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(
        pool.available(ManaColor::Black),
        1,
        "the one colour the card prints, in the pool the moment it is activated"
    );
    assert_eq!(pool.total(), 2, "one green from the Forest and one black");
    assert!(
        stack_is_empty(&engine),
        "CR 605.3b: a mana ability uses no stack"
    );
    assert!(is_tapped(&engine, mox), "the Mox paid its own {{T}}");

    // The black is spendable as black: Dark Ritual costs {B} and nothing else.
    cast_with_floating(&mut engine, p0, dark_ritual());
    pass_until(&mut engine, stack_is_empty);
    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(
        pool.available(ManaColor::Black),
        3,
        "the {{B}} was spent and Dark Ritual's three black replaced it"
    );
    assert_eq!(
        pool.available(ManaColor::Green),
        1,
        "the Forest's green paid none of it, because it could not"
    );
}
