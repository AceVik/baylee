//! Research-only attempt accounting. Operational failures never acquire game utility.
mod finalize;
#[cfg(test)]
mod tests;

pub use finalize::{collect, finalize};

use crate::{files, process};
use anyhow::{Context as _, Result, ensure};
use flate2::read::GzDecoder;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::{
    collections::BTreeSet,
    fs::{self, File, OpenOptions},
    io::{Read as _, Seek as _, SeekFrom, Write as _},
    path::{Component, Path, PathBuf},
};

const PLAN_SCHEMA: &str = "failure_terminal_attempt_plan_v1";
const LEDGER_SCHEMA: &str = "failure_terminal_attempt_ledger_v1";

/// One planned deal; being planned does not establish assignment or play.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Attempt {
    /// Stable deal index, retaining physical-seat rotation.
    pub deal: u64,
    /// Shuffle seed.
    pub seed: u64,
    /// Physical learner seat.
    pub focal_seat: usize,
    /// Declared independent policy seed before existing deal/seat derivation.
    pub policy_seed: u64,
    /// Ordered, public deck keys used by the native collector.
    pub deck_keys: Vec<String>,
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Plan {
    schema: String,
    source_run_sha256: String,
    producer_identity: process::Identity,
    attempts: Vec<Attempt>,
}

/// A live collector's ledger. Keep it alive until all worker threads have joined.
pub struct Ledger {
    root: PathBuf,
    plan: Plan,
    plan_sha256: String,
    _owner: File,
}

/// Publish a fully synced immutable JSON file without replacing an existing name.
fn publish_new(path: &Path, value: &impl Serialize) -> Result<()> {
    let parent = path.parent().context("receipt parent")?;
    fs::create_dir_all(parent)?;
    let temporary = parent.join(format!(".{}.partial", uuid::Uuid::now_v7()));
    let result = (|| -> Result<()> {
        let mut file = OpenOptions::new()
            .create_new(true)
            .write(true)
            .open(&temporary)?;
        serde_json::to_writer_pretty(&mut file, value)?;
        file.write_all(b"\n")?;
        file.sync_all()?;
        // A hard link publishes the complete inode and fails if the name exists.
        fs::hard_link(&temporary, path)?;
        fs::remove_file(&temporary)?;
        File::open(parent)?.sync_all()?;
        Ok(())
    })();
    if result.is_err() {
        let _ = fs::remove_file(temporary);
    }
    result
}

fn attempt_path(root: &Path, deal: u64) -> PathBuf {
    root.join("attempts").join(format!("{deal:020}"))
}

impl Ledger {
    /// Seal the fixed denominator before any worker can claim an attempt.
    /// # Errors
    /// Rejects ordinary sources, duplicate attempts and an existing plan.
    pub fn create(root: &Path, attempts: Vec<Attempt>) -> Result<Self> {
        Self::create_for(
            root,
            attempts,
            process::identity(std::process::id()).context("collector identity")?,
        )
    }

    fn create_for(
        root: &Path,
        attempts: Vec<Attempt>,
        producer_identity: process::Identity,
    ) -> Result<Self> {
        let root = root.canonicalize()?;
        let owner = files::lock(&root.join("attempts.live.lock"))?;
        let run = files::read_json(&root.join("run.json"))?;
        ensure!(
            run["collection_contract"]["purpose"] == "failure_terminal_research_v1"
                && run["collection_contract"]["version"] == 2
                && run["training_data"] == false,
            "attempt accounting requires separate unadmitted research source"
        );
        let first = run["deal_offset"].as_u64().context("planned first deal")?;
        let count = run["planned_deals"]
            .as_u64()
            .context("planned deal count")?;
        ensure!(
            count > 0 && count == u64::try_from(attempts.len())?,
            "attempt denominator differs from run"
        );
        let end = first
            .checked_add(count)
            .context("attempt denominator overflow")?;
        let mut ids = BTreeSet::new();
        ensure!(
            attempts.iter().all(|attempt| attempt.deal >= first
                && attempt.deal < end
                && ids.insert(attempt.deal)
                && attempt.focal_seat < attempt.deck_keys.len()),
            "invalid or duplicate planned attempt"
        );
        let plan = Plan {
            schema: PLAN_SCHEMA.into(),
            source_run_sha256: files::sha256(&root.join("run.json"))?,
            producer_identity,
            attempts,
        };
        publish_new(&root.join("attempts.plan.json"), &plan)?;
        let plan_sha256 = files::sha256(&root.join("attempts.plan.json"))?;
        Ok(Self {
            root,
            plan,
            plan_sha256,
            _owner: owner,
        })
    }

    fn planned(&self, deal: u64) -> Result<&Attempt> {
        self.plan
            .attempts
            .iter()
            .find(|attempt| attempt.deal == deal)
            .context("attempt outside fixed denominator")
    }

