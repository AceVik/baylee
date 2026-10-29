//! Replays recorded games into Playernet v4 trajectory steps (`Step4`),
//! maintaining continuous per-seat [`SeatKnowledge4`] observation streams,
//! relational graph edges, lossless candidate/action targets, auxiliary
//! self-supervised targets, and [`CoverageMatrix4`] statistics.

use baylee_core::ids::PlayerId;
use baylee_engine::choice::Pending;
use baylee_gamehost::Session;
use baylee_gamehost::record::{Line, RECORD_VERSION, Source};
use serde::{Deserialize, Serialize};

use crate::features4::{
    CoverageMatrix4, EncodedState4, HeadKind4, MAX_SEATS, PendingKind4, SeatKnowledge4,
    SourceKind4, StepProvenance4, TeacherTarget4, encode_teacher_action4, side_utility,
};
use crate::features7::{encode_candidates7_grouped, encode_state7};

/// Converter idle-pass weighting contract. Missing dataset metadata means legacy revision 1.
/// Revision 2 keeps in-progress casting/payment passes at their normal BC weight.
pub const IDLE_PASS_WEIGHTING_REVISION: u32 = 2;

/// Auxiliary self-supervised training targets attached to each [`Step4`].
#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct AuxTargets4 {
    /// Normalized remaining turns in the game (`((final_turn - turn) / 15.0).clamp(0.0, 1.0)`).
    pub turns_remaining_norm: f32,
    /// Seat's own final life total normalized (`(final_life / 40.0).clamp(-1.0, 1.0)`).
    pub my_final_life_norm: f32,
    /// Minimum final life total among other seats, including teammates
    /// (`(min_other_life / 40.0).clamp(-1.0, 1.0)`; historical label semantics).
    pub opp_min_final_life_norm: f32,
    /// Next-decision life delta for the acting seat (`(next_life - cur_life) / 10.0`).
    pub next_life_delta_norm: f32,
}

fn default_loss_weight() -> f32 {
    1.0
}

/// Per-input-step rollout provenance & constraint sidecar written by `eval4` (`P0-2`, `P0-3`).
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct RolloutStepSidecar4 {
    /// Action provenance (`Net | House | Forced | Search`, `temperature`, `K`, `behavior_logprob`, `has_logprob`).
    pub provenance: StepProvenance4,
    /// Exact rollout `free_mask` (`1` = free Bernoulli decision, `0` = forced/forbidden).
    #[serde(default)]
    pub free_mask: Vec<u8>,
    /// Exact rollout `pick_sequence` visit order.
    #[serde(default)]
    pub pick_sequence: Vec<u16>,
}

/// One v4 trajectory step (`Step4`) for supervised imitation, auxiliary heads,
/// or recurrent RL (`train4.py` / `train_rl4.py`).
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Step4 {
    /// Monotonic game identifier within a converted dataset.
    pub game_id: u32,
    /// Acting (or observing) seat (`0..8`).
    pub seat: u8,
    /// Monotonic step index for `(game_id, seat)`.
    pub step_cursor: u32,
    /// Turn number when this frame occurred.
    pub turn: u16,
    /// Whether this frame is an active decision (`true`) or a passive
    /// observation update (`false`).
    pub is_decision: bool,
    /// Per-step policy loss weight (`1.0` normal, mode-specific ordinary idle-pass
    /// discounts, `0.0` passive or ineligible supervision, `P0-4`).
    #[serde(default = "default_loss_weight")]
    pub loss_weight: f32,
    /// Whether this is the terminal step of `(game_id, seat)` (`P0-5`).
    #[serde(default)]
    pub is_terminal: bool,
    /// Encoded v4 state (`entity_feat`, `seat_feat`, `global_feat`, `edges`).
    pub state: EncodedState4,
    /// Policy head kind (`0..4`).
    pub head: u8,
    /// `PendingKind4` (`0..28`).
    pub pending_kind: u8,
    /// Candidate numeric features (`num_candidates x 48`).
    pub cand_feat: Vec<Vec<f32>>,
    /// Candidate referenced entity index (`-1` if none).
    pub cand_entity_idx: Vec<i16>,
    /// Candidate referenced relative seat index (`-1` if none).
    pub cand_rel_seat_idx: Vec<i8>,
    /// Candidate referenced `CardIndex` (`0` if none).
    pub cand_card_id: Vec<u32>,
    /// Named face, or 255 to use the referenced entity's rules face.
    pub cand_face_id: Vec<u8>,
    /// Candidate referenced ability index (`-1` if none).
    pub cand_ability_idx: Vec<i64>,
    /// Complete public semantic references for each action candidate.
    pub cand_rule_refs: Vec<Vec<crate::cardstructure::refs::RuleRef>>,
    /// Legal `(blocker_entity_idx, attacker_entity_idx)` pairs for `PairPick`.
    pub pairs: Vec<[i16; 2]>,
    /// Ground-truth teacher or rollout action target.
    pub target: TeacherTarget4,
    /// Terminal side utility in `[0.0, 1.0]` (`side_utility`).
    pub outcome: f32,
    /// Auxiliary self-supervised targets.
    pub aux: AuxTargets4,
    /// Action provenance (`Net | House | Forced | Search`, `temperature`, `K`, `behavior_logprob`).
    pub provenance: StepProvenance4,
    /// Separate verified BC advice; absent from ordinary conversion byte output.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub shadow_label: Option<shadow_bc::ShadowLabelProvenance4>,
    /// Executed external LLM target; never neural behavior probabilities.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub llm_label: Option<llm_bc::LlmLabelProvenance4>,
}

