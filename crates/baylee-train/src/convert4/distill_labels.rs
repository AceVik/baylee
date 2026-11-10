//! The whole net's distillation labels as `eval4 --distill-labels` keeps them
//! in `games.jsonl` (`distill_sidecar`) and `convert4 --bundle-distill` reads
//! them (`bundle_distillation_v1`; `netplay4::distill` makes them).

use baylee_engine::choice::PlayerAction;
use serde::{Deserialize, Serialize};
use sha2::{Digest as _, Sha256};

use super::NetAnswer4;
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
    /// At priority: what the decision is ([`PrioritySubtype4::of`]).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub subtype: Option<PrioritySubtype4>,
}

/// What a recorded seat's decision frame of a distillation dataset carries
/// beside its target: the block `convert4 --bundle-distill` appends to its
/// BST4 (`baylee_train_ops::tactician_labels::distill_block`), and where the
/// frame is labelled, what its line in the dataset's JSON steps holds
/// (`bundle-distill/game-NNNNNN.jsonl`).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct DistillFrame4 {
    /// The block's subtype code ([`PrioritySubtype4::code`], 255 for none).
    pub subtype_code: u8,
    /// The block's mask of the groups that fired (bit i: the dataset's
    /// `bundle_distillation.groups[i]`).
    pub fired_mask: u64,
    /// The label, where the policy answered; none at a fallback, which keeps
    /// its frame at weight zero.
    pub label: Option<DistillFrameLabel4>,
}

/// A labelled frame's line in the JSON steps.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct DistillFrameLabel4 {
    /// The label's kind: `priority`, `single`, `subset`, `pair` or `scalar`.
    pub label_kind: String,
    /// Whether the net's own answer `N` differs from `R` here (BST4 flags bit
    /// 7).
    pub n_ne_r: bool,
    /// `N`.
    pub net_answer: NetAnswer4,
    /// At priority: what the decision is.
    #[serde(
        rename = "distill_subtype",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub subtype: Option<PrioritySubtype4>,
    /// The rule groups that changed something on the way to `R`.
    pub fired: Vec<String>,
}

/// What a priority decision is, read from the layer's answer `R`, the net's
/// answer `N` and the position. The same rule as the priority probe's
/// decision kinds (`tools/trainer/tactician_v3/records.py`,
/// `decision_kind`). Its code is the distillation block's subtype byte, the
/// index of its name in the dataset's `bundle_distillation.subtypes`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PrioritySubtype4 {
    /// `R` casts a spell in the seat's own turn, where `N` does not pass.
    Cast = 0,
    /// `R` passes.
    Pass = 1,
    /// `R` casts or activates in the other side's turn.
    InstantTheirTurn = 2,
    /// `R` casts or activates in the seat's own turn, where `N` passes.
    Spend = 3,
    /// The seat owes a cost.
    Pay = 4,
    /// `R` activates a mana ability.
    Mana = 5,
    /// `R` activates an ability in the seat's own turn, where `N` does not
    /// pass.
    Activate = 6,
    /// `R` plays a land.
    Land = 7,
    /// Anything else: a special action, an answer that does not decode.
    Other = 8,
}

impl PrioritySubtype4 {
    /// Every subtype, in code order.
    pub const ALL: [Self; 9] = [
        Self::Cast,
        Self::Pass,
        Self::InstantTheirTurn,
        Self::Spend,
        Self::Pay,
        Self::Mana,
        Self::Activate,
        Self::Land,
        Self::Other,
    ];

