//! Content-sealed streaming merge of the still-supported v3 tensor columns.
mod stream;
#[cfg(test)]
mod tests;
use crate::files;
use anyhow::{Context, Result, ensure};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeMap,
    fs,
    path::{Component, Path, PathBuf},
};
const VERSION: u64 = 1;
#[derive(Clone)]
struct Shard {
    directory: PathBuf,
    samples: u64,
    entities: u64,
    decks: u64,
    opts: u64,
}
#[derive(Clone)]
struct Column {
    name: &'static str,
    unit: u64,
    kind: Kind,
}
#[derive(Clone, Copy)]
enum Kind {
    Samples,
    Entities,
    Decks,
    Opts,
    OffsetEntities,
    OffsetDecks,
    OffsetOpts,
}
fn count(value: &Value, key: &str) -> Result<u64> {
    value[key]
        .as_u64()
        .with_context(|| format!("invalid v3 count {key}"))
}
fn columns(meta: &Value) -> Result<Vec<Column>> {
    let width = |key: &str| -> Result<u64> {
        let n = u64::try_from(
            meta[key]
                .as_array()
                .with_context(|| format!("missing v3 columns {key}"))?
                .len(),
        )?;
        ensure!(n > 0, "empty v3 feature layout");
        Ok(n)
    };
    let seats = count(meta, "max_seats")?;
    ensure!((1..=64).contains(&seats), "invalid v3 seat count");
    Ok(vec![
        Column {
            name: "ent_card.i32",
            unit: 4,
            kind: Kind::Entities,
        },
        Column {
            name: "ent_feat.i16",
            unit: 2 * width("ent_cols")?,
            kind: Kind::Entities,
        },
        Column {
            name: "ent_off.i64",
            unit: 8,
            kind: Kind::OffsetEntities,
        },
        Column {
            name: "seat.i16",
            unit: 2 * seats * width("seat_cols")?,
            kind: Kind::Samples,
        },
        Column {
            name: "glob.i16",
            unit: 2 * width("glob_cols")?,
            kind: Kind::Samples,
        },
        Column {
            name: "meta.i32",
            unit: 4 * width("meta_cols")?,
            kind: Kind::Samples,
        },
        Column {
            name: "deck_card.i32",
            unit: 4,
            kind: Kind::Decks,
        },
        Column {
            name: "deck_feat.i16",
            unit: 2 * width("deck_cols")?,
            kind: Kind::Decks,
        },
        Column {
            name: "deck_off.i64",
            unit: 8,
            kind: Kind::OffsetDecks,
        },
        Column {
            name: "opt.i16",
            unit: 6,
            kind: Kind::Opts,
        },
        Column {
            name: "opt_off.i64",
            unit: 8,
            kind: Kind::OffsetOpts,
        },
    ])
}
fn inputs(
    root: &Path,
    meta: &Value,
    columns: &[Column],
) -> Result<(Vec<Shard>, BTreeMap<String, String>)> {
    ensure!(
        files::read_json(&root.join("dataset.json"))? == *meta,
        "v3 metadata changed while reading"
    );
    let mut shards = Vec::new();
    let mut seals = BTreeMap::from([(
        "dataset.json".into(),
        files::sha256(&root.join("dataset.json"))?,
    )]);
    for value in meta["shards"].as_array().context("v3 shard list")? {
        let name = value["dir"].as_str().context("v3 shard directory")?;
        ensure!(
            !name.is_empty()
                && Path::new(name)
                    .components()
                    .all(|v| matches!(v, Component::Normal(_))),
            "v3 shard directory escapes dataset"
        );
        let directory = root.join(name);
        ensure!(
            directory.canonicalize()?.starts_with(root),
            "v3 shard symlink escapes dataset"
        );
        let shard = Shard {
            directory,
            samples: count(value, "samples")?,
            entities: count(value, "entities")?,
            decks: count(value, "decks")?,
            opts: count(value, "opts")?,
        };
        ensure!(
            shard.samples > 0 || shard.entities == 0 && shard.decks == 0 && shard.opts == 0,
            "empty v3 shard has nonempty payload"
        );
        for column in columns {
            let path = shard.directory.join(column.name);
            ensure!(
                fs::symlink_metadata(&path)?.file_type().is_file(),
                "v3 column must be a regular file"
            );
            ensure!(
                fs::metadata(&path)?.len() == stream::source_bytes(column, &shard)?,
                "v3 column length differs: {}",
                path.display()
            );
            seals.insert(format!("{name}/{}", column.name), files::sha256(&path)?);
        }
        shards.push(shard);
    }
    ensure!(
        shards.iter().any(|s| s.samples > 0),
        "v3 dataset must contain samples"
    );
    Ok((shards, seals))
}
fn verified(cache: &Path, identity: &Value, columns: &[Column]) -> Result<bool> {
    let Ok(receipt) = files::read_json(&cache.join("done.json")) else {
        return Ok(false);
    };
    if receipt["identity"] != *identity {
        return Ok(false);
    }
    for column in columns {
        let path = cache.join(column.name);
        if !path.is_file() || receipt["outputs"][column.name] != files::sha256(&path)? {
            return Ok(false);
        }
    }
    Ok(true)
}
/// Build/reopen an immutable native cache, preserving old merged files and baselines.
/// # Errors
/// Refuses malformed lengths/offsets, path escapes, source mutation or count overflow.
pub fn merge3(path: &Path) -> Result<Value> {
    let root = path.canonicalize()?;
    let _lock = files::lock(&root.join(".merge3.lock"))?;
    let meta = files::read_json(&root.join("dataset.json"))?;
    ensure!(
        meta["encoder_version"].as_u64() == Some(3),
        "only encoder3 column datasets supported"
    );
    let columns = columns(&meta)?;
    let (shards, seals) = inputs(&root, &meta, &columns)?;
    let identity = json!({"version":VERSION,"sources":seals});
    let fingerprint = files::hex(&Sha256::digest(serde_json::to_vec(&identity)?));
    let merged = root.join("merged");
    fs::create_dir_all(&merged)?;
    let cache = merged.join(format!("native-{fingerprint}"));
    if verified(&cache, &identity, &columns)? {
        return Ok(json!({"cache_dir":cache,"fingerprint":fingerprint,"reused":true}));
    }
    if cache.exists() {
        fs::rename(
            &cache,
            merged.join(format!("rejected-{}", uuid::Uuid::now_v7())),
        )?;
    }
    let temporary = merged.join(format!(".native-{}.partial", uuid::Uuid::now_v7()));
    fs::create_dir(&temporary)?;
    let result = (|| -> Result<()> {
        let totals = stream::write(&temporary, &shards, &columns)?;
        ensure!(
            inputs(&root, &meta, &columns)?.1 == seals,
            "v3 source changed during merge"
        );
        let outputs: BTreeMap<_, _> = columns
            .iter()
            .map(|c| Ok((c.name, files::sha256(&temporary.join(c.name))?)))
            .collect::<Result<_>>()?;
        files::atomic_json(
            &temporary.join("done.json"),
            &json!({"identity":identity,"outputs":outputs,"totals":totals}),
        )?;
        fs::rename(&temporary, &cache)?;
        #[cfg(unix)]
        fs::File::open(&merged)?.sync_all()?;
        Ok(())
    })();
    if result.is_err() {
        let _ = fs::remove_dir_all(&temporary);
    }
    result?;
    Ok(json!({"cache_dir":cache,"fingerprint":fingerprint,"reused":false}))
}
