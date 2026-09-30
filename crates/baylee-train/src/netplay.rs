//! The trained AI playing a chair: a policy exported to ONNX, run through
//! ONNX Runtime (`ort`), answering a seat's questions from its view alone.
//!
//! The exported graph is the fixed-shape part of the net (`PolicyNet.tables`
//! in `tools/trainer/model.py`): per decision a handful of score tables, per
//! object a few more. The options a question offers are scored here, off
//! those tables, exactly as `PolicyNet.score` scores them in training — so a
//! question with three thousand targets runs on the same fixed shapes as one
//! with two. An answer that takes several picks is built one pick at a time
//! and sent whole ([`policy::assemble`]).

use std::path::Path;

use anyhow::Context as _;
use baylee_core::ids::ObjectId;
use baylee_engine::choice::{Pending, PlayerAction};
use baylee_view::PlayerView;
use ort::session::Session;
use ort::session::builder::GraphOptimizationLevel;
use ort::value::Tensor;

use crate::features::{ENT_COLS, encode, rel};
use crate::policy::{self, Choice, Opt, Picked, head};

/// The graph's outputs, in `PolicyNet.TABLES` order.
pub const TABLES: [&str; 13] = [
    "fixed",
    "player",
    "color",
    "subtype",
    "number",
    "mode",
    "mode_kind",
    "verb",
    "ability",
    "attack_player",
    "pair_a",
    "pair_b",
    "value",
];

/// One decision's score tables, flattened, with their row widths.
struct Tables {
    t: Vec<(Vec<f32>, usize)>,
    d: usize,
}

impl Tables {
    fn at(&self, table: usize, i: usize) -> f32 {
        let (v, w) = &self.t[table];
        v.get(i.min(w.saturating_sub(1)))
            .copied()
            .unwrap_or(f32::NEG_INFINITY)
    }

    fn at2(&self, table: usize, row: usize, j: usize) -> f32 {
        let (v, w) = &self.t[table];
        v.get(row * w + j.min(w.saturating_sub(1)))
            .copied()
            .unwrap_or(f32::NEG_INFINITY)
    }

    /// `PolicyNet.score` for one option.
    fn score(&self, opt: Opt) -> f32 {
        let first = usize::try_from(opt.a).unwrap_or(0);
        let second = usize::try_from(opt.b).unwrap_or(0);
        match opt.head {
            head::FIXED => self.at(0, first),
            head::PLAYER => self.at(1, first),
            head::COLOR => self.at(2, first),
            head::SUBTYPE => self.at(3, first),
            head::NUMBER => self.at(4, first),
            head::MODE => self.at(5, first) + self.at(6, second),
            head::ENTITY => self.at2(7, first, second),
            head::ABILITY => self.at2(8, first, second),
            head::ATTACK_PLAYER => self.at2(9, first, second),
            head::PAIR => {
                let (left, right) = (&self.t[10].0, &self.t[11].0);
                let (from, to) = (first * self.d, second * self.d);
                match (left.get(from..from + self.d), right.get(to..to + self.d)) {
                    (Some(x), Some(y)) => x.iter().zip(y).map(|(p, q)| p * q).sum(),
                    _ => f32::NEG_INFINITY,
                }
            }
            _ => f32::NEG_INFINITY,
        }
    }
}

/// What a net answer was, for the arena's counts.
#[derive(Clone, Debug)]
pub struct Answer {
    /// The answer to send.
    pub action: PlayerAction,
    /// Picks made.
    pub picks: usize,
    /// The net's win chance for the seat before its first pick.
    pub value: f32,
}

/// A policy on ONNX Runtime.
pub struct NetPlayer {
    session: Session,
    entities: usize,
    glob_width: usize,
    /// The house profile the net is asked to play as (0 novice … 4 expert).
    pub profile: i64,
}

impl NetPlayer {
    /// Loads `model` (an export of `export_onnx.py`) for `entities` rows,
    /// on one intra-op thread: the arena runs one game per core.
    ///
    /// # Errors
    /// When ONNX Runtime cannot load the model.
    pub fn load(
        model: &Path,
        entities: usize,
        glob_width: usize,
        profile: i64,
    ) -> anyhow::Result<Self> {
        let builder = Session::builder().map_err(ort_error)?;
        #[cfg(feature = "onnx-cuda")]
        let builder = if std::env::var("BAYLEE_EP").as_deref() == Ok("cpu") {
            builder
        } else {
            builder
                .with_execution_providers([ort::ep::CUDA::default().build().error_on_failure()])
                .map_err(ort_error)?
        };
        let builder = builder
            .with_optimization_level(GraphOptimizationLevel::Level3)
            .map_err(ort_error)?;
        let mut builder = builder.with_intra_threads(1).map_err(ort_error)?;
        let session = builder
            .commit_from_file(model)
            .map_err(ort_error)
            .with_context(|| format!("loading {}", model.display()))?;
        Ok(Self {
            session,
            entities,
            glob_width,
            profile,
        })
    }

