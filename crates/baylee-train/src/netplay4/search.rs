//! Belief-consistent lookahead over Playernet v4, for training and
//! evaluation harnesses that host their games in-process
//! (`docs/trained-ai.md` §"Lookahead search").
//!
//! At a priority decision in a main phase or combat, the policy's own
//! answer and its next best legal answers are each tried in worlds forked
//! from the table for the deciding seat ([`Session::fork_for`]): every card
//! the seat cannot see is dealt again from a policy seed, the house plays
//! every other chair, and the net plays the seat on from its own memory at
//! the decision. Each world ends at the game's end, when the seat holds
//! priority in a settled main phase, when its next own turn ends, or after
//! `H` decisions of its own (mana abilities and forced answers do not
//! count), and is scored by the outcome or by the net's value for the seat's
//! side. The policy's answer is played unless the [`Selection4`] finds
//! another one better: by a paired margin over the same worlds, by the
//! policy's prior weighed against the worlds, or (v1) by the best mean.
//!
//! Budgets are counts, never time; every seed is derived from the deal's
//! policy seed, the decision and the world; so a search is a function of
//! its inputs. It is never a seat's path over the gateway.

use std::str::FromStr;

use anyhow::Context as _;
use baylee_core::ids::PlayerId;
use baylee_engine::choice::{Pending, PlayerAction};
use baylee_engine::engine::{DeckKnowledge, ForkRefusal};
use baylee_gamehost::{Session, SimulationSession};
use baylee_view::{Phase, PlayerView};
use serde_json::{Value, json};

use super::{Answer4, NetPlayer4, PlayerSnapshot4, Ranked4, RootDecision4};
use crate::deckgen::Rng;
use crate::features4::{PendingKind4, SourceKind4, StepProvenance4, TeacherTarget4, side_utility};

/// The search's contract name, for run metadata and reports.
pub const SEARCH_CONTRACT: &str = "belief_lookahead_v2";

/// How a search chooses among its candidates once their worlds are played.
#[derive(Clone, Copy, Debug, PartialEq, serde::Serialize)]
#[serde(tag = "rule", rename_all = "snake_case")]
pub enum Selection4 {
    /// The best mean over complete rounds, ties to the policy's answer, then
    /// candidate order (v1).
    Mean,
    /// The policy's answer stays unless another candidate's mean improvement
    /// over it, world by world, exceeds `max(margin, z · se)`, `se` being the
    /// standard error of those paired differences. With fewer than two
    /// rounds there is no standard error, and only `z = 0` can override.
    /// Of the candidates that pass, the largest mean improvement wins.
    Paired {
        /// The smallest mean improvement that counts, in value units.
        margin: f64,
        /// Standard errors the mean improvement must also exceed.
        z: f64,
    },
    /// The PUCT exploitation score with equal visits: a candidate's mean
    /// plus `c · prior / (1 + n)`, its prior the policy's probability of it
    /// (softmax of the raw logits over every legal answer) and `n` the
    /// complete rounds. The policy's preference fades as worlds accumulate.
    Prior {
        /// Weight of the prior.
        c: f64,
    },
}

impl Default for Selection4 {
    fn default() -> Self {
        Self::Paired {
            margin: 0.05,
            z: 1.0,
        }
    }
}

impl Selection4 {
    /// Checks that every parameter is a finite, non-negative number.
    ///
    /// # Errors
    /// When one is not.
    pub fn validate(self) -> anyhow::Result<Self> {
        let parameters: &[f64] = match &self {
            Self::Mean => &[],
            Self::Paired { margin, z } => &[*margin, *z],
            Self::Prior { c } => &[*c],
        };
        anyhow::ensure!(
            parameters.iter().all(|x| x.is_finite() && *x >= 0.0),
            "search selection parameters must be finite and non-negative"
        );
        Ok(self)
    }
}

/// One searched decision's budgets (counts only, never time) and how it
/// chooses.
#[derive(Clone, Copy, Debug, PartialEq, serde::Serialize)]
pub struct SearchConfig4 {
    candidates: usize,
    particles: usize,
    horizon: usize,
    selection: Selection4,
}

