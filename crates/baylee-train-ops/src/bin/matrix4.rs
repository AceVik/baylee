//! Plans of an evaluation matrix: models × arms × tables × seed sets.
//!
//! A spec names every piece once (the runtime, the flags all runs share, each
//! model's export, each arm's flags, each table's seats and teams, each seed
//! set's first seed) and lists blocks of runs in the order they should play.
//! `matrix4 plan` writes the `train_ops pipeline` plan of those runs. A run is
//! called `MODEL-ARM-TABLE-SEEDS`, writes `ROOT/eval-NAME.json`, and is left out
//! when that file exists, so planning again after a stop resumes the matrix.
//! The names are words `phase_a4 table` reads (`MODEL-ARM-TABLE-SEEDS~...`).
//!
//! Within a block the runs go table by table, seed set by seed set, arm by arm,
//! model by model: every model's run of one arm lands before the next arm, so
//! the pairs a block exists for are readable as early as possible.
//!
//! Several specs (`--spec A --spec B`) make one plan: their runs in that
//! order, each on its own runtime, under the first spec's execution root and
//! working directory. Two runtimes of one change are planned that way.
//!
//! A block with `pilots` plays only the deals whose net pilots one of those
//! decks: a stage before its runs of each table and seed set writes them with
//! the runtime's own `eval4 --write-deals`, and the runs read them with
//! `--only-deals`. A recorded block with `react` ends in one stage that counts
//! the reactive play of all its runs side by side (`gamelog4 react`).
use anyhow::{Context, Result, bail, ensure};
use baylee_train_ops::evaluation::atomic_json;
use clap::{Parser, Subcommand};
use serde::Deserialize;
use serde_json::{Value, json};
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

#[derive(Parser)]
#[command(about = "Plans an evaluation matrix of models, arms, tables and seed sets")]
struct Args {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Writes the `train_ops pipeline` plan of a spec's runs not yet done.
    Plan {
        /// The matrix specs (JSON), their runs played in this order.
        #[arg(long, required = true)]
        spec: Vec<PathBuf>,
        /// Where the plan goes.
        #[arg(long)]
        out: PathBuf,
    },
}

/// A matrix spec. Every path is absolute.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Spec {
    /// The frozen runtime holding `eval4`.
    runtime: PathBuf,
    /// The frozen source worktree the pipeline runs in.
    cwd: PathBuf,
    /// Where evaluations go.
    root: PathBuf,
    /// The execution directory's name under `root`.
    execution: String,
    /// The deck manifest every run reads.
    manifest: PathBuf,
    /// The GPU lock; absent, the runs take none.
    #[serde(default)]
    gpu_lock: Option<PathBuf>,
    /// Flags every run shares (opponent, shape, split, policy seed, ...).
    common: Vec<String>,
    /// Model name → export directory.
    models: BTreeMap<String, PathBuf>,
    /// Arm name → its flags.
    arms: BTreeMap<String, Vec<String>>,
    /// Table name → seats, teams and the shape of the run.
    tables: BTreeMap<String, Table>,
    /// Seed set name → its first seed.
    seeds: BTreeMap<String, u64>,
    /// The runs, in order.
    blocks: Vec<Block>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Table {
    seats: u32,
    /// `1,2,1,2`; empty for every seat on its own.
    #[serde(default)]
    teams: String,
    threads: u32,
    batch: u32,
    /// Measured wall time of one deal, for the stage's timeout.
    seconds_per_deal: f64,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Block {
    models: Vec<String>,
    arms: Vec<String>,
    tables: Vec<String>,
    seeds: Vec<String>,
    deals: u64,
    /// Keep the games (`--record-out ROOT/records-NAME`).
    #[serde(default)]
    record: bool,
    /// Play only the deals of `deals` whose net pilots one of these decks
    /// (deck keys). Before the block's runs of a table and seed set, a stage
    /// writes them (`eval4 --write-deals`, `ROOT/deals-TABLE-SEEDS-bINDEX.json`)
    /// with the block's first model and the spec's common flags, so the list
    /// holds what the runs seat; each run plays only them (`--only-deals`).
    #[serde(default)]
    pilots: Vec<String>,
    /// With `pilots`: how many deals the selection holds, which a run's time
    /// limit is measured on. Without, `deals`.
    #[serde(default)]
    played: Option<u64>,
    /// After the block's runs, one stage counts their reactive play side by
    /// side (`gamelog4 react` over every run's records, `ROOT/react-bINDEX.json`).
    /// Needs `record`.
    #[serde(default)]
    react: bool,
}

/// How much longer than measured a stage may take before the guard stops it.
const TIMEOUT_FACTOR: f64 = 3.0;
/// The least time a stage gets: start-up and a slow first batch.
const TIMEOUT_FLOOR: f64 = 600.0;
const MEMORY_GIB: f64 = 36.0;
/// What a deal selection or a count of records takes: no net, no GPU.
const LIGHT_MEMORY_GIB: f64 = 16.0;
/// Games a count of records replays at once.
const REACT_THREADS: u32 = 8;

fn word(name: &str) -> Result<()> {
    ensure!(
        !name.is_empty()
            && name
                .bytes()
                .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit()),
        "a name is lowercase letters and digits only, so a run's name splits at '-': {name:?}"
    );
    Ok(())
}

