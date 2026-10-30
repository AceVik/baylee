//! Dedicated research input gate. Numerical replay remains mandatory before updates.
use super::*;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::{
    collections::{BTreeMap, BTreeSet},
    fs::{File, OpenOptions},
    io::Write,
    path::PathBuf,
};

/// Immutable caller-frozen research inputs; never inferred from the cache itself.
#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Pins {
    schema: String,
    cache_dir: PathBuf,
    conversion_manifest_sha256: String,
    cache_manifest_sha256: String,
    source_checkpoint: FilePin,
    learner_model_identity: Value,
    catalog: Value,
    producers: Vec<Producer>,
}
#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct FilePin {
    path: PathBuf,
    sha256: String,
}
#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct Producer {
    source_run_sha256: String,
    guard_job: PathBuf,
    guard_job_sha256: String,
    executable: PathBuf,
    executable_sha256: String,
    collection_contract: Value,
}

fn export_inputs(guard: &Value, identity: &Value, structure: &Value) -> Result<()> {
    let mut inputs = BTreeMap::new();
    let cwd = Path::new(guard["cwd"].as_str().context("guard cwd")?);
    for (name, digest) in guard["inputs"]
        .as_object()
        .context("producer guard inputs")?
    {
        let path = cwd.join(name);
        let path = path.canonicalize()?;
        let digest = digest.as_str().context("producer input SHA")?;
        pinned(&path, digest)?;
        inputs.insert(path, digest.to_owned());
    }
    for key in ["onnx_sha256", "metadata_sha256"] {
        let digest = identity[key]
            .as_str()
            .context("missing model content identity")?;
        ensure!(
            inputs.values().any(|sha| sha == digest),
            "producer model {key} absent from original guard input seals"
        );
    }
    for digest in identity["depth_graph_sha256"]
        .as_object()
        .context("model depth graph identities")?
        .values()
    {
        ensure!(
            inputs.values().any(|sha| digest == sha),
            "producer depth graph missing from original guard seals"
        );
    }
    let metadata_path = inputs
        .iter()
        .find(|(_, sha)| identity["metadata_sha256"] == **sha)
        .map(|(p, _)| p)
        .context("model metadata evidence")?;
    let meta = crate::files::read_json(metadata_path)?;
    ensure!(
        meta["source_checkpoint_sha256"] == identity["source_checkpoint_sha256"]
            && meta["encoder_version"] == identity["encoder_version"]
            && meta["model_version"] == identity["model_version"]
            && meta["structure_sha256"] == *structure
            && meta["sampling_version"] == 2
            && meta["thinking_steps"] == 2
            && identity["depth_graph_sha256"].get("0").is_some()
            && identity["depth_graph_sha256"].get("2").is_some(),
        "producer export contract/catalog mismatch"
    );
    Ok(())
}

