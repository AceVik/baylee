//! Decode and prepare pinned stereo recordings without changing their sound.
#![allow(clippy::cast_precision_loss)]
use super::Sample;
use crate::art::{memorial::soxr::Resampler, samples::sha256, write_file};
use anyhow::{Context, Result, ensure};
use serde_json::Value;
use std::{
    fmt::Write as _,
    io::Read as _,
    path::{Component, Path},
    process::Command,
};

fn source(raw: &Path, row: &Sample, download: bool) -> Result<std::path::PathBuf> {
    let pin = match row.repo.as_str() {
        "sfzinstruments/Osiris_Piano" => "18c6afccb60cff458edbf7c394571783e074e1e9",
        "sgossner/VSCO-2-CE" => "440300901dfe9275fd84e0b7763af1f8443ae62e",
        _ => anyhow::bail!("unrecognised source repository"),
    };
    ensure!(
        row.revision == pin
            && Path::new(&row.path)
                .components()
                .all(|c| matches!(c, Component::Normal(_))),
        "invalid source pin/path"
    );
    let path = raw.join(&row.repo).join(pin).join(&row.path);
    if !path.exists() {
        ensure!(download, "missing {}; use --download", path.display());
        let mut encoded = String::new();
        for byte in row.path.bytes() {
            if byte.is_ascii_alphanumeric() || b"-._~/".contains(&byte) {
                encoded.push(char::from(byte));
            } else {
                write!(encoded, "%{byte:02X}")?;
            }
        }
        let url = format!(
            "https://raw.githubusercontent.com/{}/{pin}/{encoded}",
            row.repo
        );
        let agent: ureq::Agent = ureq::Agent::config_builder()
            .timeout_global(Some(std::time::Duration::from_secs(120)))
            .build()
            .into();
        let mut bytes = Vec::new();
        agent
            .get(&url)
            .call()?
            .body_mut()
            .as_reader()
            .take(128 * 1024 * 1024 + 1)
            .read_to_end(&mut bytes)?;
        ensure!(
            sha256(&bytes) == row.source_sha256,
            "download hash mismatch: {}",
            row.path
        );
        write_file(&path, &bytes)?;
    }
    ensure!(
        sha256(&std::fs::read(&path)?) == row.source_sha256,
        "cached source hash mismatch: {}",
        row.path
    );
    Ok(path)
}

fn decode(path: &Path, row: &Sample) -> Result<Vec<[f64; 2]>> {
    let info = Command::new("ffprobe")
        .args([
            "-v",
            "error",
            "-show_entries",
            "stream=channels,sample_rate",
            "-of",
            "json",
        ])
        .arg(path)
        .output()
        .context("run ffprobe")?;
    ensure!(
        info.status.success(),
        "ffprobe failed: {}",
        String::from_utf8_lossy(&info.stderr)
    );
    let info: Value = serde_json::from_slice(&info.stdout)?;
    let streams = info["streams"].as_array().context("audio streams")?;
    ensure!(
        streams.len() == 1
            && streams[0]["channels"] == 2
            && streams[0]["sample_rate"].as_str() == Some(&row.source_rate.to_string()),
        "source format changed"
    );
    let decoded = Command::new("ffmpeg")
        .args(["-v", "error", "-i"])
        .arg(path)
        .args(["-f", "f64le", "-"])
        .output()
        .context("run ffmpeg")?;
    ensure!(
        decoded.status.success(),
        "ffmpeg failed: {}",
        String::from_utf8_lossy(&decoded.stderr)
    );
    ensure!(
        decoded.stdout.len() == row.source_frames * 16,
        "source frame count changed"
    );
    decoded
        .stdout
        .as_chunks::<16>()
        .0
        .iter()
        .map(|bytes| {
            Ok([
                f64::from_le_bytes(bytes[..8].try_into()?),
                f64::from_le_bytes(bytes[8..].try_into()?),
            ])
        })
        .collect()
}