    /// The subtype of a priority decision: `my_turn` whether the deciding
    /// seat is the active player, `paying` whether it owes a cost, `rules`
    /// and `net` the answers `R` and `N` as actions.
    #[must_use]
    pub const fn of(
        my_turn: bool,
        paying: bool,
        rules: Option<&PlayerAction>,
        net: Option<&PlayerAction>,
    ) -> Self {
        let acts = matches!(
            rules,
            Some(PlayerAction::CastSpell { .. } | PlayerAction::ActivateAbility { .. })
        );
        if paying {
            Self::Pay
        } else if matches!(rules, Some(PlayerAction::PassPriority)) {
            Self::Pass
        } else if acts && !my_turn {
            Self::InstantTheirTurn
        } else if acts && matches!(net, Some(PlayerAction::PassPriority)) {
            Self::Spend
        } else {
            match rules {
                Some(PlayerAction::CastSpell { .. }) => Self::Cast,
                Some(PlayerAction::ActivateAbility { .. }) => Self::Activate,
                Some(PlayerAction::ActivateManaAbility { .. }) => Self::Mana,
                Some(PlayerAction::PlayLand { .. }) => Self::Land,
                _ => Self::Other,
            }
        }
    }

    /// Its code: the distillation block's subtype byte.
    #[must_use]
    pub const fn code(self) -> u8 {
        self as u8
    }

    /// Its name, as JSON writes it.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::Cast => "cast",
            Self::Pass => "pass",
            Self::InstantTheirTurn => "instant_their_turn",
            Self::Spend => "spend",
            Self::Mana => "mana",
            Self::Pay => "pay",
            Self::Activate => "activate",
            Self::Land => "land",
            Self::Other => "other",
        }
    }
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

#[cfg(test)]
mod tests {
    use super::*;
    use baylee_core::ids::ObjectId;

    #[test]
    fn a_priority_subtype_is_the_probes_decision_kind() {
        use PrioritySubtype4 as S;
        let card = ObjectId::new(7, 0);
        let pass = PlayerAction::PassPriority;
        let cast = PlayerAction::CastSpell { card };
        let activate = PlayerAction::ActivateAbility {
            source: card,
            ability_index: 0,
        };
        let mana = PlayerAction::ActivateManaAbility { source: card };
        let land = PlayerAction::PlayLand { card };
        let of = |my_turn, paying, rules: Option<&PlayerAction>, net: Option<&PlayerAction>| {
            PrioritySubtype4::of(my_turn, paying, rules, net)
        };
        // Paying a cost comes first, whatever R does.
        assert_eq!(of(true, true, Some(&pass), Some(&pass)), S::Pay);
        assert_eq!(of(false, true, Some(&cast), None), S::Pay);
        // Then R passing, on either side's turn.
        assert_eq!(of(false, false, Some(&pass), Some(&cast)), S::Pass);
        // Casting or activating on their turn.
        assert_eq!(
            of(false, false, Some(&cast), Some(&pass)),
            S::InstantTheirTurn
        );
        assert_eq!(
            of(false, false, Some(&activate), Some(&cast)),
            S::InstantTheirTurn
        );
        // On my turn, where N passes: spending.
        assert_eq!(of(true, false, Some(&cast), Some(&pass)), S::Spend);
        assert_eq!(of(true, false, Some(&activate), Some(&pass)), S::Spend);
        // On my turn, where N acts too: by R's action.
        assert_eq!(of(true, false, Some(&cast), Some(&land)), S::Cast);
        assert_eq!(of(true, false, Some(&activate), Some(&cast)), S::Activate);
        assert_eq!(of(true, false, Some(&mana), Some(&pass)), S::Mana);
        assert_eq!(of(false, false, Some(&land), Some(&pass)), S::Land);
        assert_eq!(of(true, false, None, Some(&pass)), S::Other);
        // Codes 0–8 in the trainer's order, and the names JSON writes.
        let names = PrioritySubtype4::ALL.map(PrioritySubtype4::name);
        assert_eq!(
            names,
            [
                "cast",
                "pass",
                "instant_their_turn",
                "spend",
                "pay",
                "mana",
                "activate",
                "land",
                "other"
            ]
        );
        for (code, subtype) in (0..).zip(PrioritySubtype4::ALL) {
            assert_eq!(subtype.code(), code);
            assert_eq!(
                serde_json::to_value(subtype).unwrap(),
                serde_json::json!(subtype.name())
            );
        }
    }
}
