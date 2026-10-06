//! `cards/instants/mv_3/reviving_dose.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Reviving Dose — {2}{W} instant: "You gain 3 life" and "Draw a card."
///
/// Two printed sentences that land in two different places, so one cast reads
/// both: the life total is the caster's alone (the opponent stays at twenty,
/// which is what tells "you" from "each player"), and the draw is read as a
/// move — the very card that was on top of the library is in hand afterwards
/// while the library is one shorter, so an effect that merely emptied the top
/// of the library could not pass. The three Plains are spent to the last mana
/// and the empty stack afterwards says the {2}{W} was paid and the spell
/// resolved rather than sitting on the stack.
#[test]
fn reviving_dose_gains_three_life_and_draws_the_top_card() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[plains(), plains(), plains()])
        .hand(0, &[reviving_dose()])
        .life(0, 20)
        .life(1, 20)
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    // The list's last entry is the top of the library, so this is the card the
    // draw is about; it is named before anything is cast.
    let top = *engine
        .state()
        .zones
        .list(ZoneLocation::Library(p0))
        .last()
        .expect("p0 has a library to draw from");
    let library_before = library_size(&engine, p0);

    cast_from_hand(&mut engine, p0, reviving_dose());
    assert!(
        !stack_is_empty(&engine),
        "an instant is a spell: it goes on the stack rather than resolving as it is cast"
    );
    assert_eq!(
        engine.state().players[0].life,
        20,
        "and nothing has been gained while it is still waiting there"
    );

    pass_until(&mut engine, stack_is_empty);

    assert_eq!(engine.state().players[0].life, 23, "\"You gain 3 life.\"");
    assert_eq!(
        engine.state().players[1].life,
        20,
        "\"you\" is the seat that cast it, not the table"
    );
    assert_eq!(
        library_size(&engine, p0),
        library_before - 1,
        "\"Draw a card\": one card left the top of the library"
    );
    assert!(
        engine
            .state()
            .zones
            .list(ZoneLocation::Hand(p0))
            .contains(&top),
        "and the very card that was on top is in hand, so the draw is a move \
         and not an emptied library"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "three Plains paid the {{2}}{{W}} to the last mana"
    );
}
