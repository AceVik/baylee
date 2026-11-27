//! `cards/creatures/mv_3/anurid_swarmsnapper.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_card_testkit::card_tests::legends_kit as lk;
use baylee_core::generated::index;

fn anurid_swarmsnapper() -> CardIndex {
    card_index("fd275ea7-9ccc-4148-9b33-392a612486dd")
}

/// Anurid Swarmsnapper has reach and {1}{G} lets it block an additional creature this turn.
#[test]
fn anurid_swarmsnapper_blocks_flyer_and_additional_attacker() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[anurid_swarmsnapper(), forest(), forest()])
        .battlefield(1, &[index::ORNITHOPTER, llanowar_elves()])
        .start();
    keep_mulligans(&mut engine);
    reach_their_main_phase(&mut engine, p1);

    let snapper = on_battlefield(&engine, p0, anurid_swarmsnapper()).unwrap();
    let thopter = on_battlefield(&engine, p1, index::ORNITHOPTER).unwrap();
    let elves = on_battlefield(&engine, p1, llanowar_elves()).unwrap();

    engine.apply(p1, PlayerAction::PassPriority).unwrap();
    pass_until(
        &mut engine,
        |e| matches!(e.pending(), Pending::Priority { player, .. } if *player == p0),
    );
    tap_all_mana(&mut engine, p0);
    activate(&mut engine, p0, anurid_swarmsnapper(), 0);
    pass_until(
        &mut engine,
        |e| matches!(e.pending(), Pending::ChooseAttackers { player, .. } if *player == p1),
    );
    engine
        .apply(
            p1,
            PlayerAction::DeclareAttackers {
                attackers: vec![
                    (thopter, Defender::Player(p0)),
                    (elves, Defender::Player(p0)),
                ],
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
                blockers: vec![(snapper, elves), (snapper, thopter)],
            },
        )
        .expect("reach + additional block lets Swarmsnapper block both");
    loop {
        if engine.state().turn.phase == Phase::SecondMain && stack_is_empty(&engine) {
            break;
        }
        match engine.pending().clone() {
            Pending::Priority { player, .. } => {
                engine.apply(player, PlayerAction::PassPriority).unwrap();
            }
            Pending::ChooseNumber { player, max, .. } => {
                engine
                    .apply(player, PlayerAction::ChooseNumber(max))
                    .unwrap();
            }
            other => panic!("unexpected combat pending: {other:?}"),
        }
    }

    assert_eq!(lk::life(&engine, p0), 20);
    assert!(on_battlefield(&engine, p1, llanowar_elves()).is_none());
    assert!(on_battlefield(&engine, p1, index::ORNITHOPTER).is_some());
}
