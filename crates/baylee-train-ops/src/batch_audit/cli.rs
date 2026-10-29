//! A bounded native owner for the Tensor-only measurement worker.
use crate::{files, guard};
use anyhow::{Context, Result, ensure};
use serde_json::{Value, json};
use std::{collections::BTreeMap, path::Path, time::Duration};
fn option<'a>(args: &'a [String], name: &str) -> Result<&'a str> {
    let positions: Vec<_> = args
        .iter()
        .enumerate()
        .filter(|(_, s)| *s == name)
        .collect();
    ensure!(
        positions.len() == 1,
        "job argument missing or duplicated: {name}"
    );
    args.get(positions[0].0 + 1)
        .map(String::as_str)
        .context("missing job argument value")
}
/// Parse the job and invoke only CPU Tensor shape measurements under a hard deadline.
/// # Errors
/// Refuses reused outputs, invalid budgets, changed sources or a failed worker.
pub fn run(
    job: &Path,
    out: &Path,
    python: &Path,
    samples: u64,
    budget_mib: u64,
    timeout_seconds: f64,
) -> Result<Value> {
    ensure!(
        !out.exists()
            && (1..=64).contains(&samples)
            && budget_mib > 0
            && timeout_seconds.is_finite()
            && timeout_seconds > 0.0,
        "invalid bounded batch audit or reused output"
    );
    let _lock = files::lock(&out.with_extension("audit.lock"))?;
    let job = job.canonicalize()?;
    let saved = files::read_json(&job)?;
    let args: Vec<String> = serde_json::from_value(saved["command"].clone())?;
    ensure!(
        files::read_json(&job)? == saved,
        "audit job changed while reading"
    );
    let runtime = Path::new(
        args.get(2)
            .context("job needs reviewed Python -u script argv")?,
    )
    .parent()
    .context("job runtime directory")?
    .canonicalize()?;
    let worker = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../tools/trainer/batch_audit5/worker.py")
        .canonicalize()?;
    let executable = std::env::current_exe()?.canonicalize()?;
    let mut seals = BTreeMap::new();
    for path in [&job, &worker, &executable] {
        seals.insert(path.display().to_string(), files::sha256(path)?);
    }
    let mut frozen = BTreeMap::new();
    for name in ["dataset4.py", "sampler4.py"] {
        let path = runtime.join(name);
        let digest = files::sha256(&path)?;
        seals.insert(path.display().to_string(), digest.clone());
        frozen.insert(name, digest);
    }
    let out = std::path::absolute(out)?;
    let diagnostic = tempfile::NamedTempFile::new()?;
    let plan = json!({"native_binary":executable,"diagnostic":diagnostic.path(),"worker_sha256":seals[&worker.display().to_string()],"tool_sha256":seals[&executable.display().to_string()],"job":job,"job_sha256":seals[&job.display().to_string()],"runtime":runtime,"input_seals":seals,"frozen_code_sha256":frozen,"data":option(&args,"--data")?,"unroll":option(&args,"--unroll")?.parse::<u64>()?,"batch_size":option(&args,"--batch-size")?.parse::<u64>()?,"seed":option(&args,"--seed")?.parse::<u64>()?,"samples":samples,"budget_mib":budget_mib,"out":out});
    ensure!(
        plan["unroll"].as_u64().is_some_and(|n| n > 0)
            && plan["batch_size"].as_u64().is_some_and(|n| n > 0),
        "positive tensor batch/unroll required"
    );
    let temporary = tempfile::NamedTempFile::new()?;
    files::atomic_json(temporary.path(), &plan)?;
    let result = guard::capture(
        python,
        &[
            worker.to_str().context("worker path UTF8")?,
            temporary.path().to_str().context("plan path UTF8")?,
        ],
        Duration::from_secs_f64(timeout_seconds),
    );
    if let Err(error) = result {
        if let Ok(detail) = files::read_json(diagnostic.path()) {
            anyhow::bail!(
                "Tensor measurement failed: {}: {}",
                detail["error_type"],
                detail["detail"]
            );
        }
        return Err(error);
    }
    let report = files::read_json(&out)?;
    ensure!(
        report["schema_version"] == 2,
        "worker did not finalize native batch audit report"
    );
    Ok(report)
}
