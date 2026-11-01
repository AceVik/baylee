//! Duskworker — `{4}` 2/2 Artifact Creature — Construct (MRD).
//! Oracle: "Whenever this creature becomes blocked, regenerate it. {3}: This creature gets +1/+0 until end of turn."

use super::*;

fn duskworker() -> CardIndex {
    card_index("14d85a83-323b-4e42-88bf-71167a8174e3")
}

#[test]
fn duskworker_pumps_itself_and_regenerates_when_blocked() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let worker = duskworker();
    let hunter = fangren_hunter();
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[worker, forest(), forest(), forest()])
        .battlefield(1, &[hunter])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let worker_id = on_battlefield(&engine, p0, worker).unwrap();
    let hunter_id = on_battlefield(&engine, p1, hunter).unwrap();

    tap_all_mana_but(&mut engine, p0, None);
    activate(&mut engine, p0, worker, 1);
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(pt(&engine, worker_id), (3, 2));

    pass_until(
        &mut engine,
        |e| matches!(e.pending(), Pending::ChooseAttackers { player, .. } if *player == p0),
    );
    engine
        .apply(
            p0,
            PlayerAction::DeclareAttackers {
                attackers: vec![(worker_id, Defender::Player(p1))],
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseBlockers { .. })
    });
    engine
        .apply(
            p1,
            PlayerAction::DeclareBlockers {
                blockers: vec![(hunter_id, worker_id)],
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| {
        e.state().turn.phase == Phase::SecondMain && stack_is_empty(e)
    });

    // Duskworker regenerated from Fangren Hunter's 4 combat damage.
    assert!(on_battlefield(&engine, p0, worker).is_some());
}
