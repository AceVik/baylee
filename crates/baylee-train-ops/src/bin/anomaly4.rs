//! The anomalies of finished runs, read from what they already recorded.
//!
//! ```text
//! anomaly4 RESULT... [--harvest ~/baylee-data/v6/train/bug-harvest.md]
//! ```
//!
//! `RESULT` is an eval4 summary (`--out`), a run directory (`games.jsonl`:
//! selfplay, or eval4's `--record-out`) or a ladder4 root (`ladder.json`).
//! Each gets `anomalies-<run>.json` and `anomalies-<run>.md` beside it
//! (`baylee_train_ops::anomaly`). An eval4 summary's job (the
//! `execution-*/jobs/*/job.json` whose `--out` it is) gives its stderr,
//! its runtime and its records; a flag overrides each. No game is played:
//! the one active check replays about one recorded game in 64 with the
//! converting build's `convert4`, on one thread.

use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context as _, bail};
use baylee_train_ops::anomaly::{self, Report};
use clap::Parser;
use serde_json::Value;

#[derive(Parser)]
struct Args {
    /// eval4 summaries, run directories or ladder4 roots.
    #[arg(required = true)]
    results: Vec<PathBuf>,
    /// The records of an eval4 summary (default: its job's `--record-out`).
    #[arg(long)]
    records: Option<PathBuf>,
    /// The stderr log of an eval4 summary (default: its job's).
    #[arg(long)]
    stderr: Option<PathBuf>,
    /// The converting build's `convert4` (default: the job's runtime's).
    #[arg(long)]
    convert4: Option<PathBuf>,
    /// The `gamelog4` that counts the net's counterspells in a run's
    /// records (default: the one beside the converting `convert4`).
    #[arg(long)]
    gamelog4: Option<PathBuf>,
    /// Skip the counterspell watch, which replays every recorded game.
    #[arg(long)]
    no_counters: bool,
    /// Replay one finished recorded game in this many (0: none).
    #[arg(long, default_value_t = 64)]
    sample: u64,
    /// Where the replay audit works; created and removed (default: the
    /// system's temporary directory).
    #[arg(long)]
    scratch: Option<PathBuf>,
    /// Write the reports here instead of beside each result.
    #[arg(long)]
    out_dir: Option<PathBuf>,
    /// Append the new entries to this running list.
    #[arg(long)]
    harvest: Option<PathBuf>,
}

/// What a result is, read from what it holds.
enum Source {
    Eval4(Value),
    Run(PathBuf),
    Ladder(Value),
    /// A `train_ops` job directory whose result is missing.
    Job(Value),
}

fn source(result: &Path) -> anyhow::Result<Source> {
    if result.is_dir() {
        if result.join("job.json").is_file() {
            return Ok(Source::Job(serde_json::from_slice(&fs::read(
                result.join("job.json"),
            )?)?));
        }
        if result.join("ladder.json").is_file() {
            let ladder = serde_json::from_slice(&fs::read(result.join("ladder.json"))?)?;
            return Ok(Source::Ladder(ladder));
        }
        if result.join("games.jsonl").is_file() {
            return Ok(Source::Run(result.to_path_buf()));
        }
        bail!("{}: neither ladder.json nor games.jsonl", result.display());
    }
    let summary: Value = serde_json::from_slice(
        &fs::read(result).with_context(|| format!("reading {}", result.display()))?,
    )?;
    if summary.get("deal_details").is_none() && summary.get("deals").is_none() {
        bail!("{} is not an eval4 summary", result.display());
    }
    Ok(Source::Eval4(summary))
}

fn read_json(path: &Path) -> Option<Value> {
    serde_json::from_slice(&fs::read(path).ok()?).ok()
}

