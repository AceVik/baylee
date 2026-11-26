//! Compiled pack admission and the actual compressed record/projection boundary.
use super::*;
use baylee_core::preset::{FormatId, LoopPolicy, RulePackSetup, SeatExtensions};
use baylee_engine::{
    engine::NoLoopDetection,
    extensions::{NativeDefinition, NativeLookup, NativeRulePack},
    state::SetupError,
};
use baylee_protocol::{execution, tape};
use std::io::Read as _;

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
type Registry = NativeLookup<RegistryLookup, Pack>;
type Runner = EngineRunner<NoLoopDetection, Registry>;

fn native_setup(definition: u16) -> Envelope {
    let mut preset = two_humans(0);
    preset.format = FormatId::Custom;
    preset.house_rules.loop_policy = LoopPolicy::Off;
    preset.house_rules.rule_pack = Some(RulePackSetup {
        pack: Pack::PACK,
        revision: Pack::REVISION,
        seats: vec![SeatExtensions {
            seat: 0,
            definitions: vec![definition],
        }],
    });
    Envelope {
        msg: Some(v1::envelope::Msg::GameSetup(v1::GameSetup {
            game_id: "native".into(),
            preset_json: serde_json::to_vec(&preset).unwrap(),
            execution: execution::Identity::native(
                execution::Variant::Tower,
                baylee_build::short(),
                baylee_build::COMMIT,
                u32::from(baylee_core::preset::AIProfile::GENERATION),
            )
            .to_metadata(),
            seat_names: vec!["One".into(), "Echo".into()],
            ..Default::default()
        })),
    }
}

#[test]
fn native_transport_refuses_unavailable_definitions_and_ordinary_fallback() {
    let mut ordinary = EngineRunner::<NoLoopDetection>::default();
    let mut native = Runner::default();
    for out in [
        ordinary.handle(native_setup(1), &[]),
        native.handle(native_setup(u16::MAX), &[]),
    ] {
        assert!(matches!(
            out.as_slice(),
            [Envelope {
                msg: Some(v1::envelope::Msg::GameEnded(_))
            }]
        ));
        assert!(frames(&out).is_empty());
    }
    assert!(!ordinary.ready());
    assert!(!native.ready());
}

#[test]
fn native_host_record_projects_only_with_the_same_compiled_registry() {
    let mut runner = Runner::default();
    assert!(runner.handle(native_setup(1), &[]).is_empty());
    assert!(runner.ready());
    assert_eq!(
        runner
            .session()
            .unwrap()
            .state()
            .zones
            .list(baylee_engine::zone::ZoneLocation::Battlefield)
            .len(),
        1
    );
    let out = runner.handle(
        Envelope {
            msg: Some(v1::envelope::Msg::FlushRecord(v1::FlushRecord {
                game_id: "native".into(),
                nonce: 7,
                hold: false,
            })),
        },
        &[],
    );
    let Some(v1::envelope::Msg::GameRecordChunk(chunk)) = &out[0].msg else {
        panic!("record must precede flush acknowledgement");
    };
    assert!(
        matches!(&out[1].msg, Some(v1::envelope::Msg::RecordFlushed(ack))
        if ack.nonce == 7 && ack.pieces == 1)
    );
    let order = v1::ProjectRecord {
        job: "native-replay".into(),
        game_id: "native".into(),
        record: chunk.data.clone(),
        names: vec!["One".into(), "Echo".into()],
        perspective: v1::project_record::Perspective::Public as i32,
        verify: v1::project_record::Verify::Stop as i32,
        ..Default::default()
    };
    let mut native = Vec::new();
    let stop = crate::projection::with_registry::<NoLoopDetection, Registry>(
        &order,
        baylee_build::short(),
        &mut |chunk| native.push(chunk),
    );
    assert_eq!(stop.stop, tape::StopKind::Cut);
    assert!(stop.diverged.is_none());
    assert!(
        tape_lines(&native)
            .iter()
            .any(|line| matches!(line, tape::Line::View(_)))
    );
    let mut ordinary = Vec::new();
    let stop = crate::projection::with_detection::<NoLoopDetection>(
        &order,
        baylee_build::short(),
        &mut |chunk| ordinary.push(chunk),
    );
    assert_eq!(stop.stop, tape::StopKind::Refused);
    assert_eq!(tape_lines(&ordinary).len(), 2, "only header and refusal");
}

fn tape_lines(chunks: &[v1::TapeChunk]) -> Vec<tape::Line> {
    let bytes: Vec<_> = chunks
        .iter()
        .flat_map(|chunk| chunk.data.iter().copied())
        .collect();
    let mut text = String::new();
    flate2::read::MultiGzDecoder::new(bytes.as_slice())
        .read_to_string(&mut text)
        .unwrap();
    text.lines()
        .map(|line| tape::parse_line(line).unwrap())
        .collect()
}
