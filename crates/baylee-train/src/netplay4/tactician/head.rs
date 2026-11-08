//! The trained tactician head: an MLP over one row per unit and a null row,
//! exported by `tools/trainer/tactician4.py` as `tactician.onnx` (input
//! `rows` `[N, 2d + ROW_TAIL]`, output `score` `[N]`) beside
//! `tactician.json`. It runs on the CPU, one session per process, so a game's
//! answers do not depend on which worker or batch asked.

use std::path::Path;
use std::sync::Mutex;

use anyhow::Context as _;
use ort::session::Session;
use ort::session::builder::GraphOptimizationLevel;
use ort::value::Tensor;
use serde_json::Value;

use super::ROW_TAIL;
use crate::netplay4::ort_error;

/// The head's metadata contract.
pub const HEAD_CONTRACT: &str = "tactician_head_v1";

/// A loaded head.
#[derive(Debug)]
pub struct TacticianHead4 {
    session: Mutex<Session>,
    d_model: usize,
    meta: Value,
    sha256: String,
}

impl TacticianHead4 {
    /// Loads `dir/tactician.onnx` with its `tactician.json`.
    ///
    /// # Errors
    /// Missing files, another contract, or a graph that does not load.
    pub fn load(dir: &Path) -> anyhow::Result<Self> {
        let meta: Value = serde_json::from_slice(
            &std::fs::read(dir.join("tactician.json"))
                .with_context(|| format!("reading {}", dir.join("tactician.json").display()))?,
        )?;
        anyhow::ensure!(
            meta["contract"] == HEAD_CONTRACT,
            "tactician.json is not a {HEAD_CONTRACT} head"
        );
        let d_model = usize::try_from(meta["d_model"].as_u64().context("missing d_model")?)?;
        anyhow::ensure!(
            meta["row_tail"].as_u64() == Some(ROW_TAIL as u64),
            "the head reads rows of another width"
        );
        let path = dir.join("tactician.onnx");
        let bytes = std::fs::read(&path).with_context(|| format!("reading {}", path.display()))?;
        let sha256 = super::look::hex(&<sha2::Sha256 as sha2::Digest>::digest(&bytes));
        let session = Session::builder(crate::onnx::environment().map_err(ort_error)?)
            .map_err(ort_error)?
            .with_optimization_level(GraphOptimizationLevel::Level3)
            .map_err(ort_error)?
            .with_intra_threads(1)
            .map_err(ort_error)?
            .commit_from_memory(&bytes)
            .map_err(ort_error)?;
        Ok(Self {
            session: Mutex::new(session),
            d_model,
            meta,
            sha256,
        })
    }

    /// Its metadata and digest, for a run's record.
    #[must_use]
    pub fn identity(&self) -> Value {
        serde_json::json!({"meta": self.meta, "sha256": self.sha256})
    }

    /// The width of one row.
    #[must_use]
    pub const fn row_width(&self) -> usize {
        2 * self.d_model + ROW_TAIL
    }

    /// One score per row, the null row first: `state` `[d]`, `units`
    /// `[n × d]`, `tails` `[(n + 1) × ROW_TAIL]`.
    ///
    /// # Errors
    /// Inputs of other widths, or the session's failure.
    pub fn score(
        &self,
        state: &[f32],
        units: &[f32],
        tails: &[f32],
        n: usize,
    ) -> anyhow::Result<Vec<f32>> {
        let d = self.d_model;
        anyhow::ensure!(
            state.len() == d && units.len() == n * d && tails.len() == (n + 1) * ROW_TAIL,
            "tactician rows of the wrong width"
        );
        let rows = assemble(state, units, tails, n);
        let input = Tensor::from_array(([n + 1, self.row_width()], rows)).map_err(ort_error)?;
        let mut session = self
            .session
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let outputs = session
            .run(ort::inputs!["rows" => input])
            .map_err(ort_error)?;
        let (_, score) = outputs["score"]
            .try_extract_tensor::<f32>()
            .map_err(ort_error)?;
        anyhow::ensure!(
            score.len() == n + 1 && score.iter().all(|s| s.is_finite()),
            "the head answered other or non-finite scores"
        );
        Ok(score.to_vec())
    }
}

/// The head's input rows: `[state | unit embedding (zero for the null row) |
/// window code and look]`, the null row first. The trainer builds the same
/// rows from a collection's records.
#[must_use]
pub fn assemble(state: &[f32], units: &[f32], tails: &[f32], n: usize) -> Vec<f32> {
    let d = state.len();
    let mut rows = Vec::with_capacity((n + 1) * (2 * d + ROW_TAIL));
    for row in 0..=n {
        rows.extend_from_slice(state);
        if row == 0 {
            rows.extend(std::iter::repeat_n(0.0, d));
        } else {
            rows.extend_from_slice(&units[(row - 1) * d..row * d]);
        }
        rows.extend_from_slice(&tails[row * ROW_TAIL..(row + 1) * ROW_TAIL]);
    }
    rows
}
