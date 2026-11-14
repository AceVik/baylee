//! The compiled off path and the shared rules kernel, independently of host routing.
use super::loop_tests::{TestPool, looping_preset, play_passively};
use super::simulation::DeckKnowledge;
use super::{
    DetectLoops, Engine, EngineError, GameEvent, LoopDetection, LoopSegment, NoLoopDetection,
    Pending, PlayerAction, PlayerId, checkpoint::Checkpoint,
};
use baylee_core::preset::{GamePreset, LoopPolicy};

fn start<D: LoopDetection>(preset: &GamePreset) -> Engine<TestPool, D> {
    let mut engine = Engine::with_detection(preset, TestPool).expect("matching variant");
    for _ in 0..2 {
        let Pending::Mulligan { player, .. } = engine.pending().clone() else {
            panic!("each seat keeps its opening hand");
        };
        engine.apply(player, PlayerAction::MulliganKeep).unwrap();
    }
    engine
}

fn ordinary(policy: LoopPolicy) -> GamePreset {
    let mut preset = looping_preset(policy);
    preset.seats[0].starting_battlefield.clear();
    preset
}

#[test]
fn the_off_policy_has_no_state_and_never_evaluates_detector_inputs() {
    assert_eq!(std::mem::size_of::<NoLoopDetection>(), 0);
    assert_eq!(
        std::mem::size_of::<<NoLoopDetection as LoopDetection>::Segment>(),
        0
    );
    let mut control = NoLoopDetection;
    control.clear_break_if(|| panic!("off must not inspect stack drainage"));
    assert_eq!(
        control.observe_action(
            || panic!("off must not inspect reset actions"),
            || panic!("off must not calculate an action signature"),
        ),
        None,
    );
    assert_eq!(
        control.observe(|| panic!("off must not calculate a segment signature")),
        None,
    );
    assert_eq!(control.detected(LoopPolicy::Off), None);
    assert_eq!(control.broken_count(), 0);
    assert!(!control.is_breaking());
    assert!(control.allows_fork());
}

#[test]
fn setup_rejects_both_directions_of_a_compiled_policy_mismatch() {
    for policy in [LoopPolicy::RunOnceThenBreak, LoopPolicy::CompRulesDraw] {
        let preset = ordinary(policy);
        assert!(Engine::<_, DetectLoops>::with_detection(&preset, TestPool).is_ok());
        assert!(matches!(
            Engine::<_, NoLoopDetection>::with_detection(&preset, TestPool),
            Err(EngineError::UnsupportedLoopPolicy),
        ));
    }
    let preset = ordinary(LoopPolicy::Off);
    assert!(Engine::<_, NoLoopDetection>::with_detection(&preset, TestPool).is_ok());
    assert!(matches!(
        Engine::new(&preset, TestPool),
        Err(EngineError::UnsupportedLoopPolicy),
    ));
}

#[test]
fn disabling_detection_preserves_ordinary_decisions_events_and_state() {
    let mut enabled = start::<DetectLoops>(&ordinary(LoopPolicy::RunOnceThenBreak));
    let mut disabled = start::<NoLoopDetection>(&ordinary(LoopPolicy::Off));
    for _ in 0..256 {
        assert_eq!(
            play_passively(&mut enabled, 1),
            play_passively(&mut disabled, 1)
        );
        assert_eq!(
            enabled.state().snapshot_hash(),
            disabled.state().snapshot_hash()
        );
        assert_eq!(
            format!("{:?}", enabled.pending()),
            format!("{:?}", disabled.pending())
        );
    }
    assert_eq!(
        format!("{:?}", enabled.journal().entries()),
        format!("{:?}", disabled.journal().entries()),
    );
}

#[test]
fn an_off_game_keeps_its_loop_until_a_player_concedes() {
    let mut engine = start::<NoLoopDetection>(&looping_preset(LoopPolicy::Off));
    let limit =
        (crate::loops::LoopWatch::WATCH_AFTER + crate::loops::LoopWatch::SAMPLE_EVERY * 24) as u32;
    assert_eq!(play_passively(&mut engine, limit), limit);
    assert_eq!(engine.state().turn.number, 1);
    assert_eq!(engine.loops_broken(), 0);
    assert!(
        !engine
            .journal()
            .entries()
            .iter()
            .any(|entry| { matches!(entry.event, GameEvent::LoopDetected { .. }) })
    );
    engine
        .apply(PlayerId::new(0), PlayerAction::Concede)
        .unwrap();
    assert!(matches!(engine.pending(), Pending::GameOver(_)));
}

#[test]
fn off_checkpoints_refusals_and_fair_forks_keep_the_compiled_policy() {
    let mut engine = start::<NoLoopDetection>(&ordinary(LoopPolicy::Off));
    let before = engine.fingerprint();
    let checkpoint = Checkpoint::capture(&engine);
    play_passively(&mut engine, 12);
    checkpoint.restore(&mut engine);
    assert_eq!(engine.fingerprint(), before);
    assert!(
        engine
            .apply(
                PlayerId::new(0),
                PlayerAction::ChoosePlayer(PlayerId::new(0))
            )
            .is_err()
    );
    assert_eq!(engine.fingerprint(), before);
    let mut fork: Engine<TestPool, NoLoopDetection> = engine
        .fork_for(PlayerId::new(0), DeckKnowledge::Known, 91)
        .expect("off has no hidden-state signature to leak into a fork");
    assert_eq!(play_passively(&mut fork, 12), 12);
    assert_eq!(fork.loops_broken(), 0);
    assert_eq!(
        engine.fingerprint(),
        before,
        "simulation never changes the real game"
    );
}