impl SearchConfig4 {
    /// The most candidates one decision compares.
    pub const MAX_CANDIDATES: usize = 32;
    /// The most worlds one candidate is tried in.
    pub const MAX_PARTICLES: usize = 256;
    /// The most decisions of the searching seat one rollout counts.
    pub const MAX_HORIZON: usize = 10_000;

    /// `candidates` answers (K, the policy's own first), each tried in
    /// `particles` worlds (P), each rolled out at most `horizon` decisions
    /// of the searching seat past the answer (H; `0` scores the answer's own
    /// position), chosen among by the default [`Selection4`].
    ///
    /// # Errors
    /// When K or P is zero or any budget is past its maximum.
    pub fn new(candidates: usize, particles: usize, horizon: usize) -> anyhow::Result<Self> {
        anyhow::ensure!(
            (1..=Self::MAX_CANDIDATES).contains(&candidates),
            "search candidates must be 1..={}",
            Self::MAX_CANDIDATES
        );
        anyhow::ensure!(
            (1..=Self::MAX_PARTICLES).contains(&particles),
            "search particles must be 1..={}",
            Self::MAX_PARTICLES
        );
        anyhow::ensure!(
            horizon <= Self::MAX_HORIZON,
            "search horizon must be at most {}",
            Self::MAX_HORIZON
        );
        Ok(Self {
            candidates,
            particles,
            horizon,
            selection: Selection4::default(),
        })
    }

    /// The same budgets, chosen among by `selection`.
    ///
    /// # Errors
    /// When a selection parameter is negative or not finite.
    pub fn with_selection(self, selection: Selection4) -> anyhow::Result<Self> {
        Ok(Self {
            selection: selection.validate()?,
            ..self
        })
    }

    /// How the candidates are chosen among.
    #[must_use]
    pub const fn selection(&self) -> Selection4 {
        self.selection
    }

    /// K: answers compared, the policy's own first.
    #[must_use]
    pub const fn candidates(&self) -> usize {
        self.candidates
    }

    /// P: worlds each answer is tried in.
    #[must_use]
    pub const fn particles(&self) -> usize {
        self.particles
    }

    /// H: decisions of the searching seat a rollout counts past the answer
    /// (neither forced answers nor mana abilities count).
    #[must_use]
    pub const fn horizon(&self) -> usize {
        self.horizon
    }

    /// What a run records about its search, so a reader knows exactly what
    /// produced every searched answer.
    #[must_use]
    pub fn metadata(&self) -> Value {
        json!({
            "contract": SEARCH_CONTRACT,
            "candidates": self.candidates,
            "particles": self.particles,
            "horizon": self.horizon,
            "horizon_unit": "searching_seat_decisions_neither_forced_nor_mana_abilities",
            "roots": "priority_in_main_phases_and_combat",
            "candidate_order": "policy_answer_then_raw_logit_desc_then_menu_order",
            "decks": "known_to_the_harness",
            "worlds": "session_fork_redealt_from_purpose_tagged_policy_seeds",
            "rollout": "net_for_the_searching_seat_house_for_every_other_chair",
            "stop": "game_over_or_settled_priority_or_end_of_next_own_turn_or_horizon",
            "settled": "seat_would_pass_priority_in_a_main_phase_after_the_roots_phase_empty_stack_no_cast_no_payment_no_floating_mana",
            "leaf": "terminal_side_utility_else_k0_observation_value_of_the_seat",
            "prior": "softmax_of_raw_logits_over_every_legal_answer",
            "selection": self.selection,
            "provenance": "search_without_behavior_logprob",
        })
    }
}

impl FromStr for SearchConfig4 {
    type Err = anyhow::Error;

    /// `K,P,H`, e.g. `4,2,8`.
    fn from_str(text: &str) -> anyhow::Result<Self> {
        let parts: Vec<&str> = text.split(',').map(str::trim).collect();
        let [k, p, h] = parts.as_slice() else {
            anyhow::bail!("search budgets are K,P,H (candidates, particles, horizon)");
        };
        Self::new(
            k.parse().context("search candidates K")?,
            p.parse().context("search particles P")?,
            h.parse().context("search horizon H")?,
        )
    }
}

