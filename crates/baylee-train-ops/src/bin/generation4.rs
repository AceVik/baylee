//! Plans one generation of expert iteration: games a searching teacher
//! plays, the distillation dataset made of them, student seeds trained on
//! it and their exports, as one `train_ops pipeline` plan.
//!
//! A spec names the teacher's runtime and model, the flags its games share
//! (opponent, decks, search), each collection's table and seeds, the
//! converter's flags, the student's initial checkpoint, seeds and trainer
//! flags. `generation4 plan` writes the stages in order — `collect-NAME`,
//! `convert`, `train-sN`, `export-sN` — and leaves out a stage whose
//! outputs exist, so planning again after a stop resumes the generation.
//! What a collection may be is the converter's to check (search targets
//! come from raw-policy runs only), not this planner's.
use anyhow::{Context, Result, ensure};
use baylee_train_ops::evaluation::atomic_json;
use clap::{Parser, Subcommand};
use serde::Deserialize;
use serde_json::{Value, json};
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

#[derive(Parser)]
#[command(about = "Plans one expert-iteration generation: teacher games, dataset, students")]
struct Args {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Writes the `train_ops pipeline` plan of a generation's stages not yet done.
    Plan {
        #[arg(long)]
        spec: PathBuf,
        #[arg(long)]
        out: PathBuf,
    },
}

