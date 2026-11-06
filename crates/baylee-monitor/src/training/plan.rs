//! A pipeline's plan file (the `manifest` its `status.json` names) and the
//! `eval-<stage>.json` results that lie beside it.
//!
//! The plan gives the stage order (so "stage 5 of 14" and the stages that
//! have not started) and, for a training stage, the three command-line
//! values that make an estimate possible. Nothing else of a command line is
//! kept. An eval file can be close to a megabyte, so a finished one is parsed
//! once and remembered by size and modification time.

use super::safe::{read_json, valid_name};
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    path::{Component, Path, PathBuf},
    time::SystemTime,
};

const MAX_PLAN: u64 = 256 * 1024;
const MAX_EVAL: u64 = 4 * 1024 * 1024;
const MAX_PLAN_STAGES: usize = 64;

#[derive(Debug, Default)]
pub(super) struct Plan {
    pub stages: Vec<PlanStage>,
}

#[derive(Debug, Default, PartialEq)]
pub(super) struct PlanStage {
    pub name: String,
    /// `--epochs`, `--batch-size` and `--time-budget-seconds` of a training
    /// stage's command.
    pub epochs: Option<u64>,
    pub batch_size: Option<u64>,
    pub budget_s: Option<u64>,
}

#[derive(Deserialize)]
struct PlanFile {
    #[serde(default)]
    stages: Vec<StageEntry>,
}

#[derive(Deserialize)]
struct StageEntry {
    name: Option<String>,
    #[serde(default)]
    command: Vec<serde_json::Value>,
}

/// Reads the plan at `path`, which `status.json` supplied: it must be an
/// absolute `.json` path without `..`.
pub(super) fn read(path: &Path) -> Option<Plan> {
    if !plain_json_path(path) {
        return None;
    }
    let file: PlanFile = read_json(path, MAX_PLAN).ok()?;
    let stages = file
        .stages
        .into_iter()
        .filter_map(|entry| {
            let name = entry.name.filter(|n| valid_name(n))?;
            let args: Vec<&str> = entry
                .command
                .iter()
                .filter_map(serde_json::Value::as_str)
                .collect();
            let number = |flag: &str| {
                let at = args.iter().position(|a| *a == flag)?;
                args.get(at + 1)?.parse::<u64>().ok()
            };
            Some(PlanStage {
                name,
                epochs: number("--epochs"),
                batch_size: number("--batch-size"),
                budget_s: number("--time-budget-seconds"),
            })
        })
        .take(MAX_PLAN_STAGES)
        .collect();
    Some(Plan { stages })
}

fn plain_json_path(path: &Path) -> bool {
    path.is_absolute()
        && path.extension().is_some_and(|e| e == "json")
        && !path.components().any(|c| c == Component::ParentDir)
}

/// What an `eval-<stage>.json` says about a finished evaluation.
#[derive(Clone, Debug, Default, PartialEq, Deserialize, Serialize)]
pub(crate) struct Eval {
    pub net_wins: Option<u64>,
    pub net_losses: Option<u64>,
    pub net_draws: Option<u64>,
    pub net_win_rate: Option<f64>,
    pub deals: Option<u64>,
}

type Stamp = (u64, Option<SystemTime>);

/// Parsed eval files by path, kept while they are still being asked for.
#[derive(Default)]
pub(crate) struct EvalCache {
    entries: HashMap<PathBuf, (Stamp, Option<Eval>, bool)>,
}

impl EvalCache {
    /// The result for stage `stage` of a plan in `plan_dir`, when its file is
    /// there and readable.
    pub(super) fn get(&mut self, plan_dir: &Path, stage: &str) -> Option<Eval> {
        if !valid_name(stage) {
            return None;
        }
        let path = plan_dir.join(format!("eval-{stage}.json"));
        let meta = std::fs::symlink_metadata(&path).ok()?;
        if !meta.is_file() {
            return None;
        }
        let stamp = (meta.len(), meta.modified().ok());
        if let Some((known, eval, used)) = self.entries.get_mut(&path)
            && *known == stamp
        {
            *used = true;
            return eval.clone();
        }
        let eval = read_json::<Eval>(&path, MAX_EVAL).ok();
        self.entries.insert(path, (stamp, eval.clone(), true));
        eval
    }

    /// Forgets what was not asked for since the last sweep, so the cache
    /// stays as large as the pipelines on show.
    pub(super) fn sweep(&mut self) {
        self.entries.retain(|_, (_, _, used)| std::mem::take(used));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::training::safe::Scratch;

    #[test]
    fn the_plan_gives_stage_order_and_only_three_command_values() {
        let scratch = Scratch::new("plan");
        scratch.write(
            "plan.json",
            r#"{"stages":[
              {"name":"ref","command":["eval4","--model","x","--token","hunter2"]},
              {"name":"train-B1","command":["python","--epochs","2","--batch-size","8",
                 "--time-budget-seconds","1800","--api-key","hunter2"]},
              {"name":"../escape","command":[]},
              {"command":[]}
            ]}"#,
        );
        let plan = read(&scratch.0.join("plan.json")).unwrap();
        assert_eq!(plan.stages.len(), 2);
        assert_eq!(
            plan.stages[0],
            PlanStage {
                name: "ref".into(),
                ..PlanStage::default()
            }
        );
        assert_eq!(
            plan.stages[1],
            PlanStage {
                name: "train-B1".into(),
                epochs: Some(2),
                batch_size: Some(8),
                budget_s: Some(1800)
            }
        );
        assert!(!format!("{plan:?}").contains("hunter2"));
        assert!(read(Path::new("relative/plan.json")).is_none());
        assert!(read(Path::new("/a/../b/plan.json")).is_none());
        assert!(read(Path::new("/a/plan.txt")).is_none());
    }

    #[test]
    fn eval_results_are_parsed_once_and_forgotten_when_unused() {
        let scratch = Scratch::new("evals");
        scratch.write(
            "eval-A-dev.json",
            r#"{"net_wins":377,"net_losses":647,"net_draws":0,"net_win_rate":0.368,"deals":1024,
                "teams":[1,2,3]}"#,
        );
        let mut cache = EvalCache::default();
        let eval = cache.get(&scratch.0, "A-dev").unwrap();
        assert_eq!(
            (eval.net_wins, eval.net_losses, eval.deals),
            (Some(377), Some(647), Some(1024))
        );
        // Changed on disk: the new size is read.
        scratch.write("eval-A-dev.json", r#"{"net_wins":5}"#);
        assert_eq!(cache.get(&scratch.0, "A-dev").unwrap().net_wins, Some(5));
        assert!(cache.get(&scratch.0, "missing").is_none());
        assert!(cache.get(&scratch.0, "../x").is_none());
        cache.sweep();
        assert_eq!(cache.entries.len(), 1);
        cache.sweep();
        assert!(cache.entries.is_empty());
        scratch.write("eval-broken.json", "{nope");
        assert!(cache.get(&scratch.0, "broken").is_none());
    }
}
