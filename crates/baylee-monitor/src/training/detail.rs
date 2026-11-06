//! The details of a pipeline that need more than its status: the stage order
//! from the plan, a training stage's estimate, a failed stage's last line and
//! the systemd unit that runs the controller. All read-only and bounded.

use super::{
    pipeline::{Progress, Stage},
    plan::{Plan, PlanStage},
    safe::{clean, valid_name},
};
use std::{
    fs::{self, File},
    io::{Read, Seek, SeekFrom},
    path::Path,
};

const ERROR_TAIL: u64 = 4 * 1024;
/// Stages whose failure is looked up in their logs, newest first.
pub(super) const MAX_ERROR_LOOKUPS: usize = 4;
/// A log older than this no longer says how fast the stage goes.
const STALE_MS: u64 = 10 * 60 * 1000;

/// The job stages in the plan's order, the stages that have not started as
/// `pending`, and any stage the plan does not know after them.
pub(super) fn in_plan_order(plan: &Plan, mut jobs: Vec<Stage>) -> Vec<Stage> {
    let mut ordered = Vec::with_capacity(plan.stages.len().max(jobs.len()));
    for planned in &plan.stages {
        match jobs.iter().position(|job| job.name == planned.name) {
            Some(at) => ordered.push(jobs.remove(at)),
            None => ordered.push(Stage::pending(&planned.name)),
        }
    }
    ordered.append(&mut jobs);
    ordered
}

/// The 1-based position of the running stage, else of the first stage that is
/// not finished, else `None`.
pub(super) fn current_position(stages: &[Stage]) -> Option<usize> {
    let done = |s: &Stage| s.status.as_deref() == Some("complete");
    stages
        .iter()
        .rposition(|s| s.status.as_deref() == Some("running"))
        .or_else(|| stages.iter().position(|s| !done(s)))
        .map(|at| at + 1)
}

/// The estimate for a training stage: the total number of steps and the
/// seconds left. `header` is the start of its log, which says
/// `train_segments=N … B=M`; the plan says how many epochs.
pub(super) fn estimate(
    progress: &mut Progress,
    planned: &PlanStage,
    header: &str,
    started_ms: Option<u64>,
    now_ms: u64,
) {
    let field = |key: &str| {
        progress
            .fields
            .iter()
            .find(|(k, _)| k == key)
            .map(|(_, v)| v.as_str())
    };
    let Some(step) = field("step").and_then(|v| v.parse::<u64>().ok()) else {
        return;
    };
    progress.step = Some(step);
    let rate = field("batches_per_s").and_then(|v| v.parse::<f64>().ok());
    let segments = token_number(header, "train_segments=");
    let batch = planned.batch_size.or_else(|| token_number(header, "B="));
    let (Some(segments), Some(batch), Some(epochs)) = (segments, batch, planned.epochs) else {
        return;
    };
    if batch == 0 {
        return;
    }
    let total = segments.div_ceil(batch) * epochs;
    progress.total_steps = Some(total);
    let fresh = progress
        .updated_ms
        .is_some_and(|at| now_ms.saturating_sub(at) <= STALE_MS);
    let Some(rate) = rate.filter(|r| *r > 0.0 && r.is_finite() && fresh) else {
        return;
    };
    let steps = f64::from(u32::try_from(total.saturating_sub(step)).unwrap_or(u32::MAX));
    let mut left = (steps / rate) as u64;
    // A time budget ends training early.
    if let (Some(budget), Some(started)) = (planned.budget_s, started_ms) {
        let used = now_ms.saturating_sub(started) / 1000;
        left = left.min(budget.saturating_sub(used));
    }
    progress.remaining_s = Some(left);
}

/// The number after `key` in `text`, such as `B=8`; the key must start a token.
fn token_number(text: &str, key: &str) -> Option<u64> {
    text.split_whitespace()
        .find_map(|token| token.strip_prefix(key))?
        .parse()
        .ok()
}

/// The first 2 KiB of a log, where a trainer reports its data.
pub(super) fn log_head(path: &Path) -> String {
    let mut bytes = Vec::new();
    let Ok(meta) = fs::symlink_metadata(path) else {
        return String::new();
    };
    if !meta.is_file() {
        return String::new();
    }
    if let Ok(file) = File::open(path) {
        let _ = file.take(2048).read_to_end(&mut bytes);
    }
    String::from_utf8_lossy(&bytes).into_owned()
}

/// The last non-blank line of the stage's `stderr.log`, else of its
/// `stdout.log`: what a failed stage said last.
pub(super) fn failure_line(stage_dir: &Path) -> Option<String> {
    ["stderr.log", "stdout.log"]
        .iter()
        .find_map(|file| last_line(&stage_dir.join(file)))
        .map(|line| clean(&line))
}

fn last_line(path: &Path) -> Option<String> {
    let meta = fs::symlink_metadata(path).ok()?;
    if !meta.is_file() || meta.len() == 0 {
        return None;
    }
    let start = meta.len().saturating_sub(ERROR_TAIL);
    let mut file = File::open(path).ok()?;
    file.seek(SeekFrom::Start(start)).ok()?;
    let mut bytes = Vec::new();
    file.take(ERROR_TAIL).read_to_end(&mut bytes).ok()?;
    String::from_utf8_lossy(&bytes)
        .lines()
        .rev()
        .map(str::trim)
        .find(|line| !line.is_empty())
        .map(str::to_owned)
}

