//! Content-bound admission of complete on-policy learner histories.
//!
//! This checks technical provenance, not playing strength. The separate `PyTorch`
//! audit must reconstruct every selected behavior log probability before updating.
mod rows;
#[cfg(test)]
mod tests;

use crate::files;
use anyhow::{Context, Result, ensure};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{
    collections::{BTreeMap, BTreeSet},
    fs::{self, File},
    path::{Component, Path, PathBuf},
};

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
#[allow(
    clippy::struct_field_names,
    reason = "wire ABI uses seat and game_id throughout the dataset"
)]
struct Seat {
    seat: u8,
    role: String,
    #[serde(default)]
    checkpoint_sha256: Option<String>,
    #[serde(default)]
    thinking_steps: Option<u8>,
    #[serde(default)]
    temperature: Option<f64>,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
#[allow(
    clippy::struct_field_names,
    reason = "wire ABI game_id must match converted trajectory records"
)]
struct Game {
    game_id: u32,
    source_run_sha256: String,
    record_sha256: String,
    source_game: String,
    complete: bool,
    capped: bool,
    fallback_answers: u64,
    refused_answers: u64,
    unencoded_decisions: u64,
    seats: Vec<Seat>,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct Collection {
    version: u32,
    purpose: String,
    source_checkpoint_sha256: String,
    precision: String,
    mode: String,
    policy_provenance_version: u32,
    sampling_version: u32,
    /// Relative dataset file names with content seals, published by conversion.
    dataset_files: BTreeMap<String, String>,
    games: Vec<Game>,
}

pub(super) fn dataset_file(root: &Path, name: &str) -> Result<PathBuf> {
    let path = Path::new(name);
    ensure!(
        !name.is_empty() && path.components().all(|c| matches!(c, Component::Normal(_))),
        "invalid dataset-relative path"
    );
    let result = root.join(path).canonicalize()?;
    ensure!(
        result.starts_with(root.canonicalize()?),
        "dataset file escapes its root"
    );
    Ok(result)
}

fn shard_paths(root: &Path, meta: &Value) -> Result<Vec<PathBuf>> {
    let mut names = BTreeSet::new();
    meta["train_shards"]
        .as_array()
        .context("train_shards missing")?
        .iter()
        .map(|name| {
            let name = name.as_str().context("non-string shard name")?;
            ensure!(
                Path::new(name).components().count() == 1 && names.insert(name),
                "duplicate or nested shard name"
            );
            dataset_file(root, &format!("train/{name}"))
        })
        .collect()
}

/// Match the existing `PyTorch` dataset cache identity while separately sealing content.
/// # Errors
/// Rejects missing or invalid metadata/shards and filesystem timestamps.
pub fn dataset_fingerprint(root: &Path) -> Result<String> {
    let metadata = fs::read(root.join("meta.json"))?;
    let meta: Value = serde_json::from_slice(&metadata)?;
    let mut hash = Sha256::new();
    hash.update(&metadata);
    for path in shard_paths(root, &meta)? {
        let stat = path.metadata()?;
        let nanos = stat
            .modified()?
            .duration_since(std::time::UNIX_EPOCH)?
            .as_nanos();
        let name = path
            .file_name()
            .context("shard name")?
            .to_str()
            .context("non-UTF8 shard name")?;
        hash.update(format!("{name}:{}:{nanos}", stat.len()));
    }
    Ok(files::hex(&hash.finalize()))
}

/// Seal every model input and training shard required by policy admission.
/// # Errors
/// Rejects missing, escaping or concurrently modified dataset files.
pub fn seal_dataset_files(root: &Path, meta: &Value) -> Result<BTreeMap<String, String>> {
    let mut paths = vec![
        "meta.json".to_owned(),
        "card_structure.json".to_owned(),
        "card_dsl.bin".to_owned(),
        "card_dsl_back.bin".to_owned(),
        "token_dsl.bin".to_owned(),
    ];
    for name in meta["train_shards"]
        .as_array()
        .context("train_shards missing")?
    {
        paths.push(format!(
            "train/{}",
            name.as_str().context("invalid shard name")?
        ));
    }
    paths
        .into_iter()
        .map(|name| Ok((name.clone(), files::sha256(&dataset_file(root, &name)?)?)))
        .collect()
}

fn collection_roles(
    collection: &Collection,
    source_sha: &str,
) -> Result<BTreeMap<(u32, u8), Seat>> {
    ensure!(
        collection.version == 1
            && collection.purpose == "on_policy_training"
            && collection.mode == "raw"
            && collection.precision == "float32"
            && collection.policy_provenance_version == 2
            && collection.sampling_version == 2,
        "incompatible or non-training collection contract"
    );
    ensure!(
        collection.source_checkpoint_sha256 == source_sha,
        "collection policy differs from source checkpoint"
    );
    let mut games = BTreeSet::new();
    let mut source_games = BTreeSet::new();
    let mut roles = BTreeMap::new();
    for game in &collection.games {
        ensure!(
            !game.source_game.is_empty()
                && [&game.source_run_sha256, &game.record_sha256]
                    .iter()
                    .all(|s| s.len() == 64
                        && s.bytes()
                            .all(|b| b.is_ascii_hexdigit() && !b.is_ascii_uppercase())),
            "missing original collection/record identity"
        );
        ensure!(
            source_games.insert((&game.source_run_sha256, &game.source_game)),
            "duplicate original source game"
        );
        ensure!(
            games.insert(game.game_id)
                && game.complete
                && !game.capped
                && game.fallback_answers == 0
                && game.refused_answers == 0
                && game.unencoded_decisions == 0,
            "duplicate, incomplete, refused, capped or fallback game {}",
            game.game_id
        );
        ensure!((2..=8).contains(&game.seats.len()), "invalid seat count");
        let mut learner = false;
        for seat in &game.seats {
            ensure!(
                usize::from(seat.seat) < game.seats.len()
                    && roles
                        .insert((game.game_id, seat.seat), seat.clone())
                        .is_none(),
                "invalid or duplicate seat"
            );
            ensure!(
                matches!(
                    seat.role.as_str(),
                    "learner" | "historical" | "self_play" | "house"
                ),
                "unknown actor role"
            );
            if seat.role == "learner" {
                learner = true;
                ensure!(
                    seat.checkpoint_sha256.as_deref() == Some(source_sha)
                        && seat.thinking_steps == Some(2)
                        && seat.temperature.is_some_and(|t| (t - 0.7).abs() < 1e-6),
                    "learner source/depth/temperature mismatch"
                );
            } else if seat.role != "house" {
                ensure!(
                    seat.checkpoint_sha256
                        .as_ref()
                        .is_some_and(|s| s.len() == 64
                            && s.bytes()
                                .all(|b| b.is_ascii_hexdigit() && !b.is_ascii_uppercase())),
                    "missing neural opponent identity"
                );
            }
        }
        ensure!(learner, "game has no learner");
    }
    ensure!(!games.is_empty(), "empty collection");
    Ok(roles)
}

/// Audit raw rows and publish a receipt bound to the model, roles and dataset bytes.
/// # Errors
/// Rejects roleless/off-policy data, malformed action masks, broken histories,
/// source mismatches, incomplete games or files changed while auditing.
pub fn admit(root: &Path, checkpoint: &Path, collection_path: &Path) -> Result<Value> {
    let source_sha = files::sha256(checkpoint)?;
    let collection_sha = files::sha256(collection_path)?;
    let collection: Collection = serde_json::from_reader(File::open(collection_path)?)?;
    let roles = collection_roles(&collection, &source_sha)?;
    let meta = files::read_json(&root.join("meta.json"))?;
    ensure!(
        meta["trajectory_version"] == 2
            && meta["sampling_version"] == 2
            && meta["policy_provenance_version"] == 2
            && matches!(meta["encoder_version"].as_u64(), Some(7..=9)),
        "dataset trajectory/encoder/provenance contract mismatch"
    );
    ensure!(
        meta["ppo_eligible"] != false
            && !meta
                .as_object()
                .context("metadata object")?
                .keys()
                .any(|k| k.starts_with("llm_bc_") || k.starts_with("shadow_bc_")),
        "BC-only data cannot enter PPO"
    );
    ensure!(
        meta["val_shards"].as_array().is_some_and(Vec::is_empty),
        "on-policy admission requires an unsplit collection"
    );
    let inputs = seal_dataset_files(root, &meta)?;
    crate::trajectory::contracts::validate_encoder_rows(root, &meta)?;
    ensure!(
        inputs == collection.dataset_files,
        "collection dataset seals do not match actual inputs"
    );
    let fingerprint = dataset_fingerprint(root)?;
    let mut histories = BTreeMap::<(u32, u8), rows::History>::new();
    for path in shard_paths(root, &meta)? {
        crate::trajectory::read_shard(&path, |value| {
            let row: rows::Row = serde_json::from_value(value)?;
            let key = (row.game_id, row.seat);
            let role = roles.get(&key).context("unbound game/seat in dataset")?;
            histories.entry(key).or_default().observe(&row, role)?;
            Ok(())
        })?;
    }
    ensure!(
        histories.keys().eq(roles.keys()),
        "missing or extra seat observation history"
    );
    let mut admitted = Vec::new();
    let mut decisions = 0_u64;
    for (key, history) in &histories {
        ensure!(
            history.terminal && history.frames > 0,
            "unterminated observation history"
        );
        if roles[key].role == "learner" && history.decisions > 0 {
            admitted.push(json!({"game_id":key.0,"seat":key.1}));
            decisions += history.decisions;
        }
    }
    ensure!(
        decisions > 0 && !admitted.is_empty(),
        "no stochastic learner choices"
    );
    let learner_games: BTreeSet<_> = admitted.iter().map(|key| key["game_id"].as_u64()).collect();
    ensure!(
        learner_games.len() == collection.games.len(),
        "collection contains a game without stochastic learner choices"
    );
    ensure!(
        seal_dataset_files(root, &meta)? == inputs
            && files::sha256(checkpoint)? == source_sha
            && files::sha256(collection_path)? == collection_sha
            && dataset_fingerprint(root)? == fingerprint,
        "inputs changed during admission"
    );
    Ok(
        json!({"version":1,"admitted":true,"source_checkpoint_sha256":source_sha,"dataset_fingerprint":fingerprint,"collection_contract":{"mode":"raw","actor_role":"learner","precision":"float32","thinking_steps":2,"temperature":0.7},"admitted_learner_histories":admitted,"completed_games":collection.games.len(),"learner_histories":admitted.len(),"decisions":decisions,"excluded_invalid_games":0,"dataset_files":inputs,"collection_manifest":collection_path.canonicalize()?,"collection_manifest_sha256":collection_sha,"probability_replay_required":true}),
    )
}

/// Recheck a previously issued receipt before a `PyTorch` process consumes it.
/// # Errors
/// Rejects any changed source, input, role manifest or receipt field.
pub fn verify(root: &Path, checkpoint: &Path, receipt: &Path) -> Result<Value> {
    let saved = files::read_json(receipt)?;
    let path = saved["collection_manifest"]
        .as_str()
        .context("missing bound collection manifest")?;
    let actual = admit(root, checkpoint, Path::new(path))?;
    ensure!(
        actual == saved,
        "admission receipt no longer matches actual inputs"
    );
    Ok(actual)
}
