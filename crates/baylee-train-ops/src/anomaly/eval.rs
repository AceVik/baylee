//! An eval4 summary: its deals' refusals, fallbacks, caps and search
//! refusals, and the job that produced it.

use std::fs;
use std::path::{Path, PathBuf};

use serde_json::Value;

use super::{Class, Entry, Kind, Report, root_class};

/// The train_ops job that wrote a result: its command, its stderr, the
/// runtime it ran from and the records it kept.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Job {
    /// Its command, the program first.
    pub argv: Vec<String>,
    /// Its `stderr.log`, when there is one.
    pub stderr: Option<PathBuf>,
    /// The directory its program is in: the runtime.
    pub runtime: Option<PathBuf>,
    /// Its `--record-out`.
    pub records: Option<PathBuf>,
}

impl Job {
    /// The job of `argv` run in `dir`.
    #[must_use]
    pub fn of(argv: Vec<String>, dir: &Path) -> Self {
        let runtime = argv
            .first()
            .and_then(|program| Path::new(program).parent())
            .map(Path::to_path_buf);
        let records = flag(&argv, "--record-out").map(PathBuf::from);
        let stderr = Some(dir.join("stderr.log")).filter(|path| path.is_file());
        Self {
            argv,
            stderr,
            runtime,
            records,
        }
    }

    /// A command that plays deal `deal` again: the job's own, one deal
    /// from `deal` on (`--deal-offset` keeps the table, decks and seat
    /// rotation), one thread, a fresh output.
    #[must_use]
    pub fn reproduce(&self, deal: u64) -> Option<String> {
        let (program, args) = self.argv.split_first()?;
        let mut out = vec![program.clone()];
        let mut skip = false;
        for arg in args {
            if skip {
                skip = false;
                continue;
            }
            if matches!(
                arg.as_str(),
                "--deals" | "--deal-offset" | "--threads" | "--out" | "--record-out"
            ) {
                skip = true;
                continue;
            }
            out.push(arg.clone());
        }
        out.extend([
            "--deals".to_owned(),
            "1".to_owned(),
            "--deal-offset".to_owned(),
            deal.to_string(),
            "--threads".to_owned(),
            "1".to_owned(),
            "--out".to_owned(),
            format!("reproduce-deal-{deal}.json"),
        ]);
        Some(out.join(" "))
    }
}

fn flag<'a>(argv: &'a [String], name: &str) -> Option<&'a str> {
    argv.iter()
        .position(|arg| arg == name)
        .and_then(|at| argv.get(at + 1))
        .map(String::as_str)
}

/// The job whose `--out` is `result`, looked for in the `execution-*`
/// directories beside it (`train_ops pipeline`'s `jobs/*/job.json`).
#[must_use]
pub fn job_of(result: &Path) -> Option<Job> {
    let result = result.canonicalize().ok()?;
    let parent = result.parent()?;
    let mut executions: Vec<PathBuf> = fs::read_dir(parent)
        .ok()?
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| {
            path.is_dir()
                && path
                    .file_name()
                    .is_some_and(|name| name.to_string_lossy().starts_with("execution"))
        })
        .collect();
    executions.sort();
    for execution in executions {
        let Ok(jobs) = fs::read_dir(execution.join("jobs")) else {
            continue;
        };
        let mut dirs: Vec<PathBuf> = jobs.filter_map(Result::ok).map(|e| e.path()).collect();
        dirs.sort();
        for dir in dirs {
            let Some(job) = fs::read(dir.join("job.json"))
                .ok()
                .and_then(|bytes| serde_json::from_slice::<Value>(&bytes).ok())
            else {
                continue;
            };
            // `train_ops pipeline` writes `command`; older jobs `argv`.
            let argv: Vec<String> = job["command"]
                .as_array()
                .or_else(|| job["argv"].as_array())
                .map(|args| {
                    args.iter()
                        .filter_map(|a| a.as_str().map(str::to_owned))
                        .collect()
                })
                .unwrap_or_default();
            let writes = flag(&argv, "--out")
                .and_then(|out| Path::new(out).canonicalize().ok())
                .is_some_and(|out| out == result);
            if writes {
                return Some(Job::of(argv, &dir));
            }
        }
    }
    None
}

/// The net's house fallbacks a run may have before they are worth a look:
/// a share of the net seat's answers.
pub const FALLBACK_SHARE: f64 = 0.02;

