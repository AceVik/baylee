use super::*;
use crate::research_contract::contract;
use serde_json::json;
use std::{
    collections::{BTreeMap, BTreeSet},
    io::BufReader,
};

mod attempts;
mod producer;

pub(super) struct History {
    pub receipt: Value,
    pub path: PathBuf,
    pub learner: u64,
}
pub(super) struct Source {
    pub root: PathBuf,
    pub manifest: Value,
    pub manifest_sha: String,
    pub seals: BTreeMap<PathBuf, String>,
    pub attempts: Vec<Value>,
    pub histories: Vec<History>,
}

impl Source {
    pub fn load(root: &Path, expected: &str) -> Result<Self> {
        ensure!(
            valid_digest(expected),
            "external conversion manifest SHA256 is required"
        );
        let root = root.canonicalize()?;
        let path = bound(&root, "research.manifest.json")?;
        ensure!(
            files::sha256(&path)? == expected,
            "external conversion manifest seal mismatch"
        );
        let manifest = read_json(&path)?;
        ensure!(
            manifest["schema"] == "failure_terminal_research_conversion_v2"
                && manifest["status"] == "conversion_only_unadmitted"
                && manifest["attempt_denominator_contract"] == "failure_terminal_attempt_ledger_v1"
                && manifest["operational_abort_ledger_supported"] == true
                && manifest["ppo_eligible"] == false
                && manifest["ordinary_target_cache_compatible"] == false
                && manifest["target_contract"] == contract()
                && manifest["frame_contract"] == crate::research_contract::FRAME_CONTRACT
                && manifest["preprocessing_version"] == 1
                && manifest["sampling_version"] == 2
                && manifest["policy_provenance_version"] == 2
                && matches!(manifest["encoder_version"].as_u64(), Some(7..=9)),
            "unsupported research conversion contract"
        );
        let mut content = manifest.clone();
        content
            .as_object_mut()
            .context("conversion manifest object")?
            .remove("content_fingerprint");
        ensure!(
            manifest["content_fingerprint"] == json_digest(&content)?,
            "conversion content fingerprint mismatch"
        );
        let complete = bound(&root, "research.complete.json")?;
        ensure!(
            read_json(&complete)?
                == json!({"schema":"failure_terminal_research_conversion_receipt_v1",
            "manifest_sha256":expected,"content_fingerprint":manifest["content_fingerprint"],"admitted":false}),
            "incomplete research conversion"
        );
        let mut seals = BTreeMap::from([
            (path, expected.to_owned()),
            (complete.clone(), files::sha256(&complete)?),
        ]);
        for (name, expected) in manifest["output_files"]
            .as_object()
            .context("conversion output seals")?
        {
            let path = bound(&root, name)?;
            let expected = expected.as_str().context("output digest")?;
            ensure!(
                valid_digest(expected) && files::sha256(&path)? == expected,
                "conversion output seal mismatch: {name}"
            );
            seals.insert(path, expected.to_owned());
        }
        let mut source = Self {
            root,
            manifest,
            manifest_sha: expected.to_owned(),
            seals,
            attempts: Vec::new(),
            histories: Vec::new(),
        };
        source.reconcile()?;
        source.recheck()?;
        Ok(source)
    }