fn compute(path: &Path, expected: &str) -> Result<(Session, Value)> {
    pinned(path, expected)?;
    let pins: Pins = serde_json::from_value(crate::files::read_json(path)?)?;
    ensure!(
        pins.schema == "failure_terminal_research_input_pins_v1",
        "unsupported research pins schema"
    );
    ensure!(
        pins.cache_dir.is_absolute() && pins.cache_dir.canonicalize()? == pins.cache_dir,
        "cache pin must be canonical"
    );
    pinned(&pins.source_checkpoint.path, &pins.source_checkpoint.sha256)?;
    let session = Session::open(
        &pins.cache_dir,
        &pins.conversion_manifest_sha256,
        &pins.cache_manifest_sha256,
    )?;
    let m = session.metadata();
    ensure!(
        m["learner_model_identity"] == pins.learner_model_identity
            && m["catalog"] == pins.catalog
            && m["source_checkpoint_sha256"] == pins.source_checkpoint.sha256,
        "source checkpoint/model/catalog pins differ"
    );
    let cache = crate::files::read_json(&pins.cache_dir.join("research-cache.manifest.json"))?;
    let sources = cache["source_conversion_manifest"]["source_runs"]
        .as_array()
        .context("source producer runs")?;
    ensure!(
        sources.len() == pins.producers.len() && !sources.is_empty(),
        "incomplete producer certification pins"
    );
    let mut seen = BTreeSet::new();
    for producer in &pins.producers {
        ensure!(
            seen.insert(&producer.source_run_sha256),
            "duplicate producer pins"
        );
        let source = sources
            .iter()
            .find(|s| s["source_run_sha256"] == producer.source_run_sha256)
            .context("unexpected producer pins")?;
        let ledger = &source["finalized_attempt_ledger"];
        ensure!(
            ledger["guard_job"] == json!(producer.guard_job)
                && ledger["guard_job_sha256"] == producer.guard_job_sha256
                && source["collection_contract"] == producer.collection_contract,
            "producer source/guard/role pins differ"
        );
        pinned(&producer.guard_job, &producer.guard_job_sha256)?;
        pinned(&producer.executable, &producer.executable_sha256)?;
        let guard = crate::files::read_json(&producer.guard_job)?;
        ensure!(
            guard["schema_version"] == 3
                && guard["guard"] == "rust-owned-group-v1"
                && guard["cleanup_verified"] == true
                && guard["executable"] == json!(producer.executable)
                && guard["executable_sha256"] == producer.executable_sha256
                && guard["command"].as_array().is_some_and(|c| !c.is_empty())
                && guard["command"][0] == guard["executable_invocation"],
            "original executed producer executable is not certified"
        );
        let invocation = Path::new(
            guard["executable_invocation"]
                .as_str()
                .context("guard executable invocation")?,
        );
        ensure!(
            invocation.is_absolute() && invocation.canonicalize()? == producer.executable,
            "producer invocation no longer resolves to certified executable"
        );
        for role in ["learner_model_identity", "opponent_model_identity"] {
            export_inputs(
                &guard,
                &producer.collection_contract[role],
                &pins.catalog["structure_sha256"],
            )?;
        }
    }
    pinned(path, expected)?;
    pinned(&pins.source_checkpoint.path, &pins.source_checkpoint.sha256)?;
    let receipt = json!({"schema":"failure_terminal_research_input_admission_v1","research_input_admitted":true,
        "ordinary_ppo_admitted":false,"numerical_replay_required":true,"objective":OBJECTIVE,"tensor_schema":SCHEMA,
        "pins":path,"pins_sha256":expected,"cache_dir":pins.cache_dir,"cache_manifest_sha256":pins.cache_manifest_sha256,
        "conversion_manifest_sha256":pins.conversion_manifest_sha256,"content_fingerprint":m["content_fingerprint"],
        "source_checkpoint_sha256":pins.source_checkpoint.sha256,"learner_model_identity":pins.learner_model_identity,
        "catalog":pins.catalog,"producer_binary_certified":true,"producers":pins.producers,
        "attempt_counts":m["attempt_counts"],"frames":m["frames"],"genuine_choices":m["genuine_choices"],
        "actor_candidates":m["actor_candidates"],"learner_histories":m["histories"].as_array().context("histories")?.len(),
        "histories":m["histories"]});
    Ok((session, receipt))
}

/// Publish a dedicated native input receipt into a new file. Never grants numerical admission.
/// # Errors
/// Refuses stale/missing producer/export/checkpoint evidence and existing outputs.
pub fn admit(pins: &Path, expected: &str, out: &Path) -> Result<Value> {
    let (_, receipt) = compute(pins, expected)?;
    let mut file = OpenOptions::new().write(true).create_new(true).open(out)?;
    serde_json::to_writer_pretty(&mut file, &receipt)?;
    file.write_all(b"\n")?;
    file.sync_all()?;
    File::open(out.parent().context("admission parent")?)?.sync_all()?;
    Ok(receipt)
}
/// Recompute every source pin and compare the exact receipt before opening a training session.
/// # Errors
/// Rejects missing, changed or cross-contract receipts. Does not rewrite evidence.
pub fn open(pins: &Path, expected: &str, admission: &Path) -> Result<Session> {
    let (mut session, receipt) = compute(pins, expected)?;
    ensure!(
        crate::files::read_json(admission)? == receipt,
        "research admission no longer matches native verified inputs"
    );
    session.admitted(receipt);
    Ok(session)
}
/// Verify dedicated input admission without producing arrays or modifying artifacts.
/// # Errors
/// Rejects any source or receipt mismatch.
pub fn verify(pins: &Path, expected: &str, admission: &Path) -> Result<Value> {
    Ok(open(pins, expected, admission)?.metadata().clone())
}
