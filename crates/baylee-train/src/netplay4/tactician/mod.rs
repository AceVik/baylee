//! The tactician, stage 1: a head that learns from game outcomes where the
//! tactical layer earns most, the attack, block and single-target questions
//! (`docs/trained-ai.md` §"The tactician").
//!
//! At such a question ([`Window4`]) the seat has three answers at hand: the
//! net's raw greedy answer `N`, the tactical layer's answer `R` on the same
//! logits (asked read-only; it is never a label), and the *neighbors* of `N`,
//! the legal answers one change away: another target, one attacker's
//! inclusion flipped, one blocker's assignment changed. Every answer is a
//! mask over the question's *units*: its candidates (attackers with their
//! defender, targets) or its blocker-attacker pairs.
//!
//! - [`TacticianMode4::Explore`] (collection): `N` with probability `1 − ε`;
//!   `R` with `ε/3` when it differs from `N`; the rest of `ε` spread evenly
//!   over the neighbors. The played answer's probability under that mixture
//!   is exact, and each decision is kept as a [`TacticianRecord4`] with the
//!   net's embeddings, the window code and the look of every unit.
//! - [`TacticianMode4::Play`]: the head scores every unit and a null unit;
//!   the advantage of an answer is the null score plus the scores of the
//!   units it takes. Of `N`, `R` and the neighbors (the answers collection
//!   explored) it plays the best, and leaves `N` only when the best is
//!   better by more than the margin.
//!
//! Nothing here reads the session: the look is computed from the seat's view
//! and a prior over the training decks ([`look`]), never from a deck list.

pub mod head;
pub mod look;
#[cfg(test)]
mod tests;

use std::collections::BTreeSet;
use std::sync::Arc;

use baylee_engine::choice::{Pending, PlayerAction};
use baylee_view::PlayerView;
use serde::{Deserialize, Serialize};

use super::{StepEmbeddings4, TacticRules4, choices, sample_categorical, tactics};
use crate::deckgen::Rng;
use crate::features4::{
    EncodedCandidates4, HeadKind4, TeacherTarget4, decode_action4, encode_executed_action4,
    sample_pairs_constrained, sample_subset_constrained,
};
use head::TacticianHead4;
use look::{LOOK_WIDTH, LookPrior4};

/// The window code's width: what kind of question a row belongs to and
/// where its unit stands in the net's answer.
pub const WINDOW_WIDTH: usize = 16;
/// Per row, what the head reads beside the two embeddings.
pub const ROW_TAIL: usize = WINDOW_WIDTH + LOOK_WIDTH;
/// The probability the net's own answer is played in collection.
pub const DEFAULT_EPSILON: f64 = 0.25;

/// A question the tactician answers.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Window4 {
    /// Declaring attackers: a subset of attacker candidates.
    Attack,
    /// Declaring blockers: a set of blocker-attacker pairs.
    Block,
    /// Choosing one target (or none, where none is allowed).
    Target,
}

impl Window4 {
    /// The window `pending` opens with these candidates, if any.
    #[must_use]
    pub fn of(pending: &Pending, cands: &EncodedCandidates4) -> Option<Self> {
        match (pending, cands.head) {
            (Pending::ChooseAttackers { .. }, HeadKind4::SubsetPick) => Some(Self::Attack),
            (Pending::ChooseBlockers { .. }, HeadKind4::PairPick) => Some(Self::Block),
            (Pending::ChooseTargets { .. }, HeadKind4::SingleChoice) => Some(Self::Target),
            _ => None,
        }
    }

    /// The units a question offers: pairs at a block, candidates otherwise.
    #[must_use]
    pub const fn units(self, cands: &EncodedCandidates4) -> usize {
        match self {
            Self::Block => cands.pairs.len(),
            Self::Attack | Self::Target => cands.candidates.len(),
        }
    }