    fn tables(&mut self, enc: &crate::features::Encoded) -> anyhow::Result<Tables> {
        let e = self.entities;
        let kept = enc.cards.len().min(e);
        let mut cards = vec![0_i64; e];
        let mut feats = vec![0_i16; e * ENT_COLS.len()];
        let mut mask = vec![true; e];
        for i in 0..kept {
            cards[i] = i64::from(enc.cards[i]);
            feats[i * ENT_COLS.len()..(i + 1) * ENT_COLS.len()].copy_from_slice(&enc.rows[i]);
            mask[i] = false;
        }
        let mut glob = enc.globals.clone();
        glob.resize(self.glob_width, 0);
        let inputs = ort::inputs![
            "cards" => Tensor::from_array(([1, e], cards)).map_err(ort_error)?,
            "feats" => Tensor::from_array(([1, e, ENT_COLS.len()], feats)).map_err(ort_error)?,
            "mask" => Tensor::from_array(([1, e], mask)).map_err(ort_error)?,
            "glob" => Tensor::from_array(([1, self.glob_width], glob)).map_err(ort_error)?,
            "profile" => Tensor::from_array(([1], vec![self.profile])).map_err(ort_error)?,
        ];
        let outputs = self.session.run(inputs).map_err(ort_error)?;
        let mut t = Vec::with_capacity(TABLES.len());
        let mut d = 0;
        for name in TABLES {
            let (shape, data) = outputs[name]
                .try_extract_tensor::<f32>()
                .map_err(ort_error)?;
            let width = shape
                .last()
                .copied()
                .map_or(1, |w| usize::try_from(w).unwrap_or(1));
            if name == "pair_a" {
                d = width;
            }
            t.push((data.to_vec(), width));
        }
        Ok(Tables { t, d })
    }

    /// The net's answer to `pending`, or `None` when it does not answer
    /// this kind of question (the house answers it).
    ///
    /// # Errors
    /// When ONNX Runtime fails.
    pub fn answer(
        &mut self,
        view: &PlayerView,
        pending: &Pending,
    ) -> anyhow::Result<Option<Answer>> {
        let hand: Vec<ObjectId> = view.hand.iter().map(|h| h.id).collect();
        let seats = view.seats.len();
        let mut picked = Picked::default();
        let mut picks: Vec<Choice> = Vec::new();
        let mut value = f32::NAN;
        loop {
            let Ok(choices) = policy::options(pending, &hand, &picked) else {
                return Ok(None);
            };
            let enc = encode(view, pending, &picked);
            let tables = self.tables(&enc)?;
            if picks.is_empty() {
                value = 1.0 / (1.0 + (-tables.at(12, 0)).exp());
            }
            let e = self.entities;
            let row_of = |id| {
                enc.slots
                    .get(&id)
                    .copied()
                    .filter(|r| usize::try_from(*r).is_ok_and(|r| r < e))
            };
            let rel_of = |p| rel(view.seat, p, seats);
            let best = choices
                .iter()
                .filter_map(|c| policy::opt(*c, &row_of, &rel_of).map(|o| (*c, tables.score(o))))
                .max_by(|x, y| x.1.total_cmp(&y.1));
            let Some((choice, _)) = best else {
                return Ok(None);
            };
            picks.push(choice);
            picked.add(choice);
            if policy::finished(pending, &picked, choice) {
                break;
            }
        }
        let n = picks.len();
        // The question's own check sees what the picks cannot: a creature
        // that must attack, a limit on how many may, a block a Lure
        // demands. An answer it refuses goes to the house instead of being
        // refused.
        Ok(policy::assemble(pending, &picks)
            .ok()
            .filter(|action| pending.answer_fault(action).is_none())
            .map(|action| Answer {
                action,
                picks: n,
                value,
            }))
    }
}

/// ONNX Runtime's error, as `anyhow` takes it: the runtime's own error type
/// may carry a payload that is not `Send`, so it travels as its message.
#[allow(clippy::needless_pass_by_value)] // the shape `map_err` hands it
fn ort_error<T>(e: ort::Error<T>) -> anyhow::Error {
    anyhow::anyhow!("onnx runtime: {e}")
}
