use super::packing;
use anyhow::{Context, Result, ensure};
use flate2::read::{MultiGzDecoder, ZlibDecoder};
use serde_json::Value;
use std::{
    fs::File,
    io::{BufRead, BufReader, Read, Seek, SeekFrom},
    path::Path,
};
pub(super) const SHARD_MAGIC: &[u8; 8] = b"BS4S\x01\x00\x00\x00";
/// Read every complete JSON-gzip or BS4S/BST4 frame for native admission.
/// BST4 frames expose exact candidate/pair cardinalities as null arrays, with
/// decoded provenance, target masks and history metadata; tensor payloads stay private.
///
/// # Errors
/// Rejects malformed compression, indexes, binary frame dimensions or JSON records.
pub fn read_shard(path: &Path, mut observe: impl FnMut(Value) -> Result<()>) -> Result<u64> {
    let mut file = File::open(path)?;
    let mut magic = [0u8; 8];
    file.read_exact(&mut magic)?;
    if &magic != SHARD_MAGIC {
        file.rewind()?;
        let stream = BufReader::new(MultiGzDecoder::new(file));
        let mut count = 0;
        for line in stream.lines() {
            let line = line?;
            if line.trim().is_empty() {
                continue;
            }
            let value: Value = serde_json::from_str(&line)?;
            ensure!(
                value.get("global_feat_width").is_none(),
                "JSON frame cannot supply packed dimension metadata"
            );
            observe(value)?;
            count += 1;
        }
        return Ok(count);
    }
    let (index, records) = footer(&mut file)?;
    let data_length = index - 8;
    for row in 0..records {
        file.seek(SeekFrom::Start(index + row * 32))?;
        let mut bytes = [0u8; 32];
        file.read_exact(&mut bytes)?;
        let offset = u64::from_le_bytes(bytes[16..24].try_into()?);
        let length = u32::from_le_bytes(bytes[24..28].try_into()?);
        ensure!(
            offset
                .checked_add(u64::from(length))
                .is_some_and(|end| end <= data_length),
            "binary shard index outside payload"
        );
        file.seek(SeekFrom::Start(8 + offset))?;
        let mut raw = Vec::new();
        ZlibDecoder::new((&mut file).take(u64::from(length))).read_to_end(&mut raw)?;
        let value = if raw.starts_with(b"BST4") {
            packing::metadata(&raw)?
        } else {
            let value: Value = serde_json::from_slice(&raw)?;
            ensure!(
                value.get("global_feat_width").is_none(),
                "JSON frame cannot supply packed dimension metadata"
            );
            value
        };
        ensure!(
            value["game_id"].as_u64()
                == Some(u64::from(u32::from_le_bytes(bytes[0..4].try_into()?)))
                && value["seat"].as_u64()
                    == Some(u64::from(u32::from_le_bytes(bytes[4..8].try_into()?)))
                && value["step_cursor"].as_u64()
                    == Some(u64::from_le_bytes(bytes[8..16].try_into()?)),
            "binary index/frame identity mismatch"
        );
        let flags = u32::from(
            value["is_decision"]
                .as_bool()
                .context("missing decision flag")?,
        ) | (u32::from(value["is_terminal"].as_bool().unwrap_or(false)) << 1);
        ensure!(
            flags == u32::from_le_bytes(bytes[28..32].try_into()?),
            "binary index/frame flags mismatch"
        );
        observe(value)?;
    }
    Ok(records)
}
pub(super) fn footer(file: &mut File) -> Result<(u64, u64)> {
    let total = file.metadata()?.len();
    ensure!(total >= 32, "Truncated BS4S binary shard");
    file.seek(SeekFrom::End(-24))?;
    let mut footer = [0u8; 24];
    file.read_exact(&mut footer)?;
    let index = u64::from_le_bytes(footer[..8].try_into()?);
    let records = u64::from_le_bytes(footer[8..16].try_into()?);
    ensure!(
        &footer[16..] == b"BS4S_END"
            && index >= 8
            && records
                .checked_mul(32)
                .and_then(|size| index.checked_add(size))
                .and_then(|n| n.checked_add(24))
                == Some(total),
        "Corrupt BS4S binary shard footer"
    );
    Ok((index, records))
}
