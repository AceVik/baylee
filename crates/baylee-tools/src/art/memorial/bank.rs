//! Reproduce the approved source selection, with hashes at every boundary.
mod audio;

use super::{BANK, HERE, RATE, soxr::Resampler};
use crate::art::{samples::sha256, write_file};
use anyhow::{Context, Result, ensure};
use serde::Deserialize;
use serde_json::Value;
use std::{
    collections::BTreeMap,
    io::{Read as _, Write as _},
    path::Path,
};

#[derive(Deserialize)]
struct Manifest {
    records: usize,
    decoded_bytes: usize,
    packed_bytes: usize,
    sha256: String,
    chunks: BTreeMap<String, String>,
    samples: Vec<Sample>,
}

#[derive(Deserialize)]
struct Sample {
    instrument: u8,
    pitch: u8,
    layer: u8,
    root: u8,
    path: String,
    repo: String,
    revision: String,
    cents: f64,
    source_sha256: String,
    source_rate: u32,
    source_frames: usize,
    trim_start: usize,
    frames: usize,
    natural_frames: usize,
    extended_bows: usize,
    pcm_sha256: String,
}

fn header(bytes: &[u8], magic: &[u8], count: usize) -> Result<()> {
    ensure!(
        bytes.len() >= 12 && &bytes[..4] == magic,
        "invalid memorial header"
    );
    ensure!(
        u32::from_le_bytes(bytes[4..8].try_into()?) == RATE,
        "invalid sample rate"
    );
    ensure!(
        u32::from_le_bytes(bytes[8..12].try_into()?) as usize == count,
        "invalid sample count"
    );
    Ok(())
}

fn verify(root: &Path, manifest: &Manifest) -> Result<()> {
    ensure!(
        manifest.records == manifest.samples.len() && manifest.records <= 512,
        "invalid bank count"
    );
    ensure!(
        manifest.decoded_bytes <= 256 * 1024 * 1024 && manifest.packed_bytes <= 256 * 1024 * 1024,
        "oversized bank"
    );
    ensure!(
        manifest.samples.iter().all(|row| row.source_frames > 0
            && row.source_frames <= 192_000 * 120
            && row.frames >= 192
            && row.frames <= 48_000 * 120
            && row.natural_frames <= row.frames
            && row.cents.is_finite()
            && row.pitch <= 127
            && row.root <= 127),
        "invalid recording bounds"
    );
    ensure!(manifest.chunks.len() == 4, "expected four bank chunks");
    let mut packed = Vec::new();
    for i in 1..=4 {
        let name = format!("stereo-{i:02}.pcm.zlib");
        let data = std::fs::read(root.join(BANK).join(&name))?;
        ensure!(
            manifest.chunks.get(&name) == Some(&sha256(&data)),
            "chunk hash mismatch: {name}"
        );
        packed.extend(data);
    }
    ensure!(
        packed.len() == manifest.packed_bytes && sha256(&packed) == manifest.sha256,
        "packed bank mismatch"
    );
    let mut bytes = Vec::new();
    flate2::read::ZlibDecoder::new(packed.as_slice())
        .take(manifest.decoded_bytes as u64 + 1)
        .read_to_end(&mut bytes)?;
    ensure!(
        bytes.len() == manifest.decoded_bytes,
        "decoded bank length mismatch"
    );
    header(&bytes, b"CHM1", manifest.records)?;
    let mut remaining = &bytes[12..];
    for sample in &manifest.samples {
        let size = sample.frames.checked_mul(4).context("frame overflow")?;
        let record = remaining.get(..8).context("truncated sample header")?;
        ensure!(
            record[..4] == [sample.instrument, sample.pitch, sample.layer, 0],
            "sample identity mismatch"
        );
        ensure!(
            u32::from_le_bytes(record[4..8].try_into()?) as usize == sample.frames,
            "sample frame mismatch"
        );
        let pcm = remaining.get(8..8 + size).context("truncated PCM")?;
        ensure!(
            sha256(pcm) == sample.pcm_sha256,
            "sample PCM mismatch: {}",
            sample.path
        );
        remaining = &remaining[8 + size..];
    }
    ensure!(remaining.is_empty(), "trailing bank data");
    Ok(())
}

pub(super) fn run(
    root: &Path,
    raw: Option<&Path>,
    library: Option<&Path>,
    download: bool,
    output: Option<&Path>,
) -> Result<()> {
    let mut json: Value =
        serde_json::from_slice(&std::fs::read(root.join(HERE).join("instruments.json"))?)?;
    let manifest: Manifest = serde_json::from_value(json.clone())?;
    verify(root, &manifest)?;
    let Some(raw) = raw else {
        println!(
            "memorial bank: all {} stereo PCM records and four chunks verified",
            manifest.records
        );
        return Ok(());
    };
    if let Some(output) = output {
        ensure!(!output.exists(), "candidate output must not already exist");
    }
    let soxr = Resampler::open(library.context("--raw requires --soxr PATH to native libsoxr")?)?;
    let mut bytes = b"CHM1".to_vec();
    bytes.extend(RATE.to_le_bytes());
    bytes.extend(u32::try_from(manifest.records)?.to_le_bytes());
    for (i, row) in manifest.samples.iter().enumerate() {
        let pcm = audio::prepare(raw, row, download, &soxr)?;
        bytes.extend([row.instrument, row.pitch, row.layer, 0]);
        bytes.extend(u32::try_from(row.frames)?.to_le_bytes());
        bytes.extend(pcm);
        eprintln!("verified stereo recording {}/{}", i + 1, manifest.records);
    }
    ensure!(
        bytes.len() == manifest.decoded_bytes,
        "rebuilt bank size differs"
    );
    if let Some(output) = output {
        let mut encoder = flate2::write::ZlibEncoder::new(Vec::new(), flate2::Compression::best());
        encoder.write_all(&bytes)?;
        let packed = encoder.finish()?;
        let size = packed.len().div_ceil(4);
        let mut chunks = BTreeMap::new();
        std::fs::create_dir_all(output)?;
        for (i, chunk) in packed.chunks(size).enumerate() {
            let name = format!("stereo-{:02}.pcm.zlib", i + 1);
            write_file(&output.join(&name), chunk)?;
            chunks.insert(name, sha256(chunk));
        }
        json["packed_bytes"] = packed.len().into();
        json["sha256"] = sha256(&packed).into();
        json["chunks"] = serde_json::to_value(chunks)?;
        write_file(
            &output.join("instruments.json"),
            &serde_json::to_vec_pretty(&json)?,
        )?;
    }
    println!(
        "memorial bank: all {} reconstructed stereo recordings match the approved PCM",
        manifest.records
    );
    Ok(())
}