    /// The window's name in a list: `attack`, `block`, `target`.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::Attack => "attack",
            Self::Block => "block",
            Self::Target => "target",
        }
    }

    /// Reads a comma list of window names; `all` names every one.
    ///
    /// # Errors
    /// A word that names no window, or an empty list.
    pub fn parse_list(text: &str) -> anyhow::Result<BTreeSet<Self>> {
        let mut out = BTreeSet::new();
        for word in text.split(',').filter(|w| !w.is_empty()) {
            match word {
                "all" => out.extend([Self::Attack, Self::Block, Self::Target]),
                "attack" => {
                    out.insert(Self::Attack);
                }
                "block" => {
                    out.insert(Self::Block);
                }
                "target" => {
                    out.insert(Self::Target);
                }
                other => anyhow::bail!("no tactician window {other}"),
            }
        }
        anyhow::ensure!(!out.is_empty(), "no tactician window named");
        Ok(out)
    }
}

/// Which branch of the behavior drew the played answer (for reading a run;
/// the recorded probability is the whole mixture's).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Drawn4 {
    /// The net's answer.
    Net,
    /// The tactical layer's answer.
    Rules,
    /// A neighbor of the net's answer.
    Neighbor,
    /// The head's choice ([`TacticianMode4::Play`]).
    Head,
}

/// How the tactician answers.
#[derive(Clone, Debug)]
pub enum TacticianMode4 {
    /// Collection: explore around the net's answer at rate `epsilon`.
    Explore {
        /// The probability of leaving the net's answer, `0 ≤ ε < 1`.
        epsilon: f64,
    },
    /// Evaluation: the head chooses among the supported answers.
    Play {
        /// The trained head.
        head: Arc<TacticianHead4>,
        /// How much better (in utility) an answer must score than `N`.
        margin: f32,
    },
}

/// One window decision, as collection keeps it.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TacticianRecord4 {
    /// The deciding seat.
    pub seat: u8,
    /// The view's sequence number at the decision.
    pub seq: u64,
    /// The game turn.
    pub turn: u32,
    /// The question.
    pub window: Window4,
    /// The net's side utility at the decision: the advantage's baseline.
    pub value: f32,
    /// The units, in the graph's order: one key each, for alignment checks.
    pub keys: Vec<String>,
    /// The net's raw logit per unit.
    pub logits: Vec<f32>,
    /// The net's answer, the tactical layer's, and the one played.
    pub net: Vec<u8>,
    /// The tactical layer's answer on the same logits.
    pub rules: Vec<u8>,
    /// The answer played.
    pub played: Vec<u8>,
    /// The played answer's probability under the behavior that drew it.
    pub probability: f64,
    /// How many neighbors the behavior offered.
    pub neighbors: usize,
    /// The neighbors themselves, in the behavior's order: with `net` and
    /// `rules`, every answer a head may choose here.
    pub offered: Vec<Vec<u8>>,
    /// Which branch drew it.
    pub drawn: Drawn4,
    /// Microseconds the tactician spent on this decision (the rules' answer,
    /// the neighbors, the look and, playing, the head): for the report only,
    /// no decision reads it.
    pub micros: u64,
    /// The head's score per row, the null row first (play only).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub scores: Option<Vec<f32>>,
    /// The thought-through global slot, `[d]`.
    #[serde(skip)]
    pub state: Vec<f32>,
    /// Each unit's embedding, `[units × d]`.
    #[serde(skip)]
    pub units: Vec<f32>,
    /// Each row's window code and look, the null row first,
    /// `[(units + 1) × ROW_TAIL]`.
    #[serde(skip)]
    pub rows: Vec<f32>,
}

/// What a run's tactician did, by window.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize)]
pub struct TacticianStats4 {
    /// Window decisions answered.
    pub decisions: u64,
    /// Decisions whose played answer differed from the net's.
    pub changed: u64,
    /// Answers played, by branch.
    pub drawn_net: u64,
    /// The tactical layer's answer played.
    pub drawn_rules: u64,
    /// A neighbor played.
    pub drawn_neighbor: u64,
    /// The head's answer played.
    pub drawn_head: u64,
    /// Decisions where the tactical layer differed from the net.
    pub rules_differ: u64,
    /// Neighbors offered, summed.
    pub neighbors: u64,
}

