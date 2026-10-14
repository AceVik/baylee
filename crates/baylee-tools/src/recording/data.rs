//! Strict, bounded local inputs and valid-prefix journal recovery.
use anyhow::{Context, Result, bail, ensure};
use image::{DynamicImage, ImageDecoder, ImageReader, RgbImage};
use serde::Deserialize;
use serde_json::{Value, json};
use std::fs::File;
use std::io::{BufReader, Read, Write};
use std::path::{Path, PathBuf};

pub(super) const MAX_PIXELS: u64 = 64 * 1024 * 1024;
pub(super) const MAX_FRAME_PIXELS: u64 = 8 * 1024 * 1024;
pub(super) const MAX_FRAMES: usize = 900;
const MAX_PNG_BYTES: u64 = 128 * 1024 * 1024;

#[derive(Clone, Debug, Deserialize)]
pub(super) struct Sample {
    pub slot: u32,
    pub file: String,
    pub nominal_us: u64,
    pub requested_us: u64,
    pub virtual_us: u64,
    pub received_us: u64,
    pub width: u32,
    pub height: u32,
}

pub(super) struct Recording {
    pub directory: PathBuf,
    pub summary: Value,
    pub frames: Vec<Sample>,
}

pub(super) fn terminal(phase: &str) -> bool {
    matches!(phase, "complete" | "cancelled" | "failed")
}

pub(super) fn bounded(path: &Path, limit: u64) -> Result<Vec<u8>> {
    let file = File::open(path).with_context(|| format!("open {}", path.display()))?;
    ensure!(
        file.metadata()?.is_file(),
        "not a regular file: {}",
        path.display()
    );
    ensure!(
        file.metadata()?.len() <= limit,
        "file exceeds {limit} bytes: {}",
        path.display()
    );
    let mut bytes = Vec::new();
    file.take(limit + 1).read_to_end(&mut bytes)?;
    ensure!(
        bytes.len() as u64 <= limit,
        "file grew beyond {limit} bytes"
    );
    Ok(bytes)
}

fn confined(directory: &Path, name: &str) -> Result<PathBuf> {
    let path = directory.join(name).canonicalize()?;
    ensure!(
        path.parent() == Some(directory),
        "input must stay inside recording directory: {name}"
    );
    Ok(path)
}

pub(super) fn load(directory: &Path, allow_incomplete: bool) -> Result<Recording> {
    let directory = directory.canonicalize()?;
    ensure!(directory.is_dir(), "recording path is not a directory");
    let mut summary: Value = serde_json::from_slice(&bounded(
        &confined(&directory, "recording.json")?,
        1024 * 1024,
    )?)?;
    ensure!(
        summary["schema"].as_u64() == Some(1),
        "unsupported recording schema"
    );
    ensure!(
        allow_incomplete || terminal(summary["phase"].as_str().unwrap_or("")),
        "recording has not drained; poll status or use --allow-incomplete to recover"
    );
    let data = bounded(&confined(&directory, "frames.jsonl")?, 2 * 1024 * 1024)?;
    let mut frames = Vec::new();
    let mut boundary = 0;
    for line in data.split_inclusive(|byte| *byte == b'\n') {
        let parsed = line
            .ends_with(b"\n")
            .then(|| serde_json::from_slice::<Value>(line).ok())
            .flatten();
        let Some(value) = parsed else {
            ensure!(
                allow_incomplete,
                "damaged frame ledger; use --allow-incomplete for its valid prefix"
            );
            summary["ledger_recovery"] = json!({
                "valid_rows": frames.len(), "ignored_bytes": data.len() - boundary,
                "note": "Stopped at the first damaged/unterminated row; later bytes were not interpreted."
            });
            break;
        };
        boundary += line.len();
        let sample: Sample = serde_json::from_value(value).context(
            "invalid frame fields (nonnegative integer timestamps and dimensions required)",
        )?;
        ensure!(
            (sample.slot as usize) < MAX_FRAMES
                && sample.file == format!("frame-{:06}.png", sample.slot),
            "invalid frame filename/slot"
        );
        ensure!(
            sample.width > 0
                && sample.height > 0
                && u64::from(sample.width) * u64::from(sample.height) <= MAX_FRAME_PIXELS,
            "raw frame exceeds recorder's pixel budget"
        );
        confined(&directory, &sample.file)?;
        frames.push(sample);
        ensure!(frames.len() <= MAX_FRAMES, "too many frames");
    }
    frames.sort_by_key(|sample| sample.slot);
    ensure!(!frames.is_empty(), "recording contains no saved frames");
    ensure!(
        frames.windows(2).all(|pair| pair[0].slot != pair[1].slot),
        "duplicate frame slots"
    );
    Ok(Recording {
        directory,
        summary,
        frames,
    })
}

pub(super) fn read_frame(recording: &Recording, sample: &Sample) -> Result<RgbImage> {
    let path = confined(&recording.directory, &sample.file)?;
    let file = File::open(path)?;
    ensure!(
        file.metadata()?.is_file() && file.metadata()?.len() <= MAX_PNG_BYTES,
        "raw PNG exceeds 128 MiB or is not a regular file"
    );
    let mut reader = ImageReader::with_format(BufReader::new(file), image::ImageFormat::Png);
    let mut limits = image::Limits::default();
    limits.max_alloc = Some(128 * 1024 * 1024);
    limits.max_image_width = Some(MAX_FRAME_PIXELS as u32);
    limits.max_image_height = Some(MAX_FRAME_PIXELS as u32);
    reader.limits(limits);
    let decoder = reader.into_decoder()?;
    let (width, height) = decoder.dimensions();
    ensure!(
        width > 0 && height > 0 && u64::from(width) * u64::from(height) <= MAX_FRAME_PIXELS,
        "raw frame exceeds recorder's pixel budget"
    );
    let first = &recording.frames[0];
    ensure!(
        (width, height) == (sample.width, sample.height)
            && (width, height) == (first.width, first.height),
        "frame dimensions changed or disagree with ledger"
    );
    Ok(DynamicImage::from_decoder(decoder)?.into_rgb8())
}

/// Atomic destination replacement also avoids following an existing output symlink.
pub(super) fn output(
    directory: &Path,
    name: &str,
    write: impl FnOnce(&mut File) -> Result<()>,
) -> Result<PathBuf> {
    if Path::new(name).components().count() != 1 {
        bail!("output must be a simple filename");
    }
    let mut temp = tempfile::NamedTempFile::new_in(directory)?;
    write(temp.as_file_mut())?;
    temp.as_file_mut().flush()?;
    let destination = directory.join(name);
    temp.persist(&destination)?;
    Ok(destination)
}
