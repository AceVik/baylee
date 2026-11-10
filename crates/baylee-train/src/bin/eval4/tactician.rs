//! `--tactician-windows`: the tactician at the seat under test's attack,
//! block and target questions (`baylee_train::netplay4::tactician`), and the
//! records a collection keeps of it.
//!
//! A collection writes, per worker, one JSON line per window decision to
//! `--tactician-out/wNN.jsonl` and its embeddings, window codes and looks as
//! little-endian f32 to `wNN.f32`, at the offset the line names, with the
//! deal's outcome for the deciding seat.
//!
//! - `--tactician-explore ε` (stage 1, `tactician_records_v1`): games for
//!   training the head on outcomes only.
//! - `--tactician-imitate ε` (stage S1, `tactician_imitation_v1`): the
//!   tactical layer's answers as labels, for the tactician only. The
//!   directory's own `run.json` says so, and so does `--record-out`'s:
//!   convert4 and every other trainer refuse such a run.
//! - `--tactician DIR` with `--tactician-out`: what a `tactician_head_v2` net
//!   did (`tactician_built_v1`), for reading a run.
//!
//! These runs are evaluations, never BC or PPO data.

use std::collections::BTreeMap;
use std::fs::File;
use std::io::{BufWriter, Write as _};
use std::path::{Path, PathBuf};
use std::sync::Arc;

use anyhow::Context as _;
use baylee_train::netplay4::tactician::head::{self, TacticianHead4};
use baylee_train::netplay4::tactician::imitate::IMITATION_CONTRACT;
use baylee_train::netplay4::tactician::look::LookPrior4;
use baylee_train::netplay4::tactician::net::{NET_CONTRACT, TacticianNet4};
use baylee_train::netplay4::tactician::{
    ROW_TAIL, Tactician4, TacticianMode4, TacticianRecord4, TacticianStats4, Window4,
};
use serde_json::{Value, json};

/// The record format of an exploring collection.
pub(super) const RECORD_CONTRACT: &str = "tactician_records_v1";
/// The record format of a run where a `tactician_head_v2` net plays.
pub(super) const BUILT_CONTRACT: &str = "tactician_built_v1";

/// Sessions of a `tactician_head_v2` net shared by a run's workers.
const NET_SESSIONS: usize = 8;

