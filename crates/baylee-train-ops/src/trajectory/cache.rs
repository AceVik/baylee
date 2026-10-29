//! Native disk cache construction; persistent identities preserve existing datasets.
use super::{
    contracts, packing,
    reader::{SHARD_MAGIC, footer},
};
use crate::files;
use anyhow::{Context, Result, ensure};
use flate2::{Compression, read::MultiGzDecoder, write::ZlibEncoder};
use serde::Serialize;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeMap,
    fs::{File, OpenOptions},
    io::{BufRead, BufReader, Read, Seek, SeekFrom, Write},
    path::{Path, PathBuf},
};
/// Native cache receipt consumed by the thin `PyTorch` tensor adapter.
#[derive(Serialize)]
pub struct Receipt {
    /// Byte-compatible metadata/shard-stat identity, not a content seal.
    pub fingerprint: String,
    /// Compressed immutable trajectory frames.
    pub data_path: PathBuf,
    /// Existing 32-byte little-endian frame indexes.
    pub index_path: PathBuf,
    /// Number of indexed observations.
    pub records: u64,
}
fn paths(root: &Path, split: &str, shards: &[String]) -> Result<Vec<PathBuf>> {
    ensure!(
        ["train", "val"].contains(&split),
        "invalid trajectory split"
    );
    shards
        .iter()
        .map(|name| {
            ensure!(
                Path::new(name)
                    .file_name()
                    .is_some_and(|n| n == name.as_str()),
                "invalid shard name"
            );
            let path = root.join(split).join(name);
            ensure!(
                path.canonicalize()?.starts_with(root.canonicalize()?),
                "shard escapes dataset"
            );
            Ok(path)
        })
        .collect()
}
/// Preserve the established SHA256(meta bytes, then `name:size:mtime_ns` per shard).
///
/// # Errors
/// Rejects invalid splits/paths, missing shards or unavailable timestamps.
pub fn fingerprint(root: &Path, split: &str, shards: &[String]) -> Result<String> {
    let mut hash = Sha256::new();
    hash.update(std::fs::read(root.join("meta.json"))?);
    for path in paths(root, split, shards)? {
        let stat = path.metadata()?;
        let nanos = stat
            .modified()?
            .duration_since(std::time::UNIX_EPOCH)?
            .as_nanos();
        let name = path
            .file_name()
            .context("missing shard name")?
            .to_str()
            .context("non-UTF8 shard")?;
        hash.update(format!("{name}:{}:{nanos}", stat.len()));
    }
    Ok(files::hex(&hash.finalize()))
}
/// Build or reopen a cache only after native teacher-contract admission.
///
/// # Errors
/// Rejects tampered provenance, malformed shards and incomplete cache publication.
pub fn build(root: &Path, split: &str, shards: &[String]) -> Result<Receipt> {
    let root = root.canonicalize()?;
    let meta = files::read_json(&root.join("meta.json"))?;
    contracts::validate(&root, &meta)?;
    let fingerprint = fingerprint(&root, split, shards)?;
    let paths = paths(&root, split, shards)?;
    let cache = root.join(".trajectory-cache-v2");
    std::fs::create_dir_all(&cache)?;
    let stem = format!("{split}-{}", &fingerprint[..24]);
    let data_path = cache.join(format!("{stem}.bin"));
    let index_path = cache.join(format!("{stem}.idx"));
    let done = cache.join(format!("{stem}.json"));
    let lock = OpenOptions::new()
        .create(true)
        .truncate(false)
        .read(true)
        .write(true)
        .open(cache.join(format!("{stem}.lock")))?;
    lock.lock()?;
    let sources = source_digests(&root, &paths)?;
    if !sealed(&data_path, &index_path, &done, &fingerprint, &sources).unwrap_or(false) {
        build_files(&root, &paths, &data_path, &index_path, &done, &fingerprint)?;
    }
    ensure!(
        sealed(&data_path, &index_path, &done, &fingerprint, &sources)?,
        "Trajectory cache content seal failed after publication"
    );
    let size = index_path.metadata()?.len();
    ensure!(size % 32 == 0, "Corrupt trajectory index");
    let marker = files::read_json(&done)?;
    ensure!(
        marker["fingerprint"] == fingerprint && marker["records"].as_u64() == Some(size / 32),
        "Corrupt trajectory cache marker"
    );
    validate_index(&index_path, data_path.metadata()?.len())?;
    Ok(Receipt {
        fingerprint,
        data_path,
        index_path,
        records: size / 32,
    })
}
fn source_digests(root: &Path, paths: &[PathBuf]) -> Result<BTreeMap<String, String>> {
    let structure = root.join("card_structure.json");
    paths
        .iter()
        .chain(structure.exists().then_some(&structure))
        .map(|path| Ok((path.to_string_lossy().into_owned(), files::sha256(path)?)))
        .collect()
}
fn sealed(
    data: &Path,
    index: &Path,
    done: &Path,
    fingerprint: &str,
    sources: &BTreeMap<String, String>,
) -> Result<bool> {
    let marker = files::read_json(done)?;
    let size = index.metadata()?.len();
    if size % 32 != 0
        || marker["fingerprint"] != fingerprint
        || marker["records"].as_u64() != Some(size / 32)
        || marker["source_sha256"] != serde_json::to_value(sources)?
        || marker["data_sha256"].as_str() != Some(files::sha256(data)?.as_str())
        || marker["index_sha256"].as_str() != Some(files::sha256(index)?.as_str())
    {
        return Ok(false);
    }
    validate_index(index, data.metadata()?.len())?;
    Ok(true)
}
fn build_files(
    root: &Path,
    paths: &[PathBuf],
    data_path: &Path,
    index_path: &Path,
    done: &Path,
    fingerprint: &str,
) -> Result<()> {
    let id = uuid::Uuid::now_v7();
    let dt = data_path.with_extension(format!("bin.{id}.partial"));
    let it = index_path.with_extension(format!("idx.{id}.partial"));
    let result = (|| -> Result<()> {
        let sources = source_digests(root, paths)?;
        let mut data = OpenOptions::new().create_new(true).write(true).open(&dt)?;
        let mut index = OpenOptions::new().create_new(true).write(true).open(&it)?;
        let roots = files::read_json(&root.join("card_structure.json"))
            .ok()
            .and_then(|v| serde_json::from_value::<BTreeMap<String, u32>>(v["roots"].clone()).ok());
        let mut records = 0;
        for path in paths {
            records += append_shard(path, &mut data, &mut index, roots.as_ref())?;
        }
        data.sync_all()?;
        index.sync_all()?;
        ensure!(
            sources == source_digests(root, paths)?,
            "Trajectory source changed during cache construction"
        );
        let data_sha256 = files::sha256(&dt)?;
        let index_sha256 = files::sha256(&it)?;
        std::fs::rename(&dt, data_path)?;
        std::fs::rename(&it, index_path)?;
        files::atomic_json(
            done,
            &json!({"fingerprint":fingerprint,"records":records,"source_sha256":sources,"data_sha256":data_sha256,"index_sha256":index_sha256}),
        )?;
        Ok(())
    })();
    if result.is_err() {
        let _ = std::fs::remove_file(dt);
        let _ = std::fs::remove_file(it);
    }
    result
}
/// Build one cache part for native package recovery without publishing a dataset.
///
/// # Errors
/// Rejects malformed shards, invalid paths and failed part writes.
pub fn build_part(path: &Path, data_path: &Path, index_path: &Path) -> Result<u64> {
    let root = path
        .parent()
        .and_then(Path::parent)
        .context("shard must be inside dataset split")?;
    let roots = files::read_json(&root.join("card_structure.json"))
        .ok()
        .and_then(|v| serde_json::from_value::<BTreeMap<String, u32>>(v["roots"].clone()).ok());
    let mut data = File::create(data_path)?;
    let mut index = File::create(index_path)?;
    let count = append_shard(path, &mut data, &mut index, roots.as_ref())?;
    data.sync_all()?;
    index.sync_all()?;
    Ok(count)
}
fn append_shard(
    path: &Path,
    data: &mut File,
    index: &mut File,
    roots: Option<&BTreeMap<String, u32>>,
) -> Result<u64> {
    let mut input = File::open(path)?;
    let mut magic = [0; 8];
    input.read_exact(&mut magic)?;
    if &magic == SHARD_MAGIC {
        let (offset, records) = footer(&mut input)?;
        let base = data.stream_position()?;
        input.seek(SeekFrom::Start(8))?;
        ensure!(
            std::io::copy(&mut (&mut input).take(offset - 8), data)? == offset - 8,
            "Unexpected EOF in BS4S payload"
        );
        input.seek(SeekFrom::Start(offset))?;
        for _ in 0..records {
            let mut row = [0u8; 32];
            input.read_exact(&mut row)?;
            let old = u64::from_le_bytes(row[16..24].try_into()?);
            let size = u32::from_le_bytes(row[24..28].try_into()?);
            ensure!(
                old.checked_add(u64::from(size))
                    .is_some_and(|n| n <= offset - 8),
                "binary index outside shard payload"
            );
            row[16..24].copy_from_slice(
                &old.checked_add(base)
                    .context("cache offset overflow")?
                    .to_le_bytes(),
            );
            index.write_all(&row)?;
        }
        return Ok(records);
    }
    input.rewind()?;
    let mut records = 0;
    let mut lines = BufReader::new(MultiGzDecoder::new(input));
    let mut line = Vec::new();
    while lines.read_until(b'\n', &mut line)? > 0 {
        if line.iter().all(u8::is_ascii_whitespace) {
            line.clear();
            continue;
        }
        let step: Value = serde_json::from_slice(&line)?;
        let packed = roots.and_then(|roots| packing::pack_step(&step, roots).ok());
        let raw = packed.as_deref().unwrap_or(&line);
        let mut encoder = ZlibEncoder::new(Vec::new(), Compression::fast());
        encoder.write_all(raw)?;
        let blob = encoder.finish()?;
        let mut row = [0u8; 32];
        let game = u32::try_from(step["game_id"].as_u64().context("missing game_id")?)?;
        let seat = u32::try_from(step["seat"].as_u64().context("missing seat")?)?;
        let cursor = step["step_cursor"].as_u64().context("missing cursor")?;
        let flags = u32::from(
            step["is_decision"]
                .as_bool()
                .context("missing decision flag")?,
        ) | (u32::from(step["is_terminal"].as_bool().unwrap_or(false)) << 1);
        row[0..4].copy_from_slice(&game.to_le_bytes());
        row[4..8].copy_from_slice(&seat.to_le_bytes());
        row[8..16].copy_from_slice(&cursor.to_le_bytes());
        row[16..24].copy_from_slice(&data.stream_position()?.to_le_bytes());
        row[24..28].copy_from_slice(&u32::try_from(blob.len())?.to_le_bytes());
        row[28..32].copy_from_slice(&flags.to_le_bytes());
        index.write_all(&row)?;
        data.write_all(&blob)?;
        records += 1;
        line.clear();
    }
    Ok(records)
}
fn validate_index(path: &Path, data_len: u64) -> Result<()> {
    let mut index = File::open(path)?;
    for _ in 0..index.metadata()?.len() / 32 {
        let mut row = [0; 32];
        index.read_exact(&mut row)?;
        let offset = u64::from_le_bytes(row[16..24].try_into()?);
        let length = u32::from_le_bytes(row[24..28].try_into()?);
        ensure!(
            offset
                .checked_add(u64::from(length))
                .is_some_and(|end| end <= data_len),
            "Corrupt trajectory cache index bounds"
        );
    }
    Ok(())
}
