//! `cards/lands/check/idyllic_grange.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Idyllic Grange: "When this land enters untapped, put a +1/+1 counter on
/// target creature you control."
#[test]
fn idyllic_grange_puts_a_counter_on_a_creature_when_it_enters_untapped() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(4204, plains())
        .battlefield(0, &[plains(), plains(), plains(), llanowar_elves()])
        .hand(0, &[idyllic_grange()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let elves = on_battlefield(&engine, p0, llanowar_elves()).expect("my Elves");
    let grange = play_land(&mut engine, p0, idyllic_grange());
    assert!(!entered_tapped(&engine, grange));
    if matches!(engine.pending(), Pending::ChooseTargets { .. }) {
        answer_target(&mut engine, elves, &[]);
    }
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(counters_on(&engine, elves, CounterKind::P1P1), 1);
    assert_eq!(pt(&engine, elves), (2, 2));
}
