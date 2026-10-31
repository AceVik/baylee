//! Group-weighted self-imitation (`--game-weights`): only the net decisions
//! of the listed seats become policy targets, each at its group weight
//! (`train_ops group-weights`). Every other decision, a house opponent's
//! above all, keeps its frame for memory and value but no policy weight.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use anyhow::{Context as _, ensure};
use baylee_train::convert4::Step4;
use baylee_train::features4::{MAX_SEATS, SourceKind4};
use serde_json::{Value, json};

const CONTRACT: &str = "group_self_imitation_v1";

/// Weights per record and seat, from one `train_ops group-weights` file.
pub(super) struct Weights {
    by_record: BTreeMap<String, [f32; MAX_SEATS]>,
    sha256: String,
}

impl Weights {
    /// Reads a weight file and checks that it was computed for exactly `runs`.
    pub(super) fn load(path: &Path, runs: &[PathBuf]) -> anyhow::Result<Self> {
        let bytes = std::fs::read(path).with_context(|| format!("reading {}", path.display()))?;
        let value: Value = serde_json::from_slice(&bytes)?;
        ensure!(
            value["version"] == 1 && value["contract"] == CONTRACT,
            "{} is not a {CONTRACT} file",
            path.display()
        );
        let listed: Vec<PathBuf> = serde_json::from_value(value["runs"].clone())?;
        let canonical = |paths: &[PathBuf]| -> anyhow::Result<Vec<PathBuf>> {
            paths.iter().map(|p| Ok(p.canonicalize()?)).collect()
        };
        ensure!(
            canonical(&listed)? == canonical(runs)?,
            "the game weights were computed for other runs"
        );
        let shas = value["run_sha256"]
            .as_array()
            .context("missing run_sha256")?;
        for (run, sha) in runs.iter().zip(shas) {
            ensure!(
                Some(super::shadow::sha(&std::fs::read(run.join("run.json"))?).as_str())
                    == sha.as_str(),
                "{} changed since its game weights were computed",
                run.display()
            );
        }
        let mut seats_by_record: BTreeMap<String, [Option<f32>; MAX_SEATS]> = BTreeMap::new();
        for entry in value["weights"].as_array().context("missing weights")? {
            let record = entry["record_sha256"]
                .as_str()
                .context("weight without record")?;
            let seat = usize::try_from(entry["seat"].as_u64().context("weight without seat")?)?;
            let weight = entry["weight"].as_f64().context("weight without value")? as f32;
            ensure!(seat < MAX_SEATS, "seat {seat} out of range");
            ensure!(
                weight.is_finite() && weight > 0.0 && weight <= 1.0,
                "weight {weight} outside (0, 1]"
            );
            let slot = &mut seats_by_record.entry(record.to_owned()).or_default()[seat];
            ensure!(
                slot.is_none(),
                "record {record} seat {seat} is weighted twice"
            );
            *slot = Some(weight);
        }
        Ok(Self {
            by_record: seats_by_record
                .into_iter()
                .map(|(record, seats)| (record, seats.map(|w| w.unwrap_or(0.0))))
                .collect(),
            sha256: super::shadow::sha(&bytes),
        })
    }

    /// The seat weights of one record, or `None` when no seat is imitated.
    pub(super) fn seats(&self, record_sha256: &str) -> Option<[f32; MAX_SEATS]> {
        self.by_record.get(record_sha256).copied()
    }

    /// The dataset metadata naming this weighting.
    pub(super) fn metadata(&self, games: usize) -> Value {
        json!({"version": 1, "contract": CONTRACT, "weights_sha256": self.sha256,
            "games": games, "targets": "net decisions of positively weighted seats"})
    }
}

/// Keeps a step's policy weight only for a weighted seat's own net decision.
pub(super) fn apply(step: &mut Step4, seats: &[f32; MAX_SEATS]) {
    step.loss_weight = if step.is_decision && step.provenance.source == SourceKind4::Net {
        step.loss_weight * seats[usize::from(step.seat)]
    } else {
        0.0
    };
}
