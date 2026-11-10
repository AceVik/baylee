//! Bundle distillation (`--bundle-distill`, `bundle_distillation_v1`): the
//! tactical layer's answers `R`, recorded beside the games by `eval4
//! --distill-labels` (`distill_sidecar`), become the policy targets at every
//! labelled decision of the recorded seats, whoever played there (the layer,
//! or the net as a student under `DAgger`). The net's own answer `N` stays with
//! each frame (`net_answer`, `n_ne_r`, BST4 flags bit 7) so a trainer can
//! report and select on `N ≠ R`. Every other frame keeps its place for memory
//! and value at policy weight zero; value targets are the outcomes as usual.
//!
//! A label binds to its frame by the record's seat input it answers, and is
//! checked before it is used: the seat, the menu the targets index (head,
//! candidates' cards and abilities, pairs) and the executed answer must be the
//! record's. Any mismatch rejects the game.

use std::collections::BTreeMap;

use anyhow::{Context as _, ensure};
use baylee_train::convert4::distill_labels::{DistillLabel4, DistillStep4, menu_signature};
use baylee_train::convert4::{NetAnswer4, Step4};
use baylee_train::features4::{HeadKind4, SourceKind4, TeacherTarget4};
use serde_json::{Value, json};

pub(super) use baylee_train_ops::tactician_labels::DISTILL_CONTRACT;

/// The runs a distillation dataset is made of.
#[derive(Default)]
pub(super) struct Runs {
    sources: Vec<Value>,
}

impl Runs {
    /// Admits one run: it must declare the distillation contract.
    ///
    /// # Errors
    /// A run without `distill.contract == bundle_distillation_v1`.
    pub(super) fn admit(
        &mut self,
        run_dir: &std::path::Path,
        run_json: &Value,
    ) -> anyhow::Result<()> {
        baylee_train_ops::tactician_labels::admit_distillation(run_json)
            .with_context(|| format!("{}", run_dir.display()))?;
        self.sources.push(
            json!({"run": run_dir, "label_rules": run_json["distill"]["label_rules"],
            "players": run_json["distill"]["players"]}),
        );
        Ok(())
    }

    /// What `meta.json` says about the dataset (counts filled at the end).
    pub(super) fn metadata(&self, counts: &Counts) -> Value {
        json!({"contract": DISTILL_CONTRACT, "version": 1,
            "target": "the tactical layer's answer R at every labelled decision of the recorded seats",
            "net_answer": "N, the net's raw greedy answer, in JSON steps; BST4 flags bit 7 = N != R",
            "loss_weight": "1 at labelled decisions of the recorded seats, 0 elsewhere",
            "value_targets": "game outcomes, as usual",
            "sources": self.sources, "counts": counts})
    }
}

/// One game's labels, by the record's seat input.
#[derive(Clone, Debug)]
pub(super) struct GameLabels {
    steps: Vec<DistillStep4>,
}

impl GameLabels {
    /// The labels of one `games.jsonl` entry, aligned with its rollout sidecar.
    ///
    /// # Errors
    /// A game without `distill_sidecar`, or one not aligned with
    /// `rollout_sidecar`.
    pub(super) fn from_game(entry: &Value) -> anyhow::Result<Self> {
        let steps: Vec<DistillStep4> = serde_json::from_value(
            entry
                .get("distill_sidecar")
                .context("--bundle-distill needs each game's distill_sidecar")?
                .clone(),
        )?;
        ensure!(
            entry["rollout_sidecar"].as_array().map(Vec::len) == Some(steps.len()),
            "distill_sidecar is not aligned with rollout_sidecar"
        );
        Ok(Self { steps })
    }
}

/// What a conversion did with the labels.
#[derive(Clone, Debug, Default, serde::Serialize)]
pub(super) struct Counts {
    /// Decision frames given `R` as their target.
    pub labelled: u64,
    /// Of them, where `N` differs from `R`.
    pub n_ne_r: u64,
    /// Decision frames left at weight zero (another seat's, or a step without
    /// a label: a fallback).
    pub unlabelled: u64,
    /// Labelled frames by kind (`priority`, `single`, `subset`, `pair`, `scalar`).
    pub by_kind: BTreeMap<String, u64>,
    /// `N ≠ R` frames by kind.
    pub n_ne_r_by_kind: BTreeMap<String, u64>,
}

impl Counts {
    pub(super) fn add(&mut self, other: &Self) {
        self.labelled += other.labelled;
        self.n_ne_r += other.n_ne_r;
        self.unlabelled += other.unlabelled;
        for (kind, n) in &other.by_kind {
            *self.by_kind.entry(kind.clone()).or_default() += n;
        }
        for (kind, n) in &other.n_ne_r_by_kind {
            *self.n_ne_r_by_kind.entry(kind.clone()).or_default() += n;
        }
    }
}

/// One game's application of its labels to the emitted frames.
pub(super) struct Cursor<'a> {
    labels: &'a GameLabels,
    pub(super) counts: Counts,
}