/// Which of a session's views a player reads, by its encoder.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ViewContext4 {
    /// [`Session::view_for`] and [`Session::observe_view`].
    Plain,
    /// The `*_with_context` views.
    Choice,
    /// The `*_with_payment_context` views.
    Payment,
    /// The `*_with_announcement_context` views.
    Announcement,
}

impl ViewContext4 {
    /// The question `seat` owes in `fork` and its view of it.
    #[must_use]
    pub fn view_for(
        self,
        fork: &SimulationSession,
        seat: PlayerId,
    ) -> Option<(Pending, PlayerView)> {
        match self {
            Self::Plain => fork.view_for(seat),
            Self::Choice => fork.view_for_with_context(seat),
            Self::Payment => fork.view_for_with_payment_context(seat),
            Self::Announcement => fork.view_for_with_announcement_context(seat),
        }
    }

    /// What `seat` observes of `fork` now.
    #[must_use]
    pub fn observe(self, fork: &SimulationSession, seat: PlayerId) -> PlayerView {
        match self {
            Self::Plain => fork.observe_view(seat),
            Self::Choice => fork.observe_view_with_context(seat),
            Self::Payment => fork.observe_view_with_payment_context(seat),
            Self::Announcement => fork.observe_view_with_announcement_context(seat),
        }
    }
}

/// What a search needs of the player it searches for. [`NetPlayer4`] is
/// the one that plays; a test stands in for it without a model.
pub trait SearchPlayer4 {
    /// Everything per game the player changes while it plays.
    type Snapshot;

    /// The player's per-game state as it stands.
    fn snapshot(&self) -> Self::Snapshot;
    /// Undoes everything done since `snapshot` was taken.
    fn restore(&mut self, snapshot: &Self::Snapshot);
    /// Which views the player reads.
    fn view_context(&self) -> ViewContext4;
    /// Takes in a view, as the host has the player do before every action.
    ///
    /// # Errors
    /// When the player cannot observe (its model failed).
    fn observe(&mut self, view: &PlayerView) -> anyhow::Result<()>;
    /// The player's value for `seat`'s side from `seat`'s latest observation.
    fn observed_value(&self, seat: PlayerId) -> Option<f32>;
    /// Decides as in play, up to noting the answer, with the ranking of
    /// the legal answers when the answer is the policy's own.
    fn decide(
        &mut self,
        view: &PlayerView,
        pending: &Pending,
        teams: &[Option<u8>],
    ) -> RootDecision4;
    /// Finishes `decision` with `action` as its answer.
    fn conclude(&mut self, view: &PlayerView, decision: &RootDecision4, action: &PlayerAction);
    /// Answers a question as in play.
    fn answer(&mut self, view: &PlayerView, pending: &Pending, teams: &[Option<u8>]) -> Answer4;
    /// Restarts the player's sampling stream from `seed`, keeping the game.
    fn reseed_sampling(&mut self, seed: u64);
    /// Counts a decision recorded under `from` as searched instead.
    fn attribute_to_search(&mut self, pending: &Pending, from: SourceKind4);
}

impl SearchPlayer4 for NetPlayer4 {
    type Snapshot = PlayerSnapshot4;

    fn snapshot(&self) -> PlayerSnapshot4 {
        Self::snapshot(self)
    }
    fn restore(&mut self, snapshot: &PlayerSnapshot4) {
        Self::restore(self, snapshot);
    }
    fn view_context(&self) -> ViewContext4 {
        if self.uses_announcement_context() {
            ViewContext4::Announcement
        } else if self.uses_payment_context() {
            ViewContext4::Payment
        } else if self.is_encoder7() {
            ViewContext4::Choice
        } else {
            ViewContext4::Plain
        }
    }
    fn observe(&mut self, view: &PlayerView) -> anyhow::Result<()> {
        Self::observe(self, view)
    }
    fn observed_value(&self, seat: PlayerId) -> Option<f32> {
        Self::observed_value(self, seat)
    }
    fn decide(
        &mut self,
        view: &PlayerView,
        pending: &Pending,
        teams: &[Option<u8>],
    ) -> RootDecision4 {
        Self::decide(self, view, pending, teams, true)
    }
    fn conclude(&mut self, view: &PlayerView, decision: &RootDecision4, action: &PlayerAction) {
        Self::conclude(self, view, decision, action);
    }
    fn answer(&mut self, view: &PlayerView, pending: &Pending, teams: &[Option<u8>]) -> Answer4 {
        self.act_detailed(view, pending, teams)
    }
    fn reseed_sampling(&mut self, seed: u64) {
        Self::reseed_sampling(self, seed);
    }
    fn attribute_to_search(&mut self, pending: &Pending, from: SourceKind4) {
        let row = &mut self.coverage.counts[PendingKind4::of(pending) as usize];
        row[from as usize] = row[from as usize].saturating_sub(1);
        row[SourceKind4::Search as usize] += 1;
    }
}

