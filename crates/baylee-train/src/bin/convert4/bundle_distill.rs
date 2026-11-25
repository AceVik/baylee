//! Bundle distillation (`--bundle-distill`, `bundle_distillation_v1`): the
//! tactical layer's answers `R`, recorded beside the games by `eval4
//! --distill-labels` (`distill_sidecar`), become the policy targets at every
//! labelled decision of the recorded seats, whoever played there (the layer,
//! or the net as a student under `DAgger`). Every other frame keeps its place
//! for memory and value at policy weight zero; value targets are the outcomes
//! as usual.
//!
//! Beside `R`, each labelled frame keeps what a trainer selects and weights
//! by: the net's own answer `N` and whether it differs (`n_ne_r`), at
//! priority the decision's subtype (cast, pass, instant on their turn, spend,
//! pay, mana, activate, land, other), and the rule groups that changed
//! something on the way to `R` (`fired`). The dataset's JSON steps
//! (`bundle-distill/game-NNNNNN.jsonl`) hold all of it, one line per labelled
//! frame. In the BST4, flags bit 7 is `N ≠ R`, and every decision frame of a
//! recorded seat ends in a distillation block after its free mask
//! (`baylee_train_ops::tactician_labels::distill_block`): the subtype's code
//! (255 where none: another kind of label, or a fallback, which has no label)
//! and the mask of the groups that fired. `meta.json` names the codes
//! (`subtypes`) and the bits (`groups`).
//!
//! A label binds to its frame by the record's seat input it answers, and is
//! checked before it is used: the seat, the menu the targets index (head,
//! candidates' cards and abilities, pairs) and the executed answer must be the
//! record's, and a priority label must name its subtype. Any mismatch rejects
//! the game.
//!
//! A run that probed the layer's answers (`eval4 --search-rival distill`,
//! `distill_probe_v1`) keeps each probe with its label: the JSON steps line
//! holds it whole, and the probe sidecar (`probes.rs`,
//! `bundle-distill/probe.*`) a summary per probed frame. A probe must belong
//! to its label (a priority decision with `N ≠ R`, its rival `R`'s menu
//! entry, a score for both in every world), or the game is rejected.

use std::collections::BTreeMap;
use std::fs::{self, File, OpenOptions};
use std::io::{BufWriter, Write as _};
use std::path::{Path, PathBuf};

use anyhow::{Context as _, ensure};
use baylee_train::convert4::distill_labels::{
    DistillFrame4, DistillFrameLabel4, DistillLabel4, DistillProbe4, DistillStep4, DistillSubtype4,
    FIRED_GROUPS, menu_signature,
};
use baylee_train::convert4::{NetAnswer4, Step4};
use baylee_train::features4::{HeadKind4, SourceKind4, TeacherTarget4};
use baylee_train_ops::tactician_labels::NO_SUBTYPE;
use serde_json::{Value, json};

pub(super) use baylee_train_ops::tactician_labels::{DISTILL_CONTRACT, HOUSE_DISTILL_CONTRACT};

/// Where a game's JSON steps go, under the dataset.
pub(super) fn steps_path(game_id: u32) -> String {
    format!("bundle-distill/game-{game_id:06}.jsonl")
}

/// The decks a distillation dataset may have been played with: a manifest's
/// train split (`--train-manifest`).
pub(super) struct TrainDecks {
    path: PathBuf,
    sha256: String,
    splits: BTreeMap<String, String>,
}

impl TrainDecks {
    /// The manifest at `path`.
    ///
    /// # Errors
    /// An unreadable manifest, or one without a `decks` list of `id` and
    /// `split`.
    pub(super) fn load(path: &Path) -> anyhow::Result<Self> {
        let bytes = fs::read(path)
            .with_context(|| format!("reading --train-manifest {}", path.display()))?;
        Self::parse(path.to_path_buf(), &bytes)
    }

    fn parse(path: PathBuf, bytes: &[u8]) -> anyhow::Result<Self> {
        let manifest: Value = serde_json::from_slice(bytes)?;
        let splits = manifest["decks"]
            .as_array()
            .with_context(|| format!("{}: no decks list", path.display()))?
            .iter()
            .map(|deck| {
                Some((
                    deck["id"].as_str()?.to_owned(),
                    deck["split"].as_str()?.to_owned(),
                ))
            })
            .collect::<Option<BTreeMap<_, _>>>()
            .with_context(|| format!("{}: a deck without id or split", path.display()))?;
        Ok(Self {
            sha256: super::shadow::sha(bytes),
            path,
            splits,
        })
    }

    /// The manifest, as a dataset names it.
    #[cfg(feature = "onnx")]
    pub(super) fn describe(&self) -> Value {
        json!({"manifest": self.path, "manifest_sha256": self.sha256})
    }

