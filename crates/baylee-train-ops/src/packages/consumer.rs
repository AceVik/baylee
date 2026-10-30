mod inputs;
mod receipts;
mod recovery;
mod runtime;
use crate::files;
use anyhow::{Context, Result, ensure};
use serde::Deserialize;
use serde_json::json;
use std::{
    fs,
    path::{Path, PathBuf},
    sync::atomic::AtomicBool,
};
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
/// Explicit bounded package-training recipe; no historical curriculum is inferred.
pub struct Consumer {
    /// Manifest schema version, currently one.
    pub version: u32,
    /// Directory of immutable package folders.
    pub packages: PathBuf,
    /// CPU producer status receipt.
    pub producer_job: PathBuf,
    /// Independent fixed validation dataset.
    pub validation_data: PathBuf,
    /// New experiment directory; completed progress is content-bound.
    pub output: PathBuf,
    /// Source checkout for the literal training command.
    pub cwd: PathBuf,
    /// Explicit initial checkpoint, never selected by an implicit score.
    pub initial_checkpoint: PathBuf,
    /// Literal argv with package, output, checkpoint and validation placeholders.
    pub train_command: Vec<String>,
    /// Total bounded consumer wall time.
    pub timeout_seconds: f64,
    /// Maximum wall time for one package job.
    pub job_timeout_seconds: f64,
    /// Owned process group memory limit.
    pub memory_gib: f64,
    /// Maximum number of completed packages in this manifest.
    pub max_packages: usize,
    /// Must be the shared experiment gpu-work.lock, never an attempt-local lock.
    pub gpu_lock: PathBuf,
    /// Explicitly admit archived markers after complete-history and input checks.
    #[serde(default)]
    pub allow_legacy_ready: bool,
}

/// Consume only a newly declared manifest with bounded guarded jobs and shared GPU ownership.
/// # Errors
/// Rejects old implicit recipes, altered progress/packages and failed/reused attempts.
pub fn consume(path: &Path, interrupted: &AtomicBool) -> Result<()> {
    let manifest = path.canonicalize()?;
    let path = manifest.as_path();
    let plan: Consumer = serde_json::from_reader(fs::File::open(path)?)?;
    ensure!(
        plan.version == 1 && plan.max_packages > 0 && !plan.train_command.is_empty(),
        "explicit version1 bounded training manifest required"
    );
    for number in [
        plan.timeout_seconds,
        plan.job_timeout_seconds,
        plan.memory_gib,
    ] {
        ensure!(
            number.is_finite() && number > 0.0,
            "finite positive consumer resource bounds required"
        );
    }
    for directory in [
        &plan.packages,
        &plan.validation_data,
        &plan.output,
        &plan.cwd,
        &plan.initial_checkpoint,
        &plan.gpu_lock,
    ] {
        ensure!(
            directory.is_absolute(),
            "consumer manifest paths must be absolute"
        );
    }
    ensure!(
        plan.gpu_lock
            .file_name()
            .is_some_and(|v| v == "gpu-work.lock")
            && !plan.gpu_lock.starts_with(&plan.output),
        "use shared experiment gpu-work.lock"
    );
    fs::create_dir_all(&plan.output)?;
    let _owner = files::lock(&plan.output.join("consumer.lock"))?;
    let seal = files::sha256(path)?;
    let progress_path = plan.output.join("progress.json");
    let mut used = Vec::<String>::new();
    let mut previous = plan.initial_checkpoint.clone();
    if progress_path.exists() {
        let progress = files::read_json(&progress_path)?;
        ensure!(
            progress["manifest_sha256"] == seal,
            "preserving legacy or different consumer progress; use a new explicit experiment"
        );
        used = serde_json::from_value(progress["consumed"].clone())?;
        previous = PathBuf::from(
            progress["last_checkpoint"]
                .as_str()
                .context("previous checkpoint")?,
        );
        ensure!(
            progress["last_checkpoint_sha256"] == files::sha256(&previous)?,
            "completed checkpoint changed"
        );
        recovery::reconcile(&plan, path, &seal, &used, &previous)?;
    }
    let result = runtime::run(&plan, path, &seal, used, previous, interrupted);
    if let Err(ref error) = result {
        runtime::status(
            &plan.output,
            "package-training-failed",
            &json!({"error":format!("{error:#}")}),
        )?;
    }
    result
}