/// The table a search forks its worlds from.
pub trait ForkSource4 {
    /// A world forked for `seat` from `policy_seed`.
    ///
    /// # Errors
    /// The engine's refusal of the position.
    fn fork(&self, seat: PlayerId, policy_seed: u64) -> Result<SimulationSession, ForkRefusal>;
    /// The table's [`Session::decision_seq`], which seeds the worlds.
    fn decision_seq(&self) -> u64;
}

/// A session whose every deck list the harness asserts the searching seat
/// may know ([`DeckKnowledge::Known`]): the decks an evaluation chose, or
/// published lists. The worlds are dealt from the real remaining decks.
pub struct KnownDecks<'a>(pub &'a Session);

impl ForkSource4 for KnownDecks<'_> {
    fn fork(&self, seat: PlayerId, policy_seed: u64) -> Result<SimulationSession, ForkRefusal> {
        self.0.fork_for(seat, DeckKnowledge::Known, policy_seed)
    }
    fn decision_seq(&self) -> u64 {
        self.0.decision_seq()
    }
}

/// Why a decision was answered without a search.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, serde::Serialize)]
#[serde(rename_all = "snake_case")]
pub enum SearchSkip4 {
    /// The question is not ordinary priority.
    NotPriority,
    /// Priority outside a main phase and combat.
    Phase,
    /// The answer is not the policy's own choice: forced, unsupported,
    /// guarded against a repeated view, or a fallback.
    NotPolicy,
    /// A candidate was refused in its world before one round completed.
    CandidateRefused,
}

impl SearchSkip4 {
    /// A stable name for counts.
    #[must_use]
    pub const fn key(self) -> &'static str {
        match self {
            Self::NotPriority => "not_priority",
            Self::Phase => "phase",
            Self::NotPolicy => "not_policy",
            Self::CandidateRefused => "candidate_refused",
        }
    }
}

/// What one searched decision found.
#[derive(Clone, Debug, PartialEq, serde::Serialize)]
pub struct SearchReport4 {
    /// Candidates compared (at most K; fewer when fewer answers are legal).
    pub candidates: usize,
    /// The candidates in candidate order, the policy's own answer first.
    pub actions: Vec<PlayerAction>,
    /// Complete rounds: worlds in which every candidate was rolled out.
    pub rounds: usize,
    /// The chosen candidate's place in candidate order (`0`: the policy's).
    pub chosen: usize,
    /// Whether the search played another answer than the policy's.
    pub changed: bool,
    /// Each candidate's mean score over the complete rounds.
    pub means: Vec<f64>,
    /// Each candidate's mean improvement over the policy's answer, world by
    /// world (`0` for the policy's answer itself).
    pub diffs: Vec<f64>,
    /// The standard error of each improvement; `None` with fewer than two
    /// rounds, and for the policy's answer.
    pub ses: Vec<Option<f64>>,
    /// Each candidate's prior: the policy's probability of it.
    pub priors: Vec<f64>,
    /// The score the selection ranked each candidate by.
    pub scores: Vec<f64>,
    /// Rollouts run, those of an incomplete round included.
    pub rollouts: u64,
    /// Actions taken in rollouts past their candidates, every chair's.
    pub rollout_steps: u64,
    /// Decisions of the searching seat that counted toward the horizon.
    pub seat_decisions: u64,
    /// How the rollouts ended.
    pub leaves: LeafCounts4,
}