    /// Every deck a run can seat (its `run.json` decks, from which each
    /// table's decks are taken), each checked to be in the train split.
    ///
    /// # Errors
    /// A run that names no decks, or any deck the manifest puts in another
    /// split or does not list, named with its split.
    pub(super) fn seated(&self, run_dir: &Path, run_json: &Value) -> anyhow::Result<Vec<String>> {
        let decks = run_json["decks"]
            .as_array()
            .filter(|decks| !decks.is_empty())
            .with_context(|| format!("{}: run.json names no decks", run_dir.display()))?;
        decks
            .iter()
            .map(|deck| {
                let key = deck["key"]
                    .as_str()
                    .with_context(|| format!("{}: a deck without a key", run_dir.display()))?;
                match self.splits.get(key).map(String::as_str) {
                    Some("train") => Ok(key.to_owned()),
                    Some(split) => anyhow::bail!(
                        "{}: seats deck {key}, which {} puts in the {split} split; bundle \
                         distillation trains on train decks only",
                        run_dir.display(),
                        self.path.display()
                    ),
                    None => anyhow::bail!(
                        "{}: seats deck {key}, which {} does not list (no split); bundle \
                         distillation trains on train decks only",
                        run_dir.display(),
                        self.path.display()
                    ),
                }
            })
            .collect()
    }
}

/// The runs a distillation dataset is made of.
pub(super) struct Runs {
    decks: TrainDecks,
    seated: std::collections::BTreeSet<String>,
    sources: Vec<Value>,
    /// The contract every run declares, and for house-teacher runs the
    /// teacher they name: one of each per dataset.
    contract: Option<&'static str>,
    teacher: Option<Value>,
    /// Whether any run probed the layer's answers (`distill.probe`).
    probed: bool,
}

impl Runs {
    /// No runs yet; their decks must be in `decks`.
    pub(super) const fn new(decks: TrainDecks) -> Self {
        Self {
            decks,
            seated: std::collections::BTreeSet::new(),
            sources: Vec::new(),
            contract: None,
            teacher: None,
            probed: false,
        }
    }

    /// Whether any admitted run probed the layer's answers: the dataset then
    /// gets the probe sidecar (`probes.rs`), even if no decision was probed.
    pub(super) const fn probed(&self) -> bool {
        self.probed
    }

    /// Admits one run: it must declare a distillation contract, the same as
    /// every other run of the dataset (and a house-teacher run the same
    /// teacher), and every deck it can seat must be a train deck of the
    /// manifest.
    ///
    /// # Errors
    /// A run without `distill.contract` one of `bundle_distillation_v1` and
    /// `house_distillation_v1`, one whose contract or teacher differs from
    /// the runs before it, or one with a deck outside the train split.
    pub(super) fn admit(&mut self, run_dir: &Path, run_json: &Value) -> anyhow::Result<()> {
        let contract = baylee_train_ops::tactician_labels::admit_distillation(run_json)
            .with_context(|| format!("{}", run_dir.display()))?;
        let teacher = (contract == HOUSE_DISTILL_CONTRACT)
            .then(|| run_json["distill"]["teacher"].clone())
            .filter(Value::is_object);
        ensure!(
            contract != HOUSE_DISTILL_CONTRACT || teacher.is_some(),
            "{}: a {contract} run names no teacher",
            run_dir.display()
        );
        ensure!(
            self.contract.is_none_or(|first| first == contract),
            "{}: a {contract} run among {} runs; a dataset holds one teacher's labels",
            run_dir.display(),
            self.contract.unwrap_or_default()
        );
        ensure!(
            self.teacher.is_none() || self.teacher == teacher,
            "{}: taught by {}, the dataset's runs before by {}",
            run_dir.display(),
            teacher.as_ref().map_or(Value::Null, |t| t["level"].clone()),
            self.teacher
                .as_ref()
                .map_or(Value::Null, |t| t["level"].clone())
        );
        // A searched run's answers are the policy's own only when its search
        // measured, with the layer for a rival, in fair worlds.
        let search = &run_json["search"];
        ensure!(
            search.is_null()
                || (search["selection"]["rule"] == "measure"
                    && search["rival"] == "distill"
                    && search["decks"]
                        == "public_prior_from_the_seats_view_and_own_list_never_another_players_list"),
            "{}: a searched run's labels are read only from a fair measuring search with the \
             layer for a rival (eval4 --search-select measure --search-rival distill --search-fair)",
            run_dir.display()
        );
        let seated = self.decks.seated(run_dir, run_json)?;
        self.contract = Some(contract);
        self.teacher.clone_from(&teacher);
        self.probed |= !run_json["distill"]["probe"].is_null();
        self.sources.push(
            json!({"run": run_dir, "label_rules": run_json["distill"]["label_rules"],
            "teacher": teacher, "players": run_json["distill"]["players"], "decks": seated.len(),
            "probe": run_json["distill"]["probe"], "search": run_json["search"]}),
        );
        self.seated.extend(seated);
        Ok(())
    }

