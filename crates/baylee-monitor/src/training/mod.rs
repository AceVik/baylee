//! The training panel's data: the `baylee-*` system units and the configured
//! `train_ops` pipelines, refreshed every few seconds on their own thread so a
//! slow bus or disk never delays the 1 Hz sampler. Read-only, like the rest.

mod dbus;
mod detail;
mod discovery;
mod pipeline;
mod plan;
mod runs;
mod safe;
mod units;

use serde::Serialize;
use std::{
    path::{Component, PathBuf},
    sync::{Arc, Mutex, PoisonError},
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

pub(crate) use pipeline::Pipeline;
#[cfg(test)]
pub(crate) use pipeline::{Progress, Stage};
pub(crate) use runs::{Ladder, LlmRun};
pub(crate) use units::{Services, Unit};
#[cfg(test)]
pub(crate) use {plan::Eval, runs::Arm};

/// Training changes over minutes; five seconds keeps the panel current.
const REFRESH: Duration = Duration::from_secs(5);
/// Pipelines shown: more would not fit on a phone and would make the view and
/// every refresh large. Running ones come first, then the newest finished.
pub(crate) const MAX_PIPELINES: usize = 10;
/// `--pipeline` flags and `--pipeline-root` flags accepted.
pub(crate) const MAX_PIPELINE_FLAGS: usize = 8;
pub(crate) const MAX_ROOT_FLAGS: usize = 4;

#[derive(Clone, Debug, Default, Serialize)]
pub(crate) struct Training {
    /// When this view was assembled; zero before the first refresh.
    pub checked_ms: u64,
    pub units: Vec<Unit>,
    /// Why systemd could not be asked, when it could not.
    pub units_problem: Option<String>,
    /// The machine's other services, for a failed one to stand out.
    pub services: Option<Services>,
    pub pipelines: Vec<Pipeline>,
    /// LLM game collectors, running ones first.
    pub llm_runs: Vec<LlmRun>,
    pub ladders: Vec<Ladder>,
}

/// Where the training thread looks, from the command line.
#[derive(Clone, Debug, Default)]
pub(crate) struct Sources {
    /// Execution directories named one by one.
    pub pipelines: Vec<PathBuf>,
    /// Directories searched for execution directories.
    pub pipeline_roots: Vec<PathBuf>,
    /// Directories holding `llm-*` collector roots.
    pub llm_roots: Vec<PathBuf>,
    /// `ladder.json` files.
    pub ladders: Vec<PathBuf>,
}

/// Parses one `--pipeline` value: an absolute directory without `..`.
pub(crate) fn parse_pipeline_dir(value: &str) -> Result<PathBuf, String> {
    let path = PathBuf::from(value);
    if !path.is_absolute() {
        return Err("a directory must be an absolute path".into());
    }
    if path.components().any(|c| c == Component::ParentDir) {
        return Err("a directory must not contain '..'".into());
    }
    Ok(path)
}

/// Parses one `--ladder` value: an absolute path to a `.json` file.
pub(crate) fn parse_json_file(value: &str) -> Result<PathBuf, String> {
    let path = parse_pipeline_dir(value)?;
    if path.extension().is_none_or(|e| e != "json") {
        return Err("a ladder file must be a .json file".into());
    }
    Ok(path)
}

#[derive(Default)]
struct Caches {
    evals: plan::EvalCache,
    runs: runs::RunCache,
}

/// Starts the refresh thread, which replaces `shared` after every round.
pub(crate) fn spawn(sources: Sources, shared: Arc<Mutex<Training>>) -> std::io::Result<()> {
    std::thread::Builder::new()
        .name("training".into())
        .spawn(move || {
            let mut units = units::Units::default();
            let mut caches = Caches::default();
            loop {
                let started = Instant::now();
                let view = collect(&mut units, &mut caches, &sources);
                *shared.lock().unwrap_or_else(PoisonError::into_inner) = view;
                std::thread::sleep(REFRESH.saturating_sub(started.elapsed()));
            }
        })?;
    Ok(())
}

/// The pipelines to show: the explicit directories and everything found below
/// the roots, running ones first, then the most recently written.
fn select(
    explicit: &[PathBuf],
    roots: &[PathBuf],
    boot_id: Option<&str>,
    now_ms: u64,
    evals: &mut plan::EvalCache,
    limit: usize,
) -> Vec<Pipeline> {
    let mut seen = std::collections::HashSet::new();
    let mut all: Vec<Pipeline> = explicit
        .iter()
        .cloned()
        .chain(discovery::find(roots))
        .filter(|dir| seen.insert(dir.clone()))
        .map(|dir| pipeline::peek(&dir, boot_id))
        .collect();
    all.sort_by(|a, b| {
        b.active
            .cmp(&a.active)
            .then(b.updated_ms.cmp(&a.updated_ms))
            .then_with(|| a.name.cmp(&b.name))
    });
    all.truncate(limit);
    for pipeline in &mut all {
        pipeline::fill(pipeline, now_ms, evals);
    }
    evals.sweep();
    all
}

fn collect(units: &mut units::Units, caches: &mut Caches, sources: &Sources) -> Training {
    let now_ms = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64;
    let units_problem = units
        .refresh(now_ms)
        .err()
        .map(|error| format!("systemd nicht erreichbar: {error}"));
    let boot_id = std::fs::read_to_string("/proc/sys/kernel/random/boot_id").ok();
    let boot_id = boot_id.as_deref().map(str::trim);
    Training {
        checked_ms: now_ms,
        units: units.view(),
        services: units.services(),
        units_problem,
        pipelines: select(
            &sources.pipelines,
            &sources.pipeline_roots,
            boot_id,
            now_ms,
            &mut caches.evals,
            MAX_PIPELINES,
        ),
        llm_runs: runs::read_runs(&sources.llm_roots, &mut caches.runs),
        ladders: sources
            .ladders
            .iter()
            .map(|p| runs::read_ladder(p))
            .collect(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn run(scratch: &safe::Scratch, at: &str, phase: &str, updated: u64) {
        scratch.write(
            &format!("{at}/status.json"),
            &format!(r#"{{"phase":"{phase}","updated_unix":{updated}}}"#),
        );
        std::fs::create_dir_all(scratch.0.join(at).join("jobs")).unwrap();
    }

    #[test]
    fn running_pipelines_come_first_then_the_newest_and_new_runs_appear() {
        let scratch = safe::Scratch::new("select");
        run(&scratch, "a/execution", "complete", 300);
        run(&scratch, "b/execution", "complete", 100);
        run(&scratch, "c/execution", "train-x", 50);
        run(&scratch, "d/execution", "interrupted", 400);
        let roots = [scratch.0.clone()];
        let mut cache = plan::EvalCache::default();
        let mut names = |limit| -> Vec<String> {
            select(&[], &roots, None, 0, &mut cache, limit)
                .into_iter()
                .map(|p| p.name)
                .collect()
        };
        assert_eq!(
            names(10),
            ["c/execution", "d/execution", "a/execution", "b/execution"]
        );
        assert_eq!(names(2), ["c/execution", "d/execution"]);
        // A run that starts later shows up on the next round, first.
        run(&scratch, "e/execution", "convert", 60);
        assert_eq!(names(10)[0], "e/execution");
        // An explicit directory that is also below a root is listed once.
        let both = select(
            &[scratch.0.join("a/execution")],
            &roots,
            None,
            0,
            &mut cache,
            10,
        );
        assert_eq!(both.len(), 5);
    }

    #[test]
    fn pipeline_directories_are_absolute_and_plain() {
        assert_eq!(
            parse_pipeline_dir("/home/ace/baylee-data/v6/train/v6x-exit/execution-x1"),
            Ok(PathBuf::from(
                "/home/ace/baylee-data/v6/train/v6x-exit/execution-x1"
            ))
        );
        assert!(parse_pipeline_dir("baylee-data/x").is_err());
        assert!(parse_pipeline_dir("/home/ace/../etc").is_err());
    }
}