/// The command line's tactician options.
pub(super) struct TacticianArgs<'a> {
    pub(super) windows: Option<&'a str>,
    pub(super) explore: Option<f64>,
    pub(super) imitate: Option<f64>,
    pub(super) head: Option<&'a Path>,
    pub(super) margin: Option<f32>,
    pub(super) confidence: Option<f32>,
    pub(super) out: Option<&'a Path>,
}

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
        args: &TacticianArgs<'_>,
        train_decks: &[baylee_train::housedeck::HouseDeck],
    ) -> anyhow::Result<Option<Self>> {
        let TacticianArgs {
            windows,
            explore,
            imitate,
            head,
            margin,
            confidence,
            out,
        } = *args;
        let Some(windows) = windows else {
            anyhow::ensure!(
                explore.is_none()
                    && imitate.is_none()
                    && head.is_none()
                    && out.is_none()
                    && margin.is_none()
                    && confidence.is_none(),
                "--tactician-explore, --tactician-imitate, --tactician, --tactician-margin, \
                 --tactician-confidence and --tactician-out need --tactician-windows"
            );
            return Ok(None);
        };
        let windows = Window4::parse_list(windows)?;
        let mode = match (explore, imitate, head) {
            (Some(epsilon), None, None) => {
                anyhow::ensure!(
                    margin.is_none() && confidence.is_none(),
                    "--tactician-margin and --tactician-confidence are for a head"
                );
                anyhow::ensure!(
                    out.is_some(),
                    "an exploring collection needs --tactician-out"
                );
                TacticianMode4::Explore { epsilon }
            }
            (None, Some(epsilon), None) => {
                anyhow::ensure!(
                    margin.is_none() && confidence.is_none(),
                    "--tactician-margin and --tactician-confidence are for a head"
                );
                anyhow::ensure!(
                    out.is_some(),
                    "an imitating collection needs --tactician-out"
                );
                TacticianMode4::Imitate { epsilon }
            }
            (None, None, Some(dir)) => play_mode(dir, margin, confidence)?,
            _ => anyhow::bail!(
                "--tactician-windows needs exactly one of --tactician-explore, \
                 --tactician-imitate and --tactician"
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

    /// Whether the seat imitates the tactical layer.
    pub(super) const fn imitates(&self) -> bool {
        self.mode.imitates()
    }

    /// The contract of the records this run keeps.
    pub(super) const fn record_contract(&self) -> &'static str {
        match &self.mode {
            TacticianMode4::Explore { .. } | TacticianMode4::Play { .. } => RECORD_CONTRACT,
            TacticianMode4::Imitate { .. } => IMITATION_CONTRACT,
            TacticianMode4::Build { .. } => BUILT_CONTRACT,
        }
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
            .map(|dir| Sink::open(dir, worker, self.record_contract()))
            .transpose()
    }

    /// What run.json and the summary say about it.
    pub(super) fn metadata(&self) -> Value {
        let windows: Vec<&str> = self.windows.iter().map(|w| w.name()).collect();
        let mode = match &self.mode {
            TacticianMode4::Explore { epsilon } => json!({"explore": {"epsilon": epsilon}}),
            TacticianMode4::Imitate { epsilon } => json!({"imitate": {"epsilon": epsilon,
                "label": "the tactical layer's answer", "rows_read_rules": false}}),
            TacticianMode4::Play { head, margin } => {
                json!({"play": {"head": head.identity(), "margin": margin}})
            }
            TacticianMode4::Build { net, threshold } => json!({"build": {"net": net.identity(),
                "threshold": threshold, "passes": net.passes(), "fallback": "the net's answer"}}),
        };
        json!({"windows": windows, "mode": mode, "prior": self.prior.summary(),
            "records": self.out, "record_contract": self.record_contract(), "row_tail": ROW_TAIL})
    }

    /// Writes the record directory's own `run.json` for an imitating run:
    /// its contract, what it may train, and the run's `metadata`.
    ///
    /// # Errors
    /// I/O.
    pub(super) fn write_run(&self, run: &Value) -> anyhow::Result<()> {
        let Some(out) = self.out.as_deref().filter(|_| self.imitates()) else {
            return Ok(());
        };
        let marked = json!({
            "contract": IMITATION_CONTRACT,
            "trains": "the tactician only",
            "training_data": false,
            "tactician": self.metadata(),
            "run": run,
        });
        std::fs::write(out.join("run.json"), serde_json::to_vec_pretty(&marked)?)?;
        Ok(())
    }
}

/// The play mode a trained head's directory names.
fn play_mode(
    dir: &Path,
    margin: Option<f32>,
    confidence: Option<f32>,
) -> anyhow::Result<TacticianMode4> {
    let contract = head::contract_of(dir)?;
    if contract == NET_CONTRACT {
        anyhow::ensure!(
            margin.is_none(),
            "--tactician-margin is for a {} head",
            head::HEAD_CONTRACT
        );
        let net = TacticianNet4::load(dir, NET_SESSIONS)?;
        let threshold = confidence.unwrap_or_else(|| net.threshold());
        return Ok(TacticianMode4::Build {
            net: Arc::new(net),
            threshold,
        });
    }
    anyhow::ensure!(
        confidence.is_none(),
        "--tactician-confidence is for a {NET_CONTRACT} net"
    );
    let head = TacticianHead4::load(dir)?;
    let margin = match margin {
        Some(margin) => margin,
        None => head.identity()["meta"]["margin"]
            .as_f64()
            .context("the head names no margin; give --tactician-margin")? as f32,
    };
    Ok(TacticianMode4::Play {
        head: Arc::new(head),
        margin,
    })
}

/// One worker's record files.
pub(super) struct Sink {
    lines: BufWriter<File>,
    blob: BufWriter<File>,
    blob_name: String,
    offset: u64,
    contract: &'static str,
}

/// The deal a game's records belong to.
pub(super) struct DealKey<'a> {
    pub(super) game: &'a str,
    pub(super) deal: u64,
    pub(super) seed: u64,
    pub(super) policy_seed: u64,
    pub(super) focal_seat: usize,
    /// The seat under test's utility: win 1, loss 0, a cap ½.
    pub(super) utility: f32,
    pub(super) capped: bool,
}

impl DealKey<'_> {
    /// The utility of `seat` at a two-seat table: the seat under test's, or
    /// its complement for the self-play copy across it.
    fn utility_of(&self, seat: u8) -> f32 {
        if usize::from(seat) == self.focal_seat {
            self.utility
        } else {
            1.0 - self.utility
        }
    }
}

