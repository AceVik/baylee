//! `cards/creatures/artifacts/mv_4/junk_golem.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

fn junk_golem() -> CardIndex {
    card_index("044be66d-9334-4d4b-b420-b97a18698e95")
}

fn step_to_your_upkeep(engine: &mut Engine<RegistryLookup>, p0: PlayerId) {
    let start_turn = engine.state().turn.number;
    loop {
        if engine.state().turn.number > start_turn
            && engine.state().turn.active == p0
            && engine.state().turn.phase == Phase::Beginning
            && !matches!(engine.pending(), Pending::Priority { .. })
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
            _ => {
                let (p, a) = answer_one(engine).expect("answerable step");
                engine.apply(p, a).unwrap();
            }
        }
    }
}

/// Junk Golem enters with three +1/+1 counters (3/3), can discard with {1} to
/// add one (4/4), and on upkeep sacrifices unless you remove a +1/+1 counter.
#[test]
fn junk_golem_enters_with_three_counters_pumps_and_handles_upkeep_tax() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, plains())
        .battlefield(0, &[plains(), plains(), plains(), plains(), plains()])
        .hand(0, &[junk_golem(), forest()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let lands = all_on_battlefield(&engine, p0, plains());
    tap_mana_where(&mut engine, p0, |id| lands[..4].contains(&id));
    cast_with_floating(&mut engine, p0, junk_golem());
    pass_until(&mut engine, |e| at_rest(e, p0));

    let golem = on_battlefield(&engine, p0, junk_golem()).unwrap();
    assert_eq!(pt(&engine, golem), (3, 3));

    tap_mana_where(&mut engine, p0, |id| id == lands[4]);
    activate(&mut engine, p0, junk_golem(), 1);
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseCards { .. })
    });
    let f = in_hand(&engine, p0, forest()).unwrap();
    engine
        .apply(p0, PlayerAction::ChooseObjects { objects: vec![f] })
        .unwrap();
    pass_until(&mut engine, |e| at_rest(e, p0));
    assert_eq!(pt(&engine, golem), (4, 4));

    step_to_your_upkeep(&mut engine, p0);
    match engine.pending().clone() {
        Pending::YesNo { player, .. } => {
            engine.apply(player, PlayerAction::YesNo(true)).unwrap();
        }
        Pending::ChooseCards {
            player, options, ..
        } => {
            engine
                .apply(
                    player,
                    PlayerAction::ChooseObjects {
                        objects: vec![options[0]],
                    },
                )
                .unwrap();
        }
        other => panic!("expected upkeep choice, got {other:?}"),
    }
    reach_main_phase(&mut engine, p0);
    assert_eq!(pt(&engine, golem), (3, 3));

    step_to_your_upkeep(&mut engine, p0);
    match engine.pending().clone() {
        Pending::YesNo { player, .. } => {
            engine.apply(player, PlayerAction::YesNo(false)).unwrap();
        }
        Pending::ChooseCards { player, .. } => {
            engine
                .apply(player, PlayerAction::ChooseObjects { objects: vec![] })
                .unwrap();
        }
        other => panic!("expected upkeep choice, got {other:?}"),
    }
    reach_main_phase(&mut engine, p0);
    assert!(on_battlefield(&engine, p0, junk_golem()).is_none());
    assert!(in_graveyard(&engine, p0, junk_golem()).is_some());
}
