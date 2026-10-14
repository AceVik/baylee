//! Source pins, manifest rows, SFZ tuning and the native ffmpeg decode boundary.

use std::{
    collections::BTreeMap,
    fmt::Write as _,
    io::Read as _,
    path::{Component, Path, PathBuf},
    process::Command,
};

use anyhow::{Context, Result, bail, ensure};
use regex::Regex;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

pub(super) const SFZ: &str = "Bagpipe 20260806.sfz";
pub(super) const SFZ_SHA256: &str =
    "e83006bad23f09d3294393fe66f9942cc91392e1fdea6daaf9721b2d723b152f";

pub(super) fn revision(source: &str) -> Result<&'static str> {
    match source {
        "VCSL" => Ok("c1ea7bcc3c7309650ab0da9d15c9cd1fbc4a4c7e"),
        "VSCO 2 CE" => Ok("440300901dfe9275fd84e0b7763af1f8443ae62e"),
        "FreePats Bagpipe" => Ok("496f2f6e82f226d650e270c0f0ad5febbebec249"),
        _ => bail!("unknown sample source {source}"),
    }
}

pub(super) fn sha256(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let digest = Sha256::digest(bytes);
    let mut hex = String::with_capacity(digest.len() * 2);
    for byte in digest {
        hex.push(char::from(HEX[usize::from(byte >> 4)]));
        hex.push(char::from(HEX[usize::from(byte & 15)]));
    }
    hex
}

