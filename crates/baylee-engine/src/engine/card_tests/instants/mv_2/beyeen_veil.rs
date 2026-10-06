//! `cards/instants/mv_2/beyeen_veil.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Beyeen Veil (`Coverage::Implemented`): "Creatures your opponents control
/// get -2/-0 until end of turn."
///
/// One creature on each side confirms "your opponents" is read correctly and
/// that only power is reduced — a 1/1 opponent creature becomes -1/1, while
/// the caster's own 1/1 remains untouched at (1, 1).
#[test]
fn beyeen_veil_shrinks_only_opponent_creatures_by_two_power() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(11, island())
        .battlefield(0, &[island(), island(), llanowar_elves()])
        .hand(0, &[beyeen_veil()])
        .battlefield(1, &[llanowar_elves()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let mine = on_battlefield(&engine, p0, llanowar_elves()).expect("my Elf is on the table");
    let theirs = on_battlefield(&engine, p1, llanowar_elves()).expect("their Elf is on the table");

    assert_eq!(pt(&engine, mine), (1, 1), "a 1/1 before the spell");
    assert_eq!(pt(&engine, theirs), (1, 1), "a 1/1 before the spell");

    cast_from_hand(&mut engine, p0, beyeen_veil());
    // The spell has no targets, so nothing is asked on the way to the stack.
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        pt(&engine, theirs),
        (-1, 1),
        "the opponent's 1/1 becomes -1/1 under -2/+0"
    );
    assert_eq!(
        pt(&engine, mine),
        (1, 1),
        "\"your opponents\" — the caster's own creature is untouched"
    );
}