fn lookup<'a, T>(map: &'a BTreeMap<String, T>, key: &str, what: &str) -> Result<&'a T> {
    map.get(key)
        .with_context(|| format!("the spec names no {what} {key:?}"))
}

fn validate(spec: &Spec) -> Result<()> {
    for path in [&spec.runtime, &spec.cwd, &spec.root, &spec.manifest] {
        ensure!(path.is_absolute(), "not absolute: {}", path.display());
    }
    ensure!(
        !spec.execution.is_empty(),
        "an execution directory is needed"
    );
    for name in spec
        .models
        .keys()
        .chain(spec.arms.keys())
        .chain(spec.tables.keys())
        .chain(spec.seeds.keys())
    {
        word(name)?;
    }
    for (name, path) in &spec.models {
        ensure!(path.is_absolute(), "model {name} is not absolute");
    }
    for (name, table) in &spec.tables {
        ensure!((2..=8).contains(&table.seats), "table {name}: 2 to 8 seats");
        if !table.teams.is_empty() {
            let teams = table.teams.split(',').count();
            ensure!(
                teams == table.seats as usize,
                "table {name}: one team per seat"
            );
        }
        ensure!(
            table.seconds_per_deal.is_finite() && table.seconds_per_deal > 0.0,
            "table {name}: a positive time per deal"
        );
    }
    for flag in spec.common.iter().chain(spec.arms.values().flatten()) {
        if [
            "--model",
            "--seats",
            "--teams",
            "--first-seed",
            "--deals",
            "--out",
            "--record-out",
            "--only-deals",
            "--write-deals",
            "--pilot-decks",
        ]
        .contains(&flag.as_str())
        {
            bail!("{flag} is the matrix's to set, not a common or an arm's flag");
        }
    }
    ensure!(!spec.blocks.is_empty(), "a spec plans at least one block");
    for block in &spec.blocks {
        ensure!(block.deals > 0, "a block plays at least one deal");
        ensure!(
            block
                .pilots
                .iter()
                .all(|key| !key.is_empty() && !key.contains(',')),
            "a pilot is one deck key"
        );
        ensure!(
            block
                .played
                .is_none_or(|played| !block.pilots.is_empty() && played <= block.deals),
            "`played` counts the deals a selection of pilots holds, at most `deals`"
        );
        ensure!(
            !block.react || block.record,
            "a count of reactive play reads records"
        );
    }
    Ok(())
}

/// `eval4` with what every stage of a block's table and seed set shares:
/// the model, the deals, the decks and the table.
fn eval4(spec: &Spec, model: &Path, block: &Block, table: &Table) -> Vec<String> {
    let mut command = vec![
        spec.runtime.join("eval4").display().to_string(),
        "--model".into(),
        model.display().to_string(),
        "--deals".into(),
        block.deals.to_string(),
        "--decks-manifest".into(),
        spec.manifest.display().to_string(),
        "--seats".into(),
        table.seats.to_string(),
    ];
    if !table.teams.is_empty() {
        command.extend(["--teams".into(), table.teams.clone()]);
    }
    command.extend(spec.common.iter().cloned());
    command
}