/// Whether the record's executed answer is the one the label saw executed.
fn same_answer(head: HeadKind4, label: &TeacherTarget4, frame: &TeacherTarget4) -> bool {
    match head {
        HeadKind4::SingleChoice => label.single_idx == frame.single_idx,
        HeadKind4::SubsetPick | HeadKind4::PairPick => label.multi_mask == frame.multi_mask,
        HeadKind4::ScalarValue => (label.scalar_norm - frame.scalar_norm).abs() <= 1e-4,
    }
}

fn head_of(head: u8) -> Option<HeadKind4> {
    [
        HeadKind4::SingleChoice,
        HeadKind4::SubsetPick,
        HeadKind4::PairPick,
        HeadKind4::ScalarValue,
    ]
    .into_iter()
    .find(|kind| *kind as u8 == head)
}

/// `N` as the JSON steps carry it.
fn net_answer(head: HeadKind4, net: &TeacherTarget4) -> NetAnswer4 {
    match head {
        HeadKind4::SingleChoice => NetAnswer4 {
            single_idx: Some(net.single_idx),
            ..NetAnswer4::default()
        },
        HeadKind4::SubsetPick | HeadKind4::PairPick => NetAnswer4 {
            multi_mask: Some(net.multi_mask.clone()),
            ..NetAnswer4::default()
        },
        HeadKind4::ScalarValue => NetAnswer4 {
            scalar_norm: Some(net.scalar_norm),
            ..NetAnswer4::default()
        },
    }
}

impl<'a> Cursor<'a> {
    pub(super) fn new(labels: &'a GameLabels) -> Self {
        Self {
            labels,
            counts: Counts::default(),
        }
    }

