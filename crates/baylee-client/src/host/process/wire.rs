//! Private desktop IPC. Presets and tapes retain their JSON representation.
use super::super::{HostMessage, ResumePoint};
use baylee_core::{ids::PlayerId, preset::LoopPolicy};
use baylee_protocol::execution::{Identity, Variant};
use serde::{Deserialize, Serialize};

pub(super) const SCHEMA: u32 = 1;
// A kept tape can be 256 MiB. Ordinary messages have a smaller independent cap.
pub(super) const INITIAL_LIMIT: usize =
    baylee_client_core::bugreport::MAX_INFLATED_BYTES + 1024 * 1024;
pub(super) const FRAME_LIMIT: usize = 16 * 1024 * 1024;
pub(super) const RECORD_CHUNK: usize = 1024 * 1024;

#[derive(Serialize, Deserialize)]
pub(super) struct Contract {
    pub schema: u32,
    pub view: u32,
    pub execution: Identity,
}

impl Contract {
    pub fn new(policy: LoopPolicy) -> Self {
        let variant = match policy {
            LoopPolicy::Off => Variant::Standard,
            LoopPolicy::RunOnceThenBreak | LoopPolicy::CompRulesDraw => Variant::StandardLoops,
        };
        Self {
            schema: SCHEMA,
            view: baylee_view::VIEW_VERSION,
            execution: Identity::native(
                variant,
                baylee_build::short(),
                baylee_build::COMMIT,
                u32::from(baylee_core::preset::AIProfile::GENERATION),
            ),
        }
    }

    pub fn accepts(&self, policy: LoopPolicy) -> bool {
        let ours = Self::new(policy);
        self.schema == ours.schema && self.view == ours.view && self.execution == ours.execution
    }
}

#[derive(Serialize, Deserialize)]
pub(super) enum Initial {
    New(Vec<u8>),
    Resume {
        record: Vec<u8>,
        name: String,
        hash: String,
    },
    Replay {
        record: Vec<u8>,
        header: Vec<u8>,
        verify_note: bool,
    },
}

#[derive(Serialize, Deserialize)]
pub(super) struct Bootstrap {
    pub contract: Contract,
    pub initial: Initial,
    pub seat: PlayerId,
    pub names: Vec<String>,
    // Only the parent that opened the settings store can ask the helper to do so.
    pub persist: bool,
}

impl Bootstrap {
    pub fn preset(&self) -> Result<baylee_core::preset::GamePreset, String> {
        match &self.initial {
            Initial::New(json) => {
                serde_json::from_slice(json).map_err(|_| "the local game setup is invalid".into())
            }
            Initial::Resume { record, .. } | Initial::Replay { record, .. } => {
                recorded_preset(record)
            }
        }
    }
}

pub(super) fn recorded_preset(record: &[u8]) -> Result<baylee_core::preset::GamePreset, String> {
    let first = record
        .split(|byte| *byte == b'\n')
        .next()
        .ok_or("the record has no header")?;
    // The same independent header bound as the hosted replay selection path.
    if first.len() > 2 * 1024 * 1024 {
        return Err("the local record header is too large".into());
    }
    match serde_json::from_slice(first).map_err(|_| "the record header is invalid")? {
        baylee_gamehost::record::Line::Header {
            preset, execution, ..
        } => {
            let expected = Contract::new(preset.house_rules.loop_policy)
                .execution
                .variant;
            if execution.is_some_and(|identity| identity.variant != expected) {
                return Err("the record requires a different local engine variant".into());
            }
            Ok(*preset)
        }
        _ => Err("the record has no header".into()),
    }
}

#[derive(Serialize, Deserialize)]
pub(super) enum Input {
    Action(baylee_engine::choice::PlayerAction),
    SeatSetting(baylee_view::SeatSetting),
}

#[derive(Serialize, Deserialize)]
pub(super) enum Output {
    // Identity is checked before the first view or record enters the renderer.
    Hello(Contract),
    Progress(Reply),
    Projection {
        lines: Vec<u8>,
        percent: u8,
        done: bool,
    },
}

#[derive(Serialize, Deserialize)]
pub(super) struct Reply {
    pub messages: Vec<HostMessage>,
    pub record: Vec<u8>,
    pub resume: Option<ResumePoint>,
    pub done: bool,
}
