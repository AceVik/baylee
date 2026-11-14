//! Expected selection from declared capabilities. Never opens or spawns a binary.
//! Metadata consistency cannot prove what code an artifact actually contains.
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub(super) enum Variant {
    Standard,
    StandardLoops,
    Tower,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub(super) enum Mode {
    Standard,
    Tower,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub(super) enum Detection {
    Off,
    Break,
    Draw,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub(super) struct Contract {
    pub variant: Variant,
    pub build: String,
    pub feature_fingerprint: String,
    pub rule_pack: String,
    pub card_pool: String,
    pub ai_policy: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Artifact {
    pub contract: Contract,
    pub loop_detection: bool,
    pub tower_rules: bool,
    pub tower_ai: bool,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Input {
    pub mode: Mode,
    pub detection: Detection,
    pub required: Contract,
    pub installed: Vec<Artifact>,
    /// A single capacity pool shared by all installed variants.
    pub free_agent_slots: u16,
}

impl Contract {
    fn valid(&self) -> bool {
        [
            &self.build,
            &self.feature_fingerprint,
            &self.rule_pack,
            &self.card_pool,
            &self.ai_policy,
        ]
        .into_iter()
        .all(|label| super::bounded_label(label))
    }
}

impl Artifact {
    fn valid(&self) -> bool {
        let expected = match self.contract.variant {
            Variant::Standard => (false, false, false),
            Variant::StandardLoops => (true, false, false),
            Variant::Tower => (false, true, true),
        };
        self.contract.valid() && (self.loop_detection, self.tower_rules, self.tower_ai) == expected
    }
}

pub(super) fn select(input: &Input) -> Option<Variant> {
    let variant = match (input.mode, input.detection) {
        (Mode::Standard, Detection::Off) => Variant::Standard,
        (Mode::Standard, Detection::Break | Detection::Draw) => Variant::StandardLoops,
        (Mode::Tower, Detection::Off) => Variant::Tower,
        (Mode::Tower, Detection::Break | Detection::Draw) => return None,
    };
    if input.free_agent_slots == 0
        || input.installed.len() > 64
        || !input.required.valid()
        || input.required.variant != variant
    {
        return None;
    }
    for (index, artifact) in input.installed.iter().enumerate() {
        if !artifact.valid()
            || input.installed[..index]
                .iter()
                .any(|other| other.contract == artifact.contract)
        {
            return None;
        }
    }
    input
        .installed
        .iter()
        .any(|artifact| artifact.contract == input.required)
        .then_some(variant)
}