/// The anomalies an eval4 summary counted.
#[must_use]
#[allow(clippy::too_many_lines)] // one block per counter the summary keeps
pub fn from_eval4(summary: &Value, result: &Path, job: Option<&Job>) -> Report {
    let mut report = Report::new(result, "eval4");
    report.build = summary["evaluator_build"]
        .as_str()
        .or_else(|| summary["evaluator_revision"].as_str())
        .map(str::to_owned);
    for key in [
        "deals",
        "refused",
        "house_fallbacks",
        "stalls",
        "net_answer_caps",
        "base_answer_caps",
        "net_answers",
        "fallback_rate",
    ] {
        if !summary[key].is_null() {
            report.totals.insert(key.to_owned(), summary[key].clone());
        }
    }
    if let Some(refused) = summary.pointer("/search/counts/refused") {
        report
            .totals
            .insert("search_refused".to_owned(), refused.clone());
    }
    let fallbacks = summary["house_fallbacks"].as_f64().unwrap_or(0.0);
    let answers = summary["net_answers"].as_f64().unwrap_or(0.0);
    let heavy = answers + fallbacks > 0.0 && fallbacks / (answers + fallbacks) > FALLBACK_SHARE;
    let plain = if heavy { Class::New } else { Class::Expected };
    let Some(deals) = summary["deal_details"].as_array() else {
        report
            .not_recorded
            .push("the summary has no deal_details: nothing per deal to list".to_owned());
        report.recount();
        return report;
    };
    report.games = deals.len() as u64;
    for row in deals {
        let deal = row["deal"].as_u64();
        let seed = row["seed"].as_u64();
        let table = row["table"].as_u64();
        let focal = row["focal_seat"].as_u64();
        let reproduce = deal.and_then(|d| job.and_then(|job| job.reproduce(d)));
        let place = |entry: Entry| Entry {
            reproduce: reproduce.clone(),
            ..entry.at(deal, seed, table)
        };
        let mut stalls = 0;
        // Per reason: how often, and the details the diagnostics give.
        let mut reasons: std::collections::BTreeMap<
            String,
            (u64, std::collections::BTreeSet<String>),
        > = std::collections::BTreeMap::new();
        for event in row["fallback_events"].as_array().into_iter().flatten() {
            let seat = event["seat"].as_u64();
            let reason = event["reason"]
                .as_str()
                .or_else(|| event.pointer("/diagnostic/reason").and_then(Value::as_str))
                .unwrap_or("house_policy_answer");
            match reason {
                "engine_refusal" => {
                    let what = format!(
                        "the engine refused {} (decision {}): {}",
                        event["action"], event["decision_seq"], event["error"]
                    );
                    report
                        .entries
                        .push(place(Entry::new(Kind::EngineRefusal, Class::New, what)).seat(seat));
                }
                "outer_repeated_view" => stalls += 1,
                other => {
                    let (count, details) = reasons.entry(other.to_owned()).or_default();
                    *count += 1;
                    if let Some(detail) =
                        event.pointer("/diagnostic/detail").and_then(Value::as_str)
                    {
                        details.insert(detail.to_owned());
                    }
                }
            }
        }
        if stalls > 0 {
            let what = format!("the net's seat met one view more than four times, {stalls}x");
            report.entries.push(
                place(Entry::new(Kind::Stall, plain, what))
                    .seat(focal)
                    .times(stalls),
            );
        }
        for (reason, (count, details)) in reasons {
            // A failed inference or decode is the seat's own bug, at any rate.
            let class = if reason == "inference_or_decode" {
                Class::New
            } else {
                plain
            };
            let what = if details.is_empty() {
                format!("house answered in the net's seat: {reason}, {count}x")
            } else {
                let details: Vec<String> = details.into_iter().collect();
                format!(
                    "house answered in the net's seat: {reason}, {count}x ({})",
                    details.join("; ")
                )
            };
            report.entries.push(
                place(Entry::new(Kind::HouseFallback, class, what))
                    .seat(focal)
                    .times(count),
            );
        }
        for (capped, game) in [("net_capped", "net"), ("base_capped", "baseline")] {
            if row[capped].as_bool() == Some(true) {
                let what = format!("the {game} game hit the answer cap");
                report
                    .entries
                    .push(place(Entry::new(Kind::AnswerCap, Class::New, what)).seat(focal));
            }
        }
        for (reason, count) in row
            .pointer("/search/refused")
            .and_then(Value::as_object)
            .into_iter()
            .flatten()
        {
            let count = count.as_u64().unwrap_or(0);
            let (class, why) = root_class(reason);
            let what = format!("search roots refused: {reason} ({why}), {count}x");
            report.entries.push(
                place(Entry::new(Kind::RefusedRoot, class, what))
                    .seat(focal)
                    .times(count),
            );
        }
    }
    if job.is_some_and(|job| flag(&job.argv, "--inference-batch").is_some_and(|rows| rows != "0")) {
        report.not_recorded.push(
            "the run batched its inference (--inference-batch), which is not bitwise \
             reproducible: a reproduce command plays the deal again, not necessarily the same game"
                .to_owned(),
        );
    }
    report.not_recorded.extend([
        "a worker panic aborts eval4 before it writes a summary: panics show only in the job's \
         stderr"
            .to_owned(),
        "the baseline (house) game's refusals and fallbacks are not kept per deal".to_owned(),
        "refused search roots are counted per deal and reason, without the decision".to_owned(),
    ]);
    report.recount();
    report
}

/// The anomalies of a job that left no result: its panics, and how it
/// ended. A job stopped from outside (`interrupted`, a `deadline`) or one
/// whose stderr names no panic ended for a reason that is no game's.
#[must_use]
pub fn from_job(job: &Value, stderr: &str, dir: &Path) -> Report {
    let mut report = Report::new(dir, "job");
    for key in ["status", "returncode", "reason", "elapsed_seconds"] {
        if !job[key].is_null() {
            report.totals.insert(key.to_owned(), job[key].clone());
        }
    }
    let panics = panics_in(stderr);
    let tail: Vec<&str> = stderr.lines().rev().take(2).collect();
    let tail = tail.into_iter().rev().collect::<Vec<_>>().join(" | ");
    if job["status"].as_str() != Some("complete") {
        let what = format!(
            "the job ended {} ({}) without its result: {tail}",
            job["status"].as_str().unwrap_or("?"),
            job["reason"].as_str().unwrap_or("no reason given")
        );
        let class = if panics.is_empty() {
            Class::Expected
        } else {
            Class::New
        };
        report
            .entries
            .push(Entry::new(Kind::RunFailed, class, what));
    }
    report.entries.extend(panics);
    report.recount();
    report
}

/// The panics, and eval4's own refusal lines, in a job's stderr.
#[must_use]
pub fn panics_in(stderr: &str) -> Vec<Entry> {
    let mut out = Vec::new();
    for line in stderr.lines() {
        if line.contains("panicked at") || line.contains("worker panicked") {
            out.push(Entry::new(Kind::Panic, Class::New, line.trim().to_owned()));
        }
    }
    out
}
