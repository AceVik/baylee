//! Freeze the earliest technically admitted games, without semantic teacher filtering.
mod evidence;
mod identity;
#[cfg(test)]
mod tests;
use crate::files;
use anyhow::{Context, Result, ensure};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{
    collections::{BTreeMap, BTreeSet},
    fs::{File, OpenOptions},
    io::Write,
    path::{Path, PathBuf},
};
fn digest(bytes: &[u8]) -> String {
    files::hex(&Sha256::digest(bytes))
}
fn text<'a>(value: &'a Value, key: &str) -> Result<&'a str> {
    value[key]
        .as_str()
        .with_context(|| format!("missing collector string {key}"))
}
struct Selected {
    path: PathBuf,
    raw: Vec<u8>,
    receipt: Value,
}
fn select(collector: &Path, count: usize) -> Result<Vec<Selected>> {
    ensure!(count > 0, "A positive game count is required");
    let attempts = collector.join("attempts");
    let mut paths = std::fs::read_dir(attempts)?
        .filter_map(Result::ok)
        .map(|e| e.path())
        .filter(|p| {
            p.file_name()
                .is_some_and(|n| n.to_string_lossy().starts_with("attempt-"))
        })
        .map(|p| p.join("receipt.json"))
        .filter(|p| p.is_file())
        .collect::<Vec<_>>();
    paths.sort();
    let mut selected = Vec::new();
    for path in paths {
        let raw = std::fs::read(&path)?;
        let receipt: Value = serde_json::from_slice(&raw)?;
        if receipt["status"] == "admitted" {
            selected.push(Selected { path, raw, receipt });
            if selected.len() == count {
                return Ok(selected);
            }
        }
    }
    anyhow::bail!(
        "Need {count} complete admitted games; found {}",
        selected.len()
    )
}
fn output_path(path: &Path) -> Result<PathBuf> {
    let absolute = std::path::absolute(path)?;
    let name = absolute.file_name().context("missing output name")?;
    let parent = absolute.parent().context("missing output parent")?;
    std::fs::create_dir_all(parent)?;
    Ok(parent.canonicalize()?.join(name))
}
fn frozen_game_key(collector: &Path, attempt: &str, record_hash: &str) -> Result<String> {
    let source = format!(
        "{}\0{attempt}",
        collector
            .canonicalize()?
            .to_str()
            .context("non-UTF8 collector root")?
    );
    Ok(format!(
        "source-{}-{record_hash}",
        digest(source.as_bytes())
    ))
}
fn copy_bytes(staging: &Path, out: &Path, relative: &Path, raw: &[u8]) -> Result<Value> {
    let destination = staging.join(relative);
    std::fs::create_dir_all(destination.parent().context("missing evidence parent")?)?;
    let mut file = OpenOptions::new()
        .create_new(true)
        .write(true)
        .open(destination)?;
    file.write_all(raw)?;
    file.sync_all()?;
    Ok(json!({"path":out.join(relative),"sha256":digest(raw)}))
}
fn suffixes(path: &Path) -> Result<String> {
    let name = path
        .file_name()
        .and_then(|s| s.to_str())
        .context("invalid evidence filename")?;
    Ok(name
        .char_indices()
        .skip(1)
        .find(|(_, c)| *c == '.')
        .map_or("", |(i, _)| &name[i..])
        .to_owned())
}
/// Freeze exact evidence for the earliest admitted prefix, preserving original receipts.
///
/// # Errors
/// Rejects missing games, changed artifacts, duplicate records and invalid admission binding.
pub fn prepare(collector: &Path, out: &Path, count: usize) -> Result<Value> {
    let collector = collector.canonicalize()?;
    let selected = select(&collector, count)?;
    prepare_selected(&collector, out, &selected)
}
fn prepare_selected(collector: &Path, out: &Path, selected: &[Selected]) -> Result<Value> {
    let out = output_path(out)?;
    let name = out
        .file_name()
        .context("missing frozen root name")?
        .to_string_lossy();
    let staging = out.with_file_name(format!("{name}.partial"));
    ensure!(
        !out.exists() && !staging.exists(),
        "Output or retained partial output already exists"
    );
    std::fs::create_dir(&staging)?;
    for name in ["records", "llm", "evidence"] {
        std::fs::create_dir(staging.join(name))?;
    }
    let mut rows = Vec::new();
    let mut sources = Vec::new();
    let mut seen_records = BTreeSet::new();
    for selected in selected {
        freeze_one(
            &staging,
            &out,
            selected,
            &mut rows,
            &mut sources,
            &mut seen_records,
        )?;
    }
    let collectors = sources
        .iter()
        .filter_map(|s: &Value| s["source_collector"].as_str())
        .collect::<BTreeSet<_>>();
    files::atomic_json(
        &staging.join("run.json"),
        &json!({"schema_version":1,"llm_bc_version":1,"seats":2,"source_collector":collector,"source_collectors":collectors,"games":selected.len(),"selection":"earliest technically admitted games, independent of outcome","sources":sources,"semantic_quality_gate":false,"bc_only":true,"ppo_eligible":false}),
    )?;
    let mut games = File::create(staging.join("games.jsonl"))?;
    for row in rows {
        serde_json::to_writer(&mut games, &row)?;
        games.write_all(b"\n")?;
    }
    games.sync_all()?;
    std::fs::rename(&staging, &out)?;
    Ok(
        json!({"root":out,"games":selected.len(),"run_sha256":files::sha256(&out.join("run.json"))?}),
    )
}
fn freeze_one(
    staging: &Path,
    out: &Path,
    selected: &Selected,
    rows: &mut Vec<Value>,
    sources: &mut Vec<Value>,
    seen: &mut BTreeSet<String>,
) -> Result<()> {
    let attempt = selected
        .path
        .parent()
        .context("missing attempt directory")?
        .canonicalize()?;
    let game = attempt
        .file_name()
        .and_then(|s| s.to_str())
        .context("invalid attempt name")?;
    let request_raw = std::fs::read(attempt.join("request.json"))?;
    let technical_raw = std::fs::read(attempt.join("technical-result.json"))?;
    let request: Value = serde_json::from_slice(&request_raw)?;
    let technical: Value = serde_json::from_slice(&technical_raw)?;
    ensure!(
        request == selected.receipt["request"] && technical == selected.receipt["technical"],
        "Receipt does not bind exact request/result: {game}"
    );
    ensure!(
        request["attempt_id"] == game && technical["status"] == "complete",
        "Wrong attempt or incomplete result: {game}"
    );
    ensure!(
        technical["bc_only"] == true && technical["ppo_eligible"] == false,
        "LLM collection must be BC-only"
    );
    ensure!(
        [
            "connection",
            "view_boundary",
            "full_record",
            "replay",
            "provenance"
        ]
        .iter()
        .all(|key| technical["checks"][key] == true),
        "Incomplete technical checks: {game}"
    );
    let models = request["models"]
        .as_array()
        .filter(|m| m.len() == 2)
        .context("Two declared model seats are required")?;
    let bound = evidence::artifacts(&technical, &attempt, game)?;
    let artifacts = &bound.bytes;
    let suffix = &bound.suffixes;
    let compressed = artifacts
        .get("anonymized_record")
        .context("missing anonymized record")?;
    let record = evidence::record(&request, &technical, artifacts, seen)?;
    let record_hash = record.hash;
    let seed = record.seed;
    let draw = record.draw;
    let terminal = &technical["terminal"];
    let source_collector = attempt
        .parent()
        .and_then(Path::parent)
        .context("missing source collector")?;
    let namespace = digest(
        source_collector
            .to_str()
            .context("invalid collector root")?
            .as_bytes(),
    );
    let frozen = frozen_game_key(source_collector, game, &record_hash)?;
    let evidence = Path::new("evidence").join(&frozen);
    let mut pins = BTreeMap::new();
    for (role, raw) in artifacts {
        pins.insert(
            role.clone(),
            copy_bytes(
                staging,
                out,
                &evidence.join(format!("{role}{}", suffix[role])),
                raw,
            )?,
        );
    }
    let shard = format!("{frozen}.jsonl.gz");
    copy_bytes(staging, out, &Path::new("records").join(&shard), compressed)?;
    let mut seats = Vec::new();
    for (seat, model) in models.iter().enumerate() {
        seats.push(json!({"seat":seat,"model":model,"transcript":pins.get(&format!("transcript_seat{seat}")).context("missing transcript artifact")?,"origins":pins.get(&format!("executed_provenance_seat{seat}")).context("missing provenance artifact")?}));
    }
    let mut manifest = json!({"schema_version":1,"contract":"admitted_executed_llm_v1","source_game":game,"frozen_game":frozen,"record_sha256":record_hash,"record_artifact_sha256":digest(compressed),"request":copy_bytes(staging,out,&evidence.join("request.json"),&request_raw)?,"admitted_receipt":copy_bytes(staging,out,&evidence.join("receipt.json"),&selected.raw)?,"technical_result":copy_bytes(staging,out,&evidence.join("technical-result.json"),&technical_raw)?,"seats":seats});
    if let Some(pin) = pins.get("input_receipt") {
        let bound: Value = serde_json::from_slice(&artifacts["input_receipt"])?;
        if bound.get("requested_minds").is_some() {
            manifest["input_receipt"] = pin.clone();
        }
    }
    files::atomic_json(
        &staging.join("llm").join(format!("{frozen}.json")),
        &manifest,
    )?;
    rows.push(json!({"game":frozen,"seed":seed,"outcome":{"kind":if draw{"draw"}else{"won"},"winner":terminal["winner_seat"]},"decks":request["decks"],"shard":shard,"offset":0,"len":compressed.len()}));
    sources.push(json!({"attempt":game,"frozen_game":frozen,"source_collector":source_collector,"source_namespace":namespace,"receipt_path":selected.path.canonicalize()?,"receipt_sha256":digest(&selected.raw),"record_sha256":record_hash}));
    ensure!(
        std::fs::read(&selected.path)? == selected.raw,
        "Collector receipt changed while freezing"
    );
    Ok(())
}
