//! Clockwork Beetle — `{1}` 0/0 Artifact Creature — Insect (MRD).
//! Oracle: "This creature enters with two +1/+1 counters on it. Whenever this creature attacks or blocks, remove a +1/+1 counter from it at end of combat."

use super::*;

fn clockwork_beetle() -> CardIndex {
    card_index("7b9ddff4-82f5-4068-8105-116b8a6180d5")
}

#[test]
fn clockwork_beetle_enters_with_two_counters_and_loses_one_at_end_of_combat() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let beetle = clockwork_beetle();
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[forest()])
        .hand(0, &[beetle])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    cast_from_hand(&mut engine, p0, beetle);
    pass_until(&mut engine, stack_is_empty);

    let beetle_id = on_battlefield(&engine, p0, beetle).unwrap();
    assert_eq!(pt(&engine, beetle_id), (2, 2));

    // Advance to p0's next turn so Clockwork Beetle can attack.
    reach_their_main_phase(&mut engine, p1);
    reach_their_main_phase(&mut engine, p0);
    attack_with(&mut engine, beetle_id, None);

    assert_eq!(engine.state().players[1].life, 18);
    assert_eq!(pt(&engine, beetle_id), (1, 1));
}
