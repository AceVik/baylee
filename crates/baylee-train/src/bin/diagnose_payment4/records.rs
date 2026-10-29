//! Bounded indexed gzip-member reads, retaining source content identities.
use std::fmt::Write as _;
use std::fs::File;
use std::io::{BufRead as _, BufReader, Read as _, Seek as _, SeekFrom};
use std::path::{Path, PathBuf};

use anyhow::{Context as _, ensure};
use baylee_gamehost::record::Line;
use flate2::read::GzDecoder;
use serde::Deserialize;
use serde_json::Value;
use sha2::{Digest as _, Sha256};

#[derive(Deserialize)]
pub(super) struct Entry {
    pub i: u64,
    pub game: String,
    pub shard: String,
    pub offset: u64,
    pub len: u64,
    pub rollout_sidecar: Vec<Value>,
}

pub(super) fn directories(root: &Path) -> anyhow::Result<Vec<PathBuf>> {
    if root.join("games.jsonl").is_file() {
        return Ok(vec![root.to_path_buf()]);
    }
    let mut directories = Vec::new();
    for child in std::fs::read_dir(root)? {
        let path = child?.path();
        if path.is_dir() && path.join("games.jsonl").is_file() {
            directories.push(path);
        }
    }
    directories.sort();
    ensure!(!directories.is_empty(), "no indexed record directories");
    Ok(directories)
}

pub(super) fn entries(directory: &Path) -> anyhow::Result<(String, Vec<Entry>)> {
    let bytes = std::fs::read(directory.join("games.jsonl"))?;
    let entries = BufReader::new(bytes.as_slice())
        .lines()
        .map(|line| Ok(serde_json::from_str(&line?)?))
        .collect::<anyhow::Result<Vec<Entry>>>()?;
    Ok((digest(&bytes), entries))
}

pub(super) fn record(directory: &Path, entry: &Entry) -> anyhow::Result<(String, Vec<Line>)> {
    ensure!(
        Path::new(&entry.shard).components().count() == 1,
        "shard must be a filename"
    );
    let path = directory.join("records").join(&entry.shard);
    let mut file = File::open(&path).with_context(|| path.display().to_string())?;
    let length = file.metadata()?.len();
    ensure!(
        entry.len <= 128 * 1024 * 1024,
        "compressed record exceeds 128 MiB"
    );
    ensure!(
        entry
            .offset
            .checked_add(entry.len)
            .is_some_and(|end| end <= length),
        "record outside shard"
    );
    file.seek(SeekFrom::Start(entry.offset))?;
    let mut member = vec![0; usize::try_from(entry.len)?];
    file.read_exact(&mut member)?;
    let mut bytes = Vec::new();
    GzDecoder::new(member.as_slice())
        .take(512 * 1024 * 1024 + 1)
        .read_to_end(&mut bytes)?;
    ensure!(
        bytes.len() <= 512 * 1024 * 1024,
        "expanded record exceeds 512 MiB"
    );
    ensure!(bytes.ends_with(b"\n"), "truncated record line");
    let lines = bytes
        .split(|&b| b == b'\n')
        .filter(|line| !line.is_empty())
        .map(serde_json::from_slice)
        .collect::<Result<Vec<Line>, _>>()?;
    Ok((digest(&bytes), lines))
}

fn digest(bytes: &[u8]) -> String {
    let mut output = String::with_capacity(64);
    for byte in Sha256::digest(bytes) {
        write!(output, "{byte:02x}").expect("write to String");
    }
    output
}

#[cfg(test)]
mod tests {
    use super::*;
    use flate2::{Compression, write::GzEncoder};
    use std::io::Write as _;

    #[test]
    fn indexed_member_is_exact_and_corruption_or_escape_is_rejected() {
        let root = std::env::temp_dir().join(format!("baylee-payment-{}", uuid::Uuid::now_v7()));
        std::fs::create_dir_all(root.join("records")).unwrap();
        let payload = b"{\"kind\":\"input\",\"n\":1,\"at\":0,\"seat\":0,\"by\":\"seat\",\"action\":\"PassPriority\",\"hash\":\"zero\"}\n";
        let mut gzip = GzEncoder::new(Vec::new(), Compression::fast());
        gzip.write_all(payload).unwrap();
        let member = gzip.finish().unwrap();
        let mut bytes = vec![0, 1, 2];
        bytes.extend_from_slice(&member);
        std::fs::write(root.join("records/shard.gz"), &bytes).unwrap();
        let mut entry = Entry {
            i: 0,
            game: "test".into(),
            shard: "shard.gz".into(),
            offset: 3,
            len: u64::try_from(member.len()).unwrap(),
            rollout_sidecar: vec![],
        };
        let (sha, lines) = record(&root, &entry).unwrap();
        assert_eq!(sha, digest(payload));
        assert_eq!(lines.len(), 1);
        entry.len += 1;
        assert!(record(&root, &entry).is_err());
        entry.len -= 2;
        assert!(record(&root, &entry).is_err());
        entry.shard = "../outside".into();
        assert!(record(&root, &entry).is_err());
        std::fs::remove_dir_all(root).unwrap();
    }
}
