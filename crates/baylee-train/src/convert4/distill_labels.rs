//! The whole net's distillation labels as `eval4 --distill-labels` keeps them
//! in `games.jsonl` (`distill_sidecar`) and `convert4 --bundle-distill` reads
//! them (`bundle_distillation_v1`; `netplay4::distill` makes them).

use baylee_engine::choice::PlayerAction;
use serde::{Deserialize, Serialize};
use sha2::{Digest as _, Sha256};

use super::NetAnswer4;
use crate::features4::{EncodedCandidates4, PendingKind4, TeacherTarget4};

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
    /// What the decision is, where the distillation scope reads it: every
    /// priority label ([`DistillSubtype4::of`]), and a target question where
    /// `side` or `aim` changed `R` ([`DistillSubtype4::of_single`]).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub subtype: Option<DistillSubtype4>,
    /// What `R` is worth beside `N` at this decision, where a measuring
    /// search had the layer for a rival (`eval4 --search-rival distill`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub probe: Option<DistillProbe4>,
}

/// The contract of a probe ([`DistillProbe4`]).
pub const DISTILL_PROBE_CONTRACT: &str = "distill_probe_v1";

/// A decision's `N` and `R` played on in the same fair worlds by the net
/// (`eval4 --search 2,P,H --search-fair --search-rival distill
/// --search-select measure`): `N` was played at the table, `R` only in the
/// worlds. The fork advantage of `R` in world `p` is
/// `worlds[p][1] - worlds[p][0]`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct DistillProbe4 {
    /// `R`'s place in the menu the label's targets index, as the search
    /// ranked it (the label's `rules.single_idx`).
    pub rival_index: u16,
    /// `N`'s place in the same menu.
    pub own_index: u16,
    /// Each complete round's scores, `[N, R]`: the terminal side utility,
    /// else the value head on the seat's last observation of the rollout.
    pub worlds: Vec<[f32; 2]>,
    /// How each of those rollouts ended (`netplay4::search::LeafEnd4` by
    /// name: `terminal`, `boundary`, `house_stop`, ...), `[N, R]`.
    pub ends: Vec<[String; 2]>,
    /// Rollouts run, those of an incomplete round included.
    pub rollouts: u64,
    /// House answers played in the rollouts (none under `--search-house-free`).
    pub house_answers: u64,
}

impl DistillProbe4 {
    /// `R`'s mean fork advantage over `N` and its standard error over the
    /// worlds (`None` with fewer than two); `None` without a world.
    #[must_use]
    pub fn advantage(&self) -> Option<(f64, Option<f64>)> {
        if self.worlds.is_empty() {
            return None;
        }
        let diffs: Vec<f64> = self
            .worlds
            .iter()
            .map(|[n, r]| f64::from(*r) - f64::from(*n))
            .collect();
        let count = f64::from(u32::try_from(diffs.len()).unwrap_or(u32::MAX));
        let mean = diffs.iter().sum::<f64>() / count;
        let se = (diffs.len() >= 2).then(|| {
            let variance = diffs.iter().map(|d| (d - mean).powi(2)).sum::<f64>() / (count - 1.0);
            (variance / count).sqrt()
        });
        Some((mean, se))
    }

    /// `R`'s mean fork advantage over `N` in each half of the worlds, the
    /// even rounds and the odd ones (`None` for an empty half): a lead that
    /// holds in both halves is not one world's luck.
    #[must_use]
    pub fn halves(&self) -> [Option<f64>; 2] {
        let half = |parity: usize| {
            let diffs: Vec<f64> = self
                .worlds
                .iter()
                .enumerate()
                .filter(|(p, _)| p % 2 == parity)
                .map(|(_, [n, r])| f64::from(*r) - f64::from(*n))
                .collect();
            (!diffs.is_empty()).then(|| {
                diffs.iter().sum::<f64>()
                    / f64::from(u32::try_from(diffs.len()).unwrap_or(u32::MAX))
            })
        };
        [half(0), half(1)]
    }
}

/// What a recorded seat's decision frame of a distillation dataset carries
/// beside its target: the block `convert4 --bundle-distill` appends to its
/// BST4 (`baylee_train_ops::tactician_labels::distill_block`), and where the
/// frame is labelled, what its line in the dataset's JSON steps holds
/// (`bundle-distill/game-NNNNNN.jsonl`).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct DistillFrame4 {
    /// The block's subtype code ([`DistillSubtype4::code`], 255 for none).
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
    /// What the decision is ([`DistillLabel4::subtype`]).
    #[serde(
        rename = "distill_subtype",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub subtype: Option<DistillSubtype4>,
    /// The rule groups that changed something on the way to `R`.
    pub fired: Vec<String>,
    /// The single choice the seat played here (the record's answer, the
    /// frame's target before `R` replaced it). Not in the JSON steps: the
    /// coarse-class sidecar carries it (`p_idx`).
    #[serde(skip)]
    pub played: Option<u16>,
    /// `N` against `R` in the same fair worlds, where the run probed this
    /// decision ([`DistillLabel4::probe`]); the probe sidecar carries its
    /// summary (`bundle-distill/probe.*`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub probe: Option<DistillProbe4>,
}