    /// What `meta.json` says about the dataset (counts filled at the end).
    pub(super) fn metadata(&self, counts: &Counts) -> Value {
        let contract = self.contract();
        let target = if contract == HOUSE_DISTILL_CONTRACT {
            "the house teacher's answer H (in R's fields) at every labelled decision of the \
             recorded seats"
        } else {
            "the tactical layer's answer R at every labelled decision of the recorded seats"
        };
        json!({"contract": contract, "version": 2, "teacher": self.teacher,
            "target": target,
            "loss_weight": "1 at labelled decisions of the recorded seats, 0 elsewhere",
            "value_targets": "game outcomes, as usual",
            "subtypes": DistillSubtype4::ALL.map(DistillSubtype4::name),
            "groups": FIRED_GROUPS,
            "bst4_flags": {"bit_7": "N != R"},
            "bst4_block": {
                "frames": "every decision frame of a recorded seat (labelled, or a fallback at weight 0)",
                "after": "the free mask",
                "layout": "4 bytes BDX1, u8 subtype (index in subtypes, 255 none), 3 zero bytes, \
                           u64 little-endian mask of the groups that fired (bit i: groups[i])",
            },
            "steps": {
                "path": "bundle-distill/game-NNNNNN.jsonl",
                "lines": "a header, one label line per labelled frame, a completed footer",
                "label_line": ["game_id", "seat", "step_cursor", "label_kind", "n_ne_r",
                    "net_answer", "distill_subtype (priority frames, and target questions where side or aim fired)", "fired"],
            },
            "decks": {"manifest": self.decks.path, "manifest_sha256": self.decks.sha256,
                "count": self.seated.len(), "split": "train", "all_train": true,
                "checked": "every deck of each run's run.json, against the manifest"},
            "sources": self.sources, "counts": counts})
    }

    /// The contract the admitted runs declare (the layer's before any is).
    pub(super) fn contract(&self) -> &'static str {
        self.contract.unwrap_or(DISTILL_CONTRACT)
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
    /// Labelled frames by subtype.
    pub by_subtype: BTreeMap<String, u64>,
    /// `N ≠ R` frames by subtype.
    pub n_ne_r_by_subtype: BTreeMap<String, u64>,
    /// `N ≠ R` frames by each rule group that fired there.
    pub n_ne_r_by_group: BTreeMap<String, u64>,
    /// Frames whose `N` and `R` were probed in fair worlds.
    pub probed: u64,
    /// Probed frames by subtype.
    pub probed_by_subtype: BTreeMap<String, u64>,
}

/// Adds `other`'s counts into `into`.
fn add_counts(into: &mut BTreeMap<String, u64>, other: &BTreeMap<String, u64>) {
    for (key, n) in other {
        *into.entry(key.clone()).or_default() += n;
    }
}

impl Counts {
    pub(super) fn add(&mut self, other: &Self) {
        self.labelled += other.labelled;
        self.n_ne_r += other.n_ne_r;
        self.unlabelled += other.unlabelled;
        add_counts(&mut self.by_kind, &other.by_kind);
        add_counts(&mut self.n_ne_r_by_kind, &other.n_ne_r_by_kind);
        add_counts(&mut self.by_subtype, &other.by_subtype);
        add_counts(&mut self.n_ne_r_by_subtype, &other.n_ne_r_by_subtype);
        add_counts(&mut self.n_ne_r_by_group, &other.n_ne_r_by_group);
        self.probed += other.probed;
        add_counts(&mut self.probed_by_subtype, &other.probed_by_subtype);
    }

