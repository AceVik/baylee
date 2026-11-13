//! Strict opt-in BC labels on original Raw-policy records; never PPO data.
#![allow(missing_docs)]
use crate::features4::{EncodedCandidates4, EncodedState4, StepProvenance4, TeacherTarget4};
use anyhow::{Context, ensure};
use baylee_engine::choice::{Pending, PlayerAction};
use baylee_view::PlayerView;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::collections::BTreeMap;

mod bindings;
pub mod offer_boundary;
pub mod protocol44_boundary;
mod validation;

/// Independent teacher/behavior identities retained in JSON trajectory shards.
/// Dataset metadata additionally binds the original sidecar paths and hashes;
/// fixed-width binary caches must never be used without that dataset contract.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ShadowLabelProvenance4 {
    pub run_sha256: String,
    pub source_game: String,
    pub record_sha256: String,
    pub record_input_n: u64,
    pub teacher: String,
    pub teacher_action: PlayerAction,
    pub executed_action: PlayerAction,
    pub executed_provenance: StepProvenance4,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Label {
    kind: String,
    seat: u8,
    decision_seq: u64,
    record_byte_offset: usize,
    record_input_n: u64,
    record_input_hash: String,
    view_hash_fnv1a64: String,
    pending_sha256: String,
    encoded_state_sha256: String,
    candidates_sha256: String,
    teacher_action: PlayerAction,
    teacher_target: TeacherTarget4,
    executed_action: PlayerAction,
    executed_provenance: StepProvenance4,
    #[serde(rename = "label_use")]
    usage: String,
    teacher_source: String,
    teacher_has_logprob: bool,
    /// A watching model's seat question: its transcript's `answered` row
    /// by the mind is the label's evidence.
    #[serde(default)]
    teacher_question: Option<u64>,
}

/// A watching model's answers by seat question: the `answered` rows its
/// seat logic wrote for the mind's own answers (never the standing
/// orders', the house's or a least answer), the last where one question
/// has several.
///
/// # Errors
/// A transcript line that is not JSON, or an answer row with no question.
pub fn mind_answers(transcript: &[u8]) -> anyhow::Result<BTreeMap<u64, Value>> {
    let mut answers = BTreeMap::new();
    for line in transcript.split(|b| *b == b'\n').filter(|l| !l.is_empty()) {
        let row: Value = serde_json::from_slice(line)?;
        if row["event"] == "answered" && row["by"] == "mind" {
            answers.insert(
                row["question"].as_u64().context("Answer question")?,
                row["action"].clone(),
            );
        }
    }
    Ok(answers)
}

/// Whether `contract` is a watching model's, as eval4 writes it: a profile
/// after `llm:`, its identity, and the shadow contract's fixed terms.
pub(super) fn llm_contract(contract: &Value) -> bool {
    let Some(object) = contract.as_object() else {
        return false;
    };
    object.len() == 11
        && contract["teacher"]
            .as_str()
            .and_then(|teacher| teacher.strip_prefix("llm:"))
            .is_some_and(|profile| !profile.is_empty())
        && contract["teacher_identity"]["model"].is_string()
        && contract["schema"] == 1
        && contract["label_use"] == "bc_only"
        && contract["history_mode"] == "seat_conversation"
        && contract["scouting"] == false
        && contract["teacher_has_behavior_probability"] == false
        && contract["ppo_certification"] == false
        && contract["completion_required"] == true
        && contract["path"] == "shadow/*.jsonl"
        && contract["transcript_path"] == "shadow/*.seat.jsonl"
}

/// Per-game validated sidecar, consumed exactly once during chronological replay.
pub struct ShadowBc4 {
    pub(super) encoder_version: u32,
    header: Value,
    record_sha256: String,
    labels: BTreeMap<u64, Label>,
}

impl ShadowBc4 {
    /// Validates immutable run/record/sidecar identities before feature replay.
    /// `game` is the source games.jsonl key, never a converted numeric game ID.
    /// A watching model's labels also need its seat `transcript`.
    ///
    /// # Errors
    /// Unknown contracts, incomplete or mismatched records, malformed or duplicate labels.
    pub fn load(
        run: &[u8],
        sidecar: &[u8],
        record: &[u8],
        game: &str,
        catalog: &str,
        transcript: Option<&[u8]>,
    ) -> Result<Self, String> {
        bindings::load(run, sidecar, record, game, catalog, transcript).map_err(|e| e.to_string())
    }

    pub(super) fn has_label(&self, input_n: u64) -> bool {
        self.labels.contains_key(&input_n)
    }

    #[allow(
        clippy::too_many_arguments,
        reason = "Explicit independently reconstructed replay bindings"
    )]
    pub(super) fn target(
        &mut self,
        input_n: u64,
        decision_seq: u64,
        seat: u8,
        action: &PlayerAction,
        provenance: &StepProvenance4,
        view: &PlayerView,
        pending: &Pending,
        state: &EncodedState4,
        menu: &EncodedCandidates4,
        teams: &[Option<u8>],
    ) -> Result<Option<(TeacherTarget4, ShadowLabelProvenance4)>, String> {
        validation::target(
            self,
            input_n,
            decision_seq,
            seat,
            action,
            provenance,
            view,
            pending,
            state,
            menu,
            teams,
        )
        .map_err(|e| e.to_string())
    }

    pub(super) fn finish(&self) -> Result<(), String> {
        if self.labels.is_empty() {
            Ok(())
        } else {
            Err("Shadow labels were not consumed by eligible neural decisions".into())
        }
    }
}

pub(super) fn sha(bytes: &[u8]) -> String {
    use sha2::{Digest, Sha256};
    use std::fmt::Write as _;
    let mut result = String::new();
    for byte in Sha256::digest(bytes) {
        write!(result, "{byte:02x}").expect("String");
    }
    result
}

pub(super) fn expected_contract(profile: &str) -> Value {
    json!({"schema":1,"teacher":profile,"label_use":"bc_only","history_mode":"stateless_view_only",
        "scouting":false,"teacher_has_behavior_probability":false,"ppo_certification":false,
        "completion_required":true,"path":"shadow/*.jsonl"})
}