impl TacticianStats4 {
    /// Counts one decision.
    pub fn count(&mut self, record: &TacticianRecord4) {
        self.decisions += 1;
        self.changed += u64::from(record.played != record.net);
        self.rules_differ += u64::from(record.rules != record.net);
        self.neighbors += record.neighbors as u64;
        match record.drawn {
            Drawn4::Net => self.drawn_net += 1,
            Drawn4::Rules => self.drawn_rules += 1,
            Drawn4::Neighbor => self.drawn_neighbor += 1,
            Drawn4::Head => self.drawn_head += 1,
        }
    }
}

/// The tactician a net plays with.
#[derive(Clone, Debug)]
pub struct Tactician4 {
    windows: BTreeSet<Window4>,
    mode: TacticianMode4,
    prior: Arc<LookPrior4>,
    rng: Rng,
    records: Vec<TacticianRecord4>,
}

/// What the tactician reads of one step.
pub(in crate::netplay4) struct TacticianStep<'a> {
    pub(in crate::netplay4) cands: &'a EncodedCandidates4,
    pub(in crate::netplay4) view: &'a PlayerView,
    pub(in crate::netplay4) pending: &'a Pending,
    pub(in crate::netplay4) cand_logits: &'a [f32],
    pub(in crate::netplay4) pair_logits: &'a [f32],
    pub(in crate::netplay4) value: f32,
    pub(in crate::netplay4) embeddings: Option<&'a StepEmbeddings4>,
    pub(in crate::netplay4) active_source: Option<(baylee_core::ids::ObjectId, Option<u32>)>,
    /// The net's sampling stream, cloned: the tactical layer's answer is read
    /// without moving it.
    pub(in crate::netplay4) rng: &'a Rng,
}

impl Tactician4 {
    /// A tactician answering `windows` in `mode`, its look read with `prior`.
    ///
    /// # Errors
    /// An exploration rate outside `[0, 1)` or a negative margin.
    pub fn new(
        windows: BTreeSet<Window4>,
        mode: TacticianMode4,
        prior: Arc<LookPrior4>,
    ) -> anyhow::Result<Self> {
        match &mode {
            TacticianMode4::Explore { epsilon } => {
                anyhow::ensure!((0.0..1.0).contains(epsilon), "ε must lie in [0, 1)");
            }
            TacticianMode4::Play { margin, .. } => {
                anyhow::ensure!(*margin >= 0.0 && margin.is_finite(), "margin must be ≥ 0");
            }
        }
        anyhow::ensure!(!windows.is_empty(), "a tactician answers some window");
        Ok(Self {
            windows,
            mode,
            prior,
            rng: Rng::new(0),
            records: Vec::new(),
        })
    }

    /// The windows it answers.
    #[must_use]
    pub const fn windows(&self) -> &BTreeSet<Window4> {
        &self.windows
    }

    /// Seeds the exploration stream for a game (independent of the net's).
    pub fn reseed(&mut self, seed: u64) {
        self.rng = Rng::new(seed ^ 0x7ac7_1c1a_5eed_0001);
    }

    /// The decisions kept since this was last asked.
    pub fn take_records(&mut self) -> Vec<TacticianRecord4> {
        std::mem::take(&mut self.records)
    }