/// A generation. Every path is absolute.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Spec {
    /// The frozen runtime holding `eval4` and `convert4`.
    runtime: PathBuf,
    /// The trainer's directory (`train5.py`, `export_onnx4.py`); the
    /// pipeline runs there.
    trainer: PathBuf,
    /// The Python that runs the trainer.
    python: PathBuf,
    /// Where everything the generation makes goes.
    root: PathBuf,
    /// The execution directory's name under `root`.
    execution: String,
    gpu_lock: PathBuf,
    /// The deck manifest the teacher's games read.
    manifest: PathBuf,
    /// The teacher's export.
    teacher: PathBuf,
    /// Flags every collection shares: opponent, decks, policy, search.
    collect_flags: Vec<String>,
    collections: Vec<Collection>,
    /// `convert4`'s flags beside the runs, output and schedule seed.
    convert_flags: Vec<String>,
    schedule_seed: u64,
    student: Student,
    /// `export_onnx4.py`'s flags beside the model.
    export_flags: Vec<String>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Collection {
    /// Lowercase letters and digits: `records-NAME`, `collect-NAME.json`.
    name: String,
    seats: u32,
    /// `1,2,1,2`; empty for every seat on its own.
    #[serde(default)]
    teams: String,
    deals: u64,
    first_seed: u64,
    policy_seed: u64,
    timeout_seconds: u64,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Student {
    /// The checkpoint every seed starts from.
    init: PathBuf,
    /// One student per seed, `sN` for the N-th.
    seeds: Vec<u64>,
    /// `train5.py`'s flags beside the data, init, output and seed.
    flags: Vec<String>,
    timeout_seconds: u64,
}

const MEMORY_GIB: f64 = 48.0;
const CONVERT_SECONDS: u64 = 3600;
const EXPORT_SECONDS: u64 = 2400;

fn path(path: &Path) -> String {
    path.display().to_string()
}

fn validate(spec: &Spec) -> Result<()> {
    for path in [
        &spec.runtime,
        &spec.trainer,
        &spec.python,
        &spec.root,
        &spec.gpu_lock,
        &spec.manifest,
        &spec.teacher,
        &spec.student.init,
    ] {
        ensure!(path.is_absolute(), "not absolute: {}", path.display());
    }
    ensure!(!spec.collections.is_empty(), "a generation plays games");
    ensure!(
        !spec.student.seeds.is_empty(),
        "a generation trains a student"
    );
    let mut names = BTreeSet::new();
    for collection in &spec.collections {
        ensure!(
            !collection.name.is_empty()
                && collection
                    .name
                    .bytes()
                    .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit())
                && names.insert(&collection.name),
            "a collection's name is unique lowercase letters and digits: {:?}",
            collection.name
        );
        ensure!(
            (2..=8).contains(&collection.seats) && collection.deals > 0,
            "collection {}: 2 to 8 seats and some deals",
            collection.name
        );
    }
    for flag in &spec.collect_flags {
        ensure!(
            ![
                "--model",
                "--seats",
                "--teams",
                "--deals",
                "--first-seed",
                "--policy-seed",
                "--record-out",
                "--out",
            ]
            .contains(&flag.as_str()),
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

/// The teacher's games, one stage per collection.
fn collect_stages(spec: &Spec) -> Vec<Planned> {
    spec.collections
        .iter()
        .map(|collection| {
            let summary = spec.root.join(format!("collect-{}.json", collection.name));
            let mut command = vec![
                path(&spec.runtime.join("eval4")),
                "--model".into(),
                path(&spec.teacher),
                "--decks-manifest".into(),
                path(&spec.manifest),
                "--deals".into(),
                collection.deals.to_string(),
                "--seats".into(),
                collection.seats.to_string(),
            ];
            if !collection.teams.is_empty() {
                command.extend(["--teams".into(), collection.teams.clone()]);
            }
            command.extend(spec.collect_flags.iter().cloned());
            command.extend([
                "--first-seed".into(),
                collection.first_seed.to_string(),
                "--policy-seed".into(),
                collection.policy_seed.to_string(),
                "--record-out".into(),
                path(&records(spec, collection)),
                "--out".into(),
                path(&summary),
            ]);
            stage(
                &format!("collect-{}", collection.name),
                collection.timeout_seconds,
                Some(&spec.gpu_lock),
                &[spec.manifest.clone(), spec.teacher.join("model.onnx.json")],
                &command,
                vec![summary],
            )
        })
        .collect()
}

fn records(spec: &Spec, collection: &Collection) -> PathBuf {
    spec.root.join(format!("records-{}", collection.name))
}

fn dataset_meta(spec: &Spec) -> PathBuf {
    spec.root.join("dataset/meta.json")
}

/// The distillation dataset of every collection's games.
fn convert_stage(spec: &Spec) -> Planned {
    let runs: Vec<String> = spec
        .collections
        .iter()
        .map(|collection| path(&records(spec, collection)))
        .collect();
    let mut inputs: Vec<PathBuf> = spec
        .collections
        .iter()
        .map(|collection| spec.root.join(format!("collect-{}.json", collection.name)))
        .collect();
    inputs.push(spec.manifest.clone());
    // The collections' own manifest: a game that seated a deck outside its
    // train split stops the conversion.
    let mut command = vec![
        path(&spec.runtime.join("convert4")),
        "--runs".into(),
        runs.join(","),
        "--train-manifest".into(),
        path(&spec.manifest),
        "--out".into(),
        path(&spec.root.join("dataset")),
    ];
    command.extend(spec.convert_flags.iter().cloned());
    command.extend(["--schedule-seed".into(), spec.schedule_seed.to_string()]);
    stage(
        "convert",
        CONVERT_SECONDS,
        None,
        &inputs,
        &command,
        vec![dataset_meta(spec)],
    )
}

/// Each student's training, then each one's export.
fn student_stages(spec: &Spec) -> Vec<Planned> {
    let student = |at: usize| spec.root.join(format!("s{}", at + 1));
    let mut out = Vec::new();
    for (at, seed) in spec.student.seeds.iter().enumerate() {
        let mut command = vec![
            path(&spec.python),
            "-u".into(),
            path(&spec.trainer.join("train5.py")),
            "--data".into(),
            path(&spec.root.join("dataset")),
            "--init".into(),
            path(&spec.student.init),
            "--out".into(),
            path(&student(at)),
        ];
        command.extend(spec.student.flags.iter().cloned());
        command.extend(["--seed".into(), seed.to_string()]);
        out.push(stage(
            &format!("train-s{}", at + 1),
            spec.student.timeout_seconds,
            Some(&spec.gpu_lock),
            &[dataset_meta(spec), spec.student.init.join("net.pt")],
            &command,
            vec![student(at).join("net.pt"), student(at).join("meta.json")],
        ));
    }
    for at in 0..spec.student.seeds.len() {
        let mut command = vec![
            path(&spec.python),
            "-u".into(),
            path(&spec.trainer.join("export_onnx4.py")),
            "--model".into(),
            path(&student(at)),
        ];
        command.extend(spec.export_flags.iter().cloned());
        out.push(stage(
            &format!("export-s{}", at + 1),
            EXPORT_SECONDS,
            Some(&spec.gpu_lock),
            &[student(at).join("net.pt")],
            &command,
            vec![
                student(at).join("model.onnx.json"),
                student(at).join("fp16/model.onnx.json"),
            ],
        ));
    }
    out
}

/// Every stage of the generation, in order, with the outputs that mark it done.
fn stages(spec: &Spec) -> Vec<Planned> {
    let mut out = collect_stages(spec);
    out.push(convert_stage(spec));
    out.extend(student_stages(spec));
    out
}

fn plan(spec: &Spec, done: &dyn Fn(&Path) -> bool) -> Result<Value> {
    validate(spec)?;
    let stages: Vec<Value> = stages(spec)
        .into_iter()
        .filter(|(_, outputs)| !outputs.iter().all(|output| done(output)))
        .map(|(stage, _)| stage)
        .collect();
    ensure!(!stages.is_empty(), "every stage of the generation is done");
    let total: u64 = stages
        .iter()
        .filter_map(|stage| stage["timeout_seconds"].as_u64())
        .sum();
    Ok(json!({
        "version": 1,
        "root": spec.root.join(&spec.execution),
        "cwd": spec.trainer,
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
        .map_or_default(|stages| stages.iter().filter_map(|s| s["name"].as_str()).collect());
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
            "runtime": "/rt", "trainer": "/src/tools/trainer", "python": "/venv/python",
            "root": "/gen", "execution": "execution-g", "gpu_lock": "/lock",
            "manifest": "/decks.json", "teacher": "/teacher/fp16",
            "collect_flags": ["--against", "sharp", "--raw-policy", "--search", "4,8,6"],
            "collections": [
                {"name": "duel", "seats": 2, "deals": 512, "first_seed": 11, "policy_seed": 21, "timeout_seconds": 7200},
                {"name": "teams", "seats": 4, "teams": "1,2,1,2", "deals": 192, "first_seed": 12, "policy_seed": 22, "timeout_seconds": 7200}
            ],
            "convert_flags": ["--encoder-version", "10", "--search-targets"],
            "schedule_seed": 30,
            "student": {"init": "/r1", "seeds": [41, 42], "flags": ["--freeze-trunk"], "timeout_seconds": 3600},
            "export_flags": ["--precision", "both"]
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

    fn after<'a>(command: &'a [Value], flag: &str) -> &'a str {
        let at = command.iter().position(|f| f == flag).unwrap();
        command[at + 1].as_str().unwrap()
    }

    #[test]
    fn a_generation_collects_converts_trains_and_exports_in_order() {
        let spec = spec();
        let plan = plan(&spec, &|_| false).unwrap();
        assert_eq!(
            names(&plan),
            [
                "collect-duel",
                "collect-teams",
                "convert",
                "train-s1",
                "train-s2",
                "export-s1",
                "export-s2"
            ]
        );
        let stages = plan["stages"].as_array().unwrap();
        let teams = stages[1]["command"].as_array().unwrap();
        assert_eq!(after(teams, "--teams"), "1,2,1,2");
        assert_eq!(after(teams, "--record-out"), "/gen/records-teams");
        assert_eq!(after(teams, "--policy-seed"), "22");
        let convert = stages[2]["command"].as_array().unwrap();
        assert_eq!(
            after(convert, "--runs"),
            "/gen/records-duel,/gen/records-teams"
        );
        assert_eq!(after(convert, "--schedule-seed"), "30");
        // The collections' manifest guards the dataset's decks.
        let manifest = after(teams, "--decks-manifest");
        assert_eq!(after(convert, "--train-manifest"), manifest);
        assert!(
            stages[2]["inputs"]
                .as_array()
                .unwrap()
                .contains(&json!(manifest))
        );
        assert!(
            stages[2].get("gpu_lock").is_none(),
            "converting needs no GPU"
        );
        let train = stages[4]["command"].as_array().unwrap();
        assert_eq!(after(train, "--init"), "/r1");
        assert_eq!(after(train, "--out"), "/gen/s2");
        assert_eq!(after(train, "--seed"), "42");
        assert_eq!(stages[4]["gpu_lock"], "/lock");
        assert_eq!(plan["cwd"], "/src/tools/trainer");
        assert_eq!(plan["root"], "/gen/execution-g");
    }

    #[test]
    fn a_stage_whose_outputs_exist_is_left_out() {
        let done = |path: &Path| {
            path.to_string_lossy().starts_with("/gen/collect-") || path.starts_with("/gen/dataset")
        };
        let plan = plan(&spec(), &done).unwrap();
        assert_eq!(names(&plan)[0], "train-s1");
    }

    #[test]
    fn the_plan_sets_what_identifies_a_collection() {
        let mut spec = spec();
        spec.collect_flags.push("--record-out".into());
        assert!(plan(&spec, &|_| false).is_err());
        let mut spec = super::tests::spec();
        spec.collections[1].name = "duel".into();
        assert!(plan(&spec, &|_| false).is_err());
    }
}
