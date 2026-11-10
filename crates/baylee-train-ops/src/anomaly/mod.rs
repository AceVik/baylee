//! Anomaly reports of finished runs (`anomaly4`).
//!
//! Every eval4, selfplay and ladder4 run already counts what went wrong in
//! its games: a refused answer, a house fallback in the net's seat, a game
//! that hit the answer cap, a refused search root, a seat that timed out.
//! This reads those counts from the run's result (and its job's stderr, its
//! seat logs, its records) and lists every one with what replays it: the
//! seed, the deal, the table and the seat. It plays no game and changes no
//! number. Its one active check is a replay-exact audit of about one
//! recorded game in 64, chosen by seed, with the converting build's
//! `convert4`, whose first step is `record::replay_exact`.
//!
//! An entry is `new` when it points at an engine or seat bug and should go
//! to `bug-harvest.md`, `known` when it is a class already named (the
//! activator boundary, a fixed search refusal, the gateway's sign-in
//! limit), and `expected` when the game does it by design (a search root
//! refused over an operation in flight, house fallbacks below a threshold).

mod eval;
mod harvest;
mod ladder;
mod replay;
mod report;
mod run;

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use serde_json::Value;

pub use eval::{Job, from_eval4, from_job, job_of, panics_in};
pub use harvest::append as harvest;
pub use ladder::from_ladder;
pub use replay::{Replay, audit, sampled};
pub use report::{markdown, write};
pub use run::from_run;

/// The report's format; bumped when a field changes meaning.
pub const VERSION: u32 = 1;

/// What kind of anomaly an entry is.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Kind {
    /// A panic in the run (stderr, or a selfplay game's outcome).
    Panic,
    /// A search root the table refused to fork, by the engine's reason.
    RefusedRoot,
    /// An answer the engine refused.
    EngineRefusal,
    /// The house answered in the net's seat.
    HouseFallback,
    /// The net's seat met the same view more than four times
    /// (`outer_repeated_view`): a stall the house broke.
    Stall,
    /// A game hit the answer cap.
    AnswerCap,
    /// A game that never ended otherwise: a wall-time cap, a record that
    /// stops short, a ladder game with no end.
    NoEnd,
    /// A ladder seat that ran out of the game's time.
    SeatTimeout,
    /// A ladder seat or host that failed.
    SeatError,
    /// A sampled record whose exact replay diverged.
    ReplayDivergence,
    /// A sampled record whose replay refused one of its inputs.
    ReplayRefusal,
    /// The replay audit panicked.
    ReplayPanic,
    /// A sampled record written before the activator boundary (eb40dc593).
    Boundary,
    /// A job that ended without its result.
    RunFailed,
}

impl Kind {
    /// The words `bug-harvest.md` uses for it.
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::RunFailed => "run failed",
            Self::Panic => "panic",
            Self::RefusedRoot => "refused root",
            Self::EngineRefusal => "engine refusal",
            Self::HouseFallback => "house fallback",
            Self::Stall => "stall",
            Self::AnswerCap => "answer cap",
            Self::NoEnd => "no end",
            Self::SeatTimeout => "seat timeout",
            Self::SeatError => "seat error",
            Self::ReplayDivergence => "replay-exact divergence",
            Self::ReplayRefusal => "replay refusal",
            Self::ReplayPanic => "replay panic",
            Self::Boundary => "activator boundary",
        }
    }
}

/// How an entry is treated.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Class {
    /// Points at an engine or seat bug: harvested.
    New,
    /// A class already named and tracked.
    Known,
    /// What the game does by design, or below the threshold.
    Expected,
}

/// One anomaly, with what replays it.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Entry {
    /// What it is.
    pub kind: Kind,
    /// Whether it is harvested.
    pub class: Class,
    /// The deal (eval4), the game's number (a run's `i`, a ladder game).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub deal: Option<u64>,
    /// The deal's seed (`--first-seed` plus the table).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub seed: Option<u64>,
    /// The table the deal was played at.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub table: Option<u64>,
    /// The seat it happened to: the net's, or the one named.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub seat: Option<u64>,
    /// What happened, in the run's own words.
    pub what: String,
    /// How many times, where one entry stands for several.
    #[serde(default = "one", skip_serializing_if = "is_one")]
    pub count: u64,
    /// A command that plays the deal again, where the run says how.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reproduce: Option<String>,
}

const fn one() -> u64 {
    1
}

#[allow(clippy::trivially_copy_pass_by_ref)] // serde's skip predicate takes a reference
const fn is_one(count: &u64) -> bool {
    *count == 1
}

impl Entry {
    /// An entry with no place in a game yet.
    #[must_use]
    pub fn new(kind: Kind, class: Class, what: impl Into<String>) -> Self {
        Self {
            kind,
            class,
            deal: None,
            seed: None,
            table: None,
            seat: None,
            what: what.into(),
            count: 1,
            reproduce: None,
        }
    }