/// How rollouts ended, by kind. All but `terminal` are scored by the value
/// head.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, serde::Serialize)]
pub struct LeafCounts4 {
    /// The game ended.
    pub terminal: u64,
    /// `H = 0`: the candidate's own position.
    pub immediate: u64,
    /// The searching seat held priority in a settled position.
    pub settled: u64,
    /// The horizon's decisions were spent.
    pub horizon: u64,
    /// The searching seat's next own turn ended.
    pub turn_end: u64,
    /// Nobody could answer, an answer was refused, or the action cap was
    /// reached.
    pub stopped: u64,
}

impl LeafCounts4 {
    /// Adds `other`'s counts to these.
    pub fn add(&mut self, other: &Self) {
        self.terminal += other.terminal;
        self.immediate += other.immediate;
        self.settled += other.settled;
        self.horizon += other.horizon;
        self.turn_end += other.turn_end;
        self.stopped += other.stopped;
    }

    /// Rollouts scored by the value head.
    #[must_use]
    pub const fn value(&self) -> u64 {
        self.immediate + self.settled + self.horizon + self.turn_end + self.stopped
    }
}

/// How a decision was answered.
#[derive(Clone, Debug, PartialEq)]
pub enum SearchOutcome4 {
    /// Searched.
    Searched(Box<SearchReport4>),
    /// Answered by the policy without searching.
    Skipped(SearchSkip4),
    /// Answered by the policy: the table refused to fork.
    Refused(ForkRefusal),
}

/// A searched decision's answer and how it was found.
#[derive(Clone, Debug)]
pub struct SearchedAnswer4 {
    /// The answer to play. `SourceKind4::Search` and no behavior
    /// probability when searched; the policy's own answer otherwise.
    pub answer: Answer4,
    /// How it was found.
    pub outcome: SearchOutcome4,
}

/// Purpose tags of the seeds a search derives.
const SEARCH: u64 = u64::from_be_bytes(*b"search\0\0");
const WORLD: u64 = u64::from_be_bytes(*b"world\0\0\0");
const ROLLOUT: u64 = u64::from_be_bytes(*b"rollout\0");

/// `SplitMix64` over `seed`, a purpose and two indices: one independent
/// seed per purpose and position.
fn derive(seed: u64, purpose: u64, a: u64, b: u64) -> u64 {
    let first = Rng::new(seed ^ purpose).next();
    let second = Rng::new(first ^ a).next();
    Rng::new(second ^ b.rotate_left(17)).next()
}

/// A game's search seed, from the deal's policy seed (never the deal's
/// shuffle seed).
#[must_use]
pub fn search_seed(policy_seed: u64) -> u64 {
    derive(policy_seed, SEARCH, 0, 0)
}

/// The policy seed of world `particle` at the decision `decision_seq`.
fn world_seed(search_seed: u64, decision_seq: u64, particle: usize) -> u64 {
    derive(search_seed, WORLD, decision_seq, particle as u64)
}

/// Why `pending` is not a root this version searches.
fn root_skip(view: &PlayerView, pending: &Pending) -> Option<SearchSkip4> {
    if !matches!(pending, Pending::Priority { .. }) {
        return Some(SearchSkip4::NotPriority);
    }
    match view.phase {
        Phase::FirstMain | Phase::Combat | Phase::SecondMain => None,
        Phase::Beginning | Phase::Ending => Some(SearchSkip4::Phase),
    }
}

/// The policy's own answer, then the next `k - 1` legal answers by its
/// ranking.
fn candidates(decision: &RootDecision4, k: usize) -> Vec<Ranked4> {
    let own = &decision.answer.action;
    let ranking = decision.ranking.as_deref().unwrap_or_default();
    let first = ranking
        .iter()
        .find(|r| r.action == *own)
        .cloned()
        .unwrap_or_else(|| Ranked4 {
            index: usize::from(decision.answer.target.single_idx),
            action: own.clone(),
            logit: f32::NEG_INFINITY,
        });
    let mut out = vec![first];
    out.extend(
        ranking
            .iter()
            .filter(|r| r.action != *own)
            .take(k.saturating_sub(1))
            .cloned(),
    );
    out
}

/// The side utility of a finished game for `seat`, as the evaluator scores
/// it: `1` for a win of its side, `0` for a loss, a draw shared among the
/// sides at the table.
#[must_use]
pub fn terminal_utility(seat: u8, teams: &[Option<u8>], winners: &[u8], seats: usize) -> f32 {
    let surviving: Vec<u8> = if winners.is_empty() {
        (0..seats).filter_map(|s| u8::try_from(s).ok()).collect()
    } else {
        winners.to_vec()
    };
    side_utility(seat, teams, &surviving, winners)
}

