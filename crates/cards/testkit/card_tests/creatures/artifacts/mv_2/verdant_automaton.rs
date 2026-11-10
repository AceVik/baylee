//! `cards/creatures/artifacts/mv_2/verdant_automaton.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

fn verdant_automaton() -> CardIndex {
    card_index("d3af1c69-42ae-4d29-a9f1-72f862d26303")
}

/// Verdant Automaton: a 1/2 with "{3}{G}: Put a +1/+1 counter on this
/// creature". Each activation adds a counter; the counters stay past the turn.
#[test]
fn verdant_automaton_puts_a_counter_on_itself_per_activation() {
    let p0 = PlayerId::new(0);
    let mut board = vec![verdant_automaton()];
    board.extend_from_slice(&[forest(); 8]);
    let mut engine = table(&[], &board, &[]);
    let automaton = on_battlefield(&engine, p0, verdant_automaton()).expect("seated");
    assert_eq!(pt(&engine, automaton), (1, 2));
    let forests = all_on_battlefield(&engine, p0, forest());
    tap_mana_where(&mut engine, p0, |id| forests[..3].contains(&id));
    assert!(
        !priority_offer(&engine).abilities.contains(&(automaton, 0)),
        "three mana do not pay {{3}}{{G}}"
    );
    tap_mana_where(&mut engine, p0, |id| forests[3..4].contains(&id));
    activate(&mut engine, p0, verdant_automaton(), 0);
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(
        counters_on(&engine, automaton, baylee_cards_dsl::CounterKind::P1P1),
        1
    );
    assert_eq!(pt(&engine, automaton), (2, 3));

    tap_mana_where(&mut engine, p0, |id| forests[4..].contains(&id));
    activate(&mut engine, p0, verdant_automaton(), 0);
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(
        counters_on(&engine, automaton, baylee_cards_dsl::CounterKind::P1P1),
        2
    );
    assert_eq!(pt(&engine, automaton), (3, 4));

    to_my_next_turn(&mut engine);
    assert_eq!(pt(&engine, automaton), (3, 4), "counters are not temporary");
}