/// The systemd unit a process runs in, from its cgroup (`0::/…/name.service`).
/// Works for system and user units alike, without asking a bus.
pub(super) fn unit_of(pid: u32) -> Option<String> {
    let mut bytes = Vec::new();
    File::open(format!("/proc/{pid}/cgroup"))
        .ok()?
        .take(4096)
        .read_to_end(&mut bytes)
        .ok()?;
    unit_in_cgroup(&String::from_utf8_lossy(&bytes))
}

fn unit_in_cgroup(text: &str) -> Option<String> {
    let path = text.lines().find_map(|line| line.strip_prefix("0::"))?;
    path.rsplit('/')
        .find(|part| part.ends_with(".service") && !part.starts_with("user@"))
        .filter(|name| valid_name(&name.replace('@', "-")))
        .map(str::to_owned)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::training::safe::Scratch;

    fn stage(name: &str, status: &str) -> Stage {
        Stage {
            status: Some(status.into()),
            ..Stage::pending(name)
        }
    }

    fn planned(names: &[&str]) -> Plan {
        Plan {
            stages: names
                .iter()
                .map(|n| PlanStage {
                    name: (*n).into(),
                    ..PlanStage::default()
                })
                .collect(),
        }
    }

    #[test]
    fn stages_follow_the_plan_and_unstarted_ones_are_pending() {
        let plan = planned(&["a", "b", "c", "d"]);
        let jobs = vec![
            stage("extra", "complete"),
            stage("b", "running"),
            stage("a", "complete"),
        ];
        let ordered = in_plan_order(&plan, jobs);
        let names: Vec<_> = ordered.iter().map(|s| s.name.as_str()).collect();
        assert_eq!(names, ["a", "b", "c", "d", "extra"]);
        assert_eq!(ordered[2].status.as_deref(), Some("pending"));
        assert_eq!(current_position(&ordered), Some(2));
        let finished = vec![stage("a", "complete"), stage("b", "complete")];
        assert_eq!(current_position(&finished), None);
        let waiting = in_plan_order(&planned(&["a", "b"]), vec![stage("a", "complete")]);
        assert_eq!(current_position(&waiting), Some(2));
    }

    fn training_progress(updated_ms: u64) -> Progress {
        let fields = [("step", "3000"), ("loss", "0.9"), ("batches_per_s", "2.0")];
        Progress {
            stage: "train".into(),
            line: "[train4] step=3000".into(),
            fields: fields
                .iter()
                .map(|(k, v)| ((*k).into(), (*v).into()))
                .collect(),
            updated_ms: Some(updated_ms),
            ..Progress::default()
        }
    }

    #[test]
    fn a_training_stage_gets_steps_total_and_time_left() {
        let header =
            "[train4] device=cuda params=75.88M train_segments=51213 val_segments=6280 B=8 U=32\n";
        let plan = PlanStage {
            name: "train".into(),
            epochs: Some(1),
            ..PlanStage::default()
        };
        let mut progress = training_progress(1_000_000);
        estimate(&mut progress, &plan, header, Some(0), 1_000_000);
        assert_eq!(progress.step, Some(3000));
        assert_eq!(progress.total_steps, Some(6402));
        assert_eq!(progress.remaining_s, Some(1701));
        // The time budget ends the stage before the steps do.
        let budgeted = PlanStage {
            budget_s: Some(1800),
            ..PlanStage {
                name: "train".into(),
                epochs: Some(1),
                ..PlanStage::default()
            }
        };
        let mut progress = training_progress(1_000_000);
        estimate(&mut progress, &budgeted, header, Some(900_000), 1_000_000);
        assert_eq!(progress.remaining_s, Some(1700));
        // A stale log says nothing about the speed; unknown epochs say nothing about the total.
        let mut stale = training_progress(1_000_000);
        estimate(&mut stale, &plan, header, Some(0), 1_000_000 + STALE_MS + 1);
        assert_eq!((stale.total_steps, stale.remaining_s), (Some(6402), None));
        let mut unknown = training_progress(1_000_000);
        estimate(
            &mut unknown,
            &PlanStage::default(),
            header,
            Some(0),
            1_000_000,
        );
        assert_eq!(
            (unknown.step, unknown.total_steps, unknown.remaining_s),
            (Some(3000), None, None)
        );
    }

    #[test]
    fn a_failed_stage_shows_its_last_line_with_paths_reduced() {
        let scratch = Scratch::new("failure");
        scratch.write("stdout.log", "[t] fine\n");
        scratch.write(
            "stderr.log",
            "Traceback\n  File x\nRuntimeError: no file /home/ace/secret/key.txt\n\n",
        );
        assert_eq!(
            failure_line(&scratch.0).as_deref(),
            Some("RuntimeError: no file …/key.txt")
        );
        scratch.write("stderr.log", "");
        assert_eq!(failure_line(&scratch.0).as_deref(), Some("[t] fine"));
    }

    #[test]
    fn the_unit_comes_from_the_cgroup_for_system_and_user_units() {
        assert_eq!(
            unit_in_cgroup("0::/system.slice/baylee-v6a-p1.service\n").as_deref(),
            Some("baylee-v6a-p1.service")
        );
        assert_eq!(
            unit_in_cgroup(
                "0::/user.slice/user-1000.slice/user@1000.service/app.slice/baylee-v6a-ladder-gateway.service\n"
            )
            .as_deref(),
            Some("baylee-v6a-ladder-gateway.service")
        );
        assert_eq!(
            unit_in_cgroup("0::/user.slice/user-1000.slice/session-3.scope\n"),
            None
        );
        assert_eq!(
            unit_in_cgroup("0::/system.slice/x@a.service/payload\n").as_deref(),
            Some("x@a.service")
        );
    }
}
