//! `cards/creatures/mv_1/mogg_sentry.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Mogg Sentry — {R}, 1/1 Goblin Warrior: "Whenever an opponent casts a
/// spell, this creature gets +2/+2 until end of turn."
///
/// The whole card is one trigger, so both halves of the word *opponent* have
/// to be played: p0's own Dark Ritual must leave the Sentry the 1/1 it was
/// printed as, and p1's Dark Ritual must make it a 3/3. Dark Ritual is the
/// spell for both because it targets nothing (CR 601.2c) — a spell wanting a
/// target would be refused on a board built to measure a pump, not a spell.
/// A filter that had lost `ControlledByOpponent` would sail through a test
/// that only ever cast the opponent's spell, which is why the controller's
/// own cast comes first, while the body is still readable.
#[test]
fn mogg_sentry_pumps_for_an_opponents_spell_and_never_for_its_controllers() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(809, mountain())
        .battlefield(0, &[mountain(), mountain(), swamp(), mogg_sentry()])
        .hand(0, &[dark_ritual()])
        .battlefield(1, &[swamp()])
        .hand(1, &[dark_ritual()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let sentry = on_battlefield(&engine, p0, mogg_sentry()).expect("the Sentry is out");
    assert_eq!(pt(&engine, sentry), (1, 1), "the printed 1/1");

    cast_from_hand(&mut engine, p0, dark_ritual());
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(
        pt(&engine, sentry),
        (1, 1),
        "a spell its own controller casts is not an opponent's spell, so \
         nothing pumps it"
    );

    // Across the table the same card is a different event, and the +2/+2 has
    // to land on the 1/1: a 2/2 would mean the toughness was never read.
    reach_their_main_phase(&mut engine, p1);
    cast_from_hand(&mut engine, p1, dark_ritual());
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(
        pt(&engine, sentry),
        (3, 3),
        "\"whenever an opponent casts a spell, this creature gets +2/+2 \
         until end of turn\""
    );
}
