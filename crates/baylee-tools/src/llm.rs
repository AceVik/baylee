//! Bounded model lanes for card implementations and independent engine tests.
mod deepseek;
mod gemini;
mod policy;

use std::{
    collections::BTreeSet,
    path::{Path, PathBuf},
    sync::{
        Mutex,
        atomic::{AtomicUsize, Ordering},
    },
    thread,
    time::{Duration, Instant},
};

use anyhow::{Context, Result, ensure};
use clap::{Parser, Subcommand};
use serde::Deserialize;

#[derive(Parser)]
#[command(name = "llm")]
struct Cli {
    #[command(subcommand)]
    command: Action,
}

#[derive(Subcommand)]
enum Action {
    /// Fill card files through the bounded HTTPS lane.
    #[command(alias = "cards_deepseek")]
    CardsDeepseek {
        batch: PathBuf,
        #[arg(default_value_t = 5)]
        workers: usize,
    },
    /// Ask the HTTPS lane for independent test fragments.
    #[command(alias = "tests_deepseek")]
    TestsDeepseek {
        worklist: PathBuf,
        #[arg(default_value_t = 5)]
        workers: usize,
    },
    /// Run one agent session for a batch in a sandboxed scratch tree.
    #[command(alias = "cards_gemini")]
    CardsGemini {
        batch: PathBuf,
        #[arg(default_value_t = 60)]
        minutes: u64,
    },
    /// Run one agent session for independent test fragments.
    #[command(alias = "tests_gemini")]
    TestsGemini {
        worklist: PathBuf,
        #[arg(default_value_t = 45)]
        minutes: u64,
    },
    /// Inspect the shared lane policy and read-only transcript metrics.
    Lane {
        #[command(subcommand)]
        command: Lane,
    },
}

