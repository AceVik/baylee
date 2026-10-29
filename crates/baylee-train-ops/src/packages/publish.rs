use crate::{files, trajectory::cache};
use anyhow::{Context, Result, ensure};
use serde_json::{Value, json};
use std::{
    collections::BTreeSet,
    fs::{self, File},
    io::{BufReader, Read},
    path::Path,
};

pub(super) const ASSETS: [&str; 5] = [
    "card_structure.json",
    "card_dsl.bin",
    "card_dsl_back.bin",
    "token_dsl.bin",
    "card_abilities.bin",
];

/// Verify complete per-seat cursors and one terminal marker from native index rows.
/// # Errors
/// Rejects empty/truncated indexes, duplicate cursors, unknown games or partial histories.
pub fn verify_index(path: &Path, allowed: &BTreeSet<u32>) -> Result<(BTreeSet<u32>, u64)> {
    let length = path.metadata()?.len();
    ensure!(
        length > 0 && length.is_multiple_of(32),
        "empty or truncated trajectory index"
    );
    let mut rows = Vec::new();
    let mut reader = BufReader::new(File::open(path)?);
    for _ in 0..length / 32 {
        let mut bytes = [0; 32];
        reader.read_exact(&mut bytes)?;
        let game = u32::from_le_bytes(bytes[..4].try_into()?);
        let seat = u32::from_le_bytes(bytes[4..8].try_into()?);
        let cursor = u64::from_le_bytes(bytes[8..16].try_into()?);
        let flags = u32::from_le_bytes(bytes[28..32].try_into()?);
        ensure!(
            allowed.contains(&game) && seat < 8,
            "validation/unknown game or invalid seat in package"
        );
        rows.push((game, seat, cursor, flags));
    }
    rows.sort_unstable_by_key(|row| (row.0, row.1, row.2));
    let mut previous = None;
    let mut next = 0;
    let mut terminal = false;
    let mut games = BTreeSet::new();
    for (game, seat, cursor, flags) in rows {
        let key = (game, seat);
        if previous != Some(key) {
            ensure!(
                previous.is_none() || terminal,
                "incomplete trajectory terminal marker"
            );
            next = 0;
            terminal = false;
            previous = Some(key);
        }
        ensure!(
            !terminal && cursor == next,
            "incomplete/duplicate trajectory cursors or post-terminal row"
        );
        terminal = flags & 2 != 0;
        next = next.checked_add(1).context("cursor overflow")?;
        games.insert(game);
    }
    ensure!(terminal, "incomplete trajectory terminal marker");
    Ok((games, length / 32))
}

fn hard_link(source: &Path, relative: &Path, output: &Path) -> Result<()> {
    let input = source.join(relative);
    ensure!(
        input.symlink_metadata()?.is_file()
            && input.canonicalize()?.starts_with(source.canonicalize()?),
        "package source is not a local regular file"
    );
    fs::hard_link(input, output)?;
    Ok(())
}

/// Publish ready.json only after assets, complete games and reusable cache verify.
/// # Errors
/// Existing or invalid packages are preserved without a ready marker.
pub fn publish(source: &Path, output: &Path, entries: &[Value], header: &Value) -> Result<Value> {
    fs::create_dir(output)?;
    fs::create_dir(output.join("train"))?;
    ensure!(!entries.is_empty(), "empty package");
    let mut ids = BTreeSet::new();
    let mut names = BTreeSet::new();
    let mut shards = Vec::new();
    let mut frames = 0_u64;
    for entry in entries {
        let name = entry["shard"].as_str().context("game shard")?;
        ensure!(
            Path::new(name).components().count() == 1
                && Path::new(name).file_name().is_some_and(|n| n == name),
            "only local game shards may enter packages"
        );
        ensure!(
            entry["split"] == "train"
                && entry["structure_sha256"] == header["structure_sha256"]
                && entry["replay_hashes_checked"] == true
                && entry["unencoded_decisions"] == 0,
            "unverified/validation/unencoded game or rule catalog mismatch"
        );
        let id = u32::try_from(entry["game_id"].as_u64().context("game ID")?)?;
        ensure!(
            ids.insert(id) && names.insert(name.to_owned()),
            "duplicate package game/shard"
        );
        frames = frames
            .checked_add(
                entry["steps"]
                    .as_u64()
                    .filter(|n| *n > 0)
                    .context("nonempty game frames")?,
            )
            .context("package frame overflow")?;
        hard_link(
            source,
            &Path::new("train").join(name),
            &output.join("train").join(name),
        )?;
        shards.push(name.to_owned());
    }
    for name in ASSETS {
        hard_link(source, Path::new(name), &output.join(name))?;
    }
    let mut metadata = header.clone();
    let updates = json!({"train_shards":shards,"val_shards":[],"train_steps":frames,"val_steps":0,"games_requested":ids.len(),"games_converted":ids.len(),"games_rejected":0,"unencoded_decisions":0,"coverage_matrix":null,"source_game_ids":ids,"source_dataset":source,"package_sources":entries,"validation_runs":[]});
    metadata
        .as_object_mut()
        .context("header object")?
        .extend(updates.as_object().context("updates object")?.clone());
    files::atomic_json(&output.join("meta.json"), &metadata)?;
    let receipt = cache::build(output, "train", &shards)?;
    let (actual, count) = verify_index(&receipt.index_path, &ids)?;
    ensure!(
        actual == ids && count == frames && receipt.records == frames,
        "package count or trajectory mismatch"
    );
    let mut inputs = std::collections::BTreeMap::new();
    for name in ASSETS
        .into_iter()
        .map(str::to_owned)
        .chain(shards.iter().map(|name| format!("train/{name}")))
    {
        inputs.insert(name.clone(), files::sha256(&output.join(name))?);
    }
    let marker = json!({"version":2,"games":ids.len(),"frames":frames,"metadata_sha256":files::sha256(&output.join("meta.json"))?,"inputs":inputs});
    files::atomic_json(&output.join("ready.json"), &marker)?;
    Ok(marker)
}