    /// Counts one labelled frame.
    fn count(&mut self, frame: &DistillFrameLabel4) {
        let one = |map: &mut BTreeMap<String, u64>, key: &str| {
            *map.entry(key.to_owned()).or_default() += 1;
        };
        self.labelled += 1;
        one(&mut self.by_kind, &frame.label_kind);
        if let Some(subtype) = frame.subtype {
            one(&mut self.by_subtype, subtype.name());
        }
        if frame.n_ne_r {
            self.n_ne_r += 1;
            one(&mut self.n_ne_r_by_kind, &frame.label_kind);
            if let Some(subtype) = frame.subtype {
                one(&mut self.n_ne_r_by_subtype, subtype.name());
            }
            for group in &frame.fired {
                one(&mut self.n_ne_r_by_group, group);
            }
        }
        if frame.probe.is_some() {
            self.probed += 1;
            if let Some(subtype) = frame.subtype {
                one(&mut self.probed_by_subtype, subtype.name());
            }
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
        if !DistillSubtype4::fits(label) {
            return Err(format!(
                "distillation step {index}: a {} label with subtype {:?} (recorded before \
                 subtypes? collect again)",
                label.kind, label.subtype
            ));
        }
        Ok(Some(label))
    }

    /// Makes `R` the target of a labelled decision frame, and every other
    /// decision frame weightless; gives every decision frame of a recorded
    /// seat its distillation block.
    ///
    /// # Errors
    /// A label that does not bind to its frame, or names a group the dataset
    /// has no bit for.
    pub(super) fn apply(&mut self, step: &mut Step4) -> Result<(), String> {
        if !step.is_decision {
            return Ok(());
        }
        let Some(label) = self.label_of(step)? else {
            step.loss_weight = 0.0;
            step.provenance.has_logprob = false;
            step.provenance.behavior_logprob = 0.0;
            // A recorded seat's fallback keeps a block that says nothing.
            step.distill = step.seat_input.map(|_| DistillFrame4 {
                subtype_code: NO_SUBTYPE,
                fired_mask: 0,
                label: None,
            });
            self.counts.unlabelled += 1;
            return Ok(());
        };
        let head = head_of(step.head).ok_or_else(|| format!("unknown head {}", step.head))?;
        let fired_mask = fired_mask(&label.fired)?;
        if let Some(probe) = &label.probe {
            probe_binds(label, probe).map_err(|why| {
                format!(
                    "distillation step {}: {why}",
                    step.seat_input.unwrap_or_default()
                )
            })?;
        }
        // The record's own answer, before `R` takes its place.
        let played = (head == HeadKind4::SingleChoice).then_some(step.target.single_idx);
        step.target = label.rules.clone();
        step.loss_weight = 1.0;
        step.provenance.source = SourceKind4::House;
        step.provenance.has_logprob = false;
        step.provenance.behavior_logprob = 0.0;
        step.provenance.temperature = 0.0;
        let frame = DistillFrameLabel4 {
            label_kind: label.kind.clone(),
            n_ne_r: label.n_ne_r,
            net_answer: net_answer(head, &label.net),
            subtype: label.subtype,
            fired: label.fired.clone(),
            played,
            probe: label.probe.clone(),
        };
        self.counts.count(&frame);
        step.distill = Some(DistillFrame4 {
            subtype_code: label.subtype.map_or(NO_SUBTYPE, DistillSubtype4::code),
            fired_mask,
            label: Some(frame),
        });
        Ok(())
    }
}

/// Whether a probe belongs to its label: a priority decision where `N`
/// differs from `R`, its rival `R`'s own place in the menu, and a score and
/// an end for both answers in every world.
fn probe_binds(label: &DistillLabel4, probe: &DistillProbe4) -> Result<(), String> {
    if label.kind != "priority" || !label.n_ne_r {
        return Err(format!(
            "a probe at a {} label with N {} R",
            label.kind,
            if label.n_ne_r { "!=" } else { "=" }
        ));
    }
    if probe.rival_index != label.rules.single_idx {
        return Err(format!(
            "the probe's rival is menu entry {}, the label's R {}",
            probe.rival_index, label.rules.single_idx
        ));
    }
    if probe.own_index == probe.rival_index {
        return Err(format!(
            "the probe's N and R are one menu entry ({})",
            probe.own_index
        ));
    }
    if probe.worlds.is_empty() || probe.worlds.len() != probe.ends.len() {
        return Err(format!(
            "a probe of {} worlds and {} ends",
            probe.worlds.len(),
            probe.ends.len()
        ));
    }
    if probe
        .worlds
        .iter()
        .flatten()
        .any(|score| !score.is_finite())
    {
        return Err("a probe with a score that is not a number".into());
    }
    Ok(())
}

/// The mask of the groups that fired: bit `i` for the `i`-th of
/// [`FIRED_GROUPS`].
fn fired_mask(fired: &[String]) -> Result<u64, String> {
    let names = FIRED_GROUPS;
    fired.iter().try_fold(0_u64, |mask, group| {
        let bit = names
            .iter()
            .position(|name| name == group)
            .ok_or_else(|| format!("no bit for the rule group {group}"))?;
        Ok(mask | 1 << bit)
    })
}

/// A game's JSON steps (`bundle-distill/game-NNNNNN.jsonl`): a header, one
/// line per labelled frame, a footer. Written beside the game's BST4 and
/// published under its name only when the game is.
pub(super) struct JsonSteps {
    temporary: PathBuf,
    path: PathBuf,
    writer: BufWriter<File>,
    game_id: u32,
    lines: u64,
}

impl JsonSteps {
    /// Opens a game's JSON steps under the dataset `root`, for a dataset of
    /// `contract`.
    ///
    /// # Errors
    /// When the file exists already or cannot be written.
    pub(super) fn new(root: &Path, game_id: u32, contract: &str) -> anyhow::Result<Self> {
        let path = root.join(steps_path(game_id));
        fs::create_dir_all(path.parent().context("steps directory")?)?;
        ensure!(!path.exists(), "{} exists", path.display());
        let temporary = path.with_extension("partial");
        let mut writer = BufWriter::new(
            OpenOptions::new()
                .create(true)
                .truncate(true)
                .write(true)
                .open(&temporary)?,
        );
        serde_json::to_writer(
            &mut writer,
            &json!({"kind": "header", "contract": contract, "schema": 2,
                "game_id": game_id}),
        )?;
        writeln!(writer)?;
        Ok(Self {
            temporary,
            path,
            writer,
            game_id,
            lines: 0,
        })
    }

