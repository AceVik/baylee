//! `cards/creatures/mv_7/hellkite_punisher.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

fn hellkite_punisher() -> CardIndex {
    card_index("c3ea33f4-17ac-41a4-8320-361b6f8aba0d")
}

/// Hellkite Punisher — a 6/6 flier for {5}{R}{R}.
#[test]
fn hellkite_punisher_costs_seven_and_is_a_six_six_flier() {
    let mut engine = table7(&lots(mountain(), 7), &[hellkite_punisher()], &[]);
    let dragon = cast_and_resolve(&mut engine, hellkite_punisher());
    assert_body(&engine, dragon, (6, 6), KeywordSet::FLYING);
}

/// "{R}: This creature gets +1/+0 until end of turn." Three Mountains, three
/// activations: 9/6, and it wears off at the end of the turn.
#[test]
fn hellkite_punisher_firebreathes() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut board = lots(mountain(), 3);
    board.push(hellkite_punisher());
    let mut engine = table7(&board, &[], &[]);
    let dragon = on_battlefield(&engine, p0, hellkite_punisher()).expect("out");
    tap_all_mana(&mut engine, p0);
    for expected in [7, 8, 9] {
        activate(&mut engine, p0, hellkite_punisher(), 0);
        pass_until(&mut engine, stack_is_empty);
        assert_eq!(pt(&engine, dragon), (expected, 6), "+1/+0 each time");
    }
    pass_until(&mut engine, |e| e.state().turn.active == p1);
    assert_eq!(pt(&engine, dragon), (6, 6), "until end of turn");
}
