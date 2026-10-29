//! Strict fresh-producer bindings; diagnostic/Gate A artifacts are never inputs.
use anyhow::{Context as _, ensure};
use baylee_train::{
    convert4::{ProducerEncoder, RolloutStepSidecar4},
    failure_terminal::{VerifiedEpisode, contract, sha, verify_episode},
};
use baylee_train_ops::files;
use flate2::read::GzDecoder;
use serde_json::{Value, json};
use std::{
    collections::{BTreeMap, BTreeSet},
    fs::{self, File},
    io::{Read as _, Seek as _, SeekFrom},
    path::{Path, PathBuf},
};

pub(super) struct Run {
    pub root: PathBuf,
    pub metadata: Value,
    pub encoder: u32,
    pub seals: BTreeMap<PathBuf, String>,
    pub source_sha: String,
    pub index_sha: String,
    pub games: Vec<Game>,
}

pub(super) struct Game {
    pub row: Value,
    pub sidecar: Vec<RolloutStepSidecar4>,
    pub learner: u8,
    shard: PathBuf,
}

fn bound(root: &Path, path: &Path) -> anyhow::Result<PathBuf> {
    let canonical = path.canonicalize()?;
    ensure!(
        canonical == path && canonical.starts_with(root) && canonical.is_file(),
        "research source path is redirected or escapes its run directory"
    );
    Ok(canonical)
}

fn roles(row: &Value, model: &Value, opponent: &Value, role: &Value) -> anyhow::Result<u8> {
    let focal = u8::try_from(row["focal_seat"].as_u64().context("research focal seat")?)?;
    let seats = row["actor_roles"]
        .as_array()
        .context("research actor roles")?;
    ensure!(
        focal < 2
            && row["i"]
                .as_u64()
                .is_some_and(|index| index % 2 == u64::from(focal))
            && seats.len() == 2,
        "research 1v1 role binding mismatch"
    );
    let mut seen = BTreeSet::new();
    for seat in seats {
        let index = seat["seat"].as_u64().context("research actor seat")?;
        ensure!(index < 2 && seen.insert(index), "duplicate research seat");
        let (expected, identity) = if index == u64::from(focal) {
            ("learner", model)
        } else {
            (role.as_str().context("research opponent role")?, opponent)
        };
        ensure!(
            seat["role"] == expected
                && seat["checkpoint_sha256"] == identity["source_checkpoint_sha256"]
                && seat["thinking_steps"] == 2
                && seat["temperature"]
                    .as_f64()
                    .is_some_and(|value| (value - 0.7).abs() < 1e-6),
            "research seat role/model/sampling differs from collector assignment"
        );
    }
    Ok(focal)
}

fn validate_metadata(metadata: &Value) -> anyhow::Result<u32> {
    let contract_value = &metadata["collection_contract"];
    ensure!(
        contract_value["version"] == 2
            && contract_value["purpose"] == "failure_terminal_research_v1"
            && contract_value["research_target_contract"] == contract()
            && contract_value["precision"] == "float32"
            && contract_value["seat_rng_rule"] == "independent_policy_seed_xor_physical_seat_v1"
            && metadata["training_data"] == false
            && metadata["policy_mode"] == "raw"
            && metadata["policy_contract"] == "raw_neural_sampler2_v1"
            && metadata["policy_provenance_version"] == 2
            && metadata["sampling_version"] == 2
            && metadata["thinking_steps"] == 2
            && metadata["temperature"]
                .as_f64()
                .is_some_and(|value| (value - 0.7).abs() < 1e-6)
            && metadata["seats"] == 2
            && metadata["held_out_cards"] == false
            && metadata["holdout_tier"] == ""
            && (metadata["decks_manifest"].is_null() || metadata["deck_split"] == "train")
            && metadata["shadow_teacher"].is_null(),
        "source is not a fresh supported research collection; diagnostic/ordinary sources cannot be recertified"
    );
    ProducerEncoder::from_run(metadata).map_err(anyhow::Error::msg)?;
    let encoder = u32::try_from(
        metadata["model_identity"]["encoder_version"]
            .as_u64()
            .context("research encoder")?,
    )?;
    let model = &contract_value["learner_model_identity"];
    let opponent = &contract_value["opponent_model_identity"];
    ensure!(
        *model == metadata["model_identity"] && !opponent.is_null(),
        "research frozen learner/opponent identities required"
    );
    crate::on_policy::check_model(model)?;
    crate::on_policy::check_model(opponent)?;
    ensure!(
        opponent["encoder_version"] == model["encoder_version"]
            && matches!(
                contract_value["opponent_role"].as_str(),
                Some("historical" | "self_play")
            ),
        "research opponent/encoder contract mismatch"
    );
    ensure!(
        contract_value["opponent_role"] != "self_play" || opponent == model,
        "research self-play model differs from learner"
    );
    Ok(encoder)
}

