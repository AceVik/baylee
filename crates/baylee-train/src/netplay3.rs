//! The trained AI playing a chair with a v3 net ([`crate::features3`]): the
//! same player as [`crate::netplay`], on v3's inputs.
//!
//! The graph (`Net.tables` in `tools/trainer/model3.py`, exported by
//! `export_onnx3.py`) takes the entity rows, the seat rows, the globals and
//! the deciding seat's deck list, and returns the same score tables as v2's
//! but with players read per seat row, and the value as one logit per seat.
//! The seat's win chance is the softmax over the seats present, summed over
//! its side.

use std::path::Path;

use anyhow::Context as _;
use baylee_engine::choice::Pending;
use baylee_view::PlayerView;
use ort::session::Session;
use ort::session::builder::GraphOptimizationLevel;
use ort::value::Tensor;

use crate::features3::{
    DECK_COLS, ENT_COLS, Encoded, GLOB_COLS, MAX_DECK, MAX_SEATS, SEAT_COLS, Table, encode, options,
};
use crate::netplay::{Answer, TABLES};
use crate::policy::{self, Choice, Opt, Picked, head};

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

    /// `Net.score` for one option.
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

    /// The deciding seat's side's win chance: the softmax of the per-seat
    /// value logits (`-inf` where no seat), summed over the rows `side`
    /// marks.
    fn side_chance(&self, side: &[bool]) -> f32 {
        let (v, _) = &self.t[12];
        let top = v.iter().copied().fold(f32::NEG_INFINITY, f32::max);
        if !top.is_finite() {
            return f32::NAN;
        }
        let exp: Vec<f32> = v.iter().map(|x| (x - top).exp()).collect();
        let total: f32 = exp.iter().sum();
        exp.iter()
            .zip(side)
            .filter(|(_, s)| **s)
            .map(|(e, _)| e / total)
            .sum()
    }
}

/// How often a net may be asked in one game state before the house answers
/// for it. A fourth time is a loop of its own answers: a cast it cannot pay
/// for is undone, and at temperature 0 it starts the same cast again. No
/// finished game of a 1,500-game arena asked the net in one state more than
/// three times, and every game that looped to its cap did six times or more.
pub const STALL_VISITS: u32 = 3;

/// A v3 net on ONNX Runtime.
pub struct NetPlayer3 {
    session: Session,
    entities: usize,
    /// The house profile the net is asked to play as (0 novice … 4 expert).
    pub profile: i64,
    /// Smaller exports of the same net (`buckets` in the export's JSON),
    /// by the rows each takes: a decision runs on the smallest that holds
    /// its rows.
    buckets: Vec<(usize, Session)>,
    /// 0 plays the best-scored option; above 0 samples from the softmax of
    /// the scores over this temperature (self-play's exploration).
    pub temperature: f32,
    rng: crate::deckgen::Rng,
    /// Questions answered, and of them how many offered one option only and
    /// were answered without running the net.
    pub asked: u64,
    /// See [`Self::asked`].
    pub forced: u64,
}