fn pcm(mut audio: Vec<[f32; 2]>, frames: usize) -> Result<(Vec<u8>, usize)> {
    ensure!(
        frames >= 192 && audio.len() >= 192,
        "audio too short for fades"
    );
    audio.truncate(frames);
    let n = audio.len();
    let fade = 3840.min(n / 4);
    for (i, frame) in audio[n - fade..].iter_mut().enumerate() {
        let weight = 1.0 - i as f64 * (1.0 / (fade - 1) as f64);
        for value in frame {
            *value = (f64::from(*value) * weight) as f32;
        }
    }
    for (i, frame) in audio[..48].iter_mut().enumerate() {
        let weight = i as f64 * (1.0 / 47.0);
        for value in frame {
            *value = (f64::from(*value) * weight) as f32;
        }
    }
    // Endpoints are exactly 0/1, as in the original linear envelope.
    audio[n - 1] = [0.0; 2];
    audio.resize(frames, [0.0; 2]);
    let mut pcm = Vec::with_capacity(frames * 4);
    for value in audio.into_iter().flatten() {
        ensure!(
            value.is_finite() && value.abs() < 1.0,
            "non-finite/clipped PCM"
        );
        pcm.extend(((value * 32767.0).round_ties_even() as i16).to_le_bytes());
    }
    Ok((pcm, n))
}

pub(super) fn prepare(
    raw: &Path,
    row: &Sample,
    download: bool,
    soxr: &Resampler,
) -> Result<Vec<u8>> {
    ensure!(
        row.extended_bows == 0 && row.frames <= 48_000 * 120,
        "unsupported recording extension/length"
    );
    let path = source(raw, row, download)?;
    let mut decoded = decode(&path, row)?;
    ensure!(
        !decoded.is_empty() && decoded.iter().flatten().all(|v| v.is_finite()),
        "empty/non-finite source"
    );
    let mut sum = [0.0; 2];
    for frame in &decoded {
        for channel in 0..2 {
            sum[channel] += frame[channel];
        }
    }
    let mean = sum.map(|v| v / decoded.len() as f64);
    for frame in &mut decoded {
        for channel in 0..2 {
            frame[channel] -= mean[channel];
        }
    }
    let peak = decoded
        .iter()
        .flatten()
        .copied()
        .map(f64::abs)
        .fold(0.0, f64::max);
    let threshold = 0.00008_f64.max(peak * 0.012);
    let start = decoded
        .iter()
        .position(|x| x[0].abs().max(x[1].abs()) > threshold)
        .context("silent source")?
        .saturating_sub((f64::from(row.source_rate) * 0.003).round_ties_even() as usize);
    ensure!(start == row.trim_start, "onset differs: {}", row.path);
    let ratio =
        2.0_f64.powf((f64::from(row.pitch) - f64::from(row.root) - row.cents / 100.0) / 12.0);
    let audio = soxr.stereo(&decoded[start..], f64::from(row.source_rate) * ratio)?;
    let (pcm, natural) = pcm(audio, row.frames)?;
    ensure!(
        natural == row.natural_frames && sha256(&pcm) == row.pcm_sha256,
        "prepared PCM differs: {} (pitch {}, layer {})",
        row.path,
        row.pitch,
        row.layer
    );
    Ok(pcm)
}

#[cfg(test)]
mod tests {
    #[test]
    fn silent_padding_stereo_fades_and_invalid_pcm() {
        let (bytes, natural) = super::pcm(vec![[0.5, -0.25]; 400], 500).unwrap();
        assert_eq!(natural, 400);
        assert_eq!(bytes.len(), 2000);
        assert!(bytes[1600..].iter().all(|b| *b == 0));
        assert_eq!(&bytes[..4], &[0; 4]);
        assert!(super::pcm(vec![[f32::NAN, 0.0]; 400], 400).is_err());
        assert!(super::pcm(vec![[2.0, 0.0]; 400], 400).is_err());
        assert!(super::pcm(vec![[0.0; 2]; 2], 400).is_err());
    }
}
