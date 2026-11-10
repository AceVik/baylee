//! Flayed Nim — `{3}{B}` 2/2 Creature — Skeleton (MRD).
//! Oracle: "Whenever this creature deals combat damage to a creature, that creature's controller loses that much life. {2}{B}: Regenerate this creature."

use super::*;

fn flayed_nim() -> CardIndex {
    card_index("857818d4-6cf0-4234-b9e0-d7181ba6b54f")
}

#[test]
fn flayed_nim_regenerates_and_makes_blocked_creatures_controller_lose_life() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let nim = flayed_nim();
    let hunter = fangren_hunter();
    let mut engine = Duel::new(SEED, swamp())
        .battlefield(0, &[nim, swamp(), swamp(), swamp()])
        .battlefield(1, &[hunter])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let nim_id = on_battlefield(&engine, p0, nim).unwrap();
    let hunter_id = on_battlefield(&engine, p1, hunter).unwrap();

    // Activate {2}{B}: Regenerate this creature.
    tap_all_mana_but(&mut engine, p0, None);
    raise_a_shield(&mut engine, p0, nim_id, 0);

    pass_until(
        &mut engine,
        |e| matches!(e.pending(), Pending::ChooseAttackers { player, .. } if *player == p0),
    );
    engine
        .apply(
            p0,
            PlayerAction::DeclareAttackers {
                attackers: vec![(nim_id, Defender::Player(p1))],
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
                blockers: vec![(hunter_id, nim_id)],
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| {
        e.state().turn.phase == Phase::SecondMain && stack_is_empty(e)
    });

    assert!(on_battlefield(&engine, p0, nim).is_some());
    assert_eq!(engine.state().players[1].life, 18);
}
