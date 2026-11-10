//! Native metadata identities and admission for detached tensor feature caches.
use super::bc_metadata;
use crate::files;
use anyhow::{Context, Result, bail, ensure};
use serde_json::{Value, json};
use std::{
    collections::{BTreeMap, BTreeSet},
    fs::File,
    io::{BufRead, BufReader, Read},
    path::{Path, PathBuf},
};
/// Validate sealed records and return the eligible tensor-row indices.
/// # Errors
/// Rejects incomplete, changed, empty, incompatible or oversized feature caches.
pub fn admit(
    path: &Path,
    width: u64,
    settings: Option<&Value>,
    limit: Option<u64>,
) -> Result<Value> {
    let meta = files::read_json(&path.join("complete.json"))?;
    let count = meta["records"]
        .as_u64()
        .context("invalid calibration record count")?;
    let size = width
        .checked_mul(64)
        .and_then(|n| n.checked_add(69))
        .context("calibration record width overflow")?;
    let records = path.join("records.bin");
    ensure!(
        width > 0
            && records.metadata()?.len()
                == count
                    .checked_mul(size)
                    .context("calibration cache size overflow")?,
        "Incomplete calibration feature cache"
    );
    ensure!(
        meta["records_sha256"].as_str() == Some(files::sha256(&records)?.as_str()),
        "Calibration feature cache records digest differs"
    );
    ensure!(
        settings.is_none_or(|expected| meta["identity"] == *expected)
            && limit.is_none_or(|max| count <= max),
        "Incompatible or oversized calibration cache"
    );
    let eligible = meta["eligible_trajectories"]
        .as_array()
        .context("missing eligible trajectories")?
        .iter()
        .map(|v| v.as_i64().context("invalid eligible trajectory"))
        .collect::<Result<BTreeSet<_>>>()?;
    let offset = usize::try_from(
        width
            .checked_mul(64)
            .and_then(|n| n.checked_add(24))
            .context("record offset overflow")?,
    )?;
    let mut stream = File::open(records)?;
    let mut row = vec![0; usize::try_from(size)?];
    let mut indices = Vec::new();
    for i in 0..count {
        stream.read_exact(&mut row)?;
        let trajectory = i64::from_le_bytes(row[offset..offset + 8].try_into()?);
        if eligible.contains(&trajectory) {
            indices.push(i);
        }
    }
    ensure!(
        !indices.is_empty(),
        "No sampled learner trajectories in feature cache"
    );
    Ok(json!({"meta":meta,"indices":indices}))
}
/// The deal seed of every converted game: `ready/game-*.json` names each game's
/// source shard and offset, and the run's `games.jsonl` (the shard's grandparent)
/// gives that record's seed.
/// # Errors
/// Rejects unreadable metadata, duplicate games and any game without a seed.
fn game_seeds(data: &Path) -> Result<Value> {
    let mut wanted: BTreeMap<(PathBuf, String, u64), Vec<u64>> = BTreeMap::new();
    let mut games = 0usize;
    for entry in std::fs::read_dir(data.join("ready"))? {
        let path = entry?.path();
        if path.extension().is_none_or(|e| e != "json") {
            continue;
        }
        let ready = files::read_json(&path)?;
        let shard = Path::new(
            ready["source_shard"]
                .as_str()
                .context("ready record without source shard")?,
        );
        let run = shard
            .parent()
            .and_then(Path::parent)
            .context("source shard outside a run directory")?;
        let name = shard
            .file_name()
            .and_then(|n| n.to_str())
            .context("source shard without a name")?;
        let offset = ready["source_offset"]
            .as_u64()
            .context("ready record without source offset")?;
        let game = ready["game_id"]
            .as_u64()
            .context("ready record without game id")?;
        wanted
            .entry((run.to_path_buf(), name.to_owned(), offset))
            .or_default()
            .push(game);
        games += 1;
    }
    let runs: BTreeSet<PathBuf> = wanted.keys().map(|key| key.0.clone()).collect();
    let mut seeds = BTreeMap::new();
    for run in runs {
        for line in BufReader::new(File::open(run.join("games.jsonl"))?).lines() {
            let line = line?;
            if line.trim().is_empty() {
                continue;
            }
            let record: Value = serde_json::from_str(&line)?;
            let key = (
                run.clone(),
                record["shard"].as_str().unwrap_or_default().to_owned(),
                record["offset"].as_u64().unwrap_or(u64::MAX),
            );
            if let Some(ids) = wanted.get(&key) {
                let seed = record["seed"]
                    .as_u64()
                    .context("game record without seed")?;
                for id in ids {
                    ensure!(seeds.insert(*id, seed).is_none(), "game {id} matched twice");
                }
            }
        }
    }
    ensure!(
        seeds.len() == games,
        "{} of {games} converted games have no recorded seed",
        games - seeds.len()
    );
    Ok(json!(seeds))
}
fn identity(value: &Value) -> Value {
    json!({"version":2,"source_sha256":value["source_hash"],"dataset_fingerprint":value["dataset_fingerprint"],"extraction_sources_sha256":value["extraction_sources_sha256"],"device_type":value["device_type"],"structure_sha256":value["structure_sha256"],"sample_every":value["stride"],"unroll":value["unroll"],"board_chunk":value["board_chunk"],"thinking":"recorded positive K on Net decisions; otherwise K1; passive K0","precision":"float32","sampling":"cursor stride plus first and terminal; no frame skipping"})
}
/// Execute immutable calibration metadata operations for thin tensor adapters.
/// # Errors
/// Rejects malformed requests, cache records or collection-plan identities.
pub fn run(value: &Value) -> Result<Value> {
    match value["kind"].as_str() {
        Some("identity") => Ok(identity(value)),
        Some("key") => Ok(json!(bc_metadata::spaced_digest(&value["settings"])?)),
        Some("admit") => admit(
            Path::new(
                value["path"]
                    .as_str()
                    .context("missing feature cache path")?,
            ),
            value["width"].as_u64().context("missing feature width")?,
            value.get("settings"),
            value["max_samples"].as_u64(),
        ),
        Some("collection_plan") => {
            let Some(path) = value["path"].as_str() else {
                return Ok(Value::Null);
            };
            let path = Path::new(path).canonicalize()?;
            let plan = files::read_json(&path)?;
            ensure!(
                plan["model_sha256"] == value["source_hash"],
                "Collection plan model_sha256 differs from source checkpoint"
            );
            Ok(
                json!({"path":path,"sha256":files::sha256(&path)?,"model_sha256":value["source_hash"]}),
            )
        }
        Some("game_seeds") => game_seeds(Path::new(
            value["data_dir"]
                .as_str()
                .context("missing dataset directory")?,
        )),
        _ => bail!("unsupported calibration metadata operation"),
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn native_feature_admission_preserves_unaligned_tensor_layout_and_content_seals() -> Result<()>
    {
        let root = tempfile::tempdir()?;
        let path = root.path();
        let records = path.join("records.bin");
        let mut bytes = vec![0u8; 2 * 133];
        bytes[88..96].copy_from_slice(&3i64.to_le_bytes());
        bytes[133 + 88..133 + 96].copy_from_slice(&5i64.to_le_bytes());
        std::fs::write(&records, &bytes)?;
        let settings = json!({"model":"source"});
        files::atomic_json(
            &path.join("complete.json"),
            &json!({"records":2,"records_sha256":files::sha256(&records)?,"eligible_trajectories":[5],"identity":settings}),
        )?;
        assert_eq!(
            admit(path, 1, Some(&settings), Some(2))?["indices"],
            json!([1])
        );
        assert!(admit(path, 1, Some(&json!({"model":"other"})), None).is_err());
        assert!(admit(path, 1, None, Some(1)).is_err());
        bytes[80..84].copy_from_slice(&1.0f32.to_le_bytes());
        std::fs::write(&records, bytes)?;
        assert!(
            admit(path, 1, None, None)
                .unwrap_err()
                .to_string()
                .contains("digest")
        );
        Ok(())
    }
    #[test]
    fn collection_plans_are_sealed_and_bound_to_the_source_checkpoint() -> Result<()> {
        let root = tempfile::tempdir()?;
        let path = root.path().join("plan.json");
        files::atomic_json(&path, &json!({"model_sha256":"source"}))?;
        assert_eq!(
            run(&json!({"kind":"collection_plan","path":path,"source_hash":"source"}))?["sha256"],
            files::sha256(&path)?
        );
        assert!(run(&json!({"kind":"collection_plan","path":path,"source_hash":"other"})).is_err());
        assert!(
            run(&json!({"kind":"collection_plan","path":null}))
                .unwrap()
                .is_null()
        );
        Ok(())
    }
    #[test]
    fn game_seeds_join_ready_records_to_their_runs_games() -> Result<()> {
        let root = tempfile::tempdir()?;
        let data = root.path().join("dataset");
        let run = root.path().join("run");
        std::fs::create_dir_all(data.join("ready"))?;
        std::fs::create_dir_all(run.join("records"))?;
        let shard = run.join("records").join("w00-0000.jsonl.gz");
        for (game, offset) in [(1, 0), (2, 120), (3, 0)] {
            let shard = if game == 3 {
                run.join("records").join("w01-0000.jsonl.gz")
            } else {
                shard.clone()
            };
            files::atomic_json(
                &data.join("ready").join(format!("game-{game:06}.json")),
                &json!({"game_id":game,"source_shard":shard,"source_offset":offset,"split":"val"}),
            )?;
        }
        std::fs::write(
            run.join("games.jsonl"),
            "{\"shard\":\"w00-0000.jsonl.gz\",\"offset\":0,\"seed\":1150301000000000010}\n\
             {\"shard\":\"w00-0000.jsonl.gz\",\"offset\":120,\"seed\":1150301000000000020}\n\
             {\"shard\":\"w01-0000.jsonl.gz\",\"offset\":0,\"seed\":1150301000000000030}\n",
        )?;
        let seeds = run_kind(&data)?;
        assert_eq!(
            seeds,
            json!({"1":1_150_301_000_000_000_010_u64,"2":1_150_301_000_000_000_020_u64,
                   "3":1_150_301_000_000_000_030_u64})
        );
        std::fs::write(
            run.join("games.jsonl"),
            "{\"shard\":\"w00-0000.jsonl.gz\",\"offset\":0,\"seed\":1}\n",
        )?;
        assert!(run_kind(&data).is_err());
        Ok(())
    }
    fn run_kind(data: &Path) -> Result<Value> {
        run(&json!({"kind":"game_seeds","data_dir":data}))
    }
}
