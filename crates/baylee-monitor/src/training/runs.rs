//! LLM game runs and the ladder, as cards: what the collectors and the ladder
//! tool write into their own status files, and nothing else.
//!
//! A collector root (`llm-<name>` below an `--llm-root`, or one directory
//! below that) holds `controller.json` and `status.json`; `<root>/DRAIN` and
//! `<root>/STOP` are only looked at for existing. The `attempts`, `ledgers`
//! and `gateway` directories, `stats.json` (a copy of the status with more
//! detail), the profiles file (command paths), logs and key files are never
//! opened, and no process environment or command line is read. The ladder is
//! one `ladder.json` named with `--ladder`: its `summary` per arm and the
//! phase of each game.
//!
//! A status file of a long run is over 100 KiB, so a run is parsed again only
//! when its size or modification time changed.

use super::{
    detail::unit_of,
    safe::{clean, read_json, unix_ms, valid_name},
};
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    fs,
    path::{Path, PathBuf},
    time::SystemTime,
};

const MAX_CONTROLLER: u64 = 16 * 1024;
const MAX_STATUS: u64 = 2 * 1024 * 1024;
const MAX_LADDER: u64 = 1024 * 1024;
const MAX_MODELS: usize = 4;
const MAX_ARMS: usize = 12;
/// Directory entries read per level when looking for collector roots.
const MAX_CHILDREN: usize = 256;
const MAX_SUBDIRS: usize = 32;
/// Runs shown: running ones first, then the newest.
pub(crate) const MAX_RUNS: usize = 8;

#[derive(Clone, Debug, Default, PartialEq, Serialize)]
pub(crate) struct LlmRun {
    /// `llm-<name>/<sub>` below the root.
    pub name: String,
    pub problem: Option<String>,
    /// Controller running (`controller.json` says so and the process is a
    /// collector).
    pub active: bool,
    /// `running`, `exited` or `failed` from `controller.json`.
    pub controller: Option<String>,
    pub pid: Option<u32>,
    pub unit: Option<String>,
    pub phase: Option<String>,
    pub updated_ms: Option<u64>,
    pub started_ms: Option<u64>,
    pub ended_ms: Option<u64>,
    /// Attempts handed out so far, and the most the run may hand out.
    pub assigned: Option<u64>,
    pub max_attempts: Option<u64>,
    pub concurrency: Option<u64>,
    pub completed: Option<u64>,
    pub in_flight: Option<u64>,
    pub incomplete: u64,
    pub errors: u64,
    pub llm_answers: u64,
    pub fallback_answers: u64,
    /// Models that played, `model` and `opponent_model` of the strata.
    pub models: Vec<String>,
    pub worker_failures: usize,
    /// No new attempts are handed out / the active ones are asked to stop.
    pub drain: bool,
    pub stop: bool,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize)]
pub(crate) struct Ladder {
    pub name: String,
    pub problem: Option<String>,
    pub updated_ms: Option<u64>,
    pub games: usize,
    /// Games with a phase other than `game-complete` (a timed-out game has none).
    pub in_flight: usize,
    /// Games with a `failure` or a timeout.
    pub failures: usize,
    pub arms: Vec<Arm>,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize)]
pub(crate) struct Arm {
    pub name: String,
    pub games: Option<u64>,
    pub net_won: Option<u64>,
    pub net_lost: Option<u64>,
    pub drawn: Option<u64>,
    pub unfinished: Option<u64>,
}

#[derive(Deserialize)]
struct ControllerFile {
    pid: Option<u32>,
    status: Option<String>,
    started_unix: Option<f64>,
    ended_unix: Option<f64>,
}

#[derive(Deserialize)]
struct StatusFile {
    phase: Option<String>,
    updated_unix: Option<f64>,
    assigned_attempts: Option<u64>,
    max_attempts: Option<u64>,
    concurrency: Option<u64>,
    #[serde(default)]
    worker_failures: Vec<serde::de::IgnoredAny>,
    summary: Option<Summary>,
}

