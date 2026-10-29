use super::{digest, identity, suffixes, text};
use anyhow::{Context, Result, ensure};
use flate2::read::MultiGzDecoder;
use serde_json::{Value, json};
use std::{
    collections::{BTreeMap, BTreeSet},
    io::Read,
    path::Path,
};
pub(super) struct Artifacts {
    pub bytes: BTreeMap<String, Vec<u8>>,
    pub suffixes: BTreeMap<String, String>,
}
pub(super) struct Record {
    pub hash: String,
    pub seed: u64,
    pub draw: bool,
}
pub(super) fn artifacts(technical: &Value, attempt: &Path, game: &str) -> Result<Artifacts> {
    let mut bytes = BTreeMap::new();
    let mut suffix = BTreeMap::new();
    for artifact in technical["artifacts"]
        .as_array()
        .context("missing technical artifacts")?
    {
        let path = Path::new(text(artifact, "path")?).canonicalize()?;
        let role = text(artifact, "role")?;
        ensure!(
            path.starts_with(attempt) && !bytes.contains_key(role),
            "Escaping artifact or duplicate evidence role"
        );
        ensure!(
            role.chars().any(|c| c != '_') && role.chars().all(|c| c == '_' || c.is_alphanumeric()),
            "Invalid artifact role"
        );
        let raw = std::fs::read(&path)?;
        ensure!(
            Some(digest(&raw).as_str()) == artifact["sha256"].as_str(),
            "Artifact bytes changed: {game}/{role}"
        );
        bytes.insert(role.to_owned(), raw);
        suffix.insert(role.to_owned(), suffixes(&path)?);
    }
    Ok(Artifacts {
        bytes,
        suffixes: suffix,
    })
}
pub(super) fn record(
    request: &Value,
    technical: &Value,
    artifacts: &BTreeMap<String, Vec<u8>>,
    seen: &mut BTreeSet<String>,
) -> Result<Record> {
    let compressed = artifacts
        .get("anonymized_record")
        .context("missing anonymized record")?;
    ensure!(
        Some(digest(compressed).as_str()) == technical["record_sha256"].as_str(),
        "Record identity differs from admitted result"
    );
    let mut raw = Vec::new();
    MultiGzDecoder::new(compressed.as_slice()).read_to_end(&mut raw)?;
    let hash = digest(&raw);
    ensure!(
        seen.insert(hash.clone()),
        "Duplicate complete game in collector history"
    );
    let records = std::str::from_utf8(&raw)?
        .lines()
        .filter(|l| !l.trim().is_empty())
        .map(serde_json::from_str::<Value>)
        .collect::<serde_json::Result<Vec<_>>>()?;
    let first = records
        .first()
        .context("Record lacks complete header/end")?;
    let last = records.last().context("Record lacks complete header/end")?;
    ensure!(
        first["kind"] == "header" && last["kind"] == "end",
        "Record lacks complete header/end"
    );
    identity::verify(request, &records, artifacts)?;
    let seed = technical["actual_seed"]
        .as_u64()
        .context("invalid actual seed")?;
    ensure!(
        first["preset"]["seed"].as_u64() == Some(seed),
        "Actual server seed mismatch"
    );
    let terminal = &technical["terminal"];
    let draw = terminal["draw"]
        .as_bool()
        .context("missing terminal draw")?;
    ensure!(
        last["winners"]
            == if draw {
                json!([])
            } else {
                json!([terminal["winner_seat"]])
            },
        "Terminal result mismatch"
    );
    Ok(Record { hash, seed, draw })
}