    /// Writes a labelled frame's line; other frames have none.
    ///
    /// # Errors
    /// When the line cannot be written.
    pub(super) fn push(&mut self, step: &Step4) -> Result<(), String> {
        let Some(frame) = step.distill.as_ref().and_then(|d| d.label.as_ref()) else {
            return Ok(());
        };
        let mut line = serde_json::to_value(frame).map_err(|e| e.to_string())?;
        line["kind"] = json!("label");
        line["game_id"] = json!(self.game_id);
        line["seat"] = json!(step.seat);
        line["step_cursor"] = json!(step.step_cursor);
        serde_json::to_writer(&mut self.writer, &line).map_err(|e| e.to_string())?;
        writeln!(self.writer).map_err(|e| e.to_string())?;
        self.lines += 1;
        Ok(())
    }

    /// Writes the footer and publishes the file under its name.
    ///
    /// # Errors
    /// When the file cannot be written or renamed.
    pub(super) fn finish(mut self) -> anyhow::Result<()> {
        serde_json::to_writer(
            &mut self.writer,
            &json!({"kind": "completed", "labels": self.lines}),
        )?;
        writeln!(self.writer)?;
        self.writer.flush()?;
        self.writer.get_ref().sync_all()?;
        fs::rename(&self.temporary, &self.path)?;
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
            behavior_mixture: None,
            distill: None,
            cand_classes: None,
            search_root: None,
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
            subtype: Some(DistillSubtype4::Spend),
            probe: None,
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
        let distilled = step.distill.as_ref().unwrap();
        // The block: spend's code, and the bit of the group `spend`.
        assert_eq!(distilled.subtype_code, 3);
        let spend_bit = FIRED_GROUPS.iter().position(|g| *g == "spend").unwrap();
        assert_eq!(distilled.fired_mask, 1 << spend_bit);
        let line = distilled.label.as_ref().unwrap();
        assert!(line.n_ne_r);
        assert_eq!(line.net_answer.single_idx, Some(2));
        assert_eq!(line.subtype, Some(DistillSubtype4::Spend));
        assert_eq!(line.fired, ["spend"]);
        assert!(!step.provenance.has_logprob);
        assert_eq!(step.provenance.behavior_logprob, 0.0);
        // A step without a label (a fallback) keeps its frame at weight zero
        // with a block that says nothing; another seat's decision has no
        // block; passive frames stay as they are.
        let mut fallback = frame(0, Some(1), 0);
        cursor.apply(&mut fallback).unwrap();
        assert_eq!(fallback.loss_weight, 0.0);
        assert_eq!(
            fallback.distill,
            Some(DistillFrame4 {
                subtype_code: NO_SUBTYPE,
                fired_mask: 0,
                label: None
            })
        );
        let mut house = frame(1, None, 0);
        cursor.apply(&mut house).unwrap();
        assert_eq!((house.loss_weight, house.distill), (0.0, None));
        let mut passive = frame(0, None, 0);
        passive.is_decision = false;
        passive.loss_weight = 0.25;
        cursor.apply(&mut passive).unwrap();
        assert_eq!((passive.loss_weight, passive.distill), (0.25, None));
        assert_eq!(
            (
                cursor.counts.labelled,
                cursor.counts.n_ne_r,
                cursor.counts.unlabelled
            ),
            (1, 1, 2)
        );
        assert_eq!(cursor.counts.by_subtype["spend"], 1);
        assert_eq!(cursor.counts.n_ne_r_by_subtype["spend"], 1);
        assert_eq!(cursor.counts.n_ne_r_by_group["spend"], 1);
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
        // A priority label without its subtype (recorded before subtypes),
        // and another kind's label with one.
        let mut old = label(1, 2, 2);
        old.subtype = None;
        let labels = game(vec![Some(old)]);
        let error = Cursor::new(&labels)
            .apply(&mut frame(0, Some(0), 2))
            .unwrap_err();
        assert!(
            error.contains("priority label with subtype None"),
            "{error}"
        );
        let mut single = label(1, 2, 2);
        single.kind = "single".into();
        let labels = game(vec![Some(single)]);
        assert!(
            Cursor::new(&labels)
                .apply(&mut frame(0, Some(0), 2))
                .is_err()
        );
        // A group the dataset has no bit for.
        let mut unknown = label(1, 2, 2);
        unknown.fired = vec!["spend".into(), "nonesuch".into()];
        let labels = game(vec![Some(unknown)]);
        let error = Cursor::new(&labels)
            .apply(&mut frame(0, Some(0), 2))
            .unwrap_err();
        assert!(error.contains("nonesuch"), "{error}");
        // A sidecar out of step with the rollout sidecar.
        let entry = json!({"rollout_sidecar": [{}, {}], "distill_sidecar": []});
        assert!(GameLabels::from_game(&entry).is_err());
        assert!(GameLabels::from_game(&json!({"rollout_sidecar": []})).is_err());
    }