    /// The answer to play at this step, or `None` when the step is not one
    /// of its windows (or offers nothing to choose).
    ///
    /// # Errors
    /// A graph without embeddings, or an answer that does not decode.
    pub(in crate::netplay4) fn answer(
        &mut self,
        step: &TacticianStep<'_>,
    ) -> anyhow::Result<Option<TeacherTarget4>> {
        let Some(window) = Window4::of(step.pending, step.cands) else {
            return Ok(None);
        };
        let started = std::time::Instant::now();
        if !self.windows.contains(&window) {
            return Ok(None);
        }
        let units = window.units(step.cands);
        if units == 0 {
            return Ok(None);
        }
        let embeddings = step.embeddings.ok_or_else(|| {
            anyhow::anyhow!("the tactician needs a graph exported with --embeddings")
        })?;
        let logits = match window {
            Window4::Block => step.pair_logits,
            Window4::Attack | Window4::Target => step.cand_logits,
        };
        anyhow::ensure!(logits.len() >= units, "fewer logits than units");
        let logits = &logits[..units];
        let net = net_answer(window, step, logits);
        let rules = rules_answer(window, step, logits);
        let neighbors = neighbors(window, step, &net);
        let rows = look::rows(window, step, &net, &rules, logits, &self.prior);
        let d = embeddings.state.len();
        let unit_embeddings = match window {
            Window4::Block => &embeddings.pairs,
            Window4::Attack | Window4::Target => &embeddings.candidates,
        };
        anyhow::ensure!(
            d > 0 && unit_embeddings.len() == units * d,
            "embeddings do not match the question's units"
        );
        let (played, probability, drawn, scores) = match &self.mode {
            TacticianMode4::Explore { epsilon } => {
                let draw = self.rng.unit();
                let (played, probability, drawn) =
                    explore(*epsilon, draw, &net, &rules, &neighbors);
                (played, probability, drawn, None)
            }
            TacticianMode4::Play { head, margin } => {
                let scores = head.score(&embeddings.state, unit_embeddings, &rows, units)?;
                let played = best_supported(&scores, &net, &rules, &neighbors, *margin);
                let drawn = if played == net {
                    Drawn4::Net
                } else {
                    Drawn4::Head
                };
                (played, 1.0, drawn, Some(scores))
            }
        };
        let target = to_target(window, step, &played)?;
        let micros = u64::try_from(started.elapsed().as_micros()).unwrap_or(u64::MAX);
        self.records.push(TacticianRecord4 {
            seat: step.view.seat.get(),
            seq: step.view.seq,
            turn: step.view.turn,
            window,
            value: step.value,
            keys: unit_keys(window, step.cands),
            logits: logits.to_vec(),
            neighbors: neighbors.len(),
            offered: neighbors.clone(),
            net,
            rules,
            played,
            probability,
            drawn,
            micros,
            scores,
            state: embeddings.state.clone(),
            units: unit_embeddings.clone(),
            rows,
        });
        Ok(Some(target))
    }
}

/// A one-hot mask.
fn one_hot(units: usize, index: usize) -> Vec<u8> {
    let mut mask = vec![0; units];
    if let Some(slot) = mask.get_mut(index) {
        *slot = 1;
    }
    mask
}

/// The net's raw greedy answer, as the raw policy plays it at temperature 0.
fn net_answer(window: Window4, step: &TacticianStep<'_>, logits: &[f32]) -> Vec<u8> {
    let mut rng = step.rng.clone();
    match window {
        Window4::Target => one_hot(logits.len(), sample_categorical(logits, 0.0, &mut rng).0),
        Window4::Attack => {
            sample_subset_constrained(logits, step.cands, step.pending, 0.0, &mut rng).0
        }
        Window4::Block => sample_pairs_constrained(logits, step.cands, 0.0, &mut rng).0,
    }
}

/// The tactical layer's answer on the same logits, as `--choice-guard
/// tactical` plays it with no rule group: read on copies, so nothing the net
/// carries moves.
fn rules_answer(window: Window4, step: &TacticianStep<'_>, logits: &[f32]) -> Vec<u8> {
    let mut rng = step.rng.clone();
    let mut rules = TacticRules4::default();
    match window {
        Window4::Target => {
            let (chosen, _) = tactics::rules::single_choice(
                &mut rules,
                step.cands,
                logits,
                step.view,
                step.pending,
                step.active_source,
                0.0,
                &mut rng,
            );
            let mut guard = super::ChoiceGuard4::default();
            guard.tactical = super::TacticalParts {
                priority: true,
                single: true,
                subset: true,
                pair: true,
                scalar: true,
            };
            let chosen = tactics::guard::review(
                &mut guard,
                step.cands,
                logits,
                step.view,
                step.pending,
                chosen,
            );
            one_hot(logits.len(), chosen)
        }
        Window4::Attack => {
            let mut adjusted = choices::adjust_subset_logits(
                step.cands,
                logits,
                step.view,
                step.pending,
                step.active_source,
            );
            tactics::rules::subset(
                &mut rules,
                step.cands,
                &mut adjusted,
                step.view,
                step.pending,
                step.active_source,
            );
            sample_subset_constrained(&adjusted, step.cands, step.pending, 0.0, &mut rng).0
        }
        Window4::Block => {
            let mut adjusted =
                choices::adjust_pair_logits(step.cands, logits, step.view, step.pending);
            tactics::rules::pairs(
                &mut rules,
                step.cands,
                &mut adjusted,
                step.view,
                step.pending,
            );
            sample_pairs_constrained(&adjusted, step.cands, 0.0, &mut rng).0
        }
    }
}

