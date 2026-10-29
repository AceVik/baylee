use super::super::producer::directories;
use super::{
    Consumer,
    inputs::{command, inputs, validation},
};
use crate::{files, guard};
use anyhow::{Context, Result, ensure};
use serde_json::{Value, json};
use std::{
    fs,
    path::{Path, PathBuf},
    sync::atomic::{AtomicBool, Ordering},
    thread,
    time::{Duration, Instant},
};

pub(super) fn status(output: &Path, phase: &str, details: &Value) -> Result<()> {
    files::atomic_json(
        &output.join("status.json"),
        &json!({"phase":phase,"promoted":false,"updated_unix":files::unix_time()?,"details":details}),
    )
}
fn lock_gpu(plan: &Consumer, interrupted: &AtomicBool, started: Instant) -> Result<fs::File> {
    if let Some(parent) = plan.gpu_lock.parent() {
        fs::create_dir_all(parent)?;
    }
    let file = fs::File::options()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .open(&plan.gpu_lock)?;
    loop {
        ensure!(
            !interrupted.load(Ordering::Relaxed)
                && started.elapsed().as_secs_f64() < plan.timeout_seconds,
            "GPU ownership deadline/interruption"
        );
        match file.try_lock() {
            Ok(()) => return Ok(file),
            Err(std::fs::TryLockError::WouldBlock) => {
                status(
                    &plan.output,
                    "waiting-for-gpu",
                    &json!({"lock":plan.gpu_lock}),
                )?;
                thread::sleep(Duration::from_secs(1));
            }
            Err(error) => return Err(error.into()),
        }
    }
}
struct Completed {
    name: String,
    checkpoint: PathBuf,
    job: PathBuf,
}
fn train_package(
    plan: &Consumer,
    manifest: &Path,
    package: &Path,
    previous: &Path,
    interrupted: &AtomicBool,
    started: Instant,
) -> Result<Completed> {
    let _gpu = lock_gpu(plan, interrupted, started)?;
    let mut immutable = inputs(package, plan.allow_legacy_ready)?;
    immutable.extend([previous.to_owned(), manifest.to_owned()]);
    immutable.extend(validation(&plan.validation_data)?);
    let name = package
        .file_name()
        .context("package name")?
        .to_str()
        .context("UTF8 package name")?;
    let output = plan.output.join(name);
    fs::create_dir_all(&output)?;
    let argv = command(plan, package, &output, previous)?;
    status(
        &plan.output,
        "training-package",
        &json!({"package":package,"model":output,"command":argv}),
    )?;
    let job = guard::Job {
        directory: plan.output.join("jobs").join(name),
        command: argv,
        cwd: plan.cwd.clone(),
        timeout_seconds: plan
            .job_timeout_seconds
            .min(plan.timeout_seconds - started.elapsed().as_secs_f64()),
        memory_gib: plan.memory_gib,
        inputs: immutable,
    };
    ensure!(
        guard::run(&job, interrupted)?,
        "package training failed; inspect {}",
        job.directory.display()
    );
    let checkpoint = output.join("last.pt");
    ensure!(
        checkpoint.is_file(),
        "trainer did not publish required last.pt"
    );
    let _verified = inputs(package, plan.allow_legacy_ready)?;
    Ok(Completed {
        name: name.to_owned(),
        checkpoint,
        job: job.directory.join("job.json"),
    })
}
pub(super) fn run(
    plan: &Consumer,
    manifest: &Path,
    seal: &str,
    mut used: Vec<String>,
    mut previous: PathBuf,
    interrupted: &AtomicBool,
) -> Result<()> {
    let started = Instant::now();
    while used.len() < plan.max_packages {
        ensure!(
            !interrupted.load(Ordering::Relaxed)
                && started.elapsed().as_secs_f64() < plan.timeout_seconds,
            "consumer deadline/interruption"
        );
        let producer = files::read_json(&plan.producer_job)?;
        ensure!(
            matches!(
                producer["status"].as_str(),
                Some("starting" | "running" | "complete")
            ),
            "CPU producer failed; inspect preserved logs"
        );
        let ready = directories(&plan.packages, "package-")?
            .into_iter()
            .filter(|p| p.join("ready.json").is_file())
            .collect::<Vec<_>>();
        let pending = ready.iter().find(|p| {
            !used
                .iter()
                .any(|name| p.file_name().is_some_and(|n| n == name.as_str()))
        });
        let Some(package) = pending else {
            if plan.packages.join("complete.json").exists() {
                break;
            }
            status(
                &plan.output,
                "waiting-for-next-package",
                &json!({"completed_packages":used.len()}),
            )?;
            thread::sleep(Duration::from_secs(1));
            continue;
        };
        let completed = train_package(plan, manifest, package, &previous, interrupted, started)?;
        ensure!(
            files::sha256(manifest)? == seal,
            "consumer manifest changed during training"
        );
        used.push(completed.name);
        previous = completed.checkpoint;
        files::atomic_json(
            &plan.output.join("progress.json"),
            &json!({"version":1,"manifest_sha256":seal,"consumed":used,"last_checkpoint":previous,"last_checkpoint_sha256":files::sha256(&previous)?}),
        )?;
        super::receipts::consumed(
            package,
            seal,
            &json!({"job":completed.job,"checkpoint":previous,"manifest_sha256":seal}),
        )?;
    }
    status(
        &plan.output,
        "bounded-package-work-complete",
        &json!({"packages":used.len(),"last_checkpoint":previous,"checkpoint_sha256":files::sha256(&previous)?}),
    )
}
