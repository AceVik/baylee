//! A compiled pack survives every host reconstruction boundary and AI fork.
use super::*;
use baylee_core::preset::{LoopPolicy, RulePackSetup, SeatExtensions};
use baylee_engine::{
    engine::{DeckKnowledge, NoLoopDetection},
    extensions::{NativeDefinition, NativeLookup, NativeRulePack},
    state::SetupError,
};

#[derive(Clone)]
struct Pack;
impl NativeRulePack for Pack {
    const PACK: u16 = 1;
    const REVISION: u16 = 2;
    fn validate(preset: &GamePreset) -> Result<(), SetupError> {
        if preset.house_rules.loop_policy == LoopPolicy::Off {
            Ok(())
        } else {
            Err(SetupError::UnsupportedRulePack)
        }
    }
    fn definition(id: u16) -> Option<NativeDefinition> {
        (id == 1).then_some(NativeDefinition::Permanent(
            &baylee_cards::tokens::tower::TOWER_SEEDLING,
        ))
    }
}
type PackedLookup = NativeLookup<RegistryLookup, Pack>;
type PackedSession = Session<NoLoopDetection, PackedLookup>;

fn preset() -> GamePreset {
    let mut preset = test_preset();
    preset.format = FormatId::Custom;
    preset.house_rules.loop_policy = LoopPolicy::Off;
    preset.house_rules.rule_pack = Some(RulePackSetup {
        pack: Pack::PACK,
        revision: Pack::REVISION,
        seats: vec![SeatExtensions {
            seat: 0,
            definitions: vec![1],
        }],
    });
    for seat in &mut preset.seats {
        seat.controller = SeatController::Open;
    }
    preset
}

#[test]
fn native_pack_survives_host_resume_exact_replay_projection_and_ai_fork() {
    let mut live =
        PackedSession::recorded_with_detection(&preset(), crate::record::this_build()).unwrap();
    assert!(Session::<NoLoopDetection>::with_detection(&preset()).is_none());
    for _ in 0..20 {
        let seat = live.awaiting_seat().unwrap();
        let action = live.house_action(seat).unwrap();
        live.act_slice(seat, action, 0).unwrap();
    }
    let mut record = live.take_record();
    let mut resumed = PackedSession::resume_with_detection(&record).unwrap();
    assert_eq!(live.snapshot_hash(), resumed.snapshot_hash());
    assert!(Session::<NoLoopDetection>::resume_with_detection(&record).is_err());
    assert!(crate::record::replay_exact_with_detection::<NoLoopDetection>(&record).is_err());
    for _ in 0..12 {
        let seat = live.awaiting_seat().unwrap();
        let action = live.house_action(seat).unwrap();
        live.act_slice(seat, action.clone(), 0).unwrap();
        resumed.act_slice(seat, action, 0).unwrap();
        assert_eq!(live.snapshot_hash(), resumed.snapshot_hash());
    }
    record.extend(resumed.take_record());
    let replay =
        crate::record::replay_exact_with_registry::<NoLoopDetection, PackedLookup>(&record)
            .unwrap();
    assert_eq!(replay.engine.snapshot_hash(), live.snapshot_hash());
    let mut moments = 0;
    let projection = crate::replay::project_with_registry::<NoLoopDetection, PackedLookup>(
        &record,
        vec!["One".into(), "Two".into()],
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
    let before = (live.snapshot_hash(), live.record_pending());
    let mut fork: SimulationSession<NoLoopDetection, PackedLookup> = live
        .fork_for(PlayerId::new(0), DeckKnowledge::Known, 91)
        .unwrap();
    fork.pump_at_most(4);
    assert_eq!((live.snapshot_hash(), live.record_pending()), before);
}

#[test]
fn the_ordinary_projector_cannot_emit_a_partial_native_game() {
    let mut live =
        PackedSession::recorded_with_detection(&preset(), crate::record::this_build()).unwrap();
    let record = live.take_record();
    let mut emitted = false;
    let refusal = crate::replay::project_with_detection::<NoLoopDetection>(
        &record,
        vec!["One".into(), "Two".into()],
        crate::replay::Perspective::Public,
        crate::replay::Verify::Stop,
        |_| {
            emitted = true;
            std::ops::ControlFlow::Continue(())
        },
    );
    assert!(matches!(
        refusal,
        Err(crate::record::ReplayError::Unbuildable)
    ));
    assert!(!emitted);
}
