//! `--tactician-windows`: the tactician at the seat under test's attack,
//! block and target questions (`baylee_train::netplay4::tactician`), and the
//! records a collection keeps of it.
//!
//! A collection (`--tactician-explore ε`) writes, per worker, one JSON line
//! per window decision to `--tactician-out/wNN.jsonl` and its embeddings,
//! window codes and looks as little-endian f32 to `wNN.f32`, at the offset
//! the line names, with the deal's outcome for the seat. Such games are for
//! training the head on outcomes only: their run is an evaluation, never BC
//! or PPO data, and their records stay out of every other dataset.

use std::collections::BTreeMap;
use std::fs::File;
use std::io::{BufWriter, Write as _};
use std::path::{Path, PathBuf};
use std::sync::Arc;

use anyhow::Context as _;
use baylee_train::netplay4::tactician::head::TacticianHead4;
use baylee_train::netplay4::tactician::look::LookPrior4;
use baylee_train::netplay4::tactician::{
    ROW_TAIL, Tactician4, TacticianMode4, TacticianRecord4, TacticianStats4, Window4,
};
use serde_json::{Value, json};

/// The record format a reader checks.
pub(super) const RECORD_CONTRACT: &str = "tactician_records_v1";

/// What every worker's tactician is built from.
#[derive(Clone)]
pub(super) struct TacticianSetup {
    windows: std::collections::BTreeSet<Window4>,
    mode: TacticianMode4,
    prior: Arc<LookPrior4>,
    out: Option<PathBuf>,
}

impl TacticianSetup {
    /// Reads the options; `None` without `--tactician-windows`.
    ///
    /// # Errors
    /// Contradictory options, a head that does not load, or a prior that
    /// cannot be built from the manifest's train split.
    pub(super) fn from_args(
        windows: Option<&str>,
        explore: Option<f64>,
        head: Option<&Path>,
        margin: Option<f32>,
        out: Option<&Path>,
        train_decks: &[baylee_train::housedeck::HouseDeck],
    ) -> anyhow::Result<Option<Self>> {
        let Some(windows) = windows else {
            anyhow::ensure!(
                explore.is_none() && head.is_none() && out.is_none() && margin.is_none(),
                "--tactician-explore, --tactician, --tactician-margin and --tactician-out \
                 need --tactician-windows"
            );
            return Ok(None);
        };
        let windows = Window4::parse_list(windows)?;
        let mode = match (explore, head) {
            (Some(epsilon), None) => {
                anyhow::ensure!(margin.is_none(), "--tactician-margin is for a head");
                anyhow::ensure!(
                    out.is_some(),
                    "an exploring collection needs --tactician-out"
                );
                TacticianMode4::Explore { epsilon }
            }
            (None, Some(dir)) => {
                let head = TacticianHead4::load(dir)?;
                let margin = match margin {
                    Some(margin) => margin,
                    None => head.identity()["meta"]["margin"]
                        .as_f64()
                        .context("the head names no margin; give --tactician-margin")?
                        as f32,
                };
                TacticianMode4::Play {
                    head: Arc::new(head),
                    margin,
                }
            }
            _ => anyhow::bail!(
                "--tactician-windows needs exactly one of --tactician-explore and --tactician"
            ),
        };
        anyhow::ensure!(
            !train_decks.is_empty(),
            "the tactician's prior needs train decks"
        );
        let prior = LookPrior4::from_decks(train_decks.iter().map(|deck| &deck.deck))?;
        if let Some(out) = out {
            std::fs::create_dir_all(out)?;
        }
        Ok(Some(Self {
            windows,
            mode,
            prior: Arc::new(prior),
            out: out.map(Path::to_path_buf),
        }))
    }

    /// A worker's tactician.
    ///
    /// # Errors
    /// Never for options [`Self::from_args`] accepted.
    pub(super) fn tactician(&self) -> anyhow::Result<Tactician4> {
        Tactician4::new(self.windows.clone(), self.mode.clone(), self.prior.clone())
    }

    /// A worker's record files, when the run keeps records.
    ///
    /// # Errors
    /// When the files cannot be created.
    pub(super) fn sink(&self, worker: usize) -> anyhow::Result<Option<Sink>> {
        self.out
            .as_deref()
            .map(|dir| Sink::open(dir, worker))
            .transpose()
    }

    /// What run.json and the summary say about it.
    pub(super) fn metadata(&self) -> Value {
        let windows: Vec<&str> = self.windows.iter().map(|w| w.name()).collect();
        let mode = match &self.mode {
            TacticianMode4::Explore { epsilon } => json!({"explore": {"epsilon": epsilon}}),
            TacticianMode4::Play { head, margin } => {
                json!({"play": {"head": head.identity(), "margin": margin}})
            }
        };
        json!({"windows": windows, "mode": mode, "prior": self.prior.summary(),
            "records": self.out, "record_contract": RECORD_CONTRACT, "row_tail": ROW_TAIL})
    }
}

/// One worker's record files.
pub(super) struct Sink {
    lines: BufWriter<File>,
    blob: BufWriter<File>,
    blob_name: String,
    offset: u64,
}

/// The deal a game's records belong to.
pub(super) struct DealKey<'a> {
    pub(super) game: &'a str,
    pub(super) deal: u64,
    pub(super) seed: u64,
    pub(super) policy_seed: u64,
    pub(super) focal_seat: usize,
    /// The seat's utility: win 1, loss 0, a cap ½.
    pub(super) utility: f32,
    pub(super) capped: bool,
}

impl Sink {
    fn open(dir: &Path, worker: usize) -> anyhow::Result<Self> {
        let blob_name = format!("w{worker:02}.f32");
        Ok(Self {
            lines: BufWriter::new(File::create(dir.join(format!("w{worker:02}.jsonl")))?),
            blob: BufWriter::new(File::create(dir.join(&blob_name))?),
            blob_name,
            offset: 0,
        })
    }

    /// Writes a game's records with its outcome.
    ///
    /// # Errors
    /// I/O.
    pub(super) fn write(
        &mut self,
        key: &DealKey<'_>,
        records: &[TacticianRecord4],
    ) -> anyhow::Result<()> {
        for record in records {
            let floats = record.state.len() + record.units.len() + record.rows.len();
            let mut line = serde_json::to_value(record)?;
            line["contract"] = json!(RECORD_CONTRACT);
            line["game"] = json!(key.game);
            line["deal"] = json!(key.deal);
            line["seed"] = json!(key.seed);
            line["policy_seed"] = json!(key.policy_seed);
            line["focal_seat"] = json!(key.focal_seat);
            line["utility"] = json!(key.utility);
            line["capped"] = json!(key.capped);
            line["blob"] = json!({"file": self.blob_name, "offset": self.offset,
                "state": record.state.len(), "units": record.units.len(), "rows": record.rows.len()});
            for value in record.state.iter().chain(&record.units).chain(&record.rows) {
                self.blob.write_all(&value.to_le_bytes())?;
            }
            self.offset += 4 * floats as u64;
            writeln!(self.lines, "{}", serde_json::to_string(&line)?)?;
        }
        self.blob.flush()?;
        self.lines.flush()?;
        Ok(())
    }
}

/// Adds a game's decisions to a run's counts, by window.
pub(super) fn count(totals: &mut BTreeMap<String, TacticianStats4>, records: &[TacticianRecord4]) {
    for record in records {
        totals
            .entry(record.window.name().to_owned())
            .or_default()
            .count(record);
    }
}