/// Summary returned by [`extract_game4`].
#[derive(Clone, Debug, Default)]
pub struct ExtractedGame4 {
    /// Extracted trajectory steps across all seats.
    pub steps: Vec<Step4>,
    /// Per-`PendingKind4` coverage counts observed in this game.
    pub coverage: CoverageMatrix4,
    /// Decisions skipped because `encode_teacher_action4` returned `None`.
    pub unencoded_decisions: u64,
}

/// Replays a single JSONL game `record` and extracts v4 trajectory steps
/// (`Step4`) across all seats.
///
/// Collecting diagnostic/reference API. Production conversion uses
/// [`stream_game4`] for bounded frame memory and a full recorded-hash audit.
///
/// # Errors
/// When the record header is missing/invalid, the version does not match, or
/// an input in the record is rejected by the engine.
pub fn extract_game4(
    record: &[u8],
    game_id: u32,
    include_passive_obs: bool,
) -> Result<ExtractedGame4, String> {
    extract_game4_with_sidecar(record, game_id, include_passive_obs, None)
}

/// Replays a single JSONL game `record` and extracts v4 trajectory steps
/// (`Step4`) across all seats, optionally restoring lossless rollout provenance
/// and `free_mask` from `sidecar` (`P0-2`, `P0-3`, `P0-4`, `P0-5`).
///
/// # Errors
/// When the record header is missing/invalid, the version does not match, or
/// an input in the record is rejected by the engine.
pub fn extract_game4_with_sidecar(
    record: &[u8],
    game_id: u32,
    include_passive_obs: bool,
    sidecar: Option<&[RolloutStepSidecar4]>,
) -> Result<ExtractedGame4, String> {
    let mut raw_steps = Vec::new();
    let summary = replay::replay_frames(
        record,
        game_id,
        include_passive_obs,
        sidecar,
        |step, life| {
            raw_steps.push((step, life));
            Ok(())
        },
    )?;
    let mut next_lives = summary.targets.final_lives;
    let mut seen = [false; MAX_SEATS];
    let mut steps = Vec::with_capacity(raw_steps.len());
    for (mut step, current) in raw_steps.into_iter().rev() {
        let seat = step.seat as usize;
        summary
            .targets
            .apply(&mut step, current, next_lives[seat], !seen[seat]);
        seen[seat] = true;
        next_lives[seat] = current;
        steps.push(step);
    }
    steps.reverse();
    Ok(ExtractedGame4 {
        steps,
        coverage: summary.coverage,
        unencoded_decisions: summary.unencoded_decisions,
    })
}

mod idle_pass;
pub mod llm_bc;
mod replay;
pub mod research;
pub mod shadow_bc;
mod streaming;
pub use streaming::{
    ProducerEncoder, StreamedGame4, stream_game4, stream_game4_live_bc, stream_game4_llm_bc,
    stream_game4_shadow_bc, stream_game4_with_encoder, stream_game4_with_encoder_and_producer,
};
use streaming::{ReplaySummary, Targets};

#[cfg(test)]
mod tests;
