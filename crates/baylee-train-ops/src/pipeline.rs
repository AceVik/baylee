//! Explicit, bounded experiment manifests; no implicit training or promotion recipe.
use crate::{files, guard, process};
use anyhow::{Context, Result, ensure};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    path::{Path, PathBuf},
    sync::atomic::{AtomicBool, Ordering},
    time::Instant,
};

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct Plan {
    version: u32,
    root: PathBuf,
    cwd: PathBuf,
    total_timeout_seconds: f64,
    stages: Vec<Stage>,
}
#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct Stage {
    name: String,
    command: Vec<String>,
    timeout_seconds: f64,
    memory_gib: f64,
    #[serde(default)]
    inputs: Vec<PathBuf>,
    #[serde(default)]
    gpu_lock: Option<PathBuf>,
    required_outputs: Vec<PathBuf>,
}

fn validate(plan: &Plan) -> Result<()> {
    ensure!(
        plan.version == 1 && plan.root.is_absolute() && plan.cwd.is_absolute() && plan.cwd.is_dir(),
        "invalid manifest version or absolute root/cwd"
    );
    ensure!(
        plan.total_timeout_seconds.is_finite()
            && plan.total_timeout_seconds > 0.0
            && !plan.stages.is_empty(),
        "bounded nonempty plan required"
    );
    let mut names = BTreeSet::new();
    let mut outputs = BTreeSet::new();
    for stage in &plan.stages {
        ensure!(
            !stage.name.is_empty()
                && stage
                    .name
                    .bytes()
                    .all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_')
                && names.insert(&stage.name),
            "unsafe or duplicate stage name"
        );
        ensure!(
            !stage.command.is_empty()
                && stage.timeout_seconds.is_finite()
                && stage.timeout_seconds > 0.0
                && stage.memory_gib.is_finite()
                && stage.memory_gib > 0.0
                && !stage.required_outputs.is_empty(),
            "stage needs command, finite bounds and required artifacts"
        );
        ensure!(
            stage
                .required_outputs
                .iter()
                .all(|p| p.is_absolute() && outputs.insert(p))
                && stage.gpu_lock.as_ref().is_none_or(|p| p.is_absolute()),
            "artifact paths must be unique and absolute; GPU lock paths must be absolute"
        );
    }
    Ok(())
}

fn output_seals(stage: &Stage) -> Result<BTreeMap<PathBuf, String>> {
    stage
        .required_outputs
        .iter()
        .map(|path| {
            ensure!(path.is_file(), "missing stage artifact {}", path.display());
            if path.extension().is_some_and(|e| e == "json") {
                files::read_json(path)?;
            }
            Ok((path.clone(), files::sha256(path)?))
        })
        .collect()
}

/// Execute exactly the ordered stages in an immutable manifest under one controller.
/// # Errors
/// Rejects concurrent controllers, failed attempt reuse, changed manifests,
/// missing artifacts, resource limits or interrupted stages. All evidence remains.
pub fn run(manifest: &Path, interrupted: &AtomicBool) -> Result<Value> {
    let manifest = manifest.canonicalize()?;
    let digest = files::sha256(&manifest)?;
    let plan: Plan = serde_json::from_value(files::read_json(&manifest)?)?;
    validate(&plan)?;
    fs::create_dir_all(&plan.root)?;
    let _owner = files::lock(&plan.root.join("controller.lock"))?;
    let status_path = plan.root.join("status.json");
    if status_path.exists() {
        let status = files::read_json(&status_path)?;
        ensure!(
            status["phase"] == "complete" && status["manifest_sha256"] == digest,
            "preserving existing controller attempt; use a fresh root"
        );
        let result = files::read_json(&plan.root.join("result.json"))?;
        for stage in &plan.stages {
            ensure!(
                result["outputs"][&stage.name] == json!(output_seals(stage)?),
                "completed artifact changed"
            );
        }
        return Ok(result);
    }
    let identity = process::identity(std::process::id()).context("controller identity missing")?;
    files::atomic_json(&plan.root.join("controller.json"), &identity)?;
    let started = Instant::now();
    let mut status = json!({"phase":"starting","pid":identity.pid,"controller":identity,"manifest":manifest,"manifest_sha256":digest,"promoted":false,"automatic_restart":false,"updated_unix":files::unix_time()?});
    files::atomic_json(&status_path, &status)?;
    let outcome = (|| -> Result<Value> {
        let mut outputs = BTreeMap::new();
        for stage in &plan.stages {
            ensure!(!interrupted.load(Ordering::Relaxed), "pipeline interrupted");
            let remaining = plan.total_timeout_seconds - started.elapsed().as_secs_f64();
            ensure!(remaining > 0.0, "experiment total deadline exhausted");
            ensure!(
                files::sha256(&manifest)? == digest,
                "manifest changed during experiment"
            );
            status["phase"] = json!(stage.name);
            status["updated_unix"] = json!(files::unix_time()?);
            files::atomic_json(&status_path, &status)?;
            let _gpu = stage
                .gpu_lock
                .as_ref()
                .map(|path| files::lock(path))
                .transpose()?;
            let mut inputs = stage.inputs.clone();
            inputs.push(manifest.clone());
            let job = guard::Job {
                directory: plan.root.join("jobs").join(&stage.name),
                cwd: plan.cwd.clone(),
                command: stage.command.clone(),
                timeout_seconds: stage.timeout_seconds.min(remaining),
                memory_gib: stage.memory_gib,
                inputs,
            };
            ensure!(
                guard::run(&job, interrupted)?,
                "stage {} failed; artifacts retained",
                stage.name
            );
            outputs.insert(stage.name.clone(), output_seals(stage)?);
        }
        ensure!(
            files::sha256(&manifest)? == digest,
            "manifest changed during experiment"
        );
        for stage in &plan.stages {
            ensure!(
                outputs.get(&stage.name) == Some(&output_seals(stage)?),
                "artifact from stage {} changed before completion",
                stage.name
            );
        }
        Ok(
            json!({"version":1,"phase":"complete","root":plan.root,"manifest_sha256":digest,"outputs":outputs,"elapsed_seconds":started.elapsed().as_secs_f64(),"promoted":false}),
        )
    })();
    match outcome {
        Ok(result) => {
            files::atomic_json(&plan.root.join("result.json"), &result)?;
            status["phase"] = json!("complete");
            status["result"] = json!(plan.root.join("result.json"));
            status["updated_unix"] = json!(files::unix_time()?);
            files::atomic_json(&status_path, &status)?;
            Ok(result)
        }
        Err(error) => {
            status["phase"] = json!(if interrupted.load(Ordering::Relaxed) {
                "interrupted"
            } else {
                "failed"
            });
            status["error"] = json!(format!("{error:#}"));
            status["updated_unix"] = json!(files::unix_time()?);
            files::atomic_json(&status_path, &status)?;
            Err(error)
        }
    }
}