#[derive(Subcommand)]
enum Lane {
    /// Print a card's test module.
    Door { path: String },
    /// Check a proposed card's generated header and identity fields.
    CheckHeader { stub: PathBuf, written: PathBuf },
    /// Measure one local agy SQLite transcript without reading its text.
    Transcript { database: PathBuf },
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Card {
    name: String,
    path: String,
}

struct Job {
    name: String,
    rel: String,
    slug: String,
    source: String,
}

fn jobs(root: &Path, list: &Path, tests: bool) -> Result<Vec<Job>> {
    let text = policy::read(list, policy::FILE_LIMIT)?;
    let cards: Vec<Card> = if tests {
        text.lines()
            .map(str::trim)
            .filter(|line| !line.is_empty())
            .map(|path| Card {
                name: Path::new(path)
                    .file_stem()
                    .and_then(|s| s.to_str())
                    .unwrap_or_default()
                    .to_owned(),
                path: path.to_owned(),
            })
            .collect()
    } else {
        serde_json::from_str(&text).context("batch must be a JSON array of name/path objects")?
    };
    ensure!(
        !cards.is_empty() && cards.len() <= 1000,
        "worklist must contain 1 to 1000 cards"
    );
    policy::unique(
        &cards
            .iter()
            .map(|card| card.path.clone())
            .collect::<Vec<_>>(),
    )?;
    let mut outputs = BTreeSet::new();
    cards
        .into_iter()
        .map(|card| {
            ensure!(
                !card.name.is_empty()
                    && card.name.len() <= 512
                    && !card.name.chars().any(|c| c.is_control() || c == '|'),
                "invalid card report name"
            );
            let path = policy::card_path(root, &card.path)?;
            let slug = path
                .file_stem()
                .and_then(|s| s.to_str())
                .context("card filename is not UTF-8")?
                .to_owned();
            ensure!(
                slug.chars().all(|c| c.is_ascii_alphanumeric() || c == '_'),
                "unsafe card slug"
            );
            if tests {
                ensure!(
                    outputs.insert(slug.clone()),
                    "duplicate test output filename"
                );
            }
            let source = policy::read(&path, policy::FILE_LIMIT)?;
            ensure!(
                source.contains("card!("),
                "worklist file is not a card source"
            );
            Ok(Job {
                name: card.name,
                rel: card.path,
                slug,
                source,
            })
        })
        .collect()
}

// A single item needs the batch's immutable provider and output policy.
#[allow(clippy::too_many_arguments)]
fn one(
    root: &Path,
    output: &Path,
    stem: &str,
    job: &Job,
    provider: &deepseek::Provider,
    prefix: &str,
    rules: &str,
    tests: bool,
    deadline: Instant,
) -> Result<String> {
    let answer = provider.ask(prefix, rules, &job.source, deadline)?;
    let body = match policy::code(&answer) {
        Ok(body) => body,
        Err(error) => {
            let refusal = if tests { "SKIP:" } else { "STUB:" };
            if !answer.contains("```") && answer.trim_start().starts_with(refusal) {
                return Ok(format!(
                    "{} | {} | model declined; source unchanged",
                    job.name,
                    if tests { "skip" } else { "stub" }
                ));
            }
            return Err(error);
        }
    };
    ensure!(
        Instant::now() < deadline,
        "batch deadline exceeded before write"
    );
    ensure!(
        policy::read(&policy::card_path(root, &job.rel)?, policy::FILE_LIMIT)? == job.source,
        "card changed during request; output refused"
    );
    if tests {
        policy::test_source(body)?;
        let relative = PathBuf::from(format!("{stem}-tests"))
            .join(policy::door(&job.rel))
            .join(format!("{}.rs", job.slug));
        policy::write(output, &relative, body)?;
        Ok(format!(
            "{} | written | {}",
            job.name,
            policy::door(&job.rel)
        ))
    } else {
        policy::keeps_header(&job.source, body)?;
        policy::write(root, Path::new(&job.rel), body)?;
        Ok(format!(
            "{} | {} | written",
            job.name,
            if body.contains("Coverage::Partial") {
                "partial"
            } else {
                "implemented"
            }
        ))
    }
}

fn run_deepseek(
    root: &Path,
    jobs: &[Job],
    output: &Path,
    stem: &str,
    tests: bool,
    workers: usize,
) -> Result<()> {
    ensure!(
        (1..=32).contains(&workers),
        "workers must be between 1 and 32"
    );
    let deadline = Instant::now()
        + Duration::from_secs(policy::bound("BAYLEE_LLM_BATCH_SECONDS", 3600, 14400)?);
    let provider = deepseek::Provider::load()?;
    let prefix = deepseek::pack(root, tests)?;
    let contract = if tests {
        "scripts/llm/prompts/tests-deepseek.md"
    } else {
        "scripts/llm/prompts/cards-deepseek.md"
    };
    let rules = policy::read(&policy::relative(root, contract)?, policy::FILE_LIMIT)?;
    println!(
        "{} {}, context {} KiB, {workers} workers",
        jobs.len(),
        if tests { "tests" } else { "cards" },
        prefix.len() / 1024
    );
    let next = AtomicUsize::new(0);
    let results: Mutex<Vec<Option<(String, bool)>>> = Mutex::new(vec![None; jobs.len()]);
    // The HTTPS client and bounded threads perform all work in Rust; no
    // subprocess receives an API key, and reports retain worklist order.
    thread::scope(|scope| {
        for _ in 0..workers.min(jobs.len()) {
            scope.spawn(|| {
                loop {
                    let i = next.fetch_add(1, Ordering::Relaxed);
                    let Some(job) = jobs.get(i) else {
                        break;
                    };
                    let (line, failed) = match one(
                        root, output, stem, job, &provider, &prefix, &rules, tests, deadline,
                    ) {
                        Ok(line) => (line, false),
                        Err(error) => (format!("{} | error | {error}", job.name), true),
                    };
                    match results.lock() {
                        Ok(mut results) => {
                            results[i] = Some((line, failed));
                        }
                        Err(_) => break,
                    }
                }
            });
        }
    });
    let results = results
        .into_inner()
        .map_err(|_| anyhow::anyhow!("lane worker failed"))?;
    let mut lines = Vec::new();
    let mut failed = false;
    let mut written = 0;
    for result in results {
        let (line, error) = result.context("lane worker did not report an outcome")?;
        failed |= error;
        if line.contains("| written |")
            || line.contains("| implemented |")
            || line.contains("| partial |")
        {
            written += 1;
        }
        println!("  {line}");
        lines.push(line);
    }
    let suffix = if tests { "-tests" } else { "" };
    let report = policy::write(
        output,
        Path::new(&format!("{stem}{suffix}-report.md")),
        &(lines.join("\n") + "\n"),
    )?;
    println!(
        "{written} of {} written; report in {}",
        jobs.len(),
        report.display()
    );
    ensure!(!failed, "one or more lane items failed; see report");
    Ok(())
}

/// Run one model lane or an offline shared-policy command.
///
/// # Errors
/// Rejects unsafe paths, invalid limits, incomplete output, changed identity,
/// lock conflicts, failed providers, unavailable isolation and I/O errors.
pub fn run(root: &Path, args: &[String]) -> Result<()> {
    let cli = Cli::try_parse_from(std::iter::once("llm".to_owned()).chain(args.iter().cloned()))?;
    let root = root.canonicalize()?;
    let (list, tests, agent, amount) = match cli.command {
        Action::Lane { command } => {
            match command {
                Lane::Door { path } => println!("{}", policy::door(&path)),
                Lane::CheckHeader { stub, written } => {
                    policy::keeps_header(
                        &policy::read(&stub, policy::FILE_LIMIT)?,
                        &policy::read(&written, policy::FILE_LIMIT)?,
                    )?;
                    println!("generated header and identity preserved");
                }
                Lane::Transcript { database } => {
                    let metrics =
                        gemini::transcript(&database, Instant::now() + Duration::from_secs(5))?;
                    println!(
                        "{} steps, {} bytes sent, ~{} input tokens, {} refusals",
                        metrics.steps,
                        metrics.sent,
                        metrics.sent / 4,
                        metrics.refused
                    );
                }
            }
            return Ok(());
        }
        Action::CardsDeepseek { batch, workers } => (batch, false, false, workers as u64),
        Action::TestsDeepseek { worklist, workers } => (worklist, true, false, workers as u64),
        Action::CardsGemini { batch, minutes } => (batch, false, true, minutes),
        Action::TestsGemini { worklist, minutes } => (worklist, true, true, minutes),
    };
    let (stem, output) = policy::scratch(&list, &root)?;
    let jobs = jobs(&root, &list, tests)?;
    let _root_lock = policy::Lock::acquire(root.join(".baylee-llm.lock"))?;
    let _output_lock = policy::Lock::acquire(output.join(format!(".{stem}-llm.lock")))?;
    if agent {
        let suffix = if tests { "-tests" } else { "" };
        let report_name = format!("{stem}{suffix}-report.md");
        let report_name = Path::new(&report_name);
        let pending = "lane | running | no verified results yet\n";
        let report_path = policy::write(&output, report_name, pending)?;
        let result = gemini::run(&root, &jobs, &stem, &output, tests, amount);
        if let Err(error) = &result
            && policy::read(&report_path, policy::FILE_LIMIT)? == pending
        {
            policy::write(&output, report_name, &format!("lane | error | {error}\n"))?;
        }
        result
    } else {
        run_deepseek(
            &root,
            &jobs,
            &output,
            &stem,
            tests,
            usize::try_from(amount)?,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn command_defaults_preserve_old_lane_knobs() {
        let cli = Cli::try_parse_from(["llm", "cards-deepseek", "/tmp/batch.json"]).unwrap();
        assert!(matches!(
            cli.command,
            Action::CardsDeepseek { workers: 5, .. }
        ));
        let cli = Cli::try_parse_from(["llm", "tests-gemini", "/tmp/list.txt"]).unwrap();
        assert!(matches!(
            cli.command,
            Action::TestsGemini { minutes: 45, .. }
        ));
        assert!(Cli::try_parse_from(["llm", "cards-deepseek", "batch", "-1"]).is_err());
    }
    #[test]
    fn worklists_are_checked_before_any_provider_is_loaded() {
        let root = tempfile::tempdir().unwrap();
        let scratch = tempfile::tempdir().unwrap();
        let list = scratch.path().join("list.txt");
        std::fs::write(&list, "../../secret\n").unwrap();
        assert!(jobs(root.path(), &list, true).is_err());
        std::fs::write(&list, "").unwrap();
        assert!(jobs(root.path(), &list, true).is_err());
        std::fs::write(&list, r#"[{"name":"Oak","path":"/tmp/secret"}]"#).unwrap();
        assert!(jobs(root.path(), &list, false).is_err());
    }
}
