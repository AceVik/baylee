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

/// A v3 net on ONNX Runtime.
pub struct NetPlayer3 {
    session: Session,
    entities: usize,
    /// The house profile the net is asked to play as (0 novice … 4 expert).
    pub profile: i64,
}

impl NetPlayer3 {
    /// Loads `model` (an export of `export_onnx3.py`) for `entities` rows,
    /// on one intra-op thread.
    ///
    /// # Errors
    /// When ONNX Runtime cannot load the model.
    pub fn load(model: &Path, entities: usize, profile: i64) -> anyhow::Result<Self> {
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
        })
    }

    fn tables(&mut self, enc: &Encoded) -> anyhow::Result<Tables> {
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
        loop {
            let enc = encode(view, pending, &picked, table);
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
            let e = self.entities;
            let fits = |o: &Opt| {
                let row_heads = [head::ENTITY, head::ABILITY, head::PAIR, head::ATTACK_PLAYER];
                !row_heads.contains(&o.head)
                    || (usize::try_from(o.a).is_ok_and(|a| a < e)
                        && (o.head != head::PAIR || usize::try_from(o.b).is_ok_and(|b| b < e)))
            };
            let best = options(&enc, view, pending, &picked)
                .into_iter()
                .filter(|(_, o)| fits(o))
                .map(|(c, o)| (c, tables.score(o)))
                .max_by(|x, y| x.1.total_cmp(&y.1));
            let Some((choice, _)) = best else {
                return Ok(None);
            };
            picks.push(choice);
            picked.count += 1;
            match choice {
                Choice::Entity(o, _) | Choice::Attack(o, _) | Choice::Block(o, _) => {
                    picked.objects.insert(o);
                }
                Choice::Player(p) => {
                    picked.players.insert(p);
                }
                _ => {}
            }
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

/// ONNX Runtime's error, as `anyhow` takes it.
#[allow(clippy::needless_pass_by_value)] // the shape `map_err` hands it
fn ort_error<T>(e: ort::Error<T>) -> anyhow::Error {
    anyhow::anyhow!("onnx runtime: {e}")
}