#[derive(Clone, Debug)]
pub(super) struct Spec {
    pub family: String,
    pub name: String,
    pub source: &'static str,
    pub path: String,
    pub kind: &'static str,
    pub note: Option<String>,
    pub octave: i16,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub(super) struct Row {
    pub family: String,
    pub name: String,
    pub source: String,
    pub path: String,
    pub kind: String,
    pub revision: String,
    pub sha256: String,
    pub bytes: usize,
    pub channels: usize,
    pub stereo_corr: Option<f64>,
    pub mono: String,
    pub midi: i16,
    pub measured_cents: Option<f64>,
    pub cents: f64,
    pub pitch_basis: String,
    #[serde(rename = "loop")]
    pub looped: Option<[usize; 2]>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sfz_loop: Option<[usize; 2]>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub loop_match: Option<f64>,
    pub attack: f64,
    pub release: f64,
    pub room_send: f64,
    pub frames: usize,
    pub rate: usize,
    pub pcm_sha256: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct Region {
    pub tune: i16,
    pub looped: [usize; 2],
}

pub(super) fn sfz_regions(text: &str) -> Result<BTreeMap<String, Region>> {
    let pattern = Regex::new(
        r"(?:tune=(-?\d+)\s+)?loop_start=(\d+) loop_end=(\d+)\s+sample=samples/([^\s]+)\.flac",
    )?;
    let mut out = BTreeMap::new();
    for m in pattern.captures_iter(text) {
        let region = Region {
            tune: m.get(1).map_or(Ok(0), |value| value.as_str().parse())?,
            looped: [m[2].parse()?, m[3].parse()?],
        };
        ensure!(
            region.looped[0] < region.looped[1],
            "invalid SFZ loop for {}",
            &m[4]
        );
        ensure!(
            out.insert(m[4].to_owned(), region).is_none(),
            "duplicate SFZ region {}",
            &m[4]
        );
    }
    ensure!(!out.is_empty(), "no sample regions found in pinned SFZ");
    Ok(out)
}

pub(super) fn midi_of(note: &str) -> Result<i16> {
    let pattern = Regex::new(r"^([A-G])(#?)(-?\d)$")?;
    let m = pattern
        .captures(note)
        .with_context(|| format!("invalid note {note}"))?;
    let semitone = match &m[1] {
        "C" => 0,
        "D" => 2,
        "E" => 4,
        "F" => 5,
        "G" => 7,
        "A" => 9,
        "B" => 11,
        _ => unreachable!(),
    };
    Ok(12 * (m[3].parse::<i16>()? + 1) + semitone + i16::from(&m[2] == "#"))
}

fn source_url(source: &str, path: &str) -> Result<String> {
    let repo = match source {
        "VCSL" => "sgossner/VCSL",
        "VSCO 2 CE" => "sgossner/VSCO-2-CE",
        "FreePats Bagpipe" => "freepats/bagpipe",
        _ => bail!("unknown source {source}"),
    };
    let mut escaped = String::new();
    for byte in path.bytes() {
        if byte.is_ascii_alphanumeric() || b"-._~/".contains(&byte) {
            escaped.push(char::from(byte));
        } else {
            write!(escaped, "%{byte:02X}")?;
        }
    }
    Ok(format!(
        "https://raw.githubusercontent.com/{repo}/{}/{escaped}",
        revision(source)?
    ))
}

pub(super) fn fetch(
    raw: &Path,
    source: &str,
    path: &str,
    pin: Option<&str>,
    download: bool,
) -> Result<(PathBuf, String, usize)> {
    ensure!(
        Path::new(path)
            .components()
            .all(|part| matches!(part, Component::Normal(_))),
        "invalid source path {path}"
    );
    let local = raw
        .join(source.replace(' ', "_"))
        .join(revision(source)?)
        .join(path);
    if !local.exists() {
        ensure!(
            download,
            "missing cached source {}; use --download to fetch pinned sources",
            local.display()
        );
        let url = source_url(source, path)?;
        let agent: ureq::Agent = ureq::Agent::config_builder()
            .timeout_global(Some(std::time::Duration::from_secs(120)))
            .build()
            .into();
        let mut response = agent
            .get(&url)
            .call()
            .with_context(|| format!("download {url}"))?;
        let mut bytes = Vec::new();
        response.body_mut().as_reader().read_to_end(&mut bytes)?;
        verify_pin(&bytes, pin, source, path)?;
        let parent = local.parent().context("cache path has no parent")?;
        std::fs::create_dir_all(parent)?;
        super::write_file(&local, &bytes)?;
    }
    let bytes = std::fs::read(&local)?;
    let sha = verify_pin(&bytes, pin, source, path)?;
    Ok((local, sha, bytes.len()))
}

fn verify_pin(bytes: &[u8], pin: Option<&str>, source: &str, path: &str) -> Result<String> {
    let sha = sha256(bytes);
    if let Some(pin) = pin {
        ensure!(sha == pin, "{source} {path}: sha256 {sha}, pinned {pin}");
    }
    Ok(sha)
}

pub(super) fn decode(path: &Path) -> Result<(Vec<f64>, usize)> {
    let info = Command::new("ffprobe")
        .args([
            "-v",
            "error",
            "-show_entries",
            "stream=channels",
            "-of",
            "csv=p=0",
        ])
        .arg(path)
        .output()
        .context("run ffprobe (required for source decoding)")?;
    ensure!(
        info.status.success(),
        "ffprobe {}: {}",
        path.display(),
        String::from_utf8_lossy(&info.stderr)
    );
    let channels = String::from_utf8(info.stdout)?
        .trim()
        .parse::<usize>()
        .context("ffprobe channel count")?;
    let output = Command::new("ffmpeg")
        .args(["-v", "error", "-i"])
        .arg(path)
        .args(["-ar", "44100", "-f", "f32le", "-"])
        .output()
        .context("run ffmpeg (required for source decoding)")?;
    ensure!(
        output.status.success(),
        "ffmpeg {}: {}",
        path.display(),
        String::from_utf8_lossy(&output.stderr)
    );
    ensure!(
        channels > 0 && output.stdout.len().is_multiple_of(channels * 4),
        "invalid decoded frames"
    );
    let floats = output
        .stdout
        .as_chunks::<4>()
        .0
        .iter()
        .map(|b| f64::from(f32::from_le_bytes([b[0], b[1], b[2], b[3]])))
        .collect();
    Ok((floats, channels))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sha256_is_lowercase_hex_for_the_standard_reference_vector() {
        assert_eq!(
            sha256(b"abc"),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
    }

    #[test]
    fn pins_are_checked_even_in_offline_cache() {
        let raw = tempfile::tempdir().unwrap();
        let dir = raw.path().join("VCSL").join(revision("VCSL").unwrap());
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("test.wav"), b"changed").unwrap();
        assert!(
            fetch(
                raw.path(),
                "VCSL",
                "test.wav",
                Some(&sha256(b"original")),
                false
            )
            .is_err()
        );
        assert!(fetch(raw.path(), "VCSL", "../escape", None, false).is_err());
        assert!(fetch(raw.path(), "VCSL", "absent.wav", None, false).is_err());
    }

    #[test]
    fn source_url_pins_revision_and_escapes_names() {
        assert_eq!(
            source_url("VCSL", "8'/C#4 a.wav").unwrap(),
            "https://raw.githubusercontent.com/sgossner/VCSL/c1ea7bcc3c7309650ab0da9d15c9cd1fbc4a4c7e/8%27/C%234%20a.wav"
        );
        assert!(revision("unreviewed source").is_err());
    }

    #[test]
    fn sfz_tune_and_loop_are_kept_with_each_recording() {
        let regions = sfz_regions("tune=65 loop_start=11025 loop_end=90000 sample=samples/E5_31.flac\nloop_start=100 loop_end=4000 sample=samples/drone_G2_1.flac").unwrap();
        assert_eq!(
            regions["E5_31"],
            Region {
                tune: 65,
                looped: [11025, 90000]
            }
        );
        assert_eq!(regions["drone_G2_1"].tune, 0);
        assert_eq!(midi_of("C4").unwrap(), 60);
        assert_eq!(midi_of("A#0").unwrap(), 22);
        assert_eq!(midi_of("C-1").unwrap(), 0);
        assert!(midi_of("H4").is_err());
    }
}