/// The stage writing the deals of a block's table and seed set whose net
/// pilots the block's `pilots`, and the file it writes.
fn selection(
    spec: &Spec,
    (index, block): (usize, &Block),
    (table_name, table): (&str, &Table),
    (seeds_name, first_seed): (&str, u64),
) -> Result<(Value, PathBuf)> {
    let model_name = block.models.first().context("a block names a model")?;
    let model = lookup(&spec.models, model_name, "model")?;
    let out = spec
        .root
        .join(format!("deals-{table_name}-{seeds_name}-b{index}.json"));
    let mut command = eval4(spec, model, block, table);
    command.extend([
        "--first-seed".into(),
        first_seed.to_string(),
        "--pilot-decks".into(),
        block.pilots.join(","),
        "--write-deals".into(),
        out.display().to_string(),
    ]);
    let stage = json!({
        "name": format!("deals-{table_name}-{seeds_name}-b{index}"),
        "timeout_seconds": TIMEOUT_FLOOR,
        "memory_gib": LIGHT_MEMORY_GIB,
        "inputs": [spec.manifest, model.join("model.onnx.json")],
        "command": command,
        "required_outputs": [out],
    });
    Ok((stage, out))
}

/// The stage counting the reactive play of a block's runs, side by side.
fn react(spec: &Spec, index: usize, runs: &[String]) -> Value {
    let out = spec.root.join(format!("react-b{index}.json"));
    let mut command = vec![
        spec.runtime.join("gamelog4").display().to_string(),
        "react".into(),
    ];
    let mut inputs = Vec::new();
    for name in runs {
        let summary = spec.root.join(format!("eval-{name}.json"));
        command.extend([
            "--run".into(),
            spec.root
                .join(format!("records-{name}"))
                .display()
                .to_string(),
            "--summary".into(),
            summary.display().to_string(),
        ]);
        inputs.push(summary);
    }
    command.extend([
        "--threads".into(),
        REACT_THREADS.to_string(),
        "--out".into(),
        out.display().to_string(),
    ]);
    json!({
        "name": format!("react-b{index}"),
        "timeout_seconds": TIMEOUT_FLOOR * 3.0,
        "memory_gib": LIGHT_MEMORY_GIB,
        "inputs": inputs,
        "command": command,
        "required_outputs": [out],
    })
}

/// The stages of `spec`, without the runs whose output exists.
fn stages(spec: &Spec, done: &dyn Fn(&Path) -> bool) -> Result<Vec<Value>> {
    let mut stages = Vec::new();
    let mut names = BTreeSet::new();
    for (index, block) in spec.blocks.iter().enumerate() {
        let mut runs = Vec::new();
        for table_name in &block.tables {
            let table = lookup(&spec.tables, table_name, "table")?;
            for seeds_name in &block.seeds {
                let first_seed = *lookup(&spec.seeds, seeds_name, "seed set")?;
                let only = if block.pilots.is_empty() {
                    None
                } else {
                    let (stage, out) = selection(
                        spec,
                        (index, block),
                        (table_name, table),
                        (seeds_name, first_seed),
                    )?;
                    if !done(&out) {
                        stages.push(stage);
                    }
                    Some(out)
                };
                for arm_name in &block.arms {
                    let arm = lookup(&spec.arms, arm_name, "arm")?;
                    for model_name in &block.models {
                        let model = lookup(&spec.models, model_name, "model")?;
                        let name = format!("{model_name}-{arm_name}-{table_name}-{seeds_name}");
                        if !names.insert(name.clone()) {
                            continue;
                        }
                        runs.push(name.clone());
                        let out = spec.root.join(format!("eval-{name}.json"));
                        if done(&out) {
                            continue;
                        }
                        let mut command = eval4(spec, model, block, table);
                        command.extend(arm.iter().cloned());
                        command.extend([
                            "--threads".into(),
                            table.threads.to_string(),
                            "--inference-batch".into(),
                            table.batch.to_string(),
                            "--first-seed".into(),
                            first_seed.to_string(),
                        ]);
                        let mut inputs = vec![spec.manifest.clone(), model.join("model.onnx.json")];
                        if let Some(only) = &only {
                            command.extend(["--only-deals".into(), only.display().to_string()]);
                            inputs.push(only.clone());
                        }
                        if block.record {
                            command.extend([
                                "--record-out".into(),
                                spec.root
                                    .join(format!("records-{name}"))
                                    .display()
                                    .to_string(),
                            ]);
                        }
                        command.extend(["--out".into(), out.display().to_string()]);
                        #[allow(clippy::cast_precision_loss, reason = "deal counts are small")]
                        let measured =
                            block.played.unwrap_or(block.deals) as f64 * table.seconds_per_deal;
                        let mut stage = json!({
                            "name": name,
                            "timeout_seconds": (measured * TIMEOUT_FACTOR).max(TIMEOUT_FLOOR).ceil(),
                            "memory_gib": MEMORY_GIB,
                            "inputs": inputs,
                            "command": command,
                            "required_outputs": [out],
                        });
                        if let Some(lock) = &spec.gpu_lock {
                            stage["gpu_lock"] = json!(lock);
                        }
                        stages.push(stage);
                    }
                }
            }
        }
        if block.react && !done(&spec.root.join(format!("react-b{index}.json"))) {
            stages.push(react(spec, index, &runs));
        }
    }
    Ok(stages)
}