fn report(args: &Args, result: &Path) -> anyhow::Result<Report> {
    let (mut report, records, convert4) = match source(result)? {
        Source::Eval4(summary) => {
            let job = anomaly::job_of(result);
            let mut report = anomaly::from_eval4(&summary, result, job.as_ref());
            let stderr = args
                .stderr
                .clone()
                .or_else(|| job.as_ref().and_then(|j| j.stderr.clone()));
            match stderr.as_deref().map(fs::read_to_string) {
                Some(Ok(text)) => report.entries.extend(anomaly::panics_in(&text)),
                Some(Err(error)) => report
                    .not_recorded
                    .push(format!("stderr unreadable: {error}")),
                None => report
                    .not_recorded
                    .push("no stderr log: the job was not found beside the result".to_owned()),
            }
            let records = args
                .records
                .clone()
                .or_else(|| job.as_ref().and_then(|j| j.records.clone()));
            let convert4 = args.convert4.clone().or_else(|| {
                job.as_ref()
                    .and_then(|j| j.runtime.as_ref())
                    .map(|runtime| runtime.join("convert4"))
            });
            (report, records, convert4)
        }
        Source::Run(dir) => {
            let games = fs::read_to_string(dir.join("games.jsonl"))?;
            let report =
                anomaly::from_run(&games, read_json(&dir.join("run.json")).as_ref(), result);
            (report, Some(dir), args.convert4.clone())
        }
        Source::Ladder(ladder) => (anomaly::from_ladder(&ladder, result), None, None),
        Source::Job(job) => {
            let stderr = fs::read_to_string(result.join("stderr.log")).unwrap_or_default();
            (anomaly::from_job(&job, &stderr, result), None, None)
        }
    };
    report.counters = match (
        &records,
        args.gamelog4.clone().or_else(|| {
            convert4
                .as_ref()
                .and_then(|c| c.parent())
                .map(|dir| dir.join("gamelog4"))
        }),
    ) {
        _ if args.no_counters => Value::String("not asked (--no-counters)".to_owned()),
        (None, _) => Value::String("no records".to_owned()),
        (Some(_), None) => Value::String("no gamelog4: name --gamelog4".to_owned()),
        (Some(_), Some(gamelog4)) if !gamelog4.is_file() => {
            Value::String(format!("no gamelog4: {} is missing", gamelog4.display()))
        }
        (Some(records), Some(gamelog4)) => {
            let (counted, entries) = anomaly::counter_watch(records, &gamelog4);
            report.entries.extend(entries);
            counted
        }
    };
    report.replay = match (records, convert4) {
        _ if args.sample == 0 => anomaly::Replay::not_run("not asked (--sample 0)"),
        (None, _) => anomaly::Replay::not_run("no records"),
        (Some(_), None) => anomaly::Replay::not_run("no converting build: name --convert4"),
        (Some(_), Some(convert4)) if !convert4.is_file() => anomaly::Replay::not_run(&format!(
            "no converting build: {} is missing",
            convert4.display()
        )),
        (Some(records), Some(convert4)) => {
            let scratch = args
                .scratch
                .clone()
                .unwrap_or_else(std::env::temp_dir)
                .join(format!("anomaly4-{}-{}", std::process::id(), report.run));
            let (replay, entries) = anomaly::audit(&records, &convert4, args.sample, &scratch)?;
            report.entries.extend(entries);
            replay
        }
    };
    report.recount();
    Ok(report)
}

fn today() -> String {
    let date = time::OffsetDateTime::now_utc().date();
    format!(
        "{:04}-{:02}-{:02}",
        date.year(),
        u8::from(date.month()),
        date.day()
    )
}

fn main() -> anyhow::Result<()> {
    let args = Args::parse();
    let mut skipped = 0;
    let mut reports = Vec::new();
    for result in &args.results {
        let report = match report(&args, result) {
            Ok(report) => report,
            Err(error) => {
                // A plan or a table beside the results is no result.
                eprintln!("{}: skipped: {error:#}", result.display());
                skipped += 1;
                continue;
            }
        };
        let dir = args.out_dir.clone().unwrap_or_else(|| {
            result
                .canonicalize()
                .ok()
                .and_then(|path| path.parent().map(Path::to_path_buf))
                .unwrap_or_else(|| PathBuf::from("."))
        });
        let written = anomaly::write(&report, &dir)?;
        println!(
            "{}: {} games, {} new, replay {} -> {}",
            report.run,
            report.games,
            report.new_entries().count(),
            report.replay.status,
            written.display()
        );
        reports.push(report);
    }
    if skipped == args.results.len() {
        bail!("no result was read");
    }
    // All at once, so that a deal every run meets is one row.
    if let Some(list) = &args.harvest {
        let added = anomaly::harvest(list, &reports, &today())?;
        println!("{added} rows added to {}", list.display());
    }
    Ok(())
}