    #[test]
    fn the_json_steps_hold_one_line_per_labelled_frame_with_subtype_and_fired() {
        static NEXT: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(0);
        let root = std::env::temp_dir().join(format!(
            "bundle-distill-steps-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
        ));
        let labels = game(vec![Some(label(1, 2, 2)), None]);
        let mut cursor = Cursor::new(&labels);
        let mut steps = JsonSteps::new(&root, 7, DISTILL_CONTRACT).unwrap();
        for mut step in [
            frame(0, Some(0), 2),
            frame(0, Some(1), 0),
            frame(1, None, 0),
        ] {
            step.game_id = 7;
            step.step_cursor = step.seat_input.unwrap_or(9);
            cursor.apply(&mut step).unwrap();
            steps.push(&step).unwrap();
        }
        let path = root.join(steps_path(7));
        assert!(!path.exists(), "published before the game is");
        steps.finish().unwrap();
        let lines: Vec<Value> = fs::read_to_string(&path)
            .unwrap()
            .lines()
            .map(|line| serde_json::from_str(line).unwrap())
            .collect();
        let _ = fs::remove_dir_all(&root);
        assert_eq!(lines.len(), 3, "{lines:?}");
        assert_eq!(lines[0]["kind"], "header");
        assert_eq!(lines[0]["contract"], DISTILL_CONTRACT);
        assert_eq!(
            lines[1],
            json!({"kind": "label", "game_id": 7, "seat": 0, "step_cursor": 0,
                "label_kind": "priority", "n_ne_r": true, "net_answer": {"single_idx": 2},
                "distill_subtype": "spend", "fired": ["spend"]})
        );
        assert_eq!(lines[2], json!({"kind": "completed", "labels": 1}));
        // What the line holds reads back as the frame.
        let frame: DistillFrameLabel4 = serde_json::from_value(lines[1].clone()).unwrap();
        assert_eq!(frame.subtype, Some(DistillSubtype4::Spend));
    }

    #[test]
    fn a_target_question_where_side_fired_is_target_with_n_and_r() {
        // The layer moved a counterspell off the seat's own spell: `side`.
        let mut target = label(1, 2, 2);
        target.kind = "single".into();
        target.pending_kind = PendingKind4::ChooseTargets as u8;
        target.fired = vec!["side".into()];
        target.subtype = Some(DistillSubtype4::Target);
        let mut other = target.clone();
        other.fired = vec!["void".into()];
        other.subtype = None;
        let labels = game(vec![Some(target), Some(other)]);
        let mut cursor = Cursor::new(&labels);
        let question = |input| {
            let mut step = frame(0, Some(input), 2);
            step.pending_kind = PendingKind4::ChooseTargets as u8;
            step
        };
        let mut aimed = question(0);
        cursor.apply(&mut aimed).unwrap();
        let distilled = aimed.distill.unwrap();
        assert_eq!(distilled.subtype_code, 9);
        let line = distilled.label.unwrap();
        assert_eq!(line.subtype, Some(DistillSubtype4::Target));
        assert_eq!(line.net_answer.single_idx, Some(2));
        assert!(line.n_ne_r);
        assert_eq!(aimed.target.single_idx, 1);
        // Another target question: labelled, R its target, no subtype.
        let mut plain = question(1);
        cursor.apply(&mut plain).unwrap();
        assert_eq!(plain.distill.as_ref().unwrap().subtype_code, NO_SUBTYPE);
        assert_eq!(plain.loss_weight, 1.0);
        assert_eq!(cursor.counts.by_subtype["target"], 1);
        assert_eq!(cursor.counts.by_kind["single"], 2);
    }

    /// A manifest of three decks, one in each split.
    fn manifest() -> TrainDecks {
        let manifest = json!({"decks": [
            {"id": "adapted/A/train-deck", "split": "train", "format": "constructed"},
            {"id": "adapted/B/val-deck", "split": "val", "format": "constructed"},
            {"id": "adapted/C/test-deck", "split": "test", "format": "constructed"},
        ]});
        TrainDecks::parse(
            PathBuf::from("decks/manifest.v4.json"),
            &serde_json::to_vec(&manifest).unwrap(),
        )
        .unwrap()
    }