fn plan(specs: &[Spec], done: &dyn Fn(&Path) -> bool) -> Result<Value> {
    let first = specs.first().context("a plan needs a spec")?;
    let mut stages = Vec::new();
    for spec in specs {
        validate(spec)?;
        stages.extend(self::stages(spec, done)?);
    }
    ensure!(!stages.is_empty(), "every run of the specs is done");
    let mut names = BTreeSet::new();
    for stage in &stages {
        let name = stage["name"].as_str().unwrap_or_default();
        ensure!(
            names.insert(name.to_owned()),
            "two specs plan the run {name}"
        );
    }
    let total: f64 = stages
        .iter()
        .filter_map(|stage| stage["timeout_seconds"].as_f64())
        .sum();
    Ok(json!({
        "version": 1,
        "root": first.root.join(&first.execution),
        "cwd": first.cwd,
        "total_timeout_seconds": total,
        "stages": stages,
    }))
}

fn main() -> Result<()> {
    let args = Args::parse();
    match args.command {
        Command::Plan { spec, out } => {
            let specs: Vec<Spec> = spec
                .iter()
                .map(|path| {
                    let text = std::fs::read_to_string(path)
                        .with_context(|| format!("read {}", path.display()))?;
                    serde_json::from_str(&text)
                        .with_context(|| format!("parse the spec {}", path.display()))
                })
                .collect::<Result<_>>()?;
            let plan = plan(&specs, &|path: &Path| path.exists())?;
            let stages = plan["stages"].as_array().map_or(0, Vec::len);
            atomic_json(&out, &plan)?;
            println!(
                "{stages} runs, at most {:.0} s, in {}",
                plan["total_timeout_seconds"].as_f64().unwrap_or(0.0),
                out.display()
            );
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn spec() -> Spec {
        serde_json::from_value(json!({
            "runtime": "/rt", "cwd": "/src", "root": "/out", "execution": "execution-a",
            "manifest": "/decks/manifest.json", "gpu_lock": "/lock",
            "common": ["--against", "sharp"],
            "models": {"r1": "/m/r1", "gen4s1": "/m/g4"},
            "arms": {"raw": ["--raw-policy"], "tac": ["--raw-policy", "--choice-guard", "tactical"]},
            "tables": {
                "1v1": {"seats": 2, "threads": 16, "batch": 64, "seconds_per_deal": 0.3},
                "2v2": {"seats": 4, "teams": "1,2,1,2", "threads": 16, "batch": 128, "seconds_per_deal": 2.2}
            },
            "seeds": {"test": 11, "fresh": 19},
            "blocks": [
                {"models": ["r1", "gen4s1"], "arms": ["raw", "tac"], "tables": ["1v1"], "seeds": ["test"], "deals": 512},
                {"models": ["r1"], "arms": ["tac"], "tables": ["2v2"], "seeds": ["test", "fresh"], "deals": 128, "record": true}
            ]
        }))
        .unwrap()
    }

    fn names(plan: &Value) -> Vec<String> {
        plan["stages"]
            .as_array()
            .unwrap()
            .iter()
            .map(|stage| stage["name"].as_str().unwrap().to_owned())
            .collect()
    }

    #[test]
    fn runs_go_arm_by_arm_and_every_model_of_an_arm_lands_together() {
        let plan = plan(&[spec()], &|_| false).unwrap();
        assert_eq!(
            names(&plan),
            [
                "r1-raw-1v1-test",
                "gen4s1-raw-1v1-test",
                "r1-tac-1v1-test",
                "gen4s1-tac-1v1-test",
                "r1-tac-2v2-test",
                "r1-tac-2v2-fresh",
            ]
        );
        assert_eq!(plan["root"], "/out/execution-a");
    }

    #[test]
    fn a_run_carries_its_table_seed_and_arm() {
        let plan = plan(&[spec()], &|_| false).unwrap();
        let stage = &plan["stages"][5];
        let command: Vec<&str> = stage["command"]
            .as_array()
            .unwrap()
            .iter()
            .map(|flag| flag.as_str().unwrap())
            .collect();
        let after = |flag: &str| {
            let at = command.iter().position(|f| *f == flag).unwrap();
            command[at + 1]
        };
        assert_eq!(command[0], "/rt/eval4");
        assert_eq!(after("--model"), "/m/r1");
        assert_eq!(after("--seats"), "4");
        assert_eq!(after("--teams"), "1,2,1,2");
        assert_eq!(after("--first-seed"), "19");
        assert_eq!(after("--choice-guard"), "tactical");
        assert_eq!(after("--record-out"), "/out/records-r1-tac-2v2-fresh");
        assert_eq!(after("--out"), "/out/eval-r1-tac-2v2-fresh.json");
        assert_eq!(stage["gpu_lock"], "/lock");
        // 128 deals at 2.2 s, three times over.
        assert_eq!(stage["timeout_seconds"], 845.0);
    }

    #[test]
    fn a_done_run_is_left_out_so_a_new_plan_resumes() {
        let done = |path: &Path| path.ends_with("eval-r1-raw-1v1-test.json");
        let plan = plan(&[spec()], &done).unwrap();
        assert_eq!(names(&plan)[0], "gen4s1-raw-1v1-test");
        assert_eq!(names(&plan).len(), 5);
    }

    #[test]
    fn a_name_that_would_not_split_is_refused() {
        let mut spec = spec();
        let model = spec.models.remove("r1").unwrap();
        spec.models.insert("r1-4eb".into(), model);
        assert!(plan(std::slice::from_ref(&spec), &|_| false).is_err());
    }

    #[test]
    fn the_matrix_sets_the_model_and_seats_itself() {
        let mut spec = spec();
        spec.common.push("--seats".into());
        assert!(plan(std::slice::from_ref(&spec), &|_| false).is_err());
    }

    #[test]
    fn an_unknown_arm_is_named() {
        let mut spec = spec();
        spec.blocks[0].arms.push("search".into());
        let error = plan(std::slice::from_ref(&spec), &|_| false)
            .unwrap_err()
            .to_string();
        assert!(error.contains("search"), "{error}");
    }

    fn piloted() -> Spec {
        let mut spec = spec();
        spec.blocks = vec![
            serde_json::from_value(json!({"models": ["r1"], "arms": ["raw", "tac"],
                "tables": ["1v1"], "seeds": ["test"], "deals": 3900, "played": 150,
                "pilots": ["house/victory"], "record": true, "react": true}))
            .unwrap(),
        ];
        spec
    }

    fn command(stage: &Value) -> Vec<&str> {
        stage["command"]
            .as_array()
            .unwrap()
            .iter()
            .map(|flag| flag.as_str().unwrap())
            .collect()
    }

    fn after<'a>(command: &[&'a str], flag: &str) -> &'a str {
        let at = command.iter().position(|f| *f == flag).unwrap();
        command[at + 1]
    }

    /// The deals are chosen with the runs' own flags before the runs, which
    /// play only them; the count of reactive play reads every run after.
    #[test]
    fn pilots_select_the_deals_first_and_react_counts_the_runs_last() {
        let plan = plan(&[piloted()], &|_| false).unwrap();
        assert_eq!(
            names(&plan),
            [
                "deals-1v1-test-b0",
                "r1-raw-1v1-test",
                "r1-tac-1v1-test",
                "react-b0"
            ]
        );
        let selection = command(&plan["stages"][0]);
        assert_eq!(after(&selection, "--pilot-decks"), "house/victory");
        assert_eq!(
            after(&selection, "--write-deals"),
            "/out/deals-1v1-test-b0.json"
        );
        assert_eq!(after(&selection, "--first-seed"), "11");
        assert_eq!(after(&selection, "--deals"), "3900");
        assert!(plan["stages"][0].get("gpu_lock").is_none());
        let run = command(&plan["stages"][2]);
        assert_eq!(after(&run, "--only-deals"), "/out/deals-1v1-test-b0.json");
        assert_eq!(after(&run, "--record-out"), "/out/records-r1-tac-1v1-test");
        assert!(
            plan["stages"][2]["inputs"]
                .as_array()
                .unwrap()
                .contains(&json!("/out/deals-1v1-test-b0.json"))
        );
        // 150 deals at 0.3 s, three times over, is under the floor.
        assert_eq!(plan["stages"][2]["timeout_seconds"], 600.0);
        let react = command(&plan["stages"][3]);
        assert_eq!(react[..2], ["/rt/gamelog4", "react"]);
        assert_eq!(after(&react, "--run"), "/out/records-r1-raw-1v1-test");
        assert_eq!(after(&react, "--out"), "/out/react-b0.json");
        assert_eq!(react.iter().filter(|flag| **flag == "--summary").count(), 2);
    }

    /// A stopped plan resumes: done runs leave the plan and the count reads
    /// their summaries as they are.
    #[test]
    fn a_done_selection_and_done_runs_leave_only_what_is_missing() {
        let done = |path: &Path| {
            path.ends_with("deals-1v1-test-b0.json") || path.ends_with("eval-r1-raw-1v1-test.json")
        };
        let plan = plan(&[piloted()], &done).unwrap();
        assert_eq!(names(&plan), ["r1-tac-1v1-test", "react-b0"]);
    }

    #[test]
    fn react_without_records_and_the_matrix_flags_in_an_arm_are_refused() {
        let mut spec = piloted();
        spec.blocks[0].record = false;
        assert!(plan(std::slice::from_ref(&spec), &|_| false).is_err());
        let mut spec = piloted();
        spec.arms
            .get_mut("tac")
            .unwrap()
            .extend(["--only-deals".to_owned(), "/x.json".to_owned()]);
        assert!(plan(std::slice::from_ref(&spec), &|_| false).is_err());
        let mut spec = piloted();
        spec.blocks[0].played = Some(4000);
        assert!(plan(std::slice::from_ref(&spec), &|_| false).is_err());
    }

    /// Two runtimes of one change: the second spec's runs follow the first's,
    /// each with its own `eval4`, under the first spec's execution root.
    #[test]
    fn two_specs_make_one_plan_in_order() {
        let mut other = spec();
        other.runtime = PathBuf::from("/old");
        other.execution = "execution-b".into();
        other.models = BTreeMap::from([("r1old".to_owned(), PathBuf::from("/m/r1"))]);
        other.blocks.truncate(1);
        other.blocks[0].models = vec!["r1old".into()];
        let plan = plan(&[spec(), other], &|_| false).unwrap();
        let names = names(&plan);
        assert_eq!(names.len(), 8);
        assert_eq!(names[6..], ["r1old-raw-1v1-test", "r1old-tac-1v1-test"]);
        assert_eq!(plan["stages"][7]["command"][0], "/old/eval4");
        assert_eq!(plan["root"], "/out/execution-a");
        // The same run planned twice is refused.
        assert!(super::plan(&[spec(), spec()], &|_| false).is_err());
    }
}