/// What a labelled decision is, where the distillation scope reads it. At
/// priority it is read from the layer's answer `R`, the net's answer `N` and
/// the position, by the same rule as the priority probe's decision kinds
/// (`tools/trainer/tactician_v3/records.py`, `decision_kind`); at a
/// single-choice target question it is `target` where `side` or `aim`
/// changed `R` (a counterspell or removal aimed at the wrong side or the
/// wrong object), and none otherwise. Its code is the distillation block's
/// subtype byte, the index of its name in the dataset's
/// `bundle_distillation.subtypes`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DistillSubtype4 {
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
    /// Anything else at priority: a special action, an answer that does not
    /// decode.
    Other = 8,
    /// A single-choice target question where `side` or `aim` changed `R`.
    Target = 9,
}

/// The rule groups whose firing at a target question makes it
/// [`DistillSubtype4::Target`].
pub const TARGET_GROUPS: [&str; 2] = ["side", "aim"];

/// The tactical layer's rule groups in the order of their bit in a
/// distillation dataset's fired mask (`bundle_distillation.groups`): the
/// table of `netplay4::tactics::rules`, which a test there holds equal, kept
/// here so that a converter built without the net reads it too.
pub const FIRED_GROUPS: [&str; 22] = [
    "void", "side", "mass", "gift", "race", "trip", "aim", "spend", "react", "trade", "press",
    "walker", "chump", "keep", "land", "ping", "curve", "wall", "burn", "trick", "counter",
    "defer",
];

impl DistillSubtype4 {
    /// Every subtype, in code order.
    pub const ALL: [Self; 10] = [
        Self::Cast,
        Self::Pass,
        Self::InstantTheirTurn,
        Self::Spend,
        Self::Pay,
        Self::Mana,
        Self::Activate,
        Self::Land,
        Self::Other,
        Self::Target,
    ];

    /// The subtype of a single-choice label other than priority: `target` at
    /// a target question (`PendingKind4::ChooseTargets`) where a group of
    /// [`TARGET_GROUPS`] fired, none anywhere else.
    #[must_use]
    pub fn of_single(pending_kind: u8, fired: &[String]) -> Option<Self> {
        (pending_kind == PendingKind4::ChooseTargets as u8
            && fired
                .iter()
                .any(|group| TARGET_GROUPS.contains(&group.as_str())))
        .then_some(Self::Target)
    }

    /// Whether a label's subtype is the one its kind, question and groups
    /// give it: a priority subtype at priority, [`Self::of_single`] at
    /// another single choice, none at subsets, pairs and numbers.
    #[must_use]
    pub fn fits(label: &DistillLabel4) -> bool {
        match label.kind.as_str() {
            "priority" => label.subtype.is_some_and(|s| s != Self::Target),
            "single" => label.subtype == Self::of_single(label.pending_kind, &label.fired),
            _ => label.subtype.is_none(),
        }
    }

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
            Self::Target => "target",
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
        use DistillSubtype4 as S;
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
            DistillSubtype4::of(my_turn, paying, rules, net)
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
        let names = DistillSubtype4::ALL.map(DistillSubtype4::name);
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
                "other",
                "target"
            ]
        );
        for (code, subtype) in (0..).zip(DistillSubtype4::ALL) {
            assert_eq!(subtype.code(), code);
            assert_eq!(
                serde_json::to_value(subtype).unwrap(),
                serde_json::json!(subtype.name())
            );
        }
    }

    #[test]
    fn a_target_question_is_target_where_side_or_aim_fired_and_nothing_else() {
        let target = PendingKind4::ChooseTargets as u8;
        let groups = |names: &[&str]| names.iter().map(|n| (*n).to_owned()).collect::<Vec<_>>();
        for fired in [&["side"][..], &["aim"], &["void", "aim"]] {
            assert_eq!(
                DistillSubtype4::of_single(target, &groups(fired)),
                Some(DistillSubtype4::Target)
            );
        }
        assert_eq!(DistillSubtype4::of_single(target, &groups(&["void"])), None);
        assert_eq!(DistillSubtype4::of_single(target, &[]), None);
        let mode = PendingKind4::ChooseCastMode as u8;
        assert_eq!(DistillSubtype4::of_single(mode, &groups(&["side"])), None);
        // What each kind of label may carry.
        let label = |kind: &str, pending_kind: u8, fired: &[&str], subtype| DistillLabel4 {
            kind: kind.into(),
            pending_kind,
            menu_sha256: String::new(),
            rules: TeacherTarget4::default(),
            rules_action: None,
            net: TeacherTarget4::default(),
            net_action: None,
            executed: TeacherTarget4::default(),
            fired: groups(fired),
            n_ne_r: true,
            subtype,
            probe: None,
        };
        let s = |subtype| Some(subtype);
        for (fits, l) in [
            (true, label("priority", 0, &[], s(DistillSubtype4::Pass))),
            (false, label("priority", 0, &[], None)),
            (
                false,
                label("priority", 0, &["side"], s(DistillSubtype4::Target)),
            ),
            (
                true,
                label("single", target, &["side"], s(DistillSubtype4::Target)),
            ),
            (false, label("single", target, &["side"], None)),
            (true, label("single", target, &["void"], None)),
            (
                false,
                label("single", target, &["void"], s(DistillSubtype4::Target)),
            ),
            (false, label("single", mode, &[], s(DistillSubtype4::Cast))),
            (true, label("subset", 5, &["side"], None)),
            (false, label("pair", 6, &[], s(DistillSubtype4::Target))),
        ] {
            assert_eq!(DistillSubtype4::fits(&l), fits, "{l:?}");
        }
    }
}
