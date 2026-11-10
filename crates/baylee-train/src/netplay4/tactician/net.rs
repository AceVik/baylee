//! The tactician net of stage S1 (`tactician_head_v2`): a set transformer
//! over one question's rows that thinks in `T` passes and builds the answer,
//! exported by `tools/trainer/tactician_v3` as `tactician.onnx` beside
//! `tactician.json`.
//!
//! - input `rows` `[n + 1, 2d + ROW_TAIL]`: the null row first, then one row
//!   per unit, as [`super::head::assemble`] lays them out, with no window-code
//!   column that reads the tactical layer's answer;
//! - output `logits` `[n]`: the final pass's logit per unit, read by the
//!   question's own constrained decoder;
//! - output `confidence` `[1]`: the net's probability that its answer is the
//!   label.
//!
//! `T` is fixed in the graph (`tactician.json` `passes`), the confidence
//! threshold below which the seat plays `N` is `confidence_threshold`. Like
//! the stage-1 head it runs on the CPU with one intra-op thread per session,
//! so the answer does not depend on which worker asked; a small pool of
//! sessions keeps the workers from queueing on one.

use std::path::Path;
use std::sync::Mutex;

use anyhow::Context as _;
use ort::session::Session;
use ort::value::Tensor;
use serde_json::Value;

use super::ROW_TAIL;
use super::head::{assemble, cpu_session, read_graph};
use crate::netplay4::ort_error;

/// The net's metadata contract.
pub const NET_CONTRACT: &str = "tactician_head_v2";

/// A loaded `tactician_head_v2` net.
#[derive(Debug)]
pub struct TacticianNet4 {
    sessions: Vec<Mutex<Session>>,
    d_model: usize,
    passes: u32,
    threshold: f32,
    meta: Value,
    sha256: String,
}

impl TacticianNet4 {
    /// Loads `dir/tactician.onnx` with its `tactician.json`, in `pool`
    /// sessions (at least one).
    ///
    /// # Errors
    /// Missing files, another contract or row width, a threshold outside
    /// `[0, 1]`, or a graph that does not load.
    pub fn load(dir: &Path, pool: usize) -> anyhow::Result<Self> {
        let meta: Value = serde_json::from_slice(
            &std::fs::read(dir.join("tactician.json"))
                .with_context(|| format!("reading {}", dir.join("tactician.json").display()))?,
        )?;
        anyhow::ensure!(
            meta["contract"] == NET_CONTRACT,
            "tactician.json is not a {NET_CONTRACT} net"
        );
        let d_model = usize::try_from(meta["d_model"].as_u64().context("missing d_model")?)?;
        anyhow::ensure!(
            meta["row_tail"].as_u64() == Some(ROW_TAIL as u64),
            "the net reads rows of another width"
        );
        anyhow::ensure!(
            meta["rows_read_rules"] == false,
            "a {NET_CONTRACT} net reads rows without the tactical layer's answer"
        );
        let passes = u32::try_from(meta["passes"].as_u64().context("missing passes")?)?;
        anyhow::ensure!(passes >= 1, "a net thinks at least one pass");
        #[allow(
            clippy::cast_possible_truncation,
            reason = "a probability read from JSON"
        )]
        let threshold = meta["confidence_threshold"]
            .as_f64()
            .context("missing confidence_threshold")? as f32;
        anyhow::ensure!(
            (0.0..=1.0).contains(&threshold),
            "confidence_threshold lies in [0, 1]"
        );
        let (bytes, sha256) = read_graph(dir)?;
        let sessions = (0..pool.max(1))
            .map(|_| cpu_session(&bytes).map(Mutex::new))
            .collect::<anyhow::Result<Vec<_>>>()?;
        Ok(Self {
            sessions,
            d_model,
            passes,
            threshold,
            meta,
            sha256,
        })
    }

    /// Its metadata and digest, for a run's record.
    #[must_use]
    pub fn identity(&self) -> Value {
        serde_json::json!({"meta": self.meta, "sha256": self.sha256})
    }

    /// The confidence below which the seat plays `N`, as trained.
    #[must_use]
    pub const fn threshold(&self) -> f32 {
        self.threshold
    }

    /// The passes `T` the graph thinks.
    #[must_use]
    pub const fn passes(&self) -> u32 {
        self.passes
    }

    /// The width of one row.
    #[must_use]
    pub const fn row_width(&self) -> usize {
        2 * self.d_model + ROW_TAIL
    }

    /// The final pass's logit per unit and the confidence: `state` `[d]`,
    /// `units` `[n × d]`, `tails` `[(n + 1) × ROW_TAIL]`.
    ///
    /// # Errors
    /// Inputs of other widths, the session's failure, or outputs of another
    /// size or not finite.
    pub fn run(
        &self,
        state: &[f32],
        units: &[f32],
        tails: &[f32],
        n: usize,
    ) -> anyhow::Result<(Vec<f32>, f32)> {
        let d = self.d_model;
        anyhow::ensure!(
            n > 0 && state.len() == d && units.len() == n * d && tails.len() == (n + 1) * ROW_TAIL,
            "tactician rows of the wrong width"
        );
        let rows = assemble(state, units, tails, n);
        let input = Tensor::from_array(([n + 1, self.row_width()], rows)).map_err(ort_error)?;
        let mut session = self.session();
        let outputs = session
            .run(ort::inputs!["rows" => input])
            .map_err(ort_error)?;
        let (_, logits) = outputs["logits"]
            .try_extract_tensor::<f32>()
            .map_err(ort_error)?;
        let (_, confidence) = outputs["confidence"]
            .try_extract_tensor::<f32>()
            .map_err(ort_error)?;
        anyhow::ensure!(
            logits.len() == n
                && confidence.len() == 1
                && logits.iter().chain(confidence).all(|v| v.is_finite()),
            "the net answered other or non-finite outputs"
        );
        Ok((logits.to_vec(), confidence[0]))
    }

    /// A free session of the pool, or the first when all are busy.
    fn session(&self) -> std::sync::MutexGuard<'_, Session> {
        for slot in &self.sessions {
            if let Ok(guard) = slot.try_lock() {
                return guard;
            }
        }
        self.sessions[0]
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
    }
}
