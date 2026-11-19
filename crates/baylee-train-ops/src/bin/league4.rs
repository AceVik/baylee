//! Plans one iteration of on-policy RL for `BayleeAI` as a `train_ops
//! pipeline` plan: the iteration's collections, then the conversion and the
//! admission of its data. The learner stage comes from a separate effort and
//! is not planned here.
//!
//! A spec names the frozen runtime, the iteration's root, the deck manifest
//! (its train split), the current snapshot (its export and the checkpoint the
//! export was made from), the flags every collection shares, and three kinds
//! of runs, each with its deal count, its first deal seed and its policy seed:
//! self-play against the current snapshot, historical runs against listed
//! past snapshots, and runs against a house profile (`sharp`). Every run is an
//! `eval4 --collection-purpose on-policy-training` collection: 1v1, raw FP32,
//! K2, temperature 0.7, train decks, records, no baseline game. With
//! `exploration`, the learner explores with rule proposals (`--explore-rules`,
//! `--explore-epsilon`): the records are exploration records, outcomes their
//! only labels.
//!
//! `league4 plan` writes the stages in order — `collect-NAME` for every run,
//! `convert`, `admit` — and leaves out a stage whose outputs exist, so planning
//! again after a stop resumes the iteration. What a collection may be is
//! `eval4`'s, `convert4`'s and admission's to check; this planner checks what
//! makes runs of one iteration distinct: their names, deal seeds and policy
//! seeds.
use anyhow::{Context, Result, ensure};
use baylee_train_ops::evaluation::atomic_json;
use clap::{Parser, Subcommand};
use serde::Deserialize;
use serde_json::{Value, json};
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

#[derive(Parser)]
#[command(about = "Plans one on-policy RL iteration: collections, conversion, admission")]
struct Args {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Writes the `train_ops pipeline` plan of an iteration's stages not yet done.
    Plan {
        #[arg(long)]
        spec: PathBuf,
        #[arg(long)]
        out: PathBuf,
    },
}

