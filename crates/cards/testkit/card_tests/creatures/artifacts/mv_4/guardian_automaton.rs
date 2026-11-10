//! `cards/creatures/artifacts/mv_4/guardian_automaton.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

fn guardian_automaton() -> CardIndex {
    card_index("cd29d0fd-685f-4c26-b69c-e96f1c0d59c4")
}

/// Guardian Automaton — {4} 3/3: "When this creature dies, you gain 3
/// life." A Lightning Bolt kills the 3/3; its controller goes from 20 to 23
/// when the trigger resolves, and the caster of the Bolt does not.
#[test]
fn guardian_automaton_gains_three_life_when_it_dies() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[guardian_automaton(), mountain()])
        .hand(0, &[lightning_bolt()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let golem = on_battlefield(&engine, p0, guardian_automaton()).expect("seated");
    assert_eq!(pt(&engine, golem), (3, 3));
    assert_eq!(engine.state().players[0].life, 20);

    bolt_to_death(&mut engine, p0, golem);
    pass_until(&mut engine, stack_is_empty);

    assert!(in_graveyard(&engine, p0, guardian_automaton()).is_some());
    assert_eq!(engine.state().players[0].life, 23, "+3 from the trigger");
    assert_eq!(engine.state().players[1].life, 20);
}
