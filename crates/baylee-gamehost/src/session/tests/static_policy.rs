//! Host views, AI, recording, resumption and simulation retain the static policy.
use super::*;
use crate::record;
use baylee_core::preset::LoopPolicy;
use baylee_engine::engine::{DeckKnowledge, NoLoopDetection};

type OffSession = Session<NoLoopDetection>;

fn off_preset() -> GamePreset {
    let mut preset = test_preset();
    preset.house_rules.loop_policy = LoopPolicy::Off;
    for seat in &mut preset.seats {
        seat.controller = SeatController::Open;
    }
    preset
}

#[test]
fn static_session_construction_refuses_a_mismatched_preset() {
    assert!(Session::new(&off_preset()).is_none());
    assert!(OffSession::with_detection(&test_preset()).is_none());
    assert!(OffSession::with_detection(&off_preset()).is_some());
}

#[test]
fn both_variants_offer_the_same_views_and_expert_answers() {
    let text = std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../data/acceptance-decks.txt"
    ))
    .unwrap();
    let first = baylee_cards::decks::load_acceptance(&text, "Allytifact").unwrap();
    let second = baylee_cards::decks::load_acceptance(&text, "Victory").unwrap();
    let mut preset = baylee_cards::decks::preset_for(0x0057_a71c, &first, &second);
    for seat in &mut preset.seats {
        seat.controller = SeatController::Ai(AIProfile::TOWER_EXPERT);
    }
    let mut enabled = Session::new(&preset).unwrap();
    preset.house_rules.loop_policy = LoopPolicy::Off;
    let mut disabled = OffSession::with_detection(&preset).unwrap();
    enabled.discard_frames();
    disabled.discard_frames();
    for _ in 0..128 {
        for seat in [PlayerId::new(0), PlayerId::new(1)] {
            assert_eq!(
                serde_json::to_value(enabled.observe_view(seat)).unwrap(),
                serde_json::to_value(disabled.observe_view(seat)).unwrap(),
            );
        }
        assert_eq!(
            enabled.pump_at_most(1).len(),
            disabled.pump_at_most(1).len()
        );
        assert_eq!(
            enabled.engine.state().snapshot_hash(),
            disabled.engine.state().snapshot_hash()
        );
    }
}

#[test]
fn an_off_record_resumes_replays_and_forks_without_changing_policy() {
    let identity = baylee_protocol::execution::Identity::native(
        baylee_protocol::execution::Variant::Standard,
        record::this_build(),
        baylee_build::COMMIT,
        u32::from(AIProfile::GENERATION),
    );
    let mut live = OffSession::recorded_for_execution(&off_preset(), &identity).unwrap();
    for _ in 0..24 {
        let seat = live.awaiting_seat().expect("a human seat is asked");
        let action = live.house_action(seat).expect("a legal proposal");
        live.act_slice(seat, action, 0).unwrap();
    }
    let mut bytes = live.take_record();
    assert!(matches!(record::lines_of(&bytes).next().unwrap().unwrap(),
        record::Line::Header { execution: Some(recorded), .. } if recorded == identity));
    let mut resumed = OffSession::resume_with_detection(&bytes).unwrap();
    assert_eq!(live.snapshot_hash(), resumed.snapshot_hash());
    assert!(Session::resume_recorded(&bytes).is_err());
    assert!(record::replay_exact(&bytes).is_err());
    let replay = record::replay_exact_with_detection::<NoLoopDetection>(&bytes).unwrap();
    assert_eq!(replay.engine.snapshot_hash(), live.snapshot_hash());

    let mut moments = 0;
    let projection = crate::replay::project_with_detection::<NoLoopDetection>(
        &bytes,
        vec!["A".into(), "B".into()],
        crate::replay::Perspective::Public,
        crate::replay::Verify::Stop,
        |_| {
            moments += 1;
            std::ops::ControlFlow::Continue(())
        },
    )
    .unwrap();
    assert!(moments > 1);
    assert_eq!(projection.inputs, replay.inputs);
    assert!(projection.diverged.is_none());

    // Continue the acknowledged prefix; the resulting record is one exact replay.
    for _ in 0..12 {
        let seat = live.awaiting_seat().unwrap();
        let action = live.house_action(seat).unwrap();
        live.act_slice(seat, action.clone(), 0).unwrap();
        resumed.act_slice(seat, action, 0).unwrap();
        assert_eq!(live.snapshot_hash(), resumed.snapshot_hash());
    }
    bytes.extend(resumed.take_record());
    let replay = record::replay_exact_with_detection::<NoLoopDetection>(&bytes).unwrap();
    assert_eq!(replay.engine.snapshot_hash(), live.snapshot_hash());

    let before = (live.snapshot_hash(), live.record_pending());
    let mut fork: SimulationSession<NoLoopDetection> = live
        .fork_for(PlayerId::new(0), DeckKnowledge::Known, 0x91)
        .expect("a settled ordinary off game can fork");
    fork.pump_at_most(4);
    assert_eq!((live.snapshot_hash(), live.record_pending()), before);
}
