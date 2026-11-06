//! BayleeAI v6 Phase A: plans of paired arms, their table, the value report.
//!
//! An arm is a word: `raw@dev`, `tac-t05-s2@fresh`, `raw-menu@dev`, `raw-k4@dev`,
//! `g=tap+cast+react@dev`, `plain-search@dev:256`, `raw-vs=s1r@dev:256`. The
//! flags it stands for are in [`arm_flags`]; every arm shares [`BASE`].
use anyhow::{Context, Result, bail, ensure};
use baylee_train_ops::evaluation::arms::{compare_arm, pool_arms};
use baylee_train_ops::evaluation::value_report::{read_positions, report};
use baylee_train_ops::evaluation::{atomic_json, read_json};
use clap::{Parser, Subcommand};
use serde_json::{Value, json};
use std::path::{Path, PathBuf};

const MODEL: &str = "/home/ace/baylee-data/v6/train/v6w-newengine/B-refresh";
const MANIFEST: &str =
    "/home/ace/baylee-data/v5/experiments/fresh-relational-20261027/eval-corpus/manifest.json";
const GPU_LOCK: &str = "/home/ace/baylee-data/v5/experiments/gpu-work.lock";
const POLICY_SEED: u64 = 63_001;

#[derive(Parser)]
#[command(about = "Plans, pairs and reads the arms of Phase A")]
struct Args {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Writes a `train_ops pipeline` plan of arms.
    Plan {
        /// The frozen source worktree the guard runs in.
        #[arg(long)]
        src: PathBuf,
        /// The frozen runtime holding `eval4`.
        #[arg(long)]
        runtime: PathBuf,
        /// Where evaluations and records go.
        #[arg(long)]
        root: PathBuf,
        /// The execution directory's name under `--root`.
        #[arg(long)]
        execution: String,
        /// Workers of a run without search.
        #[arg(long, default_value_t = 48)]
        threads: usize,
        /// Rows of a batched call without search.
        #[arg(long, default_value_t = 64)]
        batch: usize,
        /// Seconds a stage may take.
        #[arg(long, default_value_t = 3000)]
        timeout: u64,
        /// Run on the CPU: no GPU lock.
        #[arg(long)]
        cpu: bool,
        #[arg(long)]
        out: PathBuf,
        /// The arms, in order.
        arms: Vec<String>,
    },
    /// Pairs arms: `CANDIDATE[,CANDIDATE]~REFERENCE`, each an arm's word.
    Table {
        /// Where the evaluations are.
        #[arg(long)]
        root: PathBuf,
        /// A second root to look in, for arms of an earlier plan.
        #[arg(long, default_value_t = 20_000)]
        resamples: usize,
        #[arg(long)]
        out: PathBuf,
        pairs: Vec<String>,
    },
    /// Runs `gamelog4 survey` on arms, once each, and tables what the brief
    /// asks of them: casts on the other seat's turn, mulligans, unused taps.
    Surveys {
        /// Where the evaluations and records are.
        #[arg(long)]
        root: PathBuf,
        /// The `gamelog4` of this build.
        #[arg(long)]
        gamelog: PathBuf,
        #[arg(long, default_value_t = 6)]
        threads: usize,
        #[arg(long)]
        out: PathBuf,
        /// The arms.
        arms: Vec<String>,
    },
    /// Reports the value heads of a run, baselines fitted on another run.
    Values {
        /// `--value-log` and summary of the run reported on.
        #[arg(long, num_args = 2)]
        test: Vec<PathBuf>,
        /// `--value-log` and summary of the run the baselines are fitted on.
        #[arg(long, num_args = 2)]
        fit: Vec<PathBuf>,
        #[arg(long, default_value_t = 2_000)]
        resamples: usize,
        #[arg(long)]
        out: PathBuf,
    },
}

/// What every arm shares with the references of `v6w-guard`.
const BASE: [&str; 16] = [
    "--seats",
    "2",
    "--against",
    "sharp",
    "--baseline",
    "steady",
    "--generated",
    "0",
    "--shape",
    "constructed",
    "--deck-split",
    "test",
    "--fair-scouting",
    "--max-decisions",
    "5000",
    "--no-baseline",
];

/// A run of an arm: its file name, deals and flags.
struct Arm {
    name: String,
    deals: u64,
    flags: Vec<String>,
    gpu: bool,
}

fn file_name(word: &str) -> String {
    word.replace(['@', ':', '#'], "-")
        .replace('=', "-")
        .replace('+', "_")
}

