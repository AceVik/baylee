//! `cards/creatures/mv_1/branded_brawlers.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

fn branded_brawlers() -> CardIndex {
    card_index("e975a8b3-f817-42f3-84da-b03ebf95cf25")
}

/// Branded Brawlers can't attack if the defending player controls an untapped land,
/// and can attack once all of the defending player's lands are tapped.
#[test]
fn branded_brawlers_cant_attack_when_defender_has_untapped_land() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, mountain())
        .battlefield(0, &[branded_brawlers()])
        .battlefield(1, &[island()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let brawlers = on_battlefield(&engine, p0, branded_brawlers()).unwrap();
    let p1_island = on_battlefield(&engine, p1, island()).unwrap();

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
        .apply(p1, PlayerAction::ActivateManaAbility { source: p1_island })
        .unwrap();

    pass_until(
        &mut engine,
        |e| matches!(e.pending(), Pending::ChooseAttackers { player, .. } if *player == p0),
    );
    let Pending::ChooseAttackers { attackers, .. } = engine.pending().clone() else {
        unreachable!();
    };
    assert!(
        attackers.contains(&brawlers),
        "Branded Brawlers can attack when defender has no untapped lands"
    );
}

/// Branded Brawlers can't block if its controller controls an untapped land,
/// and can block once its controller's lands are all tapped.
#[test]
fn branded_brawlers_cant_block_when_you_control_untapped_land() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, mountain())
        .battlefield(0, &[branded_brawlers(), mountain()])
        .battlefield(1, &[grizzly_bears()])
        .start();
    keep_mulligans(&mut engine);
    reach_their_main_phase(&mut engine, p1);

    let brawlers = on_battlefield(&engine, p0, branded_brawlers()).unwrap();
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
            .is_err(),
        "cannot block while controlling an untapped land"
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
        .expect("can block once all lands are tapped");
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