impl NetPlayer3 {
    /// Loads `model` (an export of `export_onnx3.py`) for `entities` rows,
    /// on one intra-op thread.
    ///
    /// # Errors
    /// When ONNX Runtime cannot load the model.
    pub fn load(model: &Path, entities: usize, profile: i64) -> anyhow::Result<Self> {
        let meta: Option<serde_json::Value> = std::fs::read(model.with_extension("onnx.json"))
            .ok()
            .and_then(|b| serde_json::from_slice(&b).ok());
        let open = |path: &Path| -> anyhow::Result<Session> {
            let builder = Session::builder().map_err(ort_error)?;
            let builder = builder
                .with_optimization_level(GraphOptimizationLevel::Level3)
                .map_err(ort_error)?;
            let mut builder = builder.with_intra_threads(1).map_err(ort_error)?;
            builder
                .commit_from_file(path)
                .map_err(ort_error)
                .with_context(|| format!("loading {}", path.display()))
        };
        let mut buckets = Vec::new();
        for b in meta
            .as_ref()
            .and_then(|m| m["buckets"].as_array())
            .into_iter()
            .flatten()
        {
            let (Some(rows), Some(file)) = (b["entities"].as_u64(), b["file"].as_str()) else {
                continue;
            };
            let rows = usize::try_from(rows).unwrap_or(usize::MAX);
            if rows < entities {
                buckets.push((rows, open(&model.with_file_name(file))?));
            }
        }
        buckets.sort_by_key(|(rows, _)| *rows);
        let builder = Session::builder().map_err(ort_error)?;
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
            profile,
            buckets,
            temperature: 0.0,
            rng: crate::deckgen::Rng::new(0x5e1f_91a7),
            asked: 0,
            forced: 0,
        })
    }

    /// Seeds the sampling (a game's number, so a run replays).
    pub fn seed(&mut self, seed: u64) {
        self.rng = crate::deckgen::Rng::new(seed ^ 0x5e1f_91a7);
    }

    /// The rows a decision runs on: the smallest bucket that holds its
    /// rows, else the export's full count.
    fn rows_for(&self, enc: &Encoded) -> usize {
        self.buckets
            .iter()
            .map(|(rows, _)| *rows)
            .find(|rows| *rows >= enc.cards.len())
            .unwrap_or(self.entities)
    }

    fn tables(&mut self, enc: &Encoded) -> anyhow::Result<Tables> {
        let e = self.rows_for(enc);
        let kept = enc.cards.len().min(e);
        let mut cards = vec![0_i64; e];
        let mut feats = vec![0_i16; e * ENT_COLS.len()];
        let mut mask = vec![true; e];
        for i in 0..kept {
            cards[i] = i64::from(enc.cards[i]);
            feats[i * ENT_COLS.len()..(i + 1) * ENT_COLS.len()].copy_from_slice(&enc.rows[i]);
            mask[i] = false;
        }
        let mut seats = vec![0_i16; MAX_SEATS * SEAT_COLS.len()];
        for (r, row) in enc.seats.iter().take(MAX_SEATS).enumerate() {
            seats[r * SEAT_COLS.len()..(r + 1) * SEAT_COLS.len()].copy_from_slice(row);
        }
        let mut glob = enc.globals.clone();
        glob.resize(GLOB_COLS.len(), 0);
        let k = MAX_DECK;
        let mut deck_cards = vec![0_i64; k];
        let mut deck_feats = vec![0_i16; k * DECK_COLS.len()];
        let mut deck_mask = vec![true; k];
        for (i, (card, row)) in enc
            .deck_cards
            .iter()
            .zip(&enc.deck_rows)
            .take(k)
            .enumerate()
        {
            deck_cards[i] = i64::from(*card);
            deck_feats[i * DECK_COLS.len()..(i + 1) * DECK_COLS.len()].copy_from_slice(row);
            deck_mask[i] = false;
        }
        let inputs = ort::inputs![
            "cards" => Tensor::from_array(([1, e], cards)).map_err(ort_error)?,
            "feats" => Tensor::from_array(([1, e, ENT_COLS.len()], feats)).map_err(ort_error)?,
            "mask" => Tensor::from_array(([1, e], mask)).map_err(ort_error)?,
            "seats" => Tensor::from_array(([1, MAX_SEATS, SEAT_COLS.len()], seats)).map_err(ort_error)?,
            "glob" => Tensor::from_array(([1, GLOB_COLS.len()], glob)).map_err(ort_error)?,
            "deck_cards" => Tensor::from_array(([1, k], deck_cards)).map_err(ort_error)?,
            "deck_feats" => Tensor::from_array(([1, k, DECK_COLS.len()], deck_feats)).map_err(ort_error)?,
            "deck_mask" => Tensor::from_array(([1, k], deck_mask)).map_err(ort_error)?,
            "profile" => Tensor::from_array(([1], vec![self.profile])).map_err(ort_error)?,
        ];
        let session = match self.buckets.iter_mut().find(|(rows, _)| *rows == e) {
            Some((_, s)) => s,
            None => &mut self.session,
        };
        let outputs = session.run(inputs).map_err(ort_error)?;
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
    /// this kind of question or no offered row fits its entity cap (the
    /// house answers it).
    ///
    /// # Errors
    /// When ONNX Runtime fails.
    pub fn answer(
        &mut self,
        view: &PlayerView,
        pending: &Pending,
        table: &Table<'_>,
    ) -> anyhow::Result<Option<Answer>> {
        let mut picked = Picked::default();
        let mut picks: Vec<Choice> = Vec::new();
        let mut value = f32::NAN;
        self.asked += 1;
        loop {
            let enc = encode(view, pending, &picked, table);
            let offered = options(&enc, view, pending, &picked);
            // One option: the answer is the question's, not the net's.
            if picks.is_empty() && offered.len() == 1 && !is_multi(pending) {
                let (choice, _) = offered[0];
                self.forced += 1;
                return Ok(policy::assemble(pending, &[choice])
                    .ok()
                    .map(|action| Answer {
                        action,
                        picks: 1,
                        value: f32::NAN,
                    }));
            }
            let tables = self.tables(&enc)?;
            if picks.is_empty() {
                let side: Vec<bool> = (0..MAX_SEATS)
                    .map(|r| {
                        enc.seats
                            .get(r)
                            .is_some_and(|row| row[0] == 1 && row[2] == 1)
                    })
                    .collect();
                value = tables.side_chance(&side);
            }
            let e = self.rows_for(&enc);
            let fits = |o: &Opt| {
                let row_heads = [head::ENTITY, head::ABILITY, head::PAIR, head::ATTACK_PLAYER];
                !row_heads.contains(&o.head)
                    || (usize::try_from(o.a).is_ok_and(|a| a < e)
                        && (o.head != head::PAIR || usize::try_from(o.b).is_ok_and(|b| b < e)))
            };
            let scored: Vec<(Choice, f32)> = offered
                .into_iter()
                .filter(|(_, o)| fits(o))
                .map(|(c, o)| (c, tables.score(o)))
                .collect();
            let Some(choice) = self.pick(&scored) else {
                return Ok(None);
            };
            picks.push(choice);
            picked.add(choice);
            if policy::finished(pending, &picked, choice) {
                break;
            }
        }
        let n = picks.len();
        Ok(policy::assemble(pending, &picks).ok().map(|action| Answer {
            action,
            picks: n,
            value,
        }))
    }
}

