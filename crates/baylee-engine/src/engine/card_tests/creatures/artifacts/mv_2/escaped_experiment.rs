//! `cards/creatures/artifacts/mv_2/escaped_experiment.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

fn escaped_experiment() -> CardIndex {
    card_index("d1faaa3e-8e99-48e9-bb04-98bd46b7217c")
}

/// Escaped Experiment: a 2/1; "Whenever this creature attacks, target
/// creature an opponent controls gets -X/-0 until end of turn, where X is the
/// number of artifacts you control." With itself and a Gold Myr that is two:
/// the opponent's 2/2 Bears become 0/2, and recover next turn.
#[test]
fn escaped_experiment_shrinks_an_opposing_creature_by_the_artifacts_you_control() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = table(&[], &[escaped_experiment(), gold_myr()], &[grizzly_bears()]);
    let experiment = on_battlefield(&engine, p0, escaped_experiment()).expect("seated");
    let bears = on_battlefield(&engine, p1, grizzly_bears()).expect("seated");
    assert_eq!(pt(&engine, experiment), (2, 1));
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    engine
        .apply(
            p0,
            PlayerAction::DeclareAttackers {
                attackers: vec![(experiment, Defender::Player(p1))],
            },
        )
        .expect("the Experiment attacks");
    let options = pass_until_targets(&mut engine, p0);
    assert_eq!(options, vec![bears], "only the opponent's creature");
    aim_at(&mut engine, p0, bears);
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(pt(&engine, bears), (0, 2), "-2/-0: two artifacts");
    to_my_next_turn(&mut engine);
    assert_eq!(pt(&engine, bears), (2, 2), "until end of turn");
}

/// X counts artifacts as they are: with only the Experiment itself, -1/-0.
#[test]
fn escaped_experiment_alone_gives_minus_one() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = table(&[], &[escaped_experiment()], &[grizzly_bears()]);
    let experiment = on_battlefield(&engine, p0, escaped_experiment()).expect("seated");
    let bears = on_battlefield(&engine, p1, grizzly_bears()).expect("seated");
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    engine
        .apply(
            p0,
            PlayerAction::DeclareAttackers {
                attackers: vec![(experiment, Defender::Player(p1))],
            },
        )
        .unwrap();
    pass_until_targets(&mut engine, p0);
    aim_at(&mut engine, p0, bears);
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(pt(&engine, bears), (1, 2), "-1/-0: one artifact");
}