/// The opponent of a `vs=` word, its model and flags.
fn opponent(word: &str) -> Result<Vec<String>> {
    let (model, extra): (&str, &[&str]) = match word {
        "s1r" => (
            "/mnt/xdata/baylee/opponents/v5-base/s1r",
            &["--catalog-drift"],
        ),
        "live" => (
            "/mnt/xdata/baylee/opponents/v5/20261027/model-control-final1500",
            &["--catalog-drift"],
        ),
        "v3a" => (
            "/mnt/xdata/baylee/opponents/v3/v3-a-dyn",
            &["--opponent-legacy-v3"],
        ),
        "v3r" => (
            "/mnt/xdata/baylee/opponents/v3/rl-39",
            &["--opponent-legacy-v3"],
        ),
        other => bail!("unknown opponent {other}"),
    };
    let mut flags = vec![
        "--opponent-model".to_owned(),
        model.to_owned(),
        "--numbers-only".to_owned(),
    ];
    flags.extend(extra.iter().map(|flag| (*flag).to_owned()));
    if !word.starts_with("v3") {
        flags.extend(["--opponent-policy".to_owned(), "raw".to_owned()]);
    }
    Ok(flags)
}

/// The flags of a word's left half.
fn arm_flags(left: &str) -> Result<(Vec<String>, bool)> {
    let mut flags: Vec<String> = Vec::new();
    let mut seed = POLICY_SEED;
    let mut thinking = 2;
    let mut temperature = 0.0;
    let mut search = false;
    let mut parts = left.split('-');
    match parts.next().context("empty arm")? {
        "raw" => flags.push("--raw-policy".into()),
        "tac" => flags.extend([
            "--raw-policy".into(),
            "--choice-guard".into(),
            "tactical".into(),
        ]),
        "plain" => {}
        guard if guard.starts_with("g=") => {
            let list = guard[2..].replace('_', "-").replace('+', ",");
            flags.extend(["--raw-policy".into(), "--choice-guard".into(), list]);
        }
        other => bail!("unknown arm mode {other}"),
    }
    for part in parts {
        if let Some(value) = part.strip_prefix("vs=") {
            flags.extend(opponent(value)?);
        } else if part == "val" {
            flags.push("VALUE_LOG".into());
        } else if part == "base" {
            flags.retain(|flag| flag != "--no-baseline");
            flags.push("WITH_BASELINE".into());
        } else if part == "menu" {
            flags.push("--menu-payable".into());
        } else if let Some(bias) = part.strip_prefix("menub") {
            flags.extend([
                "--menu-payable".into(),
                "--menu-payable-bias".into(),
                bias.into(),
            ]);
        } else if part == "search" {
            search = true;
            flags.extend([
                "--search".into(),
                "4,4,6".into(),
                "--search-threads".into(),
                "16".into(),
            ]);
        } else if let Some(value) = part.strip_prefix('t') {
            let tenths: u32 = value.parse().context("temperature")?;
            temperature = f64::from(tenths) / 10.0;
            flags.push("--dev-free-temperature".into());
        } else if let Some(value) = part.strip_prefix('s') {
            seed = POLICY_SEED - 1 + value.parse::<u64>().context("seed")?;
        } else if let Some(value) = part.strip_prefix('k') {
            thinking = value.parse().context("thinking steps")?;
        } else {
            bail!("unknown arm part {part}");
        }
    }
    // A bare reference logs its value heads: Arm 4 reads them, no other arm does.
    if matches!(left, "raw" | "tac" | "plain") {
        flags.push("VALUE_LOG".into());
    }
    flags.extend([
        "--policy-seed".into(),
        seed.to_string(),
        "--temperature".into(),
        temperature.to_string(),
        "--thinking-steps".into(),
        u32::to_string(&thinking),
    ]);
    Ok((flags, search))
}

fn arm(word: &str) -> Result<Arm> {
    let (left, right) = word
        .split_once('@')
        .context("an arm needs @dev or @fresh")?;
    let right = right.split_once('#').map_or(right, |(right, _)| right);
    let (split, deals) = right.split_once(':').unwrap_or((right, "1024"));
    let first_seed = match split {
        "dev" => "930301000000000000",
        "fresh" => "960301000000000000",
        other => bail!("unknown split {other}"),
    };
    let (mut flags, _) = arm_flags(left)?;
    flags.extend(["--first-seed".into(), first_seed.into()]);
    Ok(Arm {
        name: file_name(word),
        deals: deals.parse().context("deals")?,
        flags,
        gpu: true,
    })
}