impl Run {
    pub fn load(path: &Path) -> anyhow::Result<Self> {
        let root = path.canonicalize()?;
        let mut seals = BTreeMap::new();
        for name in ["run.json", "games.jsonl", "run.complete.json"] {
            let path = bound(&root, &root.join(name))?;
            seals.insert(path.clone(), files::sha256(&path)?);
        }
        let metadata = files::read_json(&root.join("run.json"))?;
        let encoder = validate_metadata(&metadata)?;
        let contract_value = &metadata["collection_contract"];
        let model = &contract_value["learner_model_identity"];
        let opponent = &contract_value["opponent_model_identity"];
        let run_sha = seals[&root.join("run.json")].clone();
        let index_sha = seals[&root.join("games.jsonl")].clone();
        let complete = files::read_json(&root.join("run.complete.json"))?;
        ensure!(
            complete["version"] == 2
                && complete["purpose"] == "failure_terminal_research_v1"
                && complete["source_run_sha256"] == run_sha
                && complete["games_index_sha256"] == index_sha
                && complete["planned_deals"] == metadata["planned_deals"]
                && complete["recorded_deals"] == metadata["planned_deals"],
            "research conversion requires complete v2 index receipt; crash denominator is unsupported"
        );
        let count = metadata["planned_deals"]
            .as_u64()
            .context("planned research attempts")?;
        let first = metadata["deal_offset"]
            .as_u64()
            .context("first research attempt")?;
        ensure!(count > 0, "empty research collection");
        let mut seen = BTreeSet::new();
        let mut keys = BTreeSet::new();
        let mut games = Vec::new();
        for line in fs::read_to_string(root.join("games.jsonl"))?
            .lines()
            .filter(|line| !line.trim().is_empty())
        {
            let row: Value = serde_json::from_str(line)?;
            let index = row["i"].as_u64().context("research attempt index")?;
            ensure!(
                index >= first && index - first < count && seen.insert(index),
                "missing/duplicate/outside research attempt"
            );
            ensure!(
                row["source_run_sha256"] == run_sha
                    && row["fallback_answers"] == 0
                    && row["outcome"] == row["research_episode"]
                    && row["research_episode"]["target_contract"] == contract(),
                "research attempt source/target/recovery mismatch"
            );
            let key = row["game"].as_str().context("research source game key")?;
            ensure!(
                !key.is_empty() && keys.insert(key.to_owned()),
                "empty or duplicate research source key"
            );
            crate::on_policy::digest(&row["record_sha256"])?;
            let learner = roles(&row, model, opponent, &contract_value["opponent_role"])?;
            let shard = row["shard"].as_str().context("research shard")?;
            ensure!(
                Path::new(shard)
                    .file_name()
                    .is_some_and(|name| name == shard),
                "research shard must be a filename"
            );
            let shard = bound(&root, &root.join("records").join(shard))?;
            if !seals.contains_key(&shard) {
                seals.insert(shard.clone(), files::sha256(&shard)?);
            }
            let sidecar = serde_json::from_value(row["rollout_sidecar"].clone())
                .context("research original sidecar")?;
            games.push(Game {
                row,
                sidecar,
                learner,
                shard,
            });
        }
        ensure!(
            u64::try_from(games.len())? == count,
            "research source does not preserve every planned indexed attempt"
        );
        games.sort_by_key(|game| game.row["i"].as_u64());
        Ok(Self {
            root,
            metadata,
            encoder,
            seals,
            source_sha: run_sha,
            index_sha,
            games,
        })
    }

    pub fn recheck(&self) -> anyhow::Result<()> {
        for (path, expected) in &self.seals {
            ensure!(
                files::sha256(path)? == *expected,
                "research source changed during conversion: {}",
                path.display()
            );
        }
        Ok(())
    }

    pub fn identity(&self) -> Value {
        json!({"directory":self.root,"source_run_sha256":self.source_sha,"games_index_sha256":self.index_sha,
            "collection_contract":self.metadata["collection_contract"],"evaluator_revision":self.metadata["evaluator_revision"],
            "evaluator_dirty":self.metadata["evaluator_dirty"],"source_files":self.seals})
    }
}

impl Game {
    pub fn record(&self) -> anyhow::Result<Vec<u8>> {
        let mut file = File::open(&self.shard)?;
        let offset = self.row["offset"]
            .as_u64()
            .context("research member offset")?;
        let len = self.row["len"].as_u64().context("research member length")?;
        ensure!(
            len <= 128 * 1024 * 1024
                && offset
                    .checked_add(len)
                    .is_some_and(|end| file.metadata().is_ok_and(|meta| end <= meta.len())),
            "research member outside bounds"
        );
        file.seek(SeekFrom::Start(offset))?;
        let mut bytes = Vec::new();
        GzDecoder::new(file.take(len))
            .take(512 * 1024 * 1024 + 1)
            .read_to_end(&mut bytes)?;
        ensure!(
            bytes.len() <= 512 * 1024 * 1024 && sha(&bytes) == self.row["record_sha256"],
            "research member size/digest mismatch"
        );
        Ok(bytes)
    }

    pub fn verify(&self, record: &[u8], encoder: u32) -> anyhow::Result<VerifiedEpisode> {
        ensure!(
            sha(record) == self.row["record_sha256"],
            "research executed record changed"
        );
        let episode = &self.row["research_episode"];
        let proof = verify_episode(record, &self.sidecar, episode, encoder, self.learner)?;
        let natural = episode["termination_kind"] == "mtg_terminal";
        let cap = episode["boundary"]["guard_reason"] == "answer_cap";
        let refused = episode["boundary"]["guard_reason"] == "engine_refusal";
        ensure!(
            self.row["complete"] == natural
                && self.row["capped"] == cap
                && self.row["refused_answers"].as_u64() == Some(u64::from(refused)),
            "research completion/cap/refusal metadata mismatch"
        );
        ensure!(
            self.row["fallback_events"]
                .as_array()
                .is_some_and(|events| events.iter().all(|event| event["accepted"] != true)),
            "executed recovery event in research source"
        );
        Ok(proof)
    }
}
