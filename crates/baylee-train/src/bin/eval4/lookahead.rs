//! `--search K,P,H`: the net's belief-consistent lookahead in evaluation
//! games (`baylee_train::netplay4::search`), and what it did.
use std::collections::BTreeMap;
use std::time::Instant;

use baylee_engine::choice::{Pending, PlayerAction};
use baylee_gamehost::Session;
use baylee_train::netplay4::Answer4;
use baylee_train::netplay4::search::{LeafCounts4, SearchConfig4, SearchOutcome4, search_seed};
use baylee_view::PlayerView;
use serde_json::{Value, json};

use super::EvaluationPlayer;

/// An action's kind: its variant's name (`PlayLand`, `PassPriority`, …).
pub(super) fn action_kind(action: &PlayerAction) -> String {
    match serde_json::to_value(action) {
        Ok(Value::String(kind)) => kind,
        Ok(Value::Object(fields)) => fields.keys().next().cloned().unwrap_or_default(),
        _ => String::from("unknown"),
    }
}

fn bump(counts: &mut BTreeMap<String, u64>, key: String) {
    *counts.entry(key).or_default() += 1;
}

fn merge(into: &mut BTreeMap<String, u64>, from: &BTreeMap<String, u64>) {
    for (key, n) in from {
        *into.entry(key.clone()).or_default() += n;
    }
}

/// What one game's searches did, and what a run's did together.
#[derive(Clone, Debug, Default, PartialEq)]
pub(super) struct SearchTally {
    /// Net decisions the search was offered.
    pub decisions: u64,
    /// Decisions answered by a search.
    pub searched: u64,
    /// Searched decisions that played another answer than the policy's.
    pub changed: u64,
    /// Searched decisions by the kind of the policy's own answer.
    pub searched_by_kind: BTreeMap<String, u64>,
    /// Changed decisions by `policy kind -> played kind`.
    pub changes: BTreeMap<String, u64>,
    /// Decisions answered without searching, by reason.
    pub skipped: BTreeMap<String, u64>,
    /// Decisions whose table refused to fork, by the engine's reason.
    pub refused: BTreeMap<String, u64>,
    /// Complete rounds of worlds, over every searched decision.
    pub rounds: u64,
    /// Rollouts, an incomplete round's included.
    pub rollouts: u64,
    /// Actions taken in rollouts past their candidates, every chair's.
    pub rollout_steps: u64,
    /// Decisions of the searching seat that counted toward the horizon.
    pub seat_decisions: u64,
    /// How the rollouts ended.
    pub leaves: LeafCounts4,
    /// Searched decisions whose rollouts ran at once, on twins of the net.
    pub parallel: u64,
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
        merge(&mut self.searched_by_kind, &other.searched_by_kind);
        merge(&mut self.changes, &other.changes);
        merge(&mut self.skipped, &other.skipped);
        merge(&mut self.refused, &other.refused);
        self.rounds += other.rounds;
        self.rollouts += other.rollouts;
        self.rollout_steps += other.rollout_steps;
        self.seat_decisions += other.seat_decisions;
        self.leaves.add(&other.leaves);
        self.parallel += other.parallel;
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
            "searched_by_kind": self.searched_by_kind,
            "changes": self.changes,
            "skipped": self.skipped,
            "refused": self.refused,
            "rounds": self.rounds,
            "rollouts": self.rollouts,
            "rollout_steps": self.rollout_steps,
            "seat_decisions": self.seat_decisions,
            "leaves": self.leaves,
            "terminal_leaves": self.leaves.terminal,
            "value_leaves": self.leaves.value(),
            "parallel": self.parallel,
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

/// One game's search: its budgets, its seed, its counts and what each
/// searched decision compared.
pub(super) struct GameSearch {
    config: SearchConfig4,
    /// Searched decisions a game may have (`0`: no limit).
    max_searched: u64,
    seed: u64,
    pub tally: SearchTally,
    /// Per searched decision: where its answer is in the record, the
    /// candidates in order (the policy's first), their means, paired
    /// improvements and standard errors, priors, selection scores and the
    /// choice. Bound to the record's inputs like a fallback event
    /// (`diagnostics::bind_fallback_events`).
    pub events: Vec<Value>,
}

impl GameSearch {
    pub fn new(config: SearchConfig4, max_searched: u64) -> Self {
        Self {
            config,
            max_searched,
            seed: 0,
            tally: SearchTally::default(),
            events: Vec::new(),
        }
    }

    /// Starts the game whose net plays on `policy_seed` (the deal's policy
    /// seed, never its shuffle seed): every world of the game is derived
    /// from it.
    pub fn begin(&mut self, policy_seed: u64) {
        self.seed = search_seed(policy_seed);
        self.tally = SearchTally::default();
        self.events.clear();
    }

    /// The table refused the last searched answer: its event names no
    /// recorded input.
    pub fn refused(&mut self) {
        if let Some(event) = self.events.last_mut() {
            event["accepted"] = json!(false);
        }
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
            bump(&mut self.tally.skipped, "budget".into());
            return Ok(None);
        }
        let started = Instant::now();
        let searched = net
            .search(table, view, pending, teams, self.config, self.seed)
            .ok_or_else(|| anyhow::anyhow!("this player cannot search"))??;
        match searched.outcome {
            SearchOutcome4::Searched(report) => {
                self.tally.ms.push(started.elapsed().as_secs_f64() * 1000.0);
                self.events.push(json!({
                    "decision_seq": table.decision_seq(),
                    "record_byte_offset": table.record_pending(),
                    "seat": view.seat.get(),
                    "action": searched.answer.action,
                    "accepted": true,
                    "candidates": report.actions,
                    "means": report.means,
                    "diffs": report.diffs,
                    "ses": report.ses,
                    "priors": report.priors,
                    "scores": report.scores,
                    "chosen": report.chosen,
                    "rounds": report.rounds,
                    "leaves": report.leaves,
                }));
                let own = report.actions.first().map(action_kind).unwrap_or_default();
                if report.changed {
                    let played = action_kind(&searched.answer.action);
                    bump(&mut self.tally.changes, format!("{own}->{played}"));
                }
                bump(&mut self.tally.searched_by_kind, own);
                self.tally.searched += 1;
                self.tally.changed += u64::from(report.changed);
                self.tally.rounds += report.rounds as u64;
                self.tally.rollouts += report.rollouts;
                self.tally.rollout_steps += report.rollout_steps;
                self.tally.seat_decisions += report.seat_decisions;
                self.tally.leaves.add(&report.leaves);
                self.tally.parallel += u64::from(report.parallel);
            }
            SearchOutcome4::Skipped(reason) => {
                bump(&mut self.tally.skipped, reason.key().into());
            }
            SearchOutcome4::Refused(refusal) => {
                bump(&mut self.tally.refused, format!("{refusal:?}"));
            }
        }
        Ok(Some(searched.answer))
    }
}

#[cfg(test)]
mod tests;