/// The answer `mask` names, as the policy's target.
fn target_of(window: Window4, mask: &[u8]) -> TeacherTarget4 {
    let picks: Vec<u16> = mask
        .iter()
        .enumerate()
        .filter(|&(_, &bit)| bit != 0)
        .map(|(i, _)| i as u16)
        .collect();
    match window {
        Window4::Target => TeacherTarget4 {
            single_idx: picks.first().copied().unwrap_or(0),
            ..TeacherTarget4::default()
        },
        Window4::Attack | Window4::Block => TeacherTarget4 {
            multi_mask: mask.to_vec(),
            free_mask: vec![1; mask.len()],
            pick_sequence: picks,
            ..TeacherTarget4::default()
        },
    }
}

/// The action `mask` names, if it is legal and decodes to exactly `mask`.
fn exact_action(window: Window4, step: &TacticianStep<'_>, mask: &[u8]) -> Option<PlayerAction> {
    let target = target_of(window, mask);
    let action = decode_action4(step.cands, step.view, step.pending, &target)?;
    if step.pending.answer_fault(&action).is_some() {
        return None;
    }
    let encoded = encode_executed_action4(step.cands, step.pending, &action)?;
    let same = match window {
        Window4::Target => encoded.single_idx == target.single_idx,
        Window4::Attack | Window4::Block => encoded.multi_mask == target.multi_mask,
    };
    same.then_some(action)
}

/// The policy target for a played mask.
///
/// # Errors
/// When the mask does not name a legal answer (never for `N`, `R` or a
/// neighbor, each checked or sampled under the question's constraints).
fn to_target(
    window: Window4,
    step: &TacticianStep<'_>,
    mask: &[u8],
) -> anyhow::Result<TeacherTarget4> {
    anyhow::ensure!(
        exact_action(window, step, mask).is_some(),
        "the tactician's {} answer does not decode exactly",
        window.name()
    );
    Ok(target_of(window, mask))
}

/// The legal answers one change from `net`, in a fixed order: another
/// target; one attacker in or out (in at its best-scored defender); one
/// blocker out, or onto another attacker it may block.
fn neighbors(window: Window4, step: &TacticianStep<'_>, net: &[u8]) -> Vec<Vec<u8>> {
    let units = net.len();
    let mut out: Vec<Vec<u8>> = Vec::new();
    let offer = |mask: Vec<u8>, out: &mut Vec<Vec<u8>>| {
        if mask != net && !out.contains(&mask) && exact_action(window, step, &mask).is_some() {
            out.push(mask);
        }
    };
    match window {
        Window4::Target => {
            for i in 0..units {
                offer(one_hot(units, i), &mut out);
            }
        }
        Window4::Attack => {
            let cands = &step.cands.candidates;
            let attackers: BTreeSet<_> = cands.iter().filter_map(|c| c.object_id).collect();
            for attacker in attackers {
                let of: Vec<usize> = (0..units)
                    .filter(|&i| cands[i].object_id == Some(attacker))
                    .collect();
                let mut mask = net.to_vec();
                if of.iter().any(|&i| net[i] != 0) {
                    for &i in &of {
                        mask[i] = 0;
                    }
                } else {
                    let logits = step.cand_logits;
                    let best = of
                        .iter()
                        .copied()
                        .max_by(|&a, &b| logits[a].total_cmp(&logits[b]).then(b.cmp(&a)));
                    if let Some(best) = best {
                        mask[best] = 1;
                    }
                }
                offer(mask, &mut out);
            }
        }
        Window4::Block => {
            let pairs = &step.cands.pair_objects;
            let blockers: BTreeSet<_> = pairs.iter().map(|&(b, _)| b).collect();
            for blocker in blockers {
                let of: Vec<usize> = (0..units).filter(|&i| pairs[i].0 == blocker).collect();
                let mut cleared = net.to_vec();
                for &i in &of {
                    cleared[i] = 0;
                }
                if cleared != net {
                    offer(cleared.clone(), &mut out);
                }
                for &i in &of {
                    if net[i] == 0 {
                        let mut mask = cleared.clone();
                        mask[i] = 1;
                        offer(mask, &mut out);
                    }
                }
            }
        }
    }
    out
}