    fn run(decks: &[&str]) -> Value {
        json!({"distill": {"contract": DISTILL_CONTRACT, "label_rules": "mass"},
            "decks": decks.iter().map(|key| json!({"key": key, "cards": [1, 2]})).collect::<Vec<_>>()})
    }

    #[test]
    fn a_run_that_can_seat_a_deck_outside_the_train_split_is_refused() {
        let dir = std::path::Path::new("runs/r");
        let mut runs = Runs::new(manifest());
        for (decks, names) in [
            (
                &["adapted/A/train-deck", "adapted/B/val-deck"][..],
                "adapted/B/val-deck, which decks/manifest.v4.json puts in the val split",
            ),
            (
                &["adapted/C/test-deck"][..],
                "adapted/C/test-deck, which decks/manifest.v4.json puts in the test split",
            ),
            (
                &["eval-d000"][..],
                "eval-d000, which decks/manifest.v4.json does not list",
            ),
            (&[][..], "run.json names no decks"),
        ] {
            let error = runs.admit(dir, &run(decks)).unwrap_err().to_string();
            assert!(error.contains(names), "{error}");
        }
        let mut no_decks = run(&[]);
        no_decks.as_object_mut().unwrap().remove("decks");
        assert!(runs.admit(dir, &no_decks).is_err());
        // Nothing refused was admitted; a train run is.
        runs.admit(dir, &run(&["adapted/A/train-deck"])).unwrap();
        let meta = runs.metadata(&Counts::default());
        assert_eq!(meta["sources"].as_array().unwrap().len(), 1);
        assert_eq!(meta["decks"]["count"], 1);
        assert_eq!(meta["decks"]["all_train"], true);
        assert_eq!(meta["decks"]["manifest"], "decks/manifest.v4.json");
        assert_eq!(meta["decks"]["manifest_sha256"].as_str().unwrap().len(), 64);
    }

    /// A searched run's labels are read only where the search played
    /// nothing the net did not: a fair measurement with the layer for a
    /// rival (eval4's metadata of it, `the_layer_is_a_rival_only_in_a_fair_
    /// measurement_of_two`).
    #[test]
    fn a_searched_run_is_admitted_only_as_a_fair_probe_of_the_layer() {
        let dir = std::path::Path::new("runs/r");
        let fair = "public_prior_from_the_seats_view_and_own_list_never_another_players_list";
        let searched = |rule: &str, rival: &str, decks: &str| {
            let mut run = run(&["adapted/A/train-deck"]);
            run["search"] = json!({"selection": {"rule": rule}, "rival": rival, "decks": decks});
            run["distill"]["probe"] = json!({"contract": "distill_probe_v1"});
            run
        };
        for refused in [
            searched("paired", "distill", fair),
            searched("measure", "ranking", fair),
            searched("measure", "distill", "known_to_the_harness"),
        ] {
            let mut runs = Runs::new(manifest());
            let error = runs.admit(dir, &refused).unwrap_err().to_string();
            assert!(error.contains("fair measuring search"), "{error}");
        }
        let mut runs = Runs::new(manifest());
        assert!(!runs.probed());
        runs.admit(dir, &searched("measure", "distill", fair))
            .unwrap();
        assert!(runs.probed(), "the dataset gets the probe sidecar");
        let meta = runs.metadata(&Counts::default());
        assert_eq!(meta["sources"][0]["probe"]["contract"], "distill_probe_v1");
        assert_eq!(meta["sources"][0]["search"]["rival"], "distill");
    }