/// The policy's answer, finished as play finishes it, and how it came about.
fn unsearched<P: SearchPlayer4>(
    player: &mut P,
    view: &PlayerView,
    decision: RootDecision4,
    outcome: SearchOutcome4,
) -> SearchedAnswer4 {
    player.conclude(view, &decision, &decision.answer.action);
    SearchedAnswer4 {
        answer: decision.answer,
        outcome,
    }
}

/// Answers `seat`'s question `pending` (`view` is its view of it) with a
/// lookahead where this version searches, and as `player` would without one
/// everywhere else.
///
/// `player` must have observed `view`'s table, as the host has it do before
/// every answer. Searched: K candidates (the policy's answer first), P
/// complete rounds of worlds forked from `table` with seeds derived from
/// `search_seed`, the decision and the world, each candidate rolled out at
/// most H actions; the best mean is played. Every world is the same for
/// every candidate (one seed per world), so candidates are compared on the
/// same deals. `player` ends exactly as if it had answered the chosen action
/// itself (only its coverage counts the decision as searched); `table` is
/// not changed.
///
/// # Errors
/// When the player fails to observe in a rollout (its model failed).
pub fn search_decision<P: SearchPlayer4, T: ForkSource4>(
    player: &mut P,
    table: &T,
    view: &PlayerView,
    pending: &Pending,
    teams: &[Option<u8>],
    config: SearchConfig4,
    search_seed: u64,
) -> anyhow::Result<SearchedAnswer4> {
    let decision = player.decide(view, pending, teams);
    let skip = root_skip(view, pending)
        .or_else(|| decision.ranking.is_none().then_some(SearchSkip4::NotPolicy));
    if let Some(reason) = skip {
        return Ok(unsearched(
            player,
            view,
            decision,
            SearchOutcome4::Skipped(reason),
        ));
    }
    let candidates = candidates(&decision, config.candidates);
    let root = player.snapshot();
    let rounds = Rounds {
        view,
        teams,
        decision: &decision,
        candidates: &candidates,
        root: &root,
    }
    .play(player, table, config, search_seed);
    player.restore(&root);
    let Played { mut report, worlds } = match rounds {
        Ok(Some(played)) => played,
        Ok(None) => {
            return Ok(unsearched(
                player,
                view,
                decision,
                SearchOutcome4::Skipped(SearchSkip4::CandidateRefused),
            ));
        }
        Err(Stop::Refused(refusal)) => {
            return Ok(unsearched(
                player,
                view,
                decision,
                SearchOutcome4::Refused(refusal),
            ));
        }
        Err(Stop::Failed(error)) => return Err(error),
    };
    report.priors = select::priors(&decision, &candidates);
    let choice = select::choose(config.selection, &worlds, &report.priors);
    report.chosen = choice.chosen;
    report.means = choice.means;
    report.diffs = choice.diffs;
    report.ses = choice.ses;
    report.scores = choice.scores;
    report.changed = report.chosen != 0;
    let pick = &candidates[report.chosen];
    player.conclude(view, &decision, &pick.action);
    player.attribute_to_search(pending, decision.answer.provenance.source);
    let own = decision.answer;
    let target = if report.changed {
        TeacherTarget4 {
            single_idx: u16::try_from(pick.index).unwrap_or(u16::MAX),
            ..TeacherTarget4::default()
        }
    } else {
        own.target
    };
    Ok(SearchedAnswer4 {
        answer: Answer4 {
            action: pick.action.clone(),
            value: own.value,
            provenance: StepProvenance4 {
                source: SourceKind4::Search,
                temperature: own.provenance.temperature,
                thinking_steps: own.provenance.thinking_steps,
                behavior_logprob: 0.0,
                has_logprob: false,
            },
            target,
        },
        outcome: SearchOutcome4::Searched(Box::new(report)),
    })
}

mod rollout;
use rollout::{Played, Rounds, Stop};
mod select;

#[cfg(test)]
mod tests;