/// An iteration. Every path is absolute.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Spec {
    /// The frozen runtime holding `eval4`, `convert4` and `train_ops`.
    runtime: PathBuf,
    /// The frozen source worktree the pipeline runs in.
    cwd: PathBuf,
    /// Where everything the iteration makes goes.
    root: PathBuf,
    /// The execution directory's name under `root`.
    execution: String,
    /// The deck manifest; collections read its train split.
    manifest: PathBuf,
    /// The current snapshot: the learner of every run.
    learner: Snapshot,
    /// Flags every collection shares beside those the plan sets, such as
    /// `--threads` or `--inference-batch`.
    #[serde(default)]
    collect_flags: Vec<String>,
    /// The GPU lock the collections take; absent, they take none.
    #[serde(default)]
    gpu_lock: Option<PathBuf>,
    /// Rule proposals as recorded exploration for the learner of every run.
    #[serde(default)]
    exploration: Option<Exploration>,
    /// Self-play against the current snapshot.
    #[serde(default)]
    self_play: Vec<Run>,
    /// Runs against past snapshots.
    #[serde(default)]
    historical: Vec<Run>,
    /// Runs against a house profile.
    #[serde(default)]
    house: Vec<Run>,
    convert: Convert,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Snapshot {
    /// The FP32 export `eval4 --model` reads.
    model: PathBuf,
    /// The checkpoint the export was made from, which admission binds.
    checkpoint: PathBuf,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Exploration {
    /// `--explore-rules`: a group list or `measured`.
    rules: String,
    /// `--explore-epsilon`, `0 < ε < 1`.
    epsilon: f64,
}

/// What every run names: `records-NAME`, `collect-NAME.json`.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Run {
    /// Lowercase letters and digits.
    name: String,
    /// Deals; a 1v1 table is two deals, each seat once the learner's.
    deals: u64,
    /// The first deal seed: the run's seeds are `first_seed` up to
    /// `first_seed + deals / 2` (rounded up).
    first_seed: u64,
    /// The policy sampling seed, independent of the deal seeds.
    policy_seed: u64,
    timeout_seconds: u64,
    /// A historical run's past snapshot: its FP32 export, on the learner's encoder.
    #[serde(default)]
    opponent: Option<PathBuf>,
    /// A house run's profile: `sharp`, `steady`, `expert`.
    #[serde(default)]
    against: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Convert {
    /// `convert4 --encoder-version`, the learner's encoder.
    encoder_version: u32,
    /// `convert4 --schedule-seed`.
    schedule_seed: u64,
    /// `convert4 --threads`; `0` all cores.
    #[serde(default)]
    threads: u32,
    timeout_seconds: u64,
}

const MEMORY_GIB: f64 = 48.0;
const ADMIT_SECONDS: u64 = 3600;

/// Who sits opposite the learner in a run.
#[derive(Clone, Copy)]
enum Opponent<'a> {
    SelfPlay,
    Historical(&'a Path),
    House(&'a str),
}

fn path(path: &Path) -> String {
    path.display().to_string()
}

/// Every run of the iteration with its opponent, in plan order: self-play,
/// historical, house.
///
/// # Errors
/// A self-play run that names an opponent, a historical run without its
/// `opponent` export, or a house run without its `against` profile.
fn runs(spec: &Spec) -> Result<Vec<(&Run, Opponent<'_>)>> {
    let self_play = spec
        .self_play
        .iter()
        .map(|run| match (&run.opponent, &run.against) {
            (None, None) => Ok((run, Opponent::SelfPlay)),
            _ => anyhow::bail!("self-play run {} plays the current snapshot", run.name),
        });
    let historical = spec
        .historical
        .iter()
        .map(|run| match (&run.opponent, &run.against) {
            (Some(model), None) => Ok((run, Opponent::Historical(model))),
            _ => anyhow::bail!("historical run {} names its past snapshot alone", run.name),
        });
    let house = spec
        .house
        .iter()
        .map(|run| match (&run.opponent, &run.against) {
            (None, Some(against)) => Ok((run, Opponent::House(against))),
            _ => anyhow::bail!("house run {} names its profile alone", run.name),
        });
    self_play.chain(historical).chain(house).collect()
}

/// The deal seeds a run plays: two deals a table.
const fn seeds(run: &Run) -> (u64, u64) {
    (run.first_seed, run.first_seed + run.deals.div_ceil(2))
}

fn validate(spec: &Spec) -> Result<()> {
    let mut paths = vec![
        &spec.runtime,
        &spec.cwd,
        &spec.root,
        &spec.manifest,
        &spec.learner.model,
        &spec.learner.checkpoint,
    ];
    paths.extend(spec.gpu_lock.iter());
    paths.extend(
        spec.historical
            .iter()
            .filter_map(|run| run.opponent.as_ref()),
    );
    for path in paths {
        ensure!(path.is_absolute(), "not absolute: {}", path.display());
    }
    ensure!(
        !spec.execution.is_empty()
            && spec
                .execution
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_'),
        "an execution name is letters, digits, - and _"
    );
    if let Some(exploration) = &spec.exploration {
        baylee_train_ops::exploration::validate_epsilon(exploration.epsilon)?;
        ensure!(
            !exploration.rules.contains(char::is_whitespace),
            "exploration rules are a comma list"
        );
    }
    let runs = runs(spec)?;
    ensure!(!runs.is_empty(), "an iteration collects some games");
    let (mut names, mut policy_seeds) = (BTreeSet::new(), BTreeSet::new());
    let mut ranges: Vec<(u64, u64, &str)> = Vec::new();
    for (run, opponent) in &runs {
        ensure!(
            !run.name.is_empty()
                && run
                    .name
                    .bytes()
                    .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit())
                && names.insert(&run.name),
            "a run's name is unique lowercase letters and digits: {:?}",
            run.name
        );
        ensure!(
            run.deals > 0 && run.timeout_seconds > 0,
            "run {}: some deals and a timeout",
            run.name
        );
        // The same policy seed would replay the same sampling streams deal by deal.
        ensure!(
            policy_seeds.insert(run.policy_seed),
            "run {}: policy seed {} is another run's",
            run.name,
            run.policy_seed
        );
        let (first, end) = seeds(run);
        ensure!(end > first, "run {}: deal seeds overflow", run.name);
        if let Some((_, _, other)) = ranges.iter().find(|&&(a, b, _)| first < b && a < end) {
            anyhow::bail!(
                "run {}: deal seeds {first}..{end} overlap run {other}'s",
                run.name
            );
        }
        ranges.push((first, end, &run.name));
        if let Opponent::House(against) = opponent {
            ensure!(
                !against.is_empty() && against.bytes().all(|b| b.is_ascii_lowercase()),
                "run {}: a house profile name",
                run.name
            );
        }
    }
    validate_flags(&spec.collect_flags)
}

/// The `eval4` flags the plan sets for every collection: the on-policy
/// contract, the run's identity and its outputs.
const PLAN_FLAGS: [&str; 22] = [
    "--model",
    "--collection-purpose",
    "--raw-policy",
    "--temperature",
    "--thinking-steps",
    "--precision",
    "--seats",
    "--teams",
    "--decks-manifest",
    "--deck-split",
    "--deals",
    "--deal-offset",
    "--first-seed",
    "--policy-seed",
    "--opponent-model",
    "--opponent-role",
    "--against",
    "--explore-rules",
    "--explore-epsilon",
    "--no-baseline",
    "--record-out",
    "--out",
];

/// Shared collection flags leave the plan's own alone.
fn validate_flags(flags: &[String]) -> Result<()> {
    for flag in flags {
        let name = flag.split('=').next().unwrap_or_default();
        ensure!(
            !PLAN_FLAGS.contains(&name),
            "{flag} is the plan's to set, not a shared collection flag"
        );
    }
    Ok(())
}

/// A stage and the outputs that, all present, mean it is done.
type Planned = (Value, Vec<PathBuf>);

fn stage(
    name: &str,
    seconds: u64,
    gpu: Option<&Path>,
    inputs: &[PathBuf],
    command: &[String],
    outputs: Vec<PathBuf>,
) -> Planned {
    let mut stage = json!({
        "name": name,
        "timeout_seconds": seconds,
        "memory_gib": MEMORY_GIB,
        "inputs": inputs,
        "command": command,
        "required_outputs": outputs,
    });
    if let Some(lock) = gpu {
        stage["gpu_lock"] = json!(lock);
    }
    (stage, outputs)
}

fn records(spec: &Spec, run: &Run) -> PathBuf {
    spec.root.join(format!("records-{}", run.name))
}

fn summary(spec: &Spec, run: &Run) -> PathBuf {
    spec.root.join(format!("collect-{}.json", run.name))
}

fn dataset(spec: &Spec) -> PathBuf {
    spec.root.join("dataset")
}

/// One collection per run.
fn collect_stages(spec: &Spec, runs: &[(&Run, Opponent<'_>)]) -> Vec<Planned> {
    runs.iter()
        .map(|&(run, opponent)| {
            let mut command = vec![
                path(&spec.runtime.join("eval4")),
                "--model".into(),
                path(&spec.learner.model),
                "--collection-purpose".into(),
                "on-policy-training".into(),
                "--raw-policy".into(),
                "--temperature".into(),
                "0.7".into(),
                "--thinking-steps".into(),
                "2".into(),
                "--precision".into(),
                "float32".into(),
                "--seats".into(),
                "2".into(),
                "--decks-manifest".into(),
                path(&spec.manifest),
                "--deck-split".into(),
                "train".into(),
                "--no-baseline".into(),
            ];
            let mut inputs = vec![
                spec.manifest.clone(),
                spec.learner.model.join("model.onnx.json"),
            ];
            match opponent {
                Opponent::SelfPlay => {
                    command.extend(["--opponent-role".into(), "self-play".into()]);
                }
                Opponent::Historical(model) => {
                    command.extend([
                        "--opponent-model".into(),
                        path(model),
                        "--opponent-role".into(),
                        "historical".into(),
                    ]);
                    inputs.push(model.join("model.onnx.json"));
                }
                Opponent::House(against) => {
                    command.extend(["--against".into(), against.to_owned()]);
                }
            }
            if let Some(exploration) = &spec.exploration {
                command.extend([
                    "--explore-rules".into(),
                    exploration.rules.clone(),
                    "--explore-epsilon".into(),
                    exploration.epsilon.to_string(),
                ]);
            }
            command.extend(spec.collect_flags.iter().cloned());
            command.extend([
                "--deals".into(),
                run.deals.to_string(),
                "--first-seed".into(),
                run.first_seed.to_string(),
                "--policy-seed".into(),
                run.policy_seed.to_string(),
                "--record-out".into(),
                path(&records(spec, run)),
                "--out".into(),
                path(&summary(spec, run)),
            ]);
            stage(
                &format!("collect-{}", run.name),
                run.timeout_seconds,
                spec.gpu_lock.as_deref(),
                &inputs,
                &command,
                vec![
                    summary(spec, run),
                    records(spec, run).join("run.complete.json"),
                ],
            )
        })
        .collect()
}

/// The iteration's dataset: every run's games, complete passive histories,
/// unsplit, with the collection manifest admission reads.
fn convert_stage(spec: &Spec, runs: &[(&Run, Opponent<'_>)]) -> Planned {
    let sources: Vec<String> = runs
        .iter()
        .map(|(run, _)| path(&records(spec, run)))
        .collect();
    let mut inputs: Vec<PathBuf> = runs
        .iter()
        .flat_map(|(run, _)| {
            [
                summary(spec, run),
                records(spec, run).join("run.complete.json"),
            ]
        })
        .collect();
    inputs.push(spec.manifest.clone());
    // The collections' own manifest: a game that seated a deck outside its
    // train split stops the conversion.
    let command = vec![
        path(&spec.runtime.join("convert4")),
        "--encoder-version".into(),
        spec.convert.encoder_version.to_string(),
        "--runs".into(),
        sources.join(","),
        "--train-manifest".into(),
        path(&spec.manifest),
        "--out".into(),
        path(&dataset(spec)),
        "--include-passive-obs".into(),
        "--val-every".into(),
        "0".into(),
        "--threads".into(),
        spec.convert.threads.to_string(),
        "--schedule-seed".into(),
        spec.convert.schedule_seed.to_string(),
    ];
    stage(
        "convert",
        spec.convert.timeout_seconds,
        None,
        &inputs,
        &command,
        vec![
            dataset(spec).join("meta.json"),
            dataset(spec).join("collection.json"),
        ],
    )
}

/// The learner-only admission of the iteration's data, bound to the
/// snapshot's checkpoint.
fn admit_stage(spec: &Spec) -> Planned {
    let receipt = spec.root.join("admission.json");
    let command = vec![
        path(&spec.runtime.join("train_ops")),
        "admit-policy".into(),
        "--data".into(),
        path(&dataset(spec)),
        "--source-checkpoint".into(),
        path(&spec.learner.checkpoint),
        "--collection-manifest".into(),
        path(&dataset(spec).join("collection.json")),
        "--out".into(),
        path(&receipt),
    ];
    stage(
        "admit",
        ADMIT_SECONDS,
        None,
        &[
            dataset(spec).join("meta.json"),
            dataset(spec).join("collection.json"),
            spec.learner.checkpoint.clone(),
        ],
        &command,
        vec![receipt],
    )
}

/// Every stage of the iteration, in order, with the outputs that mark it done.
fn stages(spec: &Spec) -> Result<Vec<Planned>> {
    let runs = runs(spec)?;
    let mut out = collect_stages(spec, &runs);
    out.push(convert_stage(spec, &runs));
    out.push(admit_stage(spec));
    Ok(out)
}

fn plan(spec: &Spec, done: &dyn Fn(&Path) -> bool) -> Result<Value> {
    validate(spec)?;
    let stages: Vec<Value> = stages(spec)?
        .into_iter()
        .filter(|(_, outputs)| !outputs.iter().all(|output| done(output)))
        .map(|(stage, _)| stage)
        .collect();
    ensure!(!stages.is_empty(), "every stage of the iteration is done");
    let total: u64 = stages
        .iter()
        .filter_map(|stage| stage["timeout_seconds"].as_u64())
        .sum();
    Ok(json!({
        "version": 1,
        "root": spec.root.join(&spec.execution),
        "cwd": spec.cwd,
        "total_timeout_seconds": total,
        "stages": stages,
    }))
}

fn main() -> Result<()> {
    let Command::Plan { spec, out } = Args::parse().command;
    let text =
        std::fs::read_to_string(&spec).with_context(|| format!("read {}", spec.display()))?;
    let spec: Spec = serde_json::from_str(&text).context("parse the spec")?;
    let plan = plan(&spec, &|path: &Path| path.exists())?;
    atomic_json(&out, &plan)?;
    let names: Vec<&str> = plan["stages"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|s| s["name"].as_str())
        .collect();
    println!(
        "{} stages ({}) in {}",
        names.len(),
        names.join(", "),
        out.display()
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn spec() -> Spec {
        serde_json::from_value(json!({
            "runtime": "/rt", "cwd": "/src", "root": "/it3", "execution": "execution-i3",
            "manifest": "/decks/manifest.v4.json",
            "learner": {"model": "/snap/s3", "checkpoint": "/snap/s3.pt"},
            "collect_flags": ["--threads", "8"],
            "gpu_lock": "/lock",
            "exploration": {"rules": "measured", "epsilon": 0.1},
            "self_play": [
                {"name": "self", "deals": 512, "first_seed": 1000, "policy_seed": 31, "timeout_seconds": 7200}
            ],
            "historical": [
                {"name": "s1", "opponent": "/snap/s1", "deals": 256, "first_seed": 2000, "policy_seed": 32, "timeout_seconds": 3600},
                {"name": "s2", "opponent": "/snap/s2", "deals": 256, "first_seed": 3000, "policy_seed": 33, "timeout_seconds": 3600}
            ],
            "house": [
                {"name": "sharp", "against": "sharp", "deals": 255, "first_seed": 4000, "policy_seed": 34, "timeout_seconds": 3600}
            ],
            "convert": {"encoder_version": 10, "schedule_seed": 7, "threads": 8, "timeout_seconds": 3600}
        }))
        .unwrap()
    }

    fn names(plan: &Value) -> Vec<&str> {
        plan["stages"]
            .as_array()
            .unwrap()
            .iter()
            .map(|stage| stage["name"].as_str().unwrap())
            .collect()
    }

    fn after<'a>(command: &'a [Value], flag: &str) -> Option<&'a str> {
        let at = command.iter().position(|f| f == flag)?;
        command[at + 1].as_str()
    }

    fn has(command: &[Value], flag: &str) -> bool {
        command.iter().any(|f| f == flag)
    }

    #[test]
    fn an_iteration_collects_converts_and_admits_in_order() {
        let plan = plan(&spec(), &|_| false).unwrap();
        assert_eq!(
            names(&plan),
            [
                "collect-self",
                "collect-s1",
                "collect-s2",
                "collect-sharp",
                "convert",
                "admit"
            ]
        );
        assert_eq!(plan["version"], 1);
        assert_eq!(plan["root"], "/it3/execution-i3");
        assert_eq!(plan["cwd"], "/src");
        assert_eq!(plan["total_timeout_seconds"], 7200 + 3600 * 3 + 3600 + 3600);
        let stages = plan["stages"].as_array().unwrap();
        for stage in &stages[..4] {
            let command = stage["command"].as_array().unwrap();
            assert_eq!(command[0], "/rt/eval4");
            assert_eq!(after(command, "--model"), Some("/snap/s3"));
            assert_eq!(
                after(command, "--collection-purpose"),
                Some("on-policy-training")
            );
            assert!(has(command, "--raw-policy") && has(command, "--no-baseline"));
            assert_eq!(after(command, "--temperature"), Some("0.7"));
            assert_eq!(after(command, "--thinking-steps"), Some("2"));
            assert_eq!(after(command, "--precision"), Some("float32"));
            assert_eq!(after(command, "--seats"), Some("2"));
            assert_eq!(after(command, "--deck-split"), Some("train"));
            assert_eq!(after(command, "--explore-rules"), Some("measured"));
            assert_eq!(after(command, "--explore-epsilon"), Some("0.1"));
            assert_eq!(after(command, "--threads"), Some("8"));
            assert_eq!(stage["gpu_lock"], "/lock");
        }
        let at = |i: usize| stages[i]["command"].as_array().unwrap();
        // Self-play: the snapshot against itself.
        assert_eq!(after(at(0), "--opponent-role"), Some("self-play"));
        assert!(!has(at(0), "--opponent-model") && !has(at(0), "--against"));
        assert_eq!(after(at(0), "--deals"), Some("512"));
        assert_eq!(after(at(0), "--first-seed"), Some("1000"));
        assert_eq!(after(at(0), "--policy-seed"), Some("31"));
        assert_eq!(after(at(0), "--record-out"), Some("/it3/records-self"));
        assert_eq!(after(at(0), "--out"), Some("/it3/collect-self.json"));
        // Historical: a past snapshot, its metadata sealed as an input.
        assert_eq!(after(at(2), "--opponent-model"), Some("/snap/s2"));
        assert_eq!(after(at(2), "--opponent-role"), Some("historical"));
        assert!(
            stages[2]["inputs"]
                .as_array()
                .unwrap()
                .iter()
                .any(|input| input == "/snap/s2/model.onnx.json")
        );
        // House: the profile, no neural opponent.
        assert_eq!(after(at(3), "--against"), Some("sharp"));
        assert!(!has(at(3), "--opponent-model") && !has(at(3), "--opponent-role"));
        assert_eq!(
            stages[3]["required_outputs"],
            json!([
                "/it3/collect-sharp.json",
                "/it3/records-sharp/run.complete.json"
            ])
        );
        // Conversion: every run, complete unsplit histories, no GPU.
        let convert = at(4);
        assert_eq!(convert[0], "/rt/convert4");
        assert_eq!(
            after(convert, "--runs"),
            Some("/it3/records-self,/it3/records-s1,/it3/records-s2,/it3/records-sharp")
        );
        assert_eq!(after(convert, "--encoder-version"), Some("10"));
        assert_eq!(
            after(convert, "--train-manifest"),
            Some("/decks/manifest.v4.json")
        );
        assert!(has(convert, "--include-passive-obs"));
        assert_eq!(after(convert, "--val-every"), Some("0"));
        assert_eq!(after(convert, "--schedule-seed"), Some("7"));
        assert!(stages[4].get("gpu_lock").is_none());
        assert_eq!(
            stages[4]["required_outputs"],
            json!(["/it3/dataset/meta.json", "/it3/dataset/collection.json"])
        );
        // Admission: bound to the snapshot's checkpoint; no learner stage.
        let admit = at(5);
        assert_eq!(
            &admit[..2],
            &[json!("/rt/train_ops"), json!("admit-policy")]
        );
        assert_eq!(after(admit, "--data"), Some("/it3/dataset"));
        assert_eq!(after(admit, "--source-checkpoint"), Some("/snap/s3.pt"));
        assert_eq!(
            after(admit, "--collection-manifest"),
            Some("/it3/dataset/collection.json")
        );
        assert_eq!(after(admit, "--out"), Some("/it3/admission.json"));
        assert!(stages[5].get("gpu_lock").is_none());
    }

    #[test]
    fn without_exploration_the_collections_explore_nothing() {
        let mut spec = spec();
        spec.exploration = None;
        spec.gpu_lock = None;
        let plan = plan(&spec, &|_| false).unwrap();
        for stage in &plan["stages"].as_array().unwrap()[..4] {
            let command = stage["command"].as_array().unwrap();
            assert!(!has(command, "--explore-rules") && !has(command, "--explore-epsilon"));
            assert!(stage.get("gpu_lock").is_none());
        }
    }

    #[test]
    fn a_stage_whose_outputs_exist_is_left_out() {
        let done = |path: &Path| {
            path.to_string_lossy().starts_with("/it3/collect-")
                || path.to_string_lossy().starts_with("/it3/records-")
                || path.starts_with("/it3/dataset")
        };
        assert_eq!(names(&plan(&spec(), &done).unwrap()), ["admit"]);
        assert!(plan(&spec(), &|_| true).is_err(), "nothing left to do");
    }

    #[test]
    fn runs_are_distinct_in_name_deal_seeds_and_policy_seeds() {
        let mut overlap = spec();
        // 512 deals from 1000 play seeds 1000..1256; 1255 is inside.
        overlap.historical[0].first_seed = 1255;
        assert!(plan(&overlap, &|_| false).is_err());
        let mut adjacent = spec();
        adjacent.historical[0].first_seed = 1256;
        plan(&adjacent, &|_| false).unwrap();
        let mut seeds = spec();
        seeds.house[0].policy_seed = 31;
        assert!(plan(&seeds, &|_| false).is_err());
        let mut name = spec();
        name.house[0].name = "s1".into();
        assert!(plan(&name, &|_| false).is_err());
    }

    #[test]
    fn a_run_names_its_own_kind_of_opponent() {
        let mut spec = spec();
        spec.self_play[0].against = Some("sharp".into());
        assert!(plan(&spec, &|_| false).is_err());
        let mut spec = super::tests::spec();
        spec.historical[0].opponent = None;
        assert!(plan(&spec, &|_| false).is_err());
        let mut spec = super::tests::spec();
        spec.house[0].opponent = Some("/snap/s1".into());
        assert!(plan(&spec, &|_| false).is_err());
    }

    #[test]
    fn the_plan_sets_what_identifies_a_collection() {
        for flag in [
            "--record-out",
            "--policy-seed",
            "--explore-epsilon=0.5",
            "--temperature",
        ] {
            let mut spec = spec();
            spec.collect_flags.push(flag.into());
            assert!(plan(&spec, &|_| false).is_err(), "{flag}");
        }
        let mut spec = spec();
        spec.exploration = Some(Exploration {
            rules: "measured".into(),
            epsilon: 1.0,
        });
        assert!(plan(&spec, &|_| false).is_err(), "ε in (0, 1)");
        let mut spec = super::tests::spec();
        spec.learner.checkpoint = "relative.pt".into();
        assert!(plan(&spec, &|_| false).is_err());
    }
}
