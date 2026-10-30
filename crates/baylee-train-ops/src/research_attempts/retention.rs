//! Opt-in accounting for an independent collection stopped by its own deadline.
use super::*;
use crate::guard;
use std::{
    collections::BTreeMap,
    sync::atomic::{AtomicBool, Ordering},
};

#[cfg(test)]
mod tests;

fn guard_inputs(job: &guard::Job, guard: &Value) -> Result<()> {
    let mut inputs = BTreeMap::new();
    for path in &job.inputs {
        inputs.insert(
            path.to_string_lossy().into_owned(),
            files::sha256(&job.cwd.join(path))?,
        );
    }
    let invocation = Path::new(
        guard["executable_invocation"]
            .as_str()
            .context("guard invocation")?,
    );
    let executable = invocation.canonicalize()?;
    ensure!(
        job.command.first().is_some_and(
            |command| Path::new(command).is_absolute() && Path::new(command) == invocation
        ) && guard["schema_version"] == 3
            && guard["command"] == json!(job.command)
            && guard["cwd"] == json!(job.cwd)
            && guard["timeout_seconds"] == json!(job.timeout_seconds)
            && guard["memory_limit_gib"] == json!(job.memory_gib)
            && guard["inputs"] == json!(inputs)
            && guard["executable"] == json!(executable)
            && guard["executable_sha256"] == files::sha256(&executable)?,
        "retained collection guard inputs or executable changed"
    );
    Ok(())
}

pub(super) fn deadline(job: &guard::Job, guard: &Value, ledger: &Value) -> Result<()> {
    ensure!(
        guard["status"] == "quarantined"
            && guard["guard"] == "rust-owned-group-v1"
            && guard["reason"] == "deadline"
            && guard["cleanup_verified"] == true
            && guard.get("returncode").is_some_and(Value::is_null),
        "only a verified cleaned-up collector deadline can be retained"
    );
    let peak = guard["peak_pss_bytes"]
        .as_u64()
        .context("guard peak memory")?;
    #[allow(
        clippy::cast_precision_loss,
        reason = "guard memory bound, not engine arithmetic"
    )]
    let within_memory = (peak as f64) <= job.memory_gib * 1_073_741_824.;
    ensure!(
        within_memory,
        "memory-limit collection cannot be retained as a deadline"
    );
    ensure!(
        ledger["attempts"]
            .as_array()
            .context("finalized attempts")?
            .iter()
            .all(|row| {
                row["status"] != "operational_unknown"
                    || row["unknown_reason"] == "process_deadline"
            }),
        "worker failures cannot be retained as a deadline"
    );
    guard_inputs(job, guard)
}

pub(super) fn account(
    job: &guard::Job,
    root: &Path,
    interrupted: &AtomicBool,
    complete: bool,
) -> Result<Value> {
    ensure!(
        !interrupted.load(Ordering::Relaxed),
        "interrupted research collection cannot continue"
    );
    let ledger = verify_finalized(root)?;
    let guard_path = Path::new(ledger["guard_job"].as_str().context("finalized guard")?);
    let guard = files::read_json(guard_path)?;
    if complete {
        ensure!(
            guard["status"] == "complete"
                && guard["reason"].is_null()
                && guard["returncode"] == 0
                && ledger["counts"]["recorded"] == ledger["counts"]["planned"],
            "successful collection accounting requires every planned attempt recorded"
        );
        guard_inputs(job, &guard)?;
    } else {
        deadline(job, &guard, &ledger)?;
    }
    // Includes ownership/cleanup and every ledger source seal; do not trust only
    // the reason string or a finalization result from before input revalidation.
    ensure!(
        verify_finalized(root)? == ledger,
        "collection changed during accounting"
    );
    let producer: process::Identity = serde_json::from_value(ledger["producer_identity"].clone())?;
    ensure!(
        process::group_members(producer.pid)?.is_empty(),
        "collector group still has live members"
    );
    ensure!(
        !interrupted.load(Ordering::Relaxed),
        "interrupted research collection cannot continue"
    );
    let receipt = json!({"schema":"failure_terminal_collection_accounting_v1",
        "status":if complete {"collection_complete"} else {"deadline_accounted"},
        "accounting_complete":true,"collection_complete":complete,"deadline_retained":!complete,
        "guard_status":guard["status"],"guard_job":guard_path,"guard_job_sha256":ledger["guard_job_sha256"],
        "ledger":root.canonicalize()?.join("attempts.final.json"),
        "ledger_sha256":files::sha256(&root.join("attempts.final.json"))?,
        "counts":ledger["counts"],"training_admitted":false,"ppo_certified":false,"automatic_restart":false});
    publish_new(&root.join("collection-accounting.json"), &receipt)?;
    Ok(receipt)
}