    /// The label of this decision frame, checked against it.
    fn label_of(&self, step: &Step4) -> Result<Option<&'a DistillLabel4>, String> {
        let Some(index) = step.seat_input else {
            return Ok(None);
        };
        let entry = self
            .labels
            .steps
            .get(index as usize)
            .ok_or_else(|| format!("no distillation step for seat input {index}"))?;
        if entry.seat != step.seat {
            return Err(format!(
                "distillation step {index} is seat {}'s, the record's is seat {}",
                entry.seat, step.seat
            ));
        }
        let Some(label) = &entry.label else {
            return Ok(None);
        };
        let menu = menu_signature(
            step.head,
            &step.cand_card_id,
            &step.cand_ability_idx,
            &step.pairs,
        );
        if menu != label.menu_sha256 {
            return Err(format!(
                "distillation step {index}: the menu differs from the record's"
            ));
        }
        let head = head_of(step.head).ok_or_else(|| format!("unknown head {}", step.head))?;
        if !same_answer(head, &label.executed, &step.target) {
            return Err(format!(
                "distillation step {index}: the executed answer differs from the record's"
            ));
        }
        Ok(Some(label))
    }

    /// Makes `R` the target of a labelled decision frame, and every other
    /// decision frame weightless.
    ///
    /// # Errors
    /// A label that does not bind to its frame.
    pub(super) fn apply(&mut self, step: &mut Step4) -> Result<(), String> {
        if !step.is_decision {
            return Ok(());
        }
        let Some(label) = self.label_of(step)? else {
            step.loss_weight = 0.0;
            step.provenance.has_logprob = false;
            step.provenance.behavior_logprob = 0.0;
            self.counts.unlabelled += 1;
            return Ok(());
        };
        let head = head_of(step.head).ok_or_else(|| format!("unknown head {}", step.head))?;
        step.target = label.rules.clone();
        step.loss_weight = 1.0;
        step.provenance.source = SourceKind4::House;
        step.provenance.has_logprob = false;
        step.provenance.behavior_logprob = 0.0;
        step.provenance.temperature = 0.0;
        step.n_ne_r = Some(label.n_ne_r);
        step.net_answer = Some(net_answer(head, &label.net));
        self.counts.labelled += 1;
        *self.counts.by_kind.entry(label.kind.clone()).or_default() += 1;
        if label.n_ne_r {
            self.counts.n_ne_r += 1;
            *self
                .counts
                .n_ne_r_by_kind
                .entry(label.kind.clone())
                .or_default() += 1;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use baylee_train::features4::PendingKind4;

    /// A priority decision frame of `seat` answering seat input `input`, its
    /// executed answer `executed`.
    fn frame(seat: u8, input: Option<u32>, executed: u16) -> Step4 {
        Step4 {
            game_id: 1,
            seat,
            step_cursor: 0,
            turn: 3,
            is_decision: true,
            loss_weight: 1.0,
            is_terminal: false,
            state: baylee_train::features4::EncodedState4::default(),
            head: HeadKind4::SingleChoice as u8,
            pending_kind: PendingKind4::Priority as u8,
            cand_feat: vec![],
            cand_entity_idx: vec![],
            cand_rel_seat_idx: vec![],
            cand_card_id: vec![0, 11, 12],
            cand_face_id: vec![],
            cand_ability_idx: vec![-1, -1, 0],
            cand_rule_refs: vec![],
            pairs: vec![],
            target: TeacherTarget4 {
                single_idx: executed,
                ..TeacherTarget4::default()
            },
            outcome: 0.5,
            aux: baylee_train::convert4::AuxTargets4::default(),
            provenance: baylee_train::features4::StepProvenance4 {
                source: SourceKind4::Net,
                has_logprob: true,
                behavior_logprob: -0.5,
                ..baylee_train::features4::StepProvenance4::default()
            },
            shadow_label: None,
            llm_label: None,
            seat_input: input,
            n_ne_r: None,
            net_answer: None,
        }
    }

    fn label(rules: u16, net: u16, executed: u16) -> DistillLabel4 {
        let single = |i| TeacherTarget4 {
            single_idx: i,
            ..TeacherTarget4::default()
        };
        DistillLabel4 {
            kind: "priority".into(),
            pending_kind: PendingKind4::Priority as u8,
            menu_sha256: menu_signature(0, &[0, 11, 12], &[-1, -1, 0], &[]),
            rules: single(rules),
            rules_action: None,
            net: single(net),
            net_action: None,
            executed: single(executed),
            fired: vec!["spend".into()],
            n_ne_r: rules != net,
        }
    }

    fn game(labels: Vec<Option<DistillLabel4>>) -> GameLabels {
        let steps: Vec<DistillStep4> = labels
            .into_iter()
            .enumerate()
            .map(|(i, label)| DistillStep4 {
                seat: 0,
                decision_seq: i as u64,
                record_byte_offset: 100 * i,
                label,
            })
            .collect();
        let entry = json!({"rollout_sidecar": vec![json!({}); steps.len()],
            "distill_sidecar": steps});
        GameLabels::from_game(&entry).unwrap()
    }

    #[test]
    fn r_becomes_the_target_and_n_stays_with_the_frame() {
        // Step 0: the student played N (index 2), the layer says 1.
        let labels = game(vec![Some(label(1, 2, 2)), None]);
        let mut cursor = Cursor::new(&labels);
        let mut step = frame(0, Some(0), 2);
        cursor.apply(&mut step).unwrap();
        assert_eq!(step.target.single_idx, 1);
        assert_eq!(step.loss_weight, 1.0);
        assert_eq!(step.n_ne_r, Some(true));
        assert_eq!(step.net_answer.as_ref().and_then(|n| n.single_idx), Some(2));
        assert!(!step.provenance.has_logprob);
        assert_eq!(step.provenance.behavior_logprob, 0.0);
        // A step without a label (a fallback) and another seat's decision
        // keep their frames at weight zero; passive frames stay as they are.
        let mut fallback = frame(0, Some(1), 0);
        cursor.apply(&mut fallback).unwrap();
        assert_eq!((fallback.loss_weight, fallback.n_ne_r), (0.0, None));
        let mut house = frame(1, None, 0);
        cursor.apply(&mut house).unwrap();
        assert_eq!(house.loss_weight, 0.0);
        let mut passive = frame(0, None, 0);
        passive.is_decision = false;
        passive.loss_weight = 0.25;
        cursor.apply(&mut passive).unwrap();
        assert_eq!(passive.loss_weight, 0.25);
        assert_eq!(
            (
                cursor.counts.labelled,
                cursor.counts.n_ne_r,
                cursor.counts.unlabelled
            ),
            (1, 1, 2)
        );
    }

    #[test]
    fn a_label_that_does_not_bind_to_its_frame_rejects_the_game() {
        let labels = game(vec![Some(label(1, 2, 2))]);
        // Another seat's input.
        assert!(
            Cursor::new(&labels)
                .apply(&mut frame(1, Some(0), 2))
                .is_err()
        );
        // Another executed answer than the label saw.
        assert!(
            Cursor::new(&labels)
                .apply(&mut frame(0, Some(0), 1))
                .is_err()
        );
        // Another menu.
        let mut other = frame(0, Some(0), 2);
        other.cand_card_id[1] = 99;
        assert!(Cursor::new(&labels).apply(&mut other).is_err());
        // An input the sidecar does not have.
        assert!(
            Cursor::new(&labels)
                .apply(&mut frame(0, Some(5), 2))
                .is_err()
        );
        // A sidecar out of step with the rollout sidecar.
        let entry = json!({"rollout_sidecar": [{}, {}], "distill_sidecar": []});
        assert!(GameLabels::from_game(&entry).is_err());
        assert!(GameLabels::from_game(&json!({"rollout_sidecar": []})).is_err());
    }

    #[test]
    fn only_a_declared_run_is_admitted_and_the_meta_names_the_contract() {
        let mut runs = Runs::default();
        let dir = std::path::Path::new("runs/r");
        assert!(
            runs.admit(dir, &json!({"labels": "tactician_imitation_v1"}))
                .is_err()
        );
        runs.admit(
            dir,
            &json!({"distill": {"contract": DISTILL_CONTRACT, "label_rules": "mass"}}),
        )
        .unwrap();
        let meta = runs.metadata(&Counts::default());
        assert_eq!(meta["contract"], DISTILL_CONTRACT);
        assert_eq!(meta["sources"][0]["label_rules"], "mass");
    }
}
