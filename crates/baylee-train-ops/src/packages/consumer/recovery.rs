//! Repair receipt publication after the consumer's durable progress commit.
use super::{Consumer, inputs, receipts};
use crate::{files, process};
use anyhow::{Context, Result, ensure};
use serde_json::json;
use std::{
    collections::{BTreeMap, BTreeSet},
    path::{Component, Path, PathBuf},
};

fn contained(root: &Path, path: &Path) -> Result<()> {
    ensure!(
        path.canonicalize()?.starts_with(root.canonicalize()?),
        "completed package path escapes its root"
    );
    Ok(())
}

fn completed_job(plan: &Consumer, job: &serde_json::Value, command: &[String]) -> Result<()> {
    ensure!(
        job["schema_version"] == 3
            && job["guard"] == "rust-owned-group-v1"
            && job["status"] == "complete"
            && job["cleanup_verified"] == true
            && job["returncode"] == 0
            && job["reason"].is_null(),
        "committed package job did not complete cleanly"
    );
    let child: process::Identity = serde_json::from_value(job["child_identity"].clone())
        .context("completed package child identity")?;
    ensure!(
        job["pid"] == child.pid && process::identity(child.pid).as_ref() != Some(&child),
        "completed package child is still live or identity differs"
    );
    ensure!(
        job["command"] == json!(command)
            && job["cwd"] == json!(plan.cwd)
            && job["memory_limit_gib"] == plan.memory_gib
            && job["timeout_seconds"]
                .as_f64()
                .is_some_and(|v| { v.is_finite() && v > 0.0 && v <= plan.job_timeout_seconds }),
        "completed package job differs from consumer manifest"
    );
    let invocation = PathBuf::from(
        job["executable_invocation"]
            .as_str()
            .context("job executable invocation")?,
    );
    let executable = PathBuf::from(job["executable"].as_str().context("job executable")?);
    ensure!(
        invocation.is_absolute()
            && invocation.canonicalize()? == executable
            && job["executable_sha256"] == files::sha256(&executable)?,
        "completed package executable changed"
    );
    Ok(())
}

/// Validate the entire committed checkpoint chain before publishing any repair.
pub(super) fn reconcile(
    plan: &Consumer,
    manifest: &Path,
    seal: &str,
    used: &[String],
    last: &Path,
) -> Result<()> {
    ensure!(
        used.len() <= plan.max_packages,
        "too many committed packages"
    );
    let mut previous = plan.initial_checkpoint.clone();
    let mut names = BTreeSet::new();
    let mut repairs = Vec::new();
    // Retain every guard lock until validation and receipt publication finish.
    let mut locks = Vec::new();
    for name in used {
        ensure!(
            Path::new(name).components().count() == 1
                && matches!(
                    Path::new(name).components().next(),
                    Some(Component::Normal(_))
                )
                && name.starts_with("package-")
                && names.insert(name),
            "invalid or duplicate committed package"
        );
        let package = plan.packages.join(name);
        let output = plan.output.join(name);
        let job_dir = plan.output.join("jobs").join(name);
        let job_path = job_dir.join("job.json");
        let checkpoint = output.join("last.pt");
        contained(&plan.packages, &package)?;
        contained(&plan.output, &output)?;
        contained(&plan.output, &job_dir)?;
        contained(&job_dir, &job_path)?;
        contained(&output, &checkpoint)?;
        let guard_lock = job_dir.join("guard.lock");
        if guard_lock.exists() {
            contained(&job_dir, &guard_lock)?;
        }
        locks.push(files::lock(&guard_lock)?);
        let job = files::read_json(&job_path)?;
        completed_job(
            plan,
            &job,
            &inputs::command(plan, &package, &output, &previous)?,
        )?;
        let mut paths = inputs::inputs(&package, plan.allow_legacy_ready)?;
        paths.extend([previous, manifest.to_owned()]);
        paths.extend(inputs::validation(&plan.validation_data)?);
        let expected = paths
            .into_iter()
            .map(|path| {
                Ok((
                    path.to_string_lossy().into_owned(),
                    files::sha256(&plan.cwd.join(&path))?,
                ))
            })
            .collect::<Result<BTreeMap<_, _>>>()?;
        ensure!(
            job["inputs"] == json!(expected),
            "completed package inputs changed"
        );
        let receipt = json!({"job":job_path,"checkpoint":checkpoint,"manifest_sha256":seal});
        let own = package.join("consumption").join(format!("{seal}.json"));
        if let Some(parent) = own.parent().filter(|parent| parent.exists()) {
            contained(&package, parent)?;
        }
        if own.exists() {
            contained(&package, &own)?;
            ensure!(
                files::read_json(&own)? == receipt,
                "completed package consumption receipt differs"
            );
        }
        repairs.push((package, receipt));
        previous = checkpoint;
    }
    ensure!(
        previous == last && files::sha256(manifest)? == seal,
        "committed package chain or manifest changed"
    );
    for (package, receipt) in repairs {
        receipts::consumed(&package, seal, &receipt)?;
    }
    Ok(())
}