#[allow(clippy::too_many_arguments, reason = "One stage of a plan")]
fn stage(
    word: &str,
    runtime: &Path,
    root: &Path,
    threads: usize,
    batch: usize,
    timeout: u64,
    cpu: bool,
) -> Result<Value> {
    let arm = arm(word)?;
    let out = root.join(format!("eval-{}.json", arm.name));
    let records = root.join(format!("eval-{}-records", arm.name));
    let mut command: Vec<String> = vec![
        runtime.join("eval4").display().to_string(),
        "--model".into(),
        MODEL.into(),
        "--deals".into(),
        arm.deals.to_string(),
        "--decks-manifest".into(),
        MANIFEST.into(),
    ];
    let with_baseline = arm.flags.iter().any(|flag| flag == "WITH_BASELINE");
    command.extend(
        BASE.iter()
            .filter(|flag| !(with_baseline && **flag == "--no-baseline"))
            .map(|flag| (*flag).to_owned()),
    );
    let search = arm.flags.iter().any(|flag| flag == "--search");
    let (threads, batch) = match word.split_once('#') {
        // `WORD#48x64`, for a bench of the two.
        Some((_, shape)) => {
            let (t, b) = shape.split_once('x').context("#THREADSxBATCH")?;
            (t.parse()?, b.parse()?)
        }
        None if search => (8, 136),
        None => (threads, batch),
    };
    command.extend([
        "--threads".into(),
        threads.to_string(),
        "--inference-batch".into(),
        batch.to_string(),
    ]);
    for flag in &arm.flags {
        match flag.as_str() {
            "WITH_BASELINE" => {}
            "VALUE_LOG" => command.extend([
                "--value-log".into(),
                root.join(format!("value-{}.jsonl", arm.name))
                    .display()
                    .to_string(),
            ]),
            other => command.push(other.to_owned()),
        }
    }
    command.extend([
        "--out".into(),
        out.display().to_string(),
        "--record-out".into(),
        records.display().to_string(),
    ]);
    let mut stage = json!({"name": arm.name, "timeout_seconds": timeout, "memory_gib": 32,
        "inputs": [MANIFEST], "command": command, "required_outputs": [out]});
    if !cpu && arm.gpu {
        stage["gpu_lock"] = json!(GPU_LOCK);
    }
    Ok(stage)
}

fn eval_path(root: &Path, word: &str) -> PathBuf {
    root.join(format!("eval-{}.json", file_name(word)))
}

fn pair(root: &Path, spec: &str, resamples: usize) -> Result<Value> {
    let (candidates, reference) = spec.split_once('~').context("CANDIDATES~REFERENCE")?;
    let runs: Vec<Value> = candidates
        .split(',')
        .map(|word| read_json(&eval_path(root, word)))
        .collect::<Result<_>>()?;
    let baseline = read_json(&eval_path(root, reference))?;
    let mut report = compare_arm(&runs, &baseline, resamples)?;
    report["arm"] = json!(candidates);
    report["reference"] = json!(reference);
    Ok(report)
}

fn row(report: &Value) -> Result<String> {
    let paired = &report["paired"];
    let difference = paired["paired_difference_with_caps_imputed_half"]
        .as_f64()
        .context("difference")?
        * 100.0;
    let interval = paired["table_bootstrap_95_percent_interval"]
        .as_array()
        .context("interval")?;
    let low = interval[0].as_f64().context("low")? * 100.0;
    let high = interval[1].as_f64().context("high")? * 100.0;
    Ok(format!(
        "| {} | {} | {} | {:.1} | {:.1} | {:+.1} pp [{:+.1}, {:+.1}] | {}/{} | {} |",
        report["arm"].as_str().unwrap_or(""),
        report["reference"].as_str().unwrap_or(""),
        report["deals"],
        report["arm_wins_mean"].as_f64().unwrap_or(0.0),
        report["reference_wins"].as_f64().unwrap_or(0.0),
        difference,
        low,
        high,
        report["better"],
        report["worse"],
        if low > 1.0 { "passes" } else { "no" },
    ))
}