    /// Durably assign an attempt before entering the engine. Never overwrites a claim.
    /// # Errors
    /// Rejects duplicate assignment, unknown deal IDs and I/O failures.
    pub fn claim(&self, deal: u64, worker: usize) -> Result<()> {
        self.planned(deal)?;
        let directory = attempt_path(&self.root, deal);
        fs::create_dir_all(directory.parent().context("attempt parent")?)?;
        File::open(&self.root)?.sync_all()?;
        fs::create_dir(&directory).context("attempt already assigned or partially claimed")?;
        File::open(directory.parent().context("attempt parent")?)?.sync_all()?;
        publish_new(
            &directory.join("claim.json"),
            &json!({"schema":PLAN_SCHEMA,
            "plan_sha256":self.plan_sha256,"source_run_sha256":self.plan.source_run_sha256,
            "producer_identity":self.plan.producer_identity,"deal":deal,"worker":worker}),
        )
    }

    /// Publish an executed game's exact index row after durable record publication.
    /// # Errors
    /// Rejects changed assignment/source metadata, truncated or mismatched gzip records.
    pub fn completed(&self, row: &Value) -> Result<()> {
        let deal = row["i"].as_u64().context("completed deal")?;
        let planned = self.planned(deal)?;
        read_claim(&self.root, &self.plan, &self.plan_sha256, planned)?;
        ensure!(
            !attempt_path(&self.root, deal).join("abort.json").exists(),
            "attempt already aborted"
        );
        verify_game(&self.root, &self.plan, planned, row)?;
        File::open(self.root.join("records"))?.sync_all()?;
        publish_new(&attempt_path(&self.root, deal).join("game.json"), row)
    }

    /// Retain an ended worker's assigned unfinished attempt without inventing an outcome.
    /// # Errors
    /// Rejects malformed claims and unknown failure classes.
    pub fn worker_failed(&self, worker: usize, reason: &str) -> Result<()> {
        ensure!(
            matches!(
                reason,
                "worker_error" | "observation_error" | "worker_panic"
            ),
            "unknown worker abort class"
        );
        for planned in &self.plan.attempts {
            let directory = attempt_path(&self.root, planned.deal);
            if !directory.join("claim.json").exists() || directory.join("game.json").exists() {
                continue;
            }
            let claim = read_claim(&self.root, &self.plan, &self.plan_sha256, planned)?;
            if claim["worker"] == worker {
                publish_new(
                    &directory.join("abort.json"),
                    &json!({"schema":LEDGER_SCHEMA,
                    "plan_sha256":self.plan_sha256,"claim_sha256":files::sha256(&directory.join("claim.json"))?,
                    "deal":planned.deal,"worker":worker,"termination_kind":"unknown","unknown_reason":reason,
                    "training_utility":null,"bootstrap":false}),
                )?;
            }
        }
        Ok(())
    }
}

fn read_claim(root: &Path, plan: &Plan, plan_sha: &str, attempt: &Attempt) -> Result<Value> {
    let claim = files::read_json(&attempt_path(root, attempt.deal).join("claim.json"))?;
    ensure!(
        claim["schema"] == PLAN_SCHEMA
            && claim["plan_sha256"] == plan_sha
            && claim["source_run_sha256"] == plan.source_run_sha256
            && claim["deal"] == attempt.deal
            && claim["producer_identity"] == json!(plan.producer_identity)
            && claim["worker"].as_u64().is_some(),
        "claim binding mismatch"
    );
    Ok(claim)
}

fn verify_game(root: &Path, plan: &Plan, attempt: &Attempt, row: &Value) -> Result<()> {
    ensure!(
        row["i"] == attempt.deal
            && row["seed"] == attempt.seed
            && row["focal_seat"] == attempt.focal_seat
            && row["policy_seed"] == attempt.policy_seed
            && row["source_run_sha256"] == plan.source_run_sha256
            && row["outcome"] == row["research_episode"]
            && row["outcome"]["kind"] == "research_termination",
        "game row differs from assigned research attempt"
    );
    let shard = row["shard"].as_str().context("completed shard")?;
    let mut components = Path::new(shard).components();
    ensure!(
        matches!(components.next(), Some(Component::Normal(_))) && components.next().is_none(),
        "shard must be a basename"
    );
    let path = root.join("records").join(shard);
    ensure!(
        fs::symlink_metadata(&path)?.file_type().is_file(),
        "record shard must be a regular file"
    );
    let mut file = File::open(path)?;
    let offset = row["offset"].as_u64().context("record offset")?;
    let len = row["len"].as_u64().context("record member length")?;
    ensure!(
        len > 0
            && len <= 128 * 1024 * 1024
            && offset
                .checked_add(len)
                .is_some_and(|end| file.metadata().is_ok_and(|metadata| end <= metadata.len())),
        "record member outside shard"
    );
    file.seek(SeekFrom::Start(offset))?;
    let mut bytes = Vec::new();
    GzDecoder::new(file.take(len))
        .take(512 * 1024 * 1024 + 1)
        .read_to_end(&mut bytes)?;
    ensure!(
        bytes.len() <= 512 * 1024 * 1024
            && bytes.ends_with(b"\n")
            && files::hex(&sha2::Sha256::digest(&bytes)) == row["record_sha256"],
        "completed record readback mismatch"
    );
    Ok(())
}

use sha2::Digest as _;