#[derive(Deserialize)]
struct Summary {
    completed_games: Option<u64>,
    in_flight_games: Option<u64>,
    #[serde(default)]
    strata: Vec<Stratum>,
}

#[derive(Deserialize)]
struct Stratum {
    model: Option<String>,
    opponent_model: Option<String>,
    incomplete: Option<u64>,
    errors: Option<u64>,
    llm_answers: Option<u64>,
    fallback_answers: Option<u64>,
}

#[derive(Deserialize)]
struct LadderFile {
    #[serde(default)]
    games: Vec<LadderGame>,
    #[serde(default)]
    summary: std::collections::BTreeMap<String, LadderArm>,
}

#[derive(Deserialize)]
struct LadderGame {
    phase: Option<String>,
    failure: Option<serde::de::IgnoredAny>,
    timed_out: Option<bool>,
}

#[derive(Deserialize)]
struct LadderArm {
    games: Option<u64>,
    net_won: Option<u64>,
    net_lost: Option<u64>,
    drawn: Option<u64>,
    unfinished: Option<u64>,
}

type Stamp = (u64, Option<SystemTime>);

/// Parsed `status.json` files by path, so a finished run costs a `stat`.
#[derive(Default)]
pub(crate) struct RunCache {
    entries: HashMap<PathBuf, (Stamp, Parsed, bool)>,
}

type Parsed = Result<LlmRun, String>;

impl RunCache {
    fn status(&mut self, path: &Path) -> Parsed {
        let stamp = match fs::symlink_metadata(path) {
            Ok(meta) if meta.is_file() => (meta.len(), meta.modified().ok()),
            Ok(_) => return Err("status.json ist keine Datei".into()),
            Err(error) => return Err(format!("status.json nicht lesbar ({error})")),
        };
        if let Some((known, parsed, used)) = self.entries.get_mut(path)
            && *known == stamp
        {
            *used = true;
            return parsed.clone();
        }
        let parsed = read_json::<StatusFile>(path, MAX_STATUS)
            .map(summarise)
            .map_err(|error| format!("status.json nicht lesbar ({error})"));
        self.entries
            .insert(path.to_owned(), (stamp, parsed.clone(), true));
        parsed
    }

    fn sweep(&mut self) {
        self.entries.retain(|_, (_, _, used)| std::mem::take(used));
    }
}

fn summarise(status: StatusFile) -> LlmRun {
    let mut run = LlmRun {
        phase: status.phase.map(|p| clean(&p)),
        updated_ms: status.updated_unix.and_then(unix_ms),
        assigned: status.assigned_attempts,
        max_attempts: status.max_attempts,
        concurrency: status.concurrency,
        worker_failures: status.worker_failures.len(),
        ..LlmRun::default()
    };
    if let Some(summary) = status.summary {
        run.completed = summary.completed_games;
        run.in_flight = summary.in_flight_games;
        for stratum in summary.strata {
            run.incomplete += stratum.incomplete.unwrap_or(0);
            run.errors += stratum.errors.unwrap_or(0);
            run.llm_answers += stratum.llm_answers.unwrap_or(0);
            run.fallback_answers += stratum.fallback_answers.unwrap_or(0);
            for model in [stratum.model, stratum.opponent_model]
                .into_iter()
                .flatten()
            {
                let model = clean(&model);
                if run.models.len() < MAX_MODELS && !run.models.contains(&model) {
                    run.models.push(model);
                }
            }
        }
    }
    run
}

/// The collector roots below the `--llm-root` directories: `llm-*` children
/// and their subdirectories that hold a `controller.json` and a `status.json`.
fn find(roots: &[PathBuf]) -> Vec<PathBuf> {
    let mut found = Vec::new();
    for root in roots {
        for child in super::discovery::subdirectories(root, MAX_CHILDREN)
            .into_iter()
            .filter(|p| {
                p.file_name()
                    .and_then(|n| n.to_str())
                    .is_some_and(|n| n.starts_with("llm-"))
            })
        {
            if is_collector(&child) {
                found.push(child.clone());
            }
            for sub in super::discovery::subdirectories(&child, MAX_SUBDIRS) {
                if is_collector(&sub) {
                    found.push(sub);
                }
            }
        }
    }
    found
}

