//! `BayleeAI` v6 Phase A: plans of paired arms, their table, the value report.
//!
//! An arm is a word: `raw@dev`, `tac-t05-s2@fresh`, `raw-menu@dev`, `raw-k4@dev`,
//! `g=tap+cast+react@dev`, `plain-search@dev:256`, `raw-vs=s1r@dev:256`,
//! `plain-r=void+race@dev`. The
//! flags it stands for are in [`arm_flags`]; every arm shares [`BASE`].
// Counts of games and resamples, well below f64's exact 2^53 range.
#![allow(clippy::cast_precision_loss)]
use anyhow::{Context, Result, bail, ensure};
use baylee_train_ops::evaluation::arms::{compare_arm, compare_exports, pool_arms};
use baylee_train_ops::evaluation::value_report::{read_positions, report, value_gap};
use baylee_train_ops::evaluation::{atomic_json, read_json, sample};
use clap::{Parser, Subcommand};
use serde_json::{Value, json};
use std::collections::BTreeMap;
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
        /// The arms are exports of one net (precision, a head): only the model
        /// may differ.
        #[arg(long)]
        exports: bool,
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
    /// Tables `ladder4` runs: only games that were really played, and apart from
    /// them those the house finished because the model's budget ran out.
    Ladder {
        #[arg(long)]
        out: PathBuf,
        /// The roots of the runs.
        roots: Vec<PathBuf>,
    },
    /// A plan of rule groups, each replaying only what a shadow run says it reaches.
    GroupPlan {
        /// The shadow runs of dev and fresh, in that order.
        #[arg(long, num_args = 2)]
        shadow: Vec<PathBuf>,
        #[arg(long)]
        src: PathBuf,
        #[arg(long)]
        runtime: PathBuf,
        #[arg(long)]
        root: PathBuf,
        #[arg(long)]
        execution: String,
        #[arg(long, default_value_t = 64)]
        threads: usize,
        #[arg(long, default_value_t = 96)]
        batch: usize,
        #[arg(long, default_value_t = 3000)]
        timeout: u64,
        #[arg(long)]
        out: PathBuf,
        /// The groups, in order.
        groups: Vec<String>,
    },
    /// Each group against the shadow runs on their whole splits, and pooled.
    GroupTable {
        /// The shadow runs of dev and fresh, in that order.
        #[arg(long, num_args = 2)]
        shadow: Vec<PathBuf>,
        #[arg(long)]
        root: PathBuf,
        #[arg(long, default_value_t = 20_000)]
        resamples: usize,
        #[arg(long)]
        out: PathBuf,
        groups: Vec<String>,
    },
    /// Two arms of `ladder4` on the same deck pairs and seats, paired by slot,
    /// clean games only, with a bootstrap over deck pairs.
    LadderPairs {
        /// Roots of the reference arm.
        #[arg(long, required = true)]
        reference: Vec<PathBuf>,
        /// Roots of the arm against it.
        #[arg(long, required = true)]
        arm: Vec<PathBuf>,
        /// Roots of the reference arm in a later round on the same deck pairs.
        /// Its games pair among themselves and join the same deck pairs.
        #[arg(long)]
        later_reference: Vec<PathBuf>,
        /// Roots of the arm in that later round.
        #[arg(long)]
        later_arm: Vec<PathBuf>,
        #[arg(long, default_value_t = 20_000)]
        resamples: usize,
        #[arg(long)]
        out: PathBuf,
    },
    /// How far two value logs of the same deals sit apart, as for two precisions.
    ValueGap {
        /// The two `--value-log` files.
        #[arg(long, num_args = 2)]
        logs: Vec<PathBuf>,
        #[arg(long)]
        out: PathBuf,
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
        "s1r" => ("/mnt/xdata/baylee/opponents/v5-base/s1r", &[]),
        "live" => (
            "/mnt/xdata/baylee/opponents/v5/20261027/model-control-final1500",
            &[],
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
    let mut flags: Vec<String> = vec!["--catalog-drift".into()];
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
        } else if let Some(groups) = part.strip_prefix("r=") {
            flags.extend(["--tactics-rules".into(), groups.replace('+', ",")]);
        } else if part == "shadow" {
            flags.push("--tactics-shadow".into());
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

fn pair(root: &Path, spec: &str, resamples: usize, exports: bool) -> Result<Value> {
    let (candidates, reference) = spec.split_once('~').context("CANDIDATES~REFERENCE")?;
    let runs: Vec<Value> = candidates
        .split(',')
        .map(|word| read_json(&eval_path(root, word)))
        .collect::<Result<_>>()?;
    let baseline = read_json(&eval_path(root, reference))?;
    let compared = if exports {
        compare_exports
    } else {
        compare_arm
    };
    let mut report = compared(&runs, &baseline, resamples)?;
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

/// The seat's closing summary of a played game: the last line of its
/// transcript, which names what the model and the house answered.
fn seat_summary(root: &Path, game: u64) -> Result<Option<Value>> {
    let directory = root.join(format!("game-{game:04}/transcripts"));
    let Ok(entries) = std::fs::read_dir(directory) else {
        return Ok(None);
    };
    for entry in entries {
        let path = entry?.path();
        if path.to_string_lossy().ends_with("seat1.jsonl") {
            let text = std::fs::read_to_string(path)?;
            let Some(last) = text.lines().next_back() else {
                continue;
            };
            let value: Value = serde_json::from_str(last)?;
            return Ok(Some(value["summary"].clone()));
        }
    }
    Ok(None)
}

/// Every game of `roots`, by arm: lost before it began, played clean, or played
/// with the house finishing for a model out of budget.
fn ladder_table(roots: &[PathBuf]) -> Result<Value> {
    let mut arms = BTreeMap::<String, [u64; 7]>::new();
    let mut games = Vec::new();
    for root in roots {
        let report = read_json(&root.join("ladder.json"))?;
        for game in report["games"].as_array().context("no games")? {
            let key = format!(
                "{} / {}",
                game["profile"].as_str().unwrap_or(""),
                game["mode"].as_str().unwrap_or("")
            );
            let tally = arms.entry(key).or_default();
            tally[0] += 1;
            let Some(won) = game["won"].as_bool() else {
                tally[1] += 1;
                games.push(json!({"root": root, "game": game["game"], "played": false,
                    "profile": game["profile"], "mode": game["mode"],
                    "why": game["error"].as_str().map_or("timed out, no seat", |e| e)}));
                continue;
            };
            let number = game["game"].as_u64().context("game number")?;
            let summary = seat_summary(root, number)?.unwrap_or(Value::Null);
            let declined = summary["fallbacks"]["declined"].as_u64().unwrap_or(0);
            // One declined answer is a stumble; a game the house finished is not the model's.
            let model = summary["answered"]["mind"].as_u64().unwrap_or(0);
            let house = summary["answered"]["house"].as_u64().unwrap_or(0);
            // A seat that left no summary died before the end: the table's result
            // says nothing of a game the model did not finish.
            let finished = summary.is_null() || house * 20 > model + house;
            let (played, wins) = if finished { (4, 5) } else { (2, 3) };
            tally[played] += 1;
            tally[wins] += u64::from(won);
            tally[6] += house;
            games.push(json!({"root": root, "game": number, "played": true,
                "profile": game["profile"], "mode": game["mode"], "net_won": won,
                "drawn": game["drawn"],
                "model_answers": model,
                "house_answers": house, "declined": declined, "house_finished": finished,
                "turns": summary["turns"], "net_deck": game["net_deck"], "llm_deck": game["llm_deck"]}));
        }
    }
    println!(
        "| arm | scheduled | lost before play | clean games | net wins | house finished | net wins there |"
    );
    println!("| --- | --- | --- | --- | --- | --- | --- |");
    for (arm, tally) in &arms {
        println!(
            "| {arm} | {} | {} | {} | {} | {} | {} |",
            tally[0], tally[1], tally[2], tally[3], tally[4], tally[5]
        );
    }
    Ok(json!({"arms": arms, "games": games,
        "columns": ["scheduled", "lost_before_play", "clean", "clean_net_wins", "house_finished",
            "house_finished_net_wins", "house_answers"]}))
}

/// The deals of a shadow run that `group` would have changed. A deal outside this
/// list plays the same with the group on, so its difference is zero.
fn deals_reached(shadow: &Value, group: &str) -> Result<Vec<u64>> {
    let mut deals = Vec::new();
    for row in shadow["deal_details"].as_array().context("no deals")? {
        let fired = row["tactic_rules_fired"].as_array().context("no firings")?;
        let hit = fired
            .iter()
            .any(|entry| entry[0] == group && entry[1].as_u64().unwrap_or(0) > 0);
        if hit {
            deals.push(row["deal"].as_u64().context("deal")?);
        }
    }
    deals.sort_unstable();
    Ok(deals)
}

/// What a group's arm is called on a split.
fn group_word(group: &str, split: &str) -> String {
    format!("plain-r={group}@{split}")
}

/// A plan that replays, for each group, only the deals a shadow run says it
/// reaches, with those deal lists written beside it.
fn group_plan(
    shadows: &[PathBuf],
    groups: &[String],
    runtime: &Path,
    root: &Path,
    shape: (usize, usize, u64),
) -> Result<Vec<Value>> {
    let (threads, batch, timeout) = shape;
    let mut stages = Vec::new();
    for (split, path) in ["dev", "fresh"].iter().zip(shadows) {
        let shadow = read_json(path)?;
        for group in groups {
            let deals = deals_reached(&shadow, group)?;
            println!("{group} {split}: {} deals", deals.len());
            if deals.is_empty() {
                continue;
            }
            let list = root.join(format!("only-{group}-{split}.json"));
            atomic_json(&list, &json!(deals))?;
            let word = group_word(group, split);
            let mut stage = stage(&word, runtime, root, threads, batch, timeout, false)?;
            let command = stage["command"].as_array_mut().context("command")?;
            command.extend([json!("--only-deals"), json!(list)]);
            stages.push(stage);
        }
    }
    Ok(stages)
}

/// A group's run on a split as the whole split: the shadow run's deals, with the
/// ones the group reached as they were replayed.
fn merged_arm(shadow: &Value, replayed: Option<&Value>) -> Result<Value> {
    let mut arm = shadow.clone();
    let Some(replayed) = replayed else {
        return Ok(arm);
    };
    ensure!(
        replayed["evaluator_revision"] == shadow["evaluator_revision"],
        "A replay and its shadow run must come from one build"
    );
    let games: BTreeMap<u64, &Value> = replayed["deal_details"]
        .as_array()
        .context("no deals")?
        .iter()
        .filter_map(|row| Some((row["deal"].as_u64()?, row)))
        .collect();
    for row in arm["deal_details"].as_array_mut().context("no deals")? {
        if let Some(game) = row["deal"].as_u64().and_then(|deal| games.get(&deal)) {
            *row = (*game).clone();
        }
    }
    Ok(arm)
}

/// Each group against the shadow runs, on both splits and pooled.
fn group_table(
    root: &Path,
    shadows: &[PathBuf],
    groups: &[String],
    resamples: usize,
) -> Result<Vec<Value>> {
    let references: Vec<Value> = shadows
        .iter()
        .map(|path| read_json(path))
        .collect::<Result<_>>()?;
    println!("| arm | reference | deals | wins | ref | paired | better/worse | Tor A |");
    println!("| --- | --- | --- | --- | --- | --- | --- | --- |");
    let mut reports = Vec::new();
    for group in groups {
        let mut parts = Vec::new();
        for (split, reference) in ["dev", "fresh"].iter().zip(&references) {
            let path = eval_path(root, &group_word(group, split));
            let replayed = path.exists().then(|| read_json(&path)).transpose()?;
            let arm = merged_arm(reference, replayed.as_ref())?;
            let mut report = compare_arm(&[arm], reference, resamples)?;
            report["arm"] = json!(format!("{group} {split}"));
            report["reference"] = json!(format!("tactics {split}"));
            report["replayed"] = json!(replayed.map(|run| run["deals"].clone()));
            println!("{}", row(&report)?);
            parts.push(report);
        }
        let mut pooled = pool_arms(&parts, resamples)?;
        pooled["arm"] = json!(format!("{group} pooled"));
        pooled["reference"] = json!("");
        println!("{}", row(&pooled)?);
        for part in &mut parts {
            if let Some(fields) = part.as_object_mut() {
                fields.remove("rows");
            }
        }
        if let Some(fields) = pooled.as_object_mut() {
            fields.remove("rows");
        }
        parts.push(pooled);
        reports.extend(parts);
    }
    Ok(reports)
}

/// A clean game's result by its slot `(opponent, net's deck, model's deck)`.
/// A slot is one seat of one deck pair; the arm's other games are left out.
fn clean_slots(roots: &[PathBuf]) -> Result<BTreeMap<(String, String, String), f64>> {
    let table = ladder_table(roots)?;
    let mut slots = BTreeMap::new();
    for game in table["games"].as_array().context("no games")? {
        if game["played"] != true || game["house_finished"] != false {
            continue;
        }
        let text = |key: &str| game[key].as_str().unwrap_or("").to_owned();
        // A draw counts one half, as everywhere else in the evaluation.
        let won = if game["net_won"] == true {
            1.0
        } else if game["drawn"] == true {
            0.5
        } else {
            0.0
        };
        slots.insert((text("profile"), text("net_deck"), text("llm_deck")), won);
    }
    Ok(slots)
}

/// The deck pair of a slot, whichever seat it is.
fn deck_pair(slot: &(String, String, String)) -> (String, String) {
    if slot.1 < slot.2 {
        (slot.1.clone(), slot.2.clone())
    } else {
        (slot.2.clone(), slot.1.clone())
    }
}

/// One slot clean in both arms.
#[derive(Clone)]
struct Slot {
    opponent: String,
    pair: (String, String),
    reference: f64,
    arm: f64,
}

/// Mean of `arm - reference` over slots, with a bootstrap that draws deck pairs,
/// each carrying all its slots.
fn slot_table(slots: &[Slot], resamples: usize) -> Value {
    let mut pairs: BTreeMap<&(String, String), (f64, usize)> = BTreeMap::new();
    for slot in slots {
        let entry = pairs.entry(&slot.pair).or_default();
        entry.0 += slot.arm - slot.reference;
        entry.1 += 1;
    }
    let pairs: Vec<(f64, usize)> = pairs.into_values().collect();
    let count = slots.len();
    let reference: f64 = slots.iter().map(|slot| slot.reference).sum();
    let arm: f64 = slots.iter().map(|slot| slot.arm).sum();
    let mut rng = 20_261_106_u64;
    let mut means: Vec<f64> = (0..resamples)
        .map(|_| {
            let (mut sum, mut seen) = (0.0, 0_usize);
            for _ in 0..pairs.len() {
                let (delta, slots) = pairs[sample(&mut rng, pairs.len())];
                sum += delta;
                seen += slots;
            }
            sum / seen.max(1) as f64
        })
        .collect();
    means.sort_by(f64::total_cmp);
    let at = |share: f64| means.get((resamples as f64 * share) as usize).copied();
    json!({"slots": count, "pairs": pairs.len(), "reference_wins": reference, "arm_wins": arm,
        "difference": (arm - reference) / count.max(1) as f64,
        "interval": [at(0.025), at(0.975)],
        "only_arm": slots.iter().filter(|slot| slot.arm > slot.reference).count(),
        "only_reference": slots.iter().filter(|slot| slot.reference > slot.arm).count()})
}

/// The slots of one round clean in both arms, and how many each arm had clean.
fn round_slots(reference: &[PathBuf], arm: &[PathBuf]) -> Result<(Vec<Slot>, usize, usize)> {
    let (before, after) = (clean_slots(reference)?, clean_slots(arm)?);
    let slots = before
        .iter()
        .filter_map(|(slot, reference)| {
            Some(Slot {
                opponent: slot.0.clone(),
                pair: deck_pair(slot),
                reference: *reference,
                arm: *after.get(slot)?,
            })
        })
        .collect();
    Ok((slots, before.len(), after.len()))
}

/// The comparison fixed in `v6d-phase-d-notes.md`: per opponent and pooled, on
/// the slots clean in both arms. `later` is a second round of the same deck pairs,
/// which pairs among itself: a slot of two rounds is two games, not one.
fn ladder_pairs(
    reference: &[PathBuf],
    arm: &[PathBuf],
    later: Option<(&[PathBuf], &[PathBuf])>,
    resamples: usize,
) -> Result<Value> {
    let (mut slots, mut clean_reference, mut clean_arm) = round_slots(reference, arm)?;
    let mut dropped = clean_reference.max(clean_arm) - slots.len();
    if let Some((reference, arm)) = later {
        let (more, before, after) = round_slots(reference, arm)?;
        dropped += before.max(after) - more.len();
        clean_reference += before;
        clean_arm += after;
        slots.extend(more);
    }
    let mut table = serde_json::Map::new();
    let opponents: std::collections::BTreeSet<&String> =
        slots.iter().map(|slot| &slot.opponent).collect();
    for opponent in opponents {
        let own: Vec<_> = slots
            .iter()
            .filter(|slot| slot.opponent == *opponent)
            .cloned()
            .collect();
        table.insert(opponent.clone(), slot_table(&own, resamples));
    }
    table.insert("pooled".to_owned(), slot_table(&slots, resamples));
    table.insert("clean_reference".to_owned(), json!(clean_reference));
    table.insert("clean_arm".to_owned(), json!(clean_arm));
    table.insert("slots_without_a_pair".to_owned(), json!(dropped));
    Ok(Value::Object(table))
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

/// The table of `pairs`, printed and written to `out`.
fn table(root: &Path, resamples: usize, exports: bool, out: &Path, pairs: &[String]) -> Result<()> {
    // `A~B/C~D` is one arm on two sets of deals, pooled.
    let reports: Vec<Value> = pairs
        .iter()
        .map(|spec| {
            if !spec.contains('/') {
                return pair(root, spec, resamples, exports);
            }
            let parts: Vec<Value> = spec
                .split('/')
                .map(|part| pair(root, part, resamples, exports))
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
    atomic_json(out, &Value::Array(kept))
}

#[allow(clippy::too_many_lines, reason = "One arm per command, nothing else")]
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
            exports,
            out,
            pairs,
        } => table(&root, resamples, exports, &out, &pairs)?,
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
        Command::Ladder { out, roots } => {
            atomic_json(&out, &ladder_table(&roots)?)?;
        }
        Command::GroupPlan {
            shadow,
            src,
            runtime,
            root,
            execution,
            threads,
            batch,
            timeout,
            out,
            groups,
        } => {
            let stages = group_plan(&shadow, &groups, &runtime, &root, (threads, batch, timeout))?;
            let total = timeout * stages.len() as u64;
            atomic_json(
                &out,
                &json!({"cwd": src.join("tools/trainer"), "root": root.join(execution),
                    "total_timeout_seconds": total, "version": 1, "stages": stages}),
            )?;
        }
        Command::GroupTable {
            shadow,
            root,
            resamples,
            out,
            groups,
        } => {
            let reports = group_table(&root, &shadow, &groups, resamples)?;
            atomic_json(&out, &Value::Array(reports))?;
        }
        Command::LadderPairs {
            reference,
            arm,
            later_reference,
            later_arm,
            resamples,
            out,
        } => {
            ensure!(
                later_reference.is_empty() == later_arm.is_empty(),
                "a later round needs both arms"
            );
            let later = (!later_arm.is_empty())
                .then_some((later_reference.as_slice(), later_arm.as_slice()));
            let table = ladder_pairs(&reference, &arm, later, resamples)?;
            println!("{}", serde_json::to_string_pretty(&table)?);
            atomic_json(&out, &table)?;
        }
        Command::ValueGap { logs, out } => atomic_json(&out, &value_gap(&logs[0], &logs[1])?)?,
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
