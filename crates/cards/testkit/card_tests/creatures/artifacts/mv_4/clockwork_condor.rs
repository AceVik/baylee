//! Clockwork Condor — `{4}` 0/0 Artifact Creature — Bird (MRD).
//! Oracle: "Flying. This creature enters with three +1/+1 counters on it. Whenever this creature attacks or blocks, remove a +1/+1 counter from it at end of combat."

use super::*;

fn clockwork_condor() -> CardIndex {
    card_index("3eff8f21-9a1b-4afc-8a7b-75a740d0a4ea")
}

#[test]
fn clockwork_condor_enters_with_three_counters_and_loses_one_at_end_of_combat() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let condor = clockwork_condor();
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[forest(), forest(), forest(), forest()])
        .hand(0, &[condor])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    cast_from_hand(&mut engine, p0, condor);
    pass_until(&mut engine, stack_is_empty);

    let condor_id = on_battlefield(&engine, p0, condor).unwrap();
    assert_eq!(pt(&engine, condor_id), (3, 3));
    assert!(keywords(&engine, condor_id).contains(KeywordSet::FLYING));

    reach_their_main_phase(&mut engine, p1);
    reach_their_main_phase(&mut engine, p0);
    pass_until(
        &mut engine,
        |e| matches!(e.pending(), Pending::ChooseAttackers { player, .. } if *player == p0),
    );
    engine
        .apply(
            p0,
            PlayerAction::DeclareAttackers {
                attackers: vec![(condor_id, Defender::Player(p1))],
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| {
        e.state().turn.phase == Phase::SecondMain && stack_is_empty(e)
    });

    assert_eq!(engine.state().players[1].life, 17);
    assert_eq!(pt(&engine, condor_id), (2, 2));
}
