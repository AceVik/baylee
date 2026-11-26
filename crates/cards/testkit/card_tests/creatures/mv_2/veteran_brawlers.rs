//! `cards/creatures/mv_2/veteran_brawlers.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

fn veteran_brawlers() -> CardIndex {
    card_index("77a963a3-473a-4c34-bc46-8ef016a92c0c")
}

/// Veteran Brawlers can't attack while defender controls an untapped land,
/// and can attack once defender has no untapped lands.
#[test]
fn veteran_brawlers_attack_gate_checks_defenders_untapped_lands() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, mountain())
        .battlefield(0, &[veteran_brawlers()])
        .battlefield(1, &[forest()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let brawlers = on_battlefield(&engine, p0, veteran_brawlers()).unwrap();
    let p1_forest = on_battlefield(&engine, p1, forest()).unwrap();

    pass_until(
        &mut engine,
        |e| matches!(e.pending(), Pending::ChooseAttackers { player, .. } if *player == p0),
    );
    let Pending::ChooseAttackers { attackers, .. } = engine.pending().clone() else {
        unreachable!();
    };
    assert!(!attackers.contains(&brawlers));
    engine
        .apply(p0, PlayerAction::DeclareAttackers { attackers: vec![] })
        .unwrap();

    advance_next_main(&mut engine, p0);
    engine.apply(p0, PlayerAction::PassPriority).unwrap();
    pass_until(
        &mut engine,
        |e| matches!(e.pending(), Pending::Priority { player, .. } if *player == p1),
    );
    engine
        .apply(p1, PlayerAction::ActivateManaAbility { source: p1_forest })
        .unwrap();

    pass_until(
        &mut engine,
        |e| matches!(e.pending(), Pending::ChooseAttackers { player, .. } if *player == p0),
    );
    let Pending::ChooseAttackers { attackers, .. } = engine.pending().clone() else {
        unreachable!();
    };
    assert!(attackers.contains(&brawlers));
}

/// Veteran Brawlers can't block while you control an untapped land, and blocks when tapped out.
#[test]
fn veteran_brawlers_block_gate_checks_your_untapped_lands() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, mountain())
        .battlefield(0, &[veteran_brawlers(), mountain()])
        .battlefield(1, &[grizzly_bears()])
        .start();
    keep_mulligans(&mut engine);
    reach_their_main_phase(&mut engine, p1);

    let brawlers = on_battlefield(&engine, p0, veteran_brawlers()).unwrap();
    let bears = on_battlefield(&engine, p1, grizzly_bears()).unwrap();

    pass_until(
        &mut engine,
        |e| matches!(e.pending(), Pending::ChooseAttackers { player, .. } if *player == p1),
    );
    engine
        .apply(
            p1,
            PlayerAction::DeclareAttackers {
                attackers: vec![(bears, Defender::Player(p0))],
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseBlockers { .. })
    });
    assert!(
        engine
            .apply(
                p0,
                PlayerAction::DeclareBlockers {
                    blockers: vec![(brawlers, bears)],
                },
            )
            .is_err()
    );
    engine
        .apply(p0, PlayerAction::DeclareBlockers { blockers: vec![] })
        .unwrap();

    advance_next_main(&mut engine, p1);
    let mtn = on_battlefield(&engine, p0, mountain()).unwrap();
    engine.apply(p1, PlayerAction::PassPriority).unwrap();
    pass_until(
        &mut engine,
        |e| matches!(e.pending(), Pending::Priority { player, .. } if *player == p0),
    );
    engine
        .apply(p0, PlayerAction::ActivateManaAbility { source: mtn })
        .unwrap();
    pass_until(
        &mut engine,
        |e| matches!(e.pending(), Pending::ChooseAttackers { player, .. } if *player == p1),
    );
    engine
        .apply(
            p1,
            PlayerAction::DeclareAttackers {
                attackers: vec![(bears, Defender::Player(p0))],
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseBlockers { .. })
    });
    engine
        .apply(
            p0,
            PlayerAction::DeclareBlockers {
                blockers: vec![(brawlers, bears)],
            },
        )
        .expect("blocks cleanly once your lands are tapped");
}

fn advance_next_main(engine: &mut Engine<RegistryLookup>, target: PlayerId) {
    let start_turn = engine.state().turn.number;
    loop {
        if engine.state().turn.number > start_turn
            && engine.state().turn.active == target
            && engine.state().turn.phase == Phase::FirstMain
            && at_rest(engine, target)
        {
            return;
        }
        match engine.pending().clone() {
            Pending::Priority { player, .. } => {
                engine.apply(player, PlayerAction::PassPriority).unwrap();
            }
            Pending::ChooseAttackers { player, .. } => {
                engine
                    .apply(player, PlayerAction::DeclareAttackers { attackers: vec![] })
                    .unwrap();
            }
            Pending::ChooseBlockers { player, .. } => {
                engine
                    .apply(player, PlayerAction::DeclareBlockers { blockers: vec![] })
                    .unwrap();
            }
            other => panic!("unexpected pending while advancing: {other:?}"),
        }
    }
}
