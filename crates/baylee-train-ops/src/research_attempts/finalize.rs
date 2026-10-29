use super::*;
use crate::guard;
use sha2::Digest as _;
use std::{
    collections::BTreeMap,
    io::{BufRead as _, BufReader},
    sync::atomic::AtomicBool,
};

fn indexed_rows(root: &Path) -> Result<BTreeMap<u64, String>> {
    let mut reader = BufReader::new(File::open(root.join("games.jsonl"))?);
    let mut line = Vec::new();
    let mut rows = BTreeMap::new();
    while reader.read_until(b'\n', &mut line)? > 0 {
        // A killed append may leave a trailing fragment. It is evidence, not a
        // completed row; the raw source file remains sealed and unchanged.
        if !line.ends_with(b"\n") {
            break;
        }
        let row: Value = serde_json::from_slice(&line)?;
        let deal = row["i"].as_u64().context("indexed deal")?;
        ensure!(
            rows.insert(
                deal,
                files::hex(&sha2::Sha256::digest(serde_json::to_vec(&row)?))
            )
            .is_none(),
            "duplicate indexed research attempt"
        );
        line.clear();
    }
    Ok(rows)
}

fn seal(path: &Path, seals: &mut BTreeMap<PathBuf, String>) -> Result<String> {
    let digest = files::sha256(path)?;
    seals.insert(path.to_owned(), digest.clone());
    Ok(digest)
}

pub(super) fn verified_guard(
    plan: &Plan,
    job: &Value,
    current: Option<&process::Identity>,
) -> Result<()> {
    ensure!(
        matches!(job["status"].as_str(), Some("complete" | "quarantined"))
            && job["guard"] == "rust-owned-group-v1"
            && job["cleanup_verified"] == true,
        "guard has not verified complete owned-group cleanup"
    );
    ensure!(
        job["child_identity"] == json!(plan.producer_identity),
        "guard does not own the recorded producer identity"
    );
    ensure!(
        current != Some(&plan.producer_identity),
        "collector identity is still live"
    );
    Ok(())
}

