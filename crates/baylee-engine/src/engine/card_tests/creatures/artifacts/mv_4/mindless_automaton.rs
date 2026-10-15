//! `cards/creatures/artifacts/mv_4/mindless_automaton.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

fn mindless_automaton() -> CardIndex {
    card_index("e7441107-57b1-4e88-a9d8-3eb7c338ef07")
}

/// Mindless Automaton — {4} 0/0: "This creature enters with two +1/+1
/// counters on it. {1}, Discard a card: Put a +1/+1 counter on this
/// creature. Remove two +1/+1 counters from this creature: Draw a card."
/// Cast off five Forests with a Sol Ring spare in hand, it is a 2/2 with two
/// counters; discarding the Sol Ring for the floating {1} makes it 3/3;
/// removing two counters draws a card and leaves a 1/1 with one counter.
#[test]
fn mindless_automaton_counters_discard_and_draw() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[forest(); 5])
        .hand(0, &[mindless_automaton(), sol_ring()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    cast_from_hand(&mut engine, p0, mindless_automaton());
    pass_until(&mut engine, stack_is_empty);
    let golem = on_battlefield(&engine, p0, mindless_automaton()).expect("it resolved");
    assert_eq!(counters_on(&engine, golem, CounterKind::P1P1), 2);
    assert_eq!(pt(&engine, golem), (2, 2));
    assert_eq!(engine.state().players[0].mana_pool.total(), 1, "one spare");

    activate(&mut engine, p0, mindless_automaton(), 0);
    answer_until_settled(&mut engine);
    assert!(
        in_graveyard(&engine, p0, sol_ring()).is_some(),
        "the discarded card"
    );
    assert_eq!(counters_on(&engine, golem, CounterKind::P1P1), 3);
    assert_eq!(pt(&engine, golem), (3, 3));
    assert_eq!(engine.state().players[0].mana_pool.total(), 0, "{{1}} paid");

    let hand_before = engine.state().zones.list(ZoneLocation::Hand(p0)).len();
    let library_before = library_size(&engine, p0);
    activate(&mut engine, p0, mindless_automaton(), 1);
    answer_until_settled(&mut engine);
    assert_eq!(counters_on(&engine, golem, CounterKind::P1P1), 1);
    assert_eq!(pt(&engine, golem), (1, 1));
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(p0)).len(),
        hand_before + 1,
        "drew a card"
    );
    assert_eq!(library_size(&engine, p0), library_before - 1);
}

/// With fewer than two counters the draw ability is not offered, and without
/// a card in hand the discard ability is not either.
#[test]
fn mindless_automaton_abilities_need_their_costs() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[forest(); 4])
        .hand(0, &[mindless_automaton()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    cast_from_hand(&mut engine, p0, mindless_automaton());
    pass_until(&mut engine, stack_is_empty);
    let golem = on_battlefield(&engine, p0, mindless_automaton()).expect("it resolved");
    assert!(
        priority_offer(&engine).abilities.contains(&(golem, 1)),
        "two counters pay the draw"
    );
    assert!(
        !priority_offer(&engine).abilities.contains(&(golem, 0)),
        "no card to discard and no {{1}} floating"
    );
}
