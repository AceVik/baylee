//! Clockwork Dragon — `{7}` 0/0 Artifact Creature — Dragon (MRD).
//! Oracle: "Flying. This creature enters with six +1/+1 counters on it. Whenever this creature attacks or blocks, remove a +1/+1 counter from it at end of combat. {3}: Put a +1/+1 counter on this creature."

use super::*;

fn clockwork_dragon() -> CardIndex {
    card_index("3361cf88-171a-4ddd-ac56-a31d42613c80")
}

#[test]
fn clockwork_dragon_enters_with_six_counters_adds_one_for_three_mana_and_loses_one_at_end_of_combat()
 {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let dragon = clockwork_dragon();
    let mut board = vec![forest(); 10];
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &board)
        .hand(0, &[dragon])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let forests = all_on_battlefield(&engine, p0, forest());
    tap_mana_where(&mut engine, p0, |id| forests[..7].contains(&id));
    cast_with_floating(&mut engine, p0, dragon);
    pass_until(&mut engine, stack_is_empty);

    let dragon_id = on_battlefield(&engine, p0, dragon).unwrap();
    assert!(keywords(&engine, dragon_id).contains(KeywordSet::FLYING));
    assert_eq!(pt(&engine, dragon_id), (6, 6));

    tap_mana_where(&mut engine, p0, |id| forests[7..10].contains(&id));
    activate(&mut engine, p0, dragon, 0);
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(pt(&engine, dragon_id), (7, 7));

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
                attackers: vec![(dragon_id, Defender::Player(p1))],
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| {
        e.state().turn.phase == Phase::SecondMain && stack_is_empty(e)
    });

    assert_eq!(engine.state().players[1].life, 13);
    assert_eq!(pt(&engine, dragon_id), (6, 6));
    let _ = &mut board;
}