/// `survey-<arm>.txt` of an arm, made once.
fn survey_text(root: &Path, gamelog: &Path, word: &str, threads: usize) -> Result<String> {
    let name = file_name(word);
    let path = root.join(format!("survey-{name}.txt"));
    if !path.exists() {
        let output = std::process::Command::new(gamelog)
            .arg("survey")
            .arg("--run")
            .arg(root.join(format!("eval-{name}-records")))
            .arg("--summary")
            .arg(eval_path(root, word))
            .arg("--threads")
            .arg(threads.to_string())
            .output()?;
        ensure!(
            output.status.success(),
            "survey of {word}: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        std::fs::write(&path, &output.stdout)?;
    }
    Ok(std::fs::read_to_string(path)?)
}

/// The first number of a survey's line that holds `label`, `0` without one:
/// a class or an answer nobody gave is not printed.
fn counted(text: &str, label: &str) -> u64 {
    text.lines()
        .filter(|line| line.contains(label))
        .find_map(|line| {
            line.replace(label, " ")
                .split_whitespace()
                .next()?
                .parse()
                .ok()
        })
        .unwrap_or(0)
}

/// What the survey says of an arm.
fn survey_row(word: &str, text: &str) -> Value {
    let taps = counted(text, "Priority / ActivateManaAbility / own")
        + counted(text, "Priority / ActivateManaAbility / their");
    json!({
        "arm": word,
        "casts_their_turn": counted(text, "Priority / CastSpell / their"),
        "casts_own_turn": counted(text, "Priority / CastSpell / own"),
        "passes_their_turn": counted(text, "Priority / PassPriority / their"),
        "mulligans_taken": counted(text, "Mulligan / MulliganTake / own")
            + counted(text, "Mulligan / MulliganTake / their"),
        "taps": taps,
        "taps_unused": counted(text, "tap: nothing to spend it on"),
        "casts_doing_nothing": counted(text, "cast: reads as doing nothing now"),
    })
}

fn main() -> Result<()> {
    match Args::parse().command {
        Command::Plan {
            src,
            runtime,
            root,
            execution,
            threads,
            batch,
            timeout,
            cpu,
            out,
            arms,
        } => {
            ensure!(!arms.is_empty(), "A plan needs arms");
            let stages: Vec<Value> = arms
                .iter()
                .map(|word| stage(word, &runtime, &root, threads, batch, timeout, cpu))
                .collect::<Result<_>>()?;
            let total = timeout * stages.len() as u64;
            atomic_json(
                &out,
                &json!({"cwd": src.join("tools/trainer"), "root": root.join(execution),
                    "total_timeout_seconds": total, "version": 1, "stages": stages}),
            )?;
        }
        Command::Table {
            root,
            resamples,
            out,
            pairs,
        } => {
            // `A~B/C~D` is one arm on two sets of deals, pooled.
            let reports: Vec<Value> = pairs
                .iter()
                .map(|spec| {
                    if !spec.contains('/') {
                        return pair(&root, spec, resamples);
                    }
                    let parts: Vec<Value> = spec
                        .split('/')
                        .map(|part| pair(&root, part, resamples))
                        .collect::<Result<_>>()?;
                    let mut pooled = pool_arms(&parts, resamples)?;
                    pooled["arm"] = json!(format!("{spec} (pooled)"));
                    pooled["reference"] = json!("");
                    Ok(pooled)
                })
                .collect::<Result<_>>()?;
            println!("| arm | reference | deals | wins | ref | paired | better/worse | Tor A |");
            println!("| --- | --- | --- | --- | --- | --- | --- | --- |");
            for report in &reports {
                println!("{}", row(report)?);
            }
            let kept: Vec<Value> = reports
                .into_iter()
                .map(|mut report| {
                    if let Some(fields) = report.as_object_mut() {
                        fields.remove("rows");
                    }
                    report
                })
                .collect();
            atomic_json(&out, &Value::Array(kept))?;
        }
        Command::Surveys {
            root,
            gamelog,
            threads,
            out,
            arms,
        } => {
            println!(
                "| arm | casts, their turn | casts, own | mulligans | taps | unused taps | casts doing nothing |"
            );
            println!("| --- | --- | --- | --- | --- | --- | --- |");
            let mut rows = Vec::new();
            for word in &arms {
                let row = survey_row(word, &survey_text(&root, &gamelog, word, threads)?);
                println!(
                    "| {word} | {} | {} | {} | {} | {} | {} |",
                    row["casts_their_turn"],
                    row["casts_own_turn"],
                    row["mulligans_taken"],
                    row["taps"],
                    row["taps_unused"],
                    row["casts_doing_nothing"]
                );
                rows.push(row);
            }
            atomic_json(&out, &Value::Array(rows))?;
        }
        Command::Values {
            test,
            fit,
            resamples,
            out,
        } => {
            let on_test = read_positions(&test[0], &test[1])?;
            let on_fit = read_positions(&fit[0], &fit[1])?;
            atomic_json(&out, &report(&on_test, &on_fit, resamples))?;
        }
    }
    Ok(())
}
