//! `--search K,P,H`: the net's belief-consistent lookahead in evaluation
//! games (`baylee_train::netplay4::search`), and what it did.
use std::collections::BTreeMap;
use std::time::Instant;

use baylee_engine::choice::Pending;
use baylee_gamehost::Session;
use baylee_train::netplay4::Answer4;
use baylee_train::netplay4::search::{SearchConfig4, SearchOutcome4, search_seed};
use baylee_view::PlayerView;
use serde_json::{Value, json};

use super::EvaluationPlayer;

/// What one game's searches did, and what a run's did together.
#[derive(Clone, Debug, Default, PartialEq)]
pub(super) struct SearchTally {
    /// Net decisions the search was offered.
    pub decisions: u64,
    /// Decisions answered by a search.
    pub searched: u64,
    /// Searched decisions that played another answer than the policy's.
    pub changed: u64,
    /// Decisions answered without searching, by reason.
    pub skipped: BTreeMap<String, u64>,
    /// Decisions whose table refused to fork, by the engine's reason.
    pub refused: BTreeMap<String, u64>,
    /// Complete rounds of worlds, over every searched decision.
    pub rounds: u64,
    /// Rollouts, an incomplete round's included.
    pub rollouts: u64,
    /// Actions taken in rollouts past their candidates.
    pub rollout_steps: u64,
    /// Rollouts that reached the end of the game.
    pub terminal_leaves: u64,
    /// Rollouts scored by the value head.
    pub value_leaves: u64,
    /// Wall time of every searched decision, for the report only: no
    /// decision reads it.
    pub ms: Vec<f64>,
}

impl SearchTally {
    /// Adds `other`'s counts to these.
    pub fn add(&mut self, other: &Self) {
        self.decisions += other.decisions;
        self.searched += other.searched;
        self.changed += other.changed;
        for (reason, n) in &other.skipped {
            *self.skipped.entry(reason.clone()).or_default() += n;
        }
        for (reason, n) in &other.refused {
            *self.refused.entry(reason.clone()).or_default() += n;
        }
        self.rounds += other.rounds;
        self.rollouts += other.rollouts;
        self.rollout_steps += other.rollout_steps;
        self.terminal_leaves += other.terminal_leaves;
        self.value_leaves += other.value_leaves;
        self.ms.extend_from_slice(&other.ms);
    }

    /// The counts, and the searched decisions' wall time.
    #[allow(
        clippy::cast_precision_loss,
        reason = "Report statistics over at most millions of decisions"
    )]
    pub fn report(&self) -> Value {
        let mut ms = self.ms.clone();
        ms.sort_by(f64::total_cmp);
        let at = |q: f64| -> f64 {
            ms.get(((ms.len().saturating_sub(1)) as f64 * q).round() as usize)
                .copied()
                .unwrap_or(0.0)
        };
        let total: f64 = ms.iter().sum();
        json!({
            "decisions": self.decisions,
            "searched": self.searched,
            "changed": self.changed,
            "skipped": self.skipped,
            "refused": self.refused,
            "rounds": self.rounds,
            "rollouts": self.rollouts,
            "rollout_steps": self.rollout_steps,
            "terminal_leaves": self.terminal_leaves,
            "value_leaves": self.value_leaves,
            "searched_ms": {
                "total": total,
                "mean": if ms.is_empty() { 0.0 } else { total / ms.len() as f64 },
                "p50": at(0.5),
                "p95": at(0.95),
                "max": ms.last().copied().unwrap_or(0.0),
            },
        })
    }
}

/// One game's search: its budgets, its seed and its counts.
pub(super) struct GameSearch {
    config: SearchConfig4,
    /// Searched decisions a game may have (`0`: no limit).
    max_searched: u64,
    seed: u64,
    pub tally: SearchTally,
}

impl GameSearch {
    pub fn new(config: SearchConfig4, max_searched: u64) -> Self {
        Self {
            config,
            max_searched,
            seed: 0,
            tally: SearchTally::default(),
        }
    }

    /// Starts the game whose net plays on `policy_seed` (the deal's policy
    /// seed, never its shuffle seed): every world of the game is derived
    /// from it.
    pub fn begin(&mut self, policy_seed: u64) {
        self.seed = search_seed(policy_seed);
        self.tally = SearchTally::default();
    }

    /// The net's answer to `view`/`pending` at `table`, searched where the
    /// search applies; `None` once the game's searched decisions are spent,
    /// and the net answers as it would without a search.
    ///
    /// # Errors
    /// When the player cannot search, or fails in a rollout.
    pub fn answer(
        &mut self,
        net: &mut dyn EvaluationPlayer,
        table: &Session,
        view: &PlayerView,
        pending: &Pending,
        teams: &[Option<u8>],
    ) -> anyhow::Result<Option<Answer4>> {
        self.tally.decisions += 1;
        if self.max_searched > 0 && self.tally.searched >= self.max_searched {
            *self.tally.skipped.entry("budget".into()).or_default() += 1;
            return Ok(None);
        }
        let started = Instant::now();
        let searched = net
            .search(table, view, pending, teams, self.config, self.seed)
            .ok_or_else(|| anyhow::anyhow!("this player cannot search"))??;
        match searched.outcome {
            SearchOutcome4::Searched(report) => {
                self.tally.ms.push(started.elapsed().as_secs_f64() * 1000.0);
                self.tally.searched += 1;
                self.tally.changed += u64::from(report.changed);
                self.tally.rounds += report.rounds as u64;
                self.tally.rollouts += report.rollouts;
                self.tally.rollout_steps += report.rollout_steps;
                self.tally.terminal_leaves += report.terminal_leaves;
                self.tally.value_leaves += report.value_leaves;
            }
            SearchOutcome4::Skipped(reason) => {
                *self.tally.skipped.entry(reason.key().into()).or_default() += 1;
            }
            SearchOutcome4::Refused(refusal) => {
                *self
                    .tally
                    .refused
                    .entry(format!("{refusal:?}"))
                    .or_default() += 1;
            }
        }
        Ok(Some(searched.answer))
    }
}

#[cfg(test)]
mod tests;