/// One draw of the exploring behavior, with `draw` uniform in `[0, 1)`: the
/// played answer, its probability under the whole mixture, and the branch
/// that drew it.
#[allow(
    clippy::cast_precision_loss,
    reason = "a question has a handful of neighbors"
)]
fn explore(
    epsilon: f64,
    draw: f64,
    net: &[u8],
    rules: &[u8],
    neighbors: &[Vec<u8>],
) -> (Vec<u8>, f64, Drawn4) {
    let differs = rules != net;
    let to_rules = if differs { epsilon / 3.0 } else { 0.0 };
    let spread = epsilon - to_rules;
    let (to_net, each) = if neighbors.is_empty() {
        (1.0 - epsilon + spread, 0.0)
    } else {
        (1.0 - epsilon, spread / neighbors.len() as f64)
    };
    let probability = |mask: &[u8]| {
        let mut p = 0.0;
        if mask == net {
            p += to_net;
        }
        if differs && mask == rules {
            p += to_rules;
        }
        if neighbors.iter().any(|n| n == mask) {
            p += each;
        }
        p
    };
    if draw < to_net {
        return (net.to_vec(), probability(net), Drawn4::Net);
    }
    if draw < to_net + to_rules {
        return (rules.to_vec(), probability(rules), Drawn4::Rules);
    }
    let index = (((draw - to_net - to_rules) / each.max(f64::MIN_POSITIVE)) as usize)
        .min(neighbors.len().saturating_sub(1));
    match neighbors.get(index) {
        Some(mask) => (mask.clone(), probability(mask), Drawn4::Neighbor),
        None => (net.to_vec(), probability(net), Drawn4::Net),
    }
}

/// The supported answer the head prefers: `N` unless another of `R` and the
/// neighbors scores more than `margin` above it. `scores[0]` is the null row,
/// `scores[1 + i]` unit `i`.
fn best_supported(
    scores: &[f32],
    net: &[u8],
    rules: &[u8],
    neighbors: &[Vec<u8>],
    margin: f32,
) -> Vec<u8> {
    let advantage = |mask: &[u8]| -> f32 {
        scores[0]
            + mask
                .iter()
                .enumerate()
                .filter(|&(_, &bit)| bit != 0)
                .map(|(i, _)| scores[1 + i])
                .sum::<f32>()
    };
    let base = advantage(net);
    let mut best: Option<(f32, &[u8])> = None;
    for mask in std::iter::once(rules).chain(neighbors.iter().map(Vec::as_slice)) {
        if mask == net {
            continue;
        }
        let gain = advantage(mask) - base;
        if gain > margin && best.is_none_or(|(g, _)| gain > g) {
            best = Some((gain, mask));
        }
    }
    best.map_or_else(|| net.to_vec(), |(_, mask)| mask.to_vec())
}

/// One key per unit: what it names, for checking that a reader's rows are
/// this question's.
fn unit_keys(window: Window4, cands: &EncodedCandidates4) -> Vec<String> {
    match window {
        Window4::Block => cands
            .pair_objects
            .iter()
            .map(|(b, a)| format!("{b:?}>{a:?}"))
            .collect(),
        Window4::Attack => cands
            .candidates
            .iter()
            .map(|c| {
                format!(
                    "{}@{}",
                    c.object_id.map_or_else(String::new, |o| format!("{o:?}")),
                    c.defender
                        .map_or_else(String::new, |d| serde_json::to_string(&d)
                            .unwrap_or_default())
                )
            })
            .collect(),
        Window4::Target => cands
            .candidates
            .iter()
            .map(|c| {
                c.action.as_ref().map_or_else(String::new, |a| {
                    serde_json::to_string(a).unwrap_or_default()
                })
            })
            .collect(),
    }
}