    /// A probe binds to its label: a priority `N ≠ R` decision, `R`'s menu
    /// entry, a finite score and an end per world.
    #[test]
    fn a_probe_that_does_not_belong_to_its_label_is_refused() {
        let probe = |rival_index, worlds: Vec<[f32; 2]>| DistillProbe4 {
            rival_index,
            own_index: 1,
            ends: worlds
                .iter()
                .map(|_| ["boundary".to_owned(), "boundary".to_owned()])
                .collect(),
            worlds,
            rollouts: 2,
            house_answers: 0,
        };
        let differs = label(2, 1, 1);
        assert!(probe_binds(&differs, &probe(2, vec![[0.4, 0.5]])).is_ok());
        assert!(
            probe_binds(&differs, &probe(3, vec![[0.4, 0.5]])).is_err(),
            "not R"
        );
        let mut own = probe(2, vec![[0.4, 0.5]]);
        own.own_index = 2;
        assert!(probe_binds(&differs, &own).is_err(), "N is R's entry");
        assert!(
            probe_binds(&differs, &probe(2, vec![])).is_err(),
            "no world"
        );
        assert!(
            probe_binds(&differs, &probe(2, vec![[f32::NAN, 0.5]])).is_err(),
            "not a number"
        );
        let mut short = probe(2, vec![[0.4, 0.5], [0.4, 0.6]]);
        short.ends.pop();
        assert!(probe_binds(&differs, &short).is_err(), "an end per world");
        let same = label(1, 1, 1);
        assert!(
            probe_binds(&same, &probe(1, vec![[0.4, 0.5]])).is_err(),
            "N = R"
        );
        let mut subset = label(2, 1, 1);
        subset.kind = "subset".into();
        assert!(probe_binds(&subset, &probe(2, vec![[0.4, 0.5]])).is_err());

        // A bound probe rides with the frame into the JSON steps.
        let mut probed = label(2, 1, 1);
        probed.probe = Some(probe(2, vec![[0.4, 0.5]]));
        let labels = game(vec![Some(probed)]);
        let mut cursor = Cursor::new(&labels);
        let mut step = frame(0, Some(0), 1);
        cursor.apply(&mut step).unwrap();
        assert_eq!(cursor.counts.probed, 1);
        let kept = step.distill.as_ref().unwrap().label.as_ref().unwrap();
        assert_eq!(kept.probe.as_ref().unwrap().worlds, [[0.4, 0.5]]);
        let line = serde_json::to_value(kept).unwrap();
        assert_eq!(line["probe"]["rival_index"], 2);
    }

    #[test]
    fn only_a_declared_run_is_admitted_and_the_meta_names_the_contract() {
        let mut runs = Runs::new(manifest());
        let dir = std::path::Path::new("runs/r");
        let mut imitation = run(&["adapted/A/train-deck"]);
        imitation.as_object_mut().unwrap().remove("distill");
        imitation["labels"] = json!("tactician_imitation_v1");
        assert!(runs.admit(dir, &imitation).is_err());
        runs.admit(dir, &run(&["adapted/A/train-deck"])).unwrap();
        let meta = runs.metadata(&Counts::default());
        assert_eq!(meta["contract"], DISTILL_CONTRACT);
        assert_eq!(meta["sources"][0]["label_rules"], "mass");
        assert_eq!(meta["subtypes"][2], "instant_their_turn");
        assert_eq!(meta["subtypes"].as_array().unwrap().len(), 10);
        assert_eq!(meta["subtypes"][9], "target");
        assert_eq!(meta["groups"].as_array().unwrap().len(), 22);
        assert_eq!(meta["groups"][7], "spend");
    }

    #[test]
    fn a_house_teacher_dataset_names_its_teacher_and_takes_no_other_labels() {
        let dir = std::path::Path::new("runs/h");
        let house = |level: &str| {
            let mut run = run(&["adapted/A/train-deck"]);
            run["distill"] = json!({"contract": HOUSE_DISTILL_CONTRACT,
                "teacher": {"kind": "house", "level": level, "fair": true}});
            run
        };
        let mut runs = Runs::new(manifest());
        let mut nameless = house("house:expert");
        nameless["distill"]
            .as_object_mut()
            .unwrap()
            .remove("teacher");
        assert!(runs.admit(dir, &nameless).is_err());
        runs.admit(dir, &house("house:expert")).unwrap();
        runs.admit(dir, &house("house:expert")).unwrap();
        let other = runs
            .admit(dir, &house("house:sharp"))
            .unwrap_err()
            .to_string();
        assert!(other.contains("house:sharp"), "{other}");
        let layer = runs
            .admit(dir, &run(&["adapted/A/train-deck"]))
            .unwrap_err()
            .to_string();
        assert!(layer.contains("one teacher"), "{layer}");
        let meta = runs.metadata(&Counts::default());
        assert_eq!(meta["contract"], HOUSE_DISTILL_CONTRACT);
        assert_eq!(meta["teacher"]["level"], "house:expert");
        assert_eq!(meta["sources"].as_array().unwrap().len(), 2);
        assert_eq!(meta["sources"][0]["teacher"]["level"], "house:expert");
        assert!(meta["target"].as_str().unwrap().contains("house teacher"));
        assert_eq!(runs.contract(), HOUSE_DISTILL_CONTRACT);
        // And the other way round: a dataset of the layer's labels takes no
        // house-teacher run, and names no teacher.
        let mut layer_runs = Runs::new(manifest());
        layer_runs
            .admit(dir, &run(&["adapted/A/train-deck"]))
            .unwrap();
        assert!(layer_runs.admit(dir, &house("house:expert")).is_err());
        assert_eq!(
            layer_runs.metadata(&Counts::default())["teacher"],
            Value::Null
        );
        assert_eq!(layer_runs.contract(), DISTILL_CONTRACT);
    }
}