impl Sink {
    fn open(dir: &Path, worker: usize, contract: &'static str) -> anyhow::Result<Self> {
        let blob_name = format!("w{worker:02}.f32");
        Ok(Self {
            lines: BufWriter::new(File::create(dir.join(format!("w{worker:02}.jsonl")))?),
            blob: BufWriter::new(File::create(dir.join(&blob_name))?),
            blob_name,
            offset: 0,
            contract,
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
            line["contract"] = json!(self.contract);
            line["game"] = json!(key.game);
            line["deal"] = json!(key.deal);
            line["seed"] = json!(key.seed);
            line["policy_seed"] = json!(key.policy_seed);
            line["focal_seat"] = json!(key.focal_seat);
            line["utility"] = json!(key.utility_of(record.seat));
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

/// What the tactician cost per decision, by window and in all: the whole
/// tactician and, for a `tactician_head_v2` net, its forward alone.
pub(super) fn latency(totals: &BTreeMap<String, TacticianStats4>) -> Value {
    let mut all = Vec::new();
    let mut all_net = Vec::new();
    let mut out = serde_json::Map::new();
    for (window, stats) in totals {
        all.extend_from_slice(&stats.micros);
        all_net.extend_from_slice(&stats.net_micros);
        out.insert(
            window.clone(),
            json!({"tactician": TacticianStats4::latency(&stats.micros),
                "net": TacticianStats4::latency(&stats.net_micros)}),
        );
    }
    out.insert(
        "all".into(),
        json!({"tactician": TacticianStats4::latency(&all),
            "net": TacticianStats4::latency(&all_net)}),
    );
    Value::Object(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use baylee_train::netplay4::tactician::Drawn4;

    #[allow(clippy::cast_precision_loss, reason = "test values")]
    fn record(seat: u8, units: usize, d: usize) -> TacticianRecord4 {
        TacticianRecord4 {
            seat,
            seq: 4,
            turn: 2,
            window: Window4::Attack,
            value: 0.4,
            keys: (0..units).map(|i| format!("u{i}")).collect(),
            logits: vec![0.5; units],
            net: vec![1; units],
            rules: vec![0; units],
            played: vec![0; units],
            probability: 0.9,
            neighbors: 0,
            offered: Vec::new(),
            drawn: Drawn4::Rules,
            micros: 30,
            scores: None,
            groups: (0..units as u32).collect(),
            label: Some("rules".into()),
            rows_read_rules: Some(false),
            built_logits: None,
            built: None,
            confidence: None,
            net_micros: None,
            state: (0..d).map(|i| i as f32).collect(),
            units: (0..units * d).map(|i| 100.0 + i as f32).collect(),
            rows: (0..(units + 1) * ROW_TAIL).map(|i| -(i as f32)).collect(),
        }
    }

    /// Lines and blob read back as the trainers read them: every float at
    /// the offset its line names, each seat with its own utility, and the
    /// imitation contract on every line.
    #[test]
    fn imitation_records_round_trip_through_their_files() {
        let dir =
            std::env::temp_dir().join(format!("tactician-sink-{}-{}", std::process::id(), line!()));
        std::fs::create_dir_all(&dir).unwrap();
        let mut sink = Sink::open(&dir, 3, IMITATION_CONTRACT).unwrap();
        let key = DealKey {
            game: "eval4-00007",
            deal: 7,
            seed: 70,
            policy_seed: 9,
            focal_seat: 1,
            utility: 1.0,
            capped: false,
        };
        let records = [record(1, 2, 3), record(0, 3, 3)];
        sink.write(&key, &records).unwrap();
        drop(sink);
        let text = std::fs::read_to_string(dir.join("w03.jsonl")).unwrap();
        let blob = std::fs::read(dir.join("w03.f32")).unwrap();
        let floats: Vec<f32> = blob
            .as_chunks::<4>()
            .0
            .iter()
            .map(|&b| f32::from_le_bytes(b))
            .collect();
        let lines: Vec<Value> = text
            .lines()
            .map(|l| serde_json::from_str(l).unwrap())
            .collect();
        assert_eq!(lines.len(), 2);
        for (line, want) in lines.iter().zip(&records) {
            assert_eq!(line["contract"], IMITATION_CONTRACT);
            assert_eq!(line["label"], "rules");
            assert_eq!(line["rows_read_rules"], false);
            let back: TacticianRecord4 = serde_json::from_value(line.clone()).unwrap();
            assert_eq!(back.rules, want.rules);
            assert_eq!(back.groups, want.groups);
            let start = usize::try_from(line["blob"]["offset"].as_u64().unwrap()).unwrap() / 4;
            let (s, u, r) = (want.state.len(), want.units.len(), want.rows.len());
            assert_eq!(&floats[start..start + s], want.state.as_slice());
            assert_eq!(&floats[start + s..start + s + u], want.units.as_slice());
            assert_eq!(
                &floats[start + s + u..start + s + u + r],
                want.rows.as_slice()
            );
        }
        assert_eq!(lines[0]["utility"], 1.0);
        assert_eq!(lines[1]["utility"], 0.0);
        std::fs::remove_dir_all(&dir).unwrap();
    }
}