impl NetPlayer3 {
    /// The best-scored option, or at a temperature one sampled from the
    /// softmax of the scores.
    fn pick(&mut self, scored: &[(Choice, f32)]) -> Option<Choice> {
        let best = scored.iter().max_by(|x, y| x.1.total_cmp(&y.1))?;
        if self.temperature <= 0.0 || scored.len() == 1 {
            return Some(best.0);
        }
        let weights: Vec<f64> = scored
            .iter()
            .map(|(_, s)| f64::from((s - best.1) / self.temperature).exp())
            .collect();
        Some(scored[self.rng.weighted(&weights)].0)
    }
}

/// Whether `pending` takes an answer of several picks.
fn is_multi(pending: &Pending) -> bool {
    matches!(
        pending,
        Pending::ChooseAttackers { .. }
            | Pending::ChooseBlockers { .. }
            | Pending::MulliganBottom { .. }
            | Pending::DiscardChoice { .. }
            | Pending::LegendChoice { .. }
            | Pending::ChooseCards { .. }
            | Pending::ChooseTargets { .. }
    )
}

/// ONNX Runtime's error, as `anyhow` takes it.
#[allow(clippy::needless_pass_by_value)] // the shape `map_err` hands it
fn ort_error<T>(e: ort::Error<T>) -> anyhow::Error {
    anyhow::anyhow!("onnx runtime: {e}")
}