    fn reconcile(&mut self) -> Result<()> {
        let mut proofs = producer::load(&self.manifest, &mut self.seals)?;
        let mut rows = Vec::new();
        lines(
            BufReader::new(File::open(self.root.join("attempts.jsonl"))?),
            |raw| {
                rows.push(serde_json::from_slice::<Value>(raw)?);
                Ok(())
            },
        )?;
        let mut known = Vec::new();
        let mut counts = BTreeMap::from([
            ("planned", 0_u64),
            ("assigned", 0),
            ("recorded", 0),
            ("operational_unknown", 0),
            ("not_started", 0),
            ("recorded_unknown", 0),
            ("target_episodes", 0),
        ]);
        for (index, row) in rows.iter().enumerate() {
            ensure!(
                uint(&row["attempt_id"])? == u64::try_from(index)? + 1,
                "noncontiguous planned attempt IDs"
            );
            let key = (
                row["source_run_sha256"]
                    .as_str()
                    .context("source run identity")?
                    .to_owned(),
                uint(&row["source_index"])?,
            );
            let proof = proofs
                .remove(&key)
                .context("duplicate or unbound planned attempt")?;
            ensure!(
                row["source_attempt_ledger_entry"] == proof.ledger
                    && row["planned_attempt"] == proof.planned
                    && row["source_attempt_status"] == proof.ledger["status"]
                    && row["assigned"] == proof.ledger["assigned"],
                "planned attempt differs from original ledger"
            );
            *counts.get_mut("planned").expect("planned counter") += 1;
            if row["assigned"] == true {
                *counts.get_mut("assigned").expect("assigned counter") += 1;
            }
            let status = row["source_attempt_status"]
                .as_str()
                .context("attempt status")?;
            *counts
                .get_mut(status)
                .context("unsupported attempt status")? += 1;
            if let Some(history) = attempts::history(&self.root, row, &proof)? {
                known.push(row.clone());
                self.histories.push(history);
            } else if status == "recorded" {
                *counts.get_mut("recorded_unknown").expect("counter") += 1;
            }
        }
        *counts.get_mut("target_episodes").expect("counter") = u64::try_from(known.len())?;
        ensure!(
            proofs.is_empty()
                && self.manifest["target_episodes"] == json!(known)
                && self.manifest["attempt_counts"] == json!(counts)
                && self.manifest["planned_attempts"] == counts["planned"]
                && self.manifest["unknown_attempts"]
                    == counts["recorded_unknown"] + counts["operational_unknown"],
            "research attempt denominator mismatch"
        );
        self.attempts = rows;
        self.exact_files()?;
        Ok(())
    }

    fn exact_files(&self) -> Result<()> {
        let mut expected: BTreeSet<String> = [
            "attempts.jsonl",
            "card_structure.json",
            "card_dsl.bin",
            "card_dsl_back.bin",
            "token_dsl.bin",
            "card_abilities.bin",
        ]
        .map(str::to_owned)
        .into();
        for history in &self.histories {
            ensure!(
                expected.insert(
                    history.receipt["episode_file"]
                        .as_str()
                        .context("episode filename")?
                        .to_owned()
                ),
                "duplicate episode file"
            );
        }
        let declared: BTreeSet<_> = self.manifest["output_files"]
            .as_object()
            .context("output files")?
            .keys()
            .cloned()
            .collect();
        ensure!(
            expected == declared,
            "missing or unexpected research output files"
        );
        let mut actual = BTreeSet::new();
        for entry in fs::read_dir(&self.root)? {
            let entry = entry?;
            let name = entry
                .file_name()
                .into_string()
                .map_err(|_| anyhow::anyhow!("non-UTF8 research member"))?;
            if name == "episodes" {
                ensure!(entry.file_type()?.is_dir(), "invalid episode directory");
                for episode in fs::read_dir(entry.path())? {
                    let episode = episode?;
                    let name = episode
                        .file_name()
                        .into_string()
                        .map_err(|_| anyhow::anyhow!("non-UTF8 episode"))?;
                    actual.insert(format!("episodes/{name}"));
                }
            } else if !["research.manifest.json", "research.complete.json"].contains(&name.as_str())
            {
                actual.insert(name);
            }
        }
        ensure!(actual == expected, "unlisted research conversion evidence");
        Ok(())
    }

    pub fn recheck(&self) -> Result<()> {
        for (path, expected) in &self.seals {
            regular(path)?;
            ensure!(
                files::sha256(path)? == *expected,
                "research source changed: {}",
                path.display()
            );
        }
        producer::recheck(&self.manifest)?;
        self.exact_files()
    }
}

fn validate_recorded(row: &Value, game: &Value) -> Result<()> {
    ensure!(
        row["source_game"] == game["game"]
            && row["source_record_sha256"] == game["record_sha256"]
            && row["source_row_sha256"] == json_digest(game)?
            && row["actor_roles"] == game["actor_roles"]
            && row["source_sidecars_sha256"] == json_digest(&game["rollout_sidecar"])?
            && row["source_episode_sha256"] == json_digest(&game["research_episode"])?
            && game["outcome"] == game["research_episode"]
            && game["fallback_answers"] == 0,
        "recorded attempt differs from original producer receipt"
    );
    Ok(())
}
