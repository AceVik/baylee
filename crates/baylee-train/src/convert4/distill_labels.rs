//! The whole net's distillation labels as `eval4 --distill-labels` keeps them
//! in `games.jsonl` (`distill_sidecar`) and `convert4 --bundle-distill` reads
//! them (`bundle_distillation_v1`; `netplay4::distill` makes them).

use baylee_engine::choice::PlayerAction;
use serde::{Deserialize, Serialize};
use sha2::{Digest as _, Sha256};

use crate::features4::{EncodedCandidates4, TeacherTarget4};

/// One decision's labels.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct DistillLabel4 {
    /// `priority`, `single`, `subset`, `pair` or `scalar`.
    pub kind: String,
    /// The question's `PendingKind4`.
    pub pending_kind: u8,
    /// The menu the targets index ([`menu_signature`]): a converter checks it
    /// against the menu it rebuilds.
    pub menu_sha256: String,
    /// `R`: the tactical layer's answer under the label groups.
    pub rules: TeacherTarget4,
    /// `R` as an action.
    pub rules_action: Option<PlayerAction>,
    /// `N`: the net's raw greedy answer.
    pub net: TeacherTarget4,
    /// `N` as an action.
    pub net_action: Option<PlayerAction>,
    /// The answer this decision's policy produced (before a host's repair).
    pub executed: TeacherTarget4,
    /// The groups that changed something on the way to `R`.
    pub fired: Vec<String>,
    /// Whether `N` and `R` are different answers (by action where both decode).
    pub n_ne_r: bool,
}

/// One step of the net's seats, in `rollout_sidecar`'s order.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct DistillStep4 {
    /// The acting seat.
    pub seat: u8,
    /// The table's decision clock before the step.
    pub decision_seq: u64,
    /// The record's byte offset of the step's input.
    pub record_byte_offset: usize,
    /// The labels, when the policy answered (none for forced answers and
    /// fallbacks).
    pub label: Option<DistillLabel4>,
}

/// The menu a target indexes: the head, each candidate's card and ability
/// and each pair, hashed. A converter that rebuilds the question computes
/// the same from its frame.
#[must_use]
pub fn menu_signature(
    head: u8,
    card_ids: &[u32],
    ability_idx: &[i64],
    pairs: &[[i16; 2]],
) -> String {
    let mut hasher = Sha256::new();
    hasher.update([head]);
    hasher.update((card_ids.len() as u64).to_le_bytes());
    for (card, ability) in card_ids.iter().zip(ability_idx) {
        hasher.update(card.to_le_bytes());
        hasher.update(ability.to_le_bytes());
    }
    hasher.update((pairs.len() as u64).to_le_bytes());
    for [b, a] in pairs {
        hasher.update(b.to_le_bytes());
        hasher.update(a.to_le_bytes());
    }
    hasher
        .finalize()
        .iter()
        .fold(String::new(), |mut out, byte| {
            use std::fmt::Write as _;
            let _ = write!(out, "{byte:02x}");
            out
        })
}

/// The menu signature of a question's candidates.
#[must_use]
pub fn menu_of(cands: &EncodedCandidates4) -> String {
    let card_ids: Vec<u32> = cands.candidates.iter().map(|c| c.card_id).collect();
    let ability_idx: Vec<i64> = cands.candidates.iter().map(|c| c.ability_idx).collect();
    menu_signature(cands.head as u8, &card_ids, &ability_idx, &cands.pairs)
}