fn is_collector(dir: &Path) -> bool {
    ["controller.json", "status.json"]
        .iter()
        .all(|file| fs::symlink_metadata(dir.join(file)).is_ok_and(|m| m.is_file()))
}

/// Whether process `pid` exists and is a collector, judged by its `comm`
/// (the executable's name) alone.
fn collector_alive(pid: u32) -> bool {
    fs::read_to_string(format!("/proc/{pid}/stat")).is_ok_and(|stat| {
        stat.split_once('(')
            .and_then(|(_, rest)| rest.rsplit_once(')'))
            .is_some_and(|(comm, _)| comm.starts_with("llm_collector"))
    })
}

fn exists(path: PathBuf) -> bool {
    fs::symlink_metadata(path).is_ok()
}

/// A candidate before its status is parsed: enough to order the runs.
struct Candidate {
    dir: PathBuf,
    controller: Result<ControllerFile, String>,
    alive: bool,
    modified: Option<SystemTime>,
}

pub(super) fn read_runs(roots: &[PathBuf], cache: &mut RunCache) -> Vec<LlmRun> {
    let mut candidates: Vec<(Candidate, String)> = find(roots)
        .into_iter()
        .map(|dir| {
            let controller =
                read_json::<ControllerFile>(&dir.join("controller.json"), MAX_CONTROLLER)
                    .map_err(|error| format!("controller.json nicht lesbar ({error})"));
            let alive = controller.as_ref().is_ok_and(|c| {
                c.status.as_deref() == Some("running") && c.pid.is_some_and(collector_alive)
            });
            let modified = fs::symlink_metadata(dir.join("status.json"))
                .and_then(|m| m.modified())
                .ok();
            let name = run_name(roots, &dir);
            (
                Candidate {
                    dir,
                    controller,
                    alive,
                    modified,
                },
                name,
            )
        })
        .collect();
    candidates.sort_by(|(a, an), (b, bn)| {
        b.alive
            .cmp(&a.alive)
            .then(b.modified.cmp(&a.modified))
            .then_with(|| an.cmp(bn))
    });
    candidates.truncate(MAX_RUNS);
    let runs = candidates
        .into_iter()
        .map(|(candidate, name)| {
            let mut run = match cache.status(&candidate.dir.join("status.json")) {
                Ok(run) => run,
                Err(problem) => LlmRun {
                    problem: Some(problem),
                    ..LlmRun::default()
                },
            };
            run.name = name;
            run.active = candidate.alive;
            match &candidate.controller {
                Ok(controller) => {
                    run.controller = controller.status.as_deref().map(clean);
                    run.pid = controller.pid;
                    run.started_ms = controller.started_unix.and_then(unix_ms);
                    run.ended_ms = controller.ended_unix.and_then(unix_ms);
                    if candidate.alive {
                        run.unit = controller.pid.and_then(unit_of);
                    }
                }
                Err(problem) => {
                    run.problem.get_or_insert_with(|| problem.clone());
                }
            }
            run.drain = exists(candidate.dir.join("DRAIN"));
            run.stop = exists(candidate.dir.join("STOP"));
            run
        })
        .collect();
    cache.sweep();
    runs
}

/// `llm-name/sub`: the last two components, which `valid_name` already
/// vouched for while scanning.
fn run_name(roots: &[PathBuf], dir: &Path) -> String {
    let relative = roots
        .iter()
        .find_map(|root| dir.strip_prefix(root).ok())
        .unwrap_or(dir);
    let parts: Vec<_> = relative
        .components()
        .filter_map(|c| c.as_os_str().to_str())
        .filter(|name| valid_name(name))
        .collect();
    parts.join("/")
}