    /// The entry at a deal of an eval4 run, or a game of another.
    #[must_use]
    pub const fn at(mut self, deal: Option<u64>, seed: Option<u64>, table: Option<u64>) -> Self {
        self.deal = deal;
        self.seed = seed;
        self.table = table;
        self
    }

    /// The entry at a seat.
    #[must_use]
    pub const fn seat(mut self, seat: Option<u64>) -> Self {
        self.seat = seat;
        self
    }

    /// The entry standing for `count` occurrences.
    #[must_use]
    pub const fn times(mut self, count: u64) -> Self {
        self.count = count;
        self
    }
}

/// A run's anomalies.
#[derive(Clone, Debug, Serialize)]
pub struct Report {
    /// [`VERSION`].
    pub anomaly_version: u32,
    /// The run's name: the result's file or directory name.
    pub run: String,
    /// The result read.
    pub result: PathBuf,
    /// `eval4`, `run` (a selfplay or eval4 record directory) or `ladder4`.
    pub source: &'static str,
    /// The build that played it, as the run says.
    pub build: Option<String>,
    /// Games (deals) the result covers.
    pub games: u64,
    /// The run's own counters, as it wrote them.
    pub totals: BTreeMap<String, Value>,
    /// Entries by kind and class.
    pub counts: BTreeMap<String, u64>,
    /// Every anomaly, in the order the result holds them.
    pub entries: Vec<Entry>,
    /// The replay-exact audit.
    pub replay: Replay,
    /// Which of the engine's own checks the replay ran.
    pub invariants: String,
    /// What would be useful and the run does not record.
    pub not_recorded: Vec<String>,
}

/// What the replay audit can and cannot check, the same for every report.
pub const INVARIANTS: &str = "The sampled replay applies every recorded input with the converting \
build's engine and compares its snapshot hash after each one (record::replay_exact), so it runs \
the engine's always-on checks: the answerable-question assertion every accepted answer settles \
(Engine::settle_question), the numeric-capacity refusal and the generated-mana dead end. Debug \
assertions and the fuzz feature's checks (Engine::projection_is_fresh, the refusal fingerprint) are \
not compiled into a runtime build, so a replay cannot run them.";

impl Report {
    /// An empty report of `result`.
    #[must_use]
    pub fn new(result: &Path, source: &'static str) -> Self {
        Self {
            anomaly_version: VERSION,
            run: run_name(result),
            result: result.to_path_buf(),
            source,
            build: None,
            games: 0,
            totals: BTreeMap::new(),
            counts: BTreeMap::new(),
            entries: Vec::new(),
            replay: Replay::not_run("not asked"),
            invariants: INVARIANTS.to_owned(),
            not_recorded: Vec::new(),
        }
    }

    /// Recounts `counts` from the entries: `kind` and `kind/class`.
    pub fn recount(&mut self) {
        self.counts.clear();
        for entry in &self.entries {
            let kind = serde_json::to_value(entry.kind)
                .ok()
                .and_then(|v| v.as_str().map(str::to_owned))
                .unwrap_or_default();
            let class = serde_json::to_value(entry.class)
                .ok()
                .and_then(|v| v.as_str().map(str::to_owned))
                .unwrap_or_default();
            *self.counts.entry(kind.clone()).or_default() += entry.count;
            *self.counts.entry(format!("{kind}/{class}")).or_default() += entry.count;
        }
    }

    /// The entries that point at a bug.
    pub fn new_entries(&self) -> impl Iterator<Item = &Entry> {
        self.entries.iter().filter(|e| e.class == Class::New)
    }
}

/// A result's run name: its file stem, or its directory's name.
#[must_use]
pub fn run_name(result: &Path) -> String {
    let named = if result.is_dir() || result.extension().is_none() {
        result.file_name()
    } else {
        result.file_stem()
    };
    named.map_or_else(
        || "run".to_owned(),
        |name| name.to_string_lossy().into_owned(),
    )
}

/// The engine's reason for refusing a search root, as eval4 counts it.
///
/// Two reasons were engine bugs, now fixed: what an activation captured
/// outliving it (95c26f029) and its activator outliving it (eb40dc593).
/// Every other names an operation in flight, which the fork refuses by
/// design.
#[must_use]
pub fn root_class(reason: &str) -> (Class, &'static str) {
    if reason.contains("activating_abilities") || reason.contains("activating_grant") {
        (Class::Known, "stale activation capture, fixed in 95c26f029")
    } else if reason.contains("activation_target_choice") {
        (Class::Known, "stale activator, fixed in eb40dc593")
    } else {
        (Class::Expected, "an operation in flight")
    }
}

#[cfg(test)]
mod tests;
