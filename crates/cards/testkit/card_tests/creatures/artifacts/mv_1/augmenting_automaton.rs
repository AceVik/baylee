//! `cards/creatures/artifacts/mv_1/augmenting_automaton.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

fn augmenting_automaton() -> CardIndex {
    card_index("5ec0e58c-99b5-4ebf-8053-f59c30b86ef7")
}

/// "{1}{B}: This creature gets +1/+1 until end of turn." The ability needs
/// the black: two colorless is not on the offer. Each payment stacks another
/// +1/+1, and the bonus is gone in the next turn.
#[test]
fn augmenting_automaton_pumps_itself_for_one_black_and_one_more() {
    let p0 = PlayerId::new(0);
    let mut engine = main_with(&[augmenting_automaton()], &[]);
    let me = on_battlefield(&engine, p0, augmenting_automaton()).expect("seated");
    assert_eq!(pt(&engine, me), (1, 1), "the printed body");

    float_mana(&mut engine, ManaColor::Colorless, 2);
    assert!(
        !is_offered(&engine, me, 0),
        "{{1}}{{B}} wants a black; two colorless is not it"
    );
    float_mana(&mut engine, ManaColor::Black, 1);
    assert!(is_offered(&engine, me, 0));

    activate(&mut engine, p0, augmenting_automaton(), 0);
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(pt(&engine, me), (2, 2), "one activation");
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        1,
        "{{1}}{{B}} of three"
    );

    float_mana(&mut engine, ManaColor::Black, 1);
    activate(&mut engine, p0, augmenting_automaton(), 0);
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(pt(&engine, me), (3, 3), "a second activation stacks");

    pass_until(&mut engine, |e| {
        e.state().turn.active == PlayerId::new(1) && e.state().turn.phase == Phase::FirstMain
    });
    assert_eq!(pt(&engine, me), (1, 1), "until end of turn");
}