/// Reads one `ladder.json`: the summary of each arm and the phase of each
/// game. A broken file is the card's problem.
pub(super) fn read_ladder(path: &Path) -> Ladder {
    let name = path
        .parent()
        .map(|dir| {
            let mut parts: Vec<_> = dir
                .components()
                .rev()
                .filter_map(|c| c.as_os_str().to_str())
                .filter(|n| valid_name(n))
                .take(2)
                .collect();
            parts.reverse();
            parts.join("/")
        })
        .unwrap_or_default();
    let mut ladder = Ladder {
        name,
        ..Ladder::default()
    };
    ladder.updated_ms = fs::symlink_metadata(path)
        .and_then(|m| m.modified())
        .ok()
        .and_then(|t| t.duration_since(SystemTime::UNIX_EPOCH).ok())
        .map(|d| d.as_millis() as u64);
    match read_json::<LadderFile>(path, MAX_LADDER) {
        Ok(file) => {
            ladder.games = file.games.len();
            ladder.in_flight = file
                .games
                .iter()
                .filter(|g| g.phase.as_deref().is_some_and(|p| p != "game-complete"))
                .count();
            ladder.failures = file
                .games
                .iter()
                .filter(|g| g.failure.is_some() || g.timed_out == Some(true))
                .count();
            ladder.arms = file
                .summary
                .into_iter()
                .take(MAX_ARMS)
                .map(|(name, arm)| Arm {
                    name: clean(&name),
                    games: arm.games,
                    net_won: arm.net_won,
                    net_lost: arm.net_lost,
                    drawn: arm.drawn,
                    unfinished: arm.unfinished,
                })
                .collect();
        }
        Err(error) => ladder.problem = Some(format!("ladder.json nicht lesbar ({error})")),
    }
    ladder
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::training::safe::Scratch;

    const STATUS: &str = r#"{"assigned_attempts":66,"concurrency":3,"max_attempts":120,
        "phase":"collecting","updated_unix":1793951340.5,"worker_failures":[],
        "summary":{"attempts":66,"completed_games":60,"in_flight_games":3,
          "by_deck":{"x":{"wins":1}},
          "strata":[{"model":"claude-haiku-5-5","opponent_model":"gemini-4-argon","errors":1,
                     "incomplete":2,"llm_answers":100,"fallback_answers":4},
                    {"model":"claude-haiku-5-5","opponent_model":"claude-haiku-5-5",
                     "llm_answers":50}]}}"#;

    fn collector(scratch: &Scratch, at: &str, controller: &str, status: &str) {
        scratch.write(&format!("{at}/controller.json"), controller);
        scratch.write(&format!("{at}/status.json"), status);
    }

    #[test]
    fn a_run_shows_admitted_against_planned_models_and_flags() {
        let scratch = Scratch::new("runs");
        collector(
            &scratch,
            "llm-a-20261106/train",
            r#"{"pid":4294967,"status":"running","started_unix":1793940000.0,
                "gateway":"http://127.0.0.1:1","token":"hunter2"}"#,
            STATUS,
        );
        collector(
            &scratch,
            "llm-old-20261031",
            r#"{"pid":1,"status":"exited","ended_unix":1793951340.0}"#,
            r#"{"phase":"stopped"}"#,
        );
        scratch.write("llm-a-20261106/train/DRAIN", "");
        scratch.write("llm-a-20261106/profiles.json", r#"{"key":"hunter2"}"#);
        scratch.write("not-llm/train/status.json", "{}");
        let mut cache = RunCache::default();
        let runs = read_runs(std::slice::from_ref(&scratch.0), &mut cache);
        assert_eq!(runs.len(), 2);
        let by_name = |name: &str| runs.iter().find(|r| r.name == name).unwrap();
        let run = by_name("llm-a-20261106/train");
        assert_eq!(run.phase.as_deref(), Some("collecting"));
        assert_eq!((run.assigned, run.max_attempts), (Some(66), Some(120)));
        assert_eq!((run.completed, run.in_flight), (Some(60), Some(3)));
        assert_eq!(
            (run.errors, run.incomplete, run.fallback_answers),
            (1, 2, 4)
        );
        assert_eq!(run.llm_answers, 150);
        assert_eq!(run.models, ["claude-haiku-5-5", "gemini-4-argon"]);
        assert!(run.drain && !run.stop);
        assert!(
            !run.active,
            "a pid that is not a collector is not a live run"
        );
        assert_eq!(run.controller.as_deref(), Some("running"));
        assert_eq!(
            by_name("llm-old-20261031").ended_ms,
            Some(1_793_951_340_000)
        );
        let json = format!("{runs:?}");
        assert!(
            !json.contains("hunter2") && !json.contains("127.0.0.1"),
            "{json}"
        );
        assert_eq!(cache.entries.len(), 2);
    }

    #[test]
    fn a_broken_file_shows_on_its_card_and_the_rest_still_load() {
        let scratch = Scratch::new("runs-broken");
        collector(&scratch, "llm-b/train", r#"{"status":"exited"}"#, "{broken");
        collector(&scratch, "llm-c/train", "[]", r#"{"phase":"stopped"}"#);
        collector(
            &scratch,
            "llm-d/train",
            r#"{"status":"exited"}"#,
            r#"{"phase":"stopped"}"#,
        );
        let runs = read_runs(std::slice::from_ref(&scratch.0), &mut RunCache::default());
        assert_eq!(runs.len(), 3);
        let broken = runs.iter().find(|r| r.name == "llm-b/train").unwrap();
        assert!(
            broken
                .problem
                .as_deref()
                .unwrap()
                .starts_with("status.json nicht lesbar")
        );
        let no_controller = runs.iter().find(|r| r.name == "llm-c/train").unwrap();
        assert!(
            no_controller
                .problem
                .as_deref()
                .unwrap()
                .starts_with("controller.json")
        );
        assert_eq!(runs.iter().filter(|r| r.problem.is_none()).count(), 1);
    }

    #[test]
    fn only_a_collector_process_counts_as_alive() {
        assert!(!collector_alive(u32::MAX));
        assert!(
            !collector_alive(std::process::id()),
            "the test binary is no collector"
        );
    }

    #[test]
    fn the_ladder_reads_arms_and_game_phases() {
        let scratch = Scratch::new("ladder");
        scratch.write(
            "train/ladder-x/run1/ladder.json",
            r#"{"games":[{"phase":"game-complete","failure":null,"won":true,"room":"secret-room"},
                         {"phase":"playing","failure":null},
                         {"phase":"game-complete","failure":"boom","timed_out":false},
                         {"phase":"game-complete","timed_out":true},
                         {"phase":null,"timed_out":true}],
                "summary":{"deepseek-low / raw":{"drawn":0,"games":20,"net_lost":17,"net_won":1,"unfinished":2},
                           "deepseek-low / tactical":{"games":11,"net_lost":9,"net_won":2}}}"#,
        );
        let ladder = read_ladder(&scratch.0.join("train/ladder-x/run1/ladder.json"));
        assert_eq!(ladder.name, "ladder-x/run1");
        assert_eq!((ladder.games, ladder.in_flight, ladder.failures), (5, 1, 3));
        assert_eq!(ladder.arms.len(), 2);
        assert_eq!(ladder.arms[0].name, "deepseek-low / raw");
        assert_eq!(
            (ladder.arms[0].net_won, ladder.arms[0].net_lost),
            (Some(1), Some(17))
        );
        assert!(ladder.updated_ms.is_some());
        assert!(!format!("{ladder:?}").contains("secret-room"));
        let missing = read_ladder(&scratch.0.join("nope/ladder.json"));
        assert!(missing.problem.is_some() && missing.arms.is_empty());
    }
}