/// Reconcile only a dead, guard-owned collector. Never signals or restarts processes.
/// # Errors
/// Rejects live/unverified ownership, changed source receipts and corrupt completions.
#[allow(
    clippy::too_many_lines,
    reason = "Ordered ownership, denominator and source seals"
)]
pub fn finalize(root: &Path, job: &Path) -> Result<Value> {
    let root = root.canonicalize()?;
    let job = job.canonicalize()?;
    let _finalizer = files::lock(&root.join("attempts.finalize.lock"))?;
    let _producer = files::lock(&root.join("attempts.live.lock"))?;
    let mut seals = BTreeMap::new();
    let plan_path = root.join("attempts.plan.json");
    let plan_sha = seal(&plan_path, &mut seals)?;
    let plan: Plan = serde_json::from_value(files::read_json(&plan_path)?)?;
    ensure!(plan.schema == PLAN_SCHEMA, "unknown attempt plan contract");
    let job_sha = seal(&job, &mut seals)?;
    let guard = files::read_json(&job)?;
    verified_guard(
        &plan,
        &guard,
        process::identity(plan.producer_identity.pid).as_ref(),
    )?;
    let source_run_sha = seal(&root.join("run.json"), &mut seals)?;
    ensure!(
        plan.source_run_sha256 == source_run_sha,
        "source run changed after planning"
    );
    let run = files::read_json(&root.join("run.json"))?;
    ensure!(
        run["collection_contract"]["purpose"] == "failure_terminal_research_v1"
            && run["collection_contract"]["version"] == 2
            && run["training_data"] == false,
        "ordinary sources cannot acquire research attempt accounting"
    );
    seal(&root.join("games.jsonl"), &mut seals)?;
    let index = indexed_rows(&root)?;
    if root.join("records").exists() {
        for entry in fs::read_dir(root.join("records"))? {
            let entry = entry?;
            ensure!(
                entry.file_type()?.is_file(),
                "record evidence must be regular files"
            );
            // Include partial/orphan shards too: absence of a completed receipt
            // does not permit discarding the bytes left by an interrupted writer.
            seal(&entry.path(), &mut seals)?;
        }
    }
    let first = run["deal_offset"].as_u64().context("first deal")?;
    let count = run["planned_deals"].as_u64().context("planned count")?;
    let end = first.checked_add(count).context("planned range overflow")?;
    let mut ids = BTreeSet::new();
    ensure!(
        count > 0
            && u64::try_from(plan.attempts.len())? == count
            && plan.attempts.iter().all(|attempt| attempt.deal >= first
                && attempt.deal < end
                && ids.insert(attempt.deal)),
        "changed or incomplete planned denominator"
    );
    for deal in index.keys() {
        ensure!(
            ids.contains(deal) && attempt_path(&root, *deal).join("claim.json").is_file(),
            "indexed game is outside the planned/assigned denominator"
        );
    }
    if root.join("attempts").exists() {
        for entry in fs::read_dir(root.join("attempts"))? {
            let entry = entry?;
            let name = entry.file_name();
            let valid = name.to_str().is_some_and(|name| {
                name.parse::<u64>()
                    .is_ok_and(|deal| name == format!("{deal:020}") && ids.contains(&deal))
            });
            ensure!(
                entry.file_type()?.is_dir() && valid,
                "unplanned assignment evidence outside fixed denominator"
            );
        }
    }
    let mut rows = Vec::new();
    let mut assigned = 0_u64;
    let mut recorded = 0_u64;
    for attempt in &plan.attempts {
        let directory = attempt_path(&root, attempt.deal);
        if !directory.join("claim.json").exists() {
            ensure!(
                !directory.join("game.json").exists() && !directory.join("abort.json").exists(),
                "result without an assigned claim"
            );
            rows.push(
                json!({"deal":attempt.deal,"status":"not_started","assigned":false,
                "training_utility":null,"bootstrap":false,"game_receipt":null}),
            );
            continue;
        }
        assigned += 1;
        let claim_path = directory.join("claim.json");
        let claim_sha = seal(&claim_path, &mut seals)?;
        let claim = read_claim(&root, &plan, &plan_sha, attempt)?;
        let mut row = json!({"deal":attempt.deal,"assigned":true,"claim":claim_path,
            "claim_sha256":claim_sha,"worker":claim["worker"]});
        let game_path = directory.join("game.json");
        if game_path.exists() {
            ensure!(
                !directory.join("abort.json").exists(),
                "conflicting completion and abort receipts"
            );
            let game_sha = seal(&game_path, &mut seals)?;
            let game = files::read_json(&game_path)?;
            verify_game(&root, &plan, attempt, &game)?;
            let row_sha = files::hex(&sha2::Sha256::digest(serde_json::to_vec(&game)?));
            ensure!(
                index.get(&attempt.deal) == Some(&row_sha),
                "completed receipt differs from original index row"
            );
            seal(
                &root
                    .join("records")
                    .join(game["shard"].as_str().context("recorded shard")?),
                &mut seals,
            )?;
            recorded += 1;
            row["status"] = json!("recorded");
            row["game_receipt"] = json!(game_path);
            row["game_receipt_sha256"] = json!(game_sha);
            row["game_row_sha256"] = json!(row_sha);
            row["record_sha256"] = game["record_sha256"].clone();
        } else {
            let abort_path = directory.join("abort.json");
            let reason = if abort_path.exists() {
                seal(&abort_path, &mut seals)?;
                let abort = files::read_json(&abort_path)?;
                ensure!(
                    abort["schema"] == LEDGER_SCHEMA
                        && abort["plan_sha256"] == plan_sha
                        && abort["claim_sha256"] == claim_sha
                        && abort["deal"] == attempt.deal
                        && abort["worker"] == claim["worker"]
                        && abort["termination_kind"] == "unknown"
                        && abort["training_utility"].is_null()
                        && abort["bootstrap"] == false,
                    "abort receipt binding mismatch"
                );
                let reason = abort["unknown_reason"]
                    .as_str()
                    .context("worker abort class")?
                    .to_owned();
                ensure!(
                    matches!(
                        reason.as_str(),
                        "worker_error" | "observation_error" | "worker_panic"
                    ),
                    "unrecognized worker abort reason"
                );
                reason
            } else {
                match guard["reason"].as_str() {
                    Some("memory_limit") => "process_memory_limit",
                    Some("deadline") => "process_deadline",
                    Some("interrupted") => "process_interrupted",
                    _ => "producer_ended_without_completion",
                }
                .to_owned()
            };
            row["status"] = json!("operational_unknown");
            row["termination_kind"] = json!("unknown");
            row["unknown_reason"] = json!(reason);
            row["training_utility"] = Value::Null;
            row["bootstrap"] = json!(false);
            row["game_receipt"] = Value::Null;
        }
        rows.push(row);
    }
    let output = root.join("attempts.final.json");
    let receipt = root.join("attempts.complete.json");
    for (path, expected) in &seals {
        ensure!(
            files::sha256(path)? == *expected,
            "source changed during attempt finalization"
        );
    }
    let ledger = json!({"schema":LEDGER_SCHEMA,"source_run_sha256":source_run_sha,
        "plan_sha256":plan_sha,"guard_job":job,"guard_job_sha256":job_sha,
        "guard_cleanup_verified":true,"producer_identity":plan.producer_identity,
        "counts":{"planned":count,"assigned":assigned,"recorded":recorded,
            "operational_unknown":assigned-recorded,"not_started":count-assigned},
        "attempts":rows,"source_files":seals,"training_admitted":false,
        "ppo_certified":false,"automatic_restart":false});
    if output.exists() {
        let mut expected = serde_json::to_vec_pretty(&ledger)?;
        expected.push(b'\n');
        ensure!(
            fs::read(&output)? == expected,
            "changed finalized ledger; immutable evidence cannot be replaced"
        );
    } else {
        ensure!(!receipt.exists(), "completion seal has no final ledger");
        publish_new(&output, &ledger)?;
    }
    let completion = json!({"schema":LEDGER_SCHEMA,"ledger_sha256":files::sha256(&output)?,
        "plan_sha256":plan_sha,"source_run_sha256":source_run_sha,"guard_job_sha256":job_sha});
    if receipt.exists() {
        ensure!(
            files::read_json(&receipt)? == completion,
            "changed finalized completion seal"
        );
    } else {
        // Recover the narrow crash window after immutable ledger publication.
        // The existing bytes have been recomputed and matched, never overwritten.
        publish_new(&receipt, &completion)?;
    }
    Ok(ledger)
}

/// Run an explicit CPU research collector, then account for its owned attempts.
/// # Errors
/// Preserves failed jobs and refuses live/unverified finalization; never retries games.
pub fn collect(job: &guard::Job, root: &Path, interrupted: &AtomicBool) -> Result<Value> {
    ensure!(
        std::env::var("BAYLEE_EP").as_deref() != Ok("cuda"),
        "research collection wrapper is CPU-only"
    );
    ensure!(
        job.command
            .windows(2)
            .any(|pair| pair == ["--collection-purpose", "failure_terminal_research_v1"]),
        "explicit failure-terminal research command required"
    );
    let outcome = guard::run(job, interrupted);
    let ledger = finalize(root, &job.directory.join("job.json"))?;
    ensure!(
        outcome?,
        "research collector failed; finalized unknown attempts are retained"
    );
    Ok(ledger)
}
