//! Provenance, record anonymisation, and technical BC audit for LLM self-play games.
use std::collections::BTreeSet;
use std::fmt::Write as _;
use std::io::{Read, Write};
use std::path::Path;

use anyhow::{Context as _, ensure};
use baylee_db::sea_orm::DatabaseConnection;
use flate2::Compression;
use flate2::read::MultiGzDecoder;
use flate2::write::GzEncoder;
use serde_json::{Map, Value, json};
use sha2::{Digest, Sha256};

const KINDS: [&str; 5] = ["header", "input", "chair", "declared_mind", "end"];
const NOT_SEARCHED: [&str; 4] = ["kind", "hash", "seen", "scryfall_id"];
const SHORTEST_SEARCHED: usize = 3;

/// Hex-encoded SHA-256 digest of a byte slice.
pub fn sha256_bytes(bytes: &[u8]) -> String {
    let digest = Sha256::digest(bytes);
    let mut out = String::with_capacity(64);
    for b in digest.as_slice() {
        let _ = write!(out, "{b:02x}");
    }
    out
}

/// Hex-encoded SHA-256 digest of a file on disk.
pub fn sha256_file(path: &Path) -> anyhow::Result<String> {
    let bytes = std::fs::read(path).with_context(|| format!("reading {}", path.display()))?;
    Ok(sha256_bytes(&bytes))
}

/// Write a JSON value atomically via a `.part` sibling.
pub fn write_atomic_json(path: &Path, value: &Value) -> anyhow::Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("creating directory {}", parent.display()))?;
    }
    let tmp = path.with_extension(format!(
        "{}.part.{}",
        path.extension().and_then(|s| s.to_str()).unwrap_or("json"),
        std::process::id()
    ));
    let mut text = serde_json::to_string_pretty(value)?;
    text.push('\n');
    std::fs::write(&tmp, text.as_bytes()).with_context(|| format!("writing {}", tmp.display()))?;
    std::fs::rename(&tmp, path)
        .with_context(|| format!("renaming {} -> {}", tmp.display(), path.display()))?;
    Ok(())
}

fn forget_addresses(line: &mut Map<String, Value>) {
    let Some(model) = line.get_mut("mind").and_then(|m| m.get_mut("model")) else {
        return;
    };
    let address = model
        .as_str()
        .and_then(|m| m.split_once('@'))
        .is_some_and(|(_, after)| after.contains('.'));
    if address {
        *model = Value::from("unnamed");
    }
}

fn forget_accounts(header: &mut Map<String, Value>) {
    let Some(seats) = header
        .get_mut("preset")
        .and_then(|p| p.get_mut("seats"))
        .and_then(Value::as_array_mut)
    else {
        return;
    };
    for seat in seats {
        if let Some(human) = seat
            .get_mut("controller")
            .and_then(|c| c.get_mut("Human"))
            .and_then(Value::as_object_mut)
        {
            for number in human.values_mut() {
                *number = Value::from(0_u64);
            }
        }
    }
}

fn mentions(value: &Value, name: &str) -> bool {
    let name = name.to_lowercase();
    let mut stack = vec![value];
    while let Some(value) = stack.pop() {
        match value {
            Value::String(text) => {
                if text.to_lowercase().contains(&name) {
                    return true;
                }
            }
            Value::Array(items) => stack.extend(items),
            Value::Object(fields) => stack.extend(
                fields
                    .iter()
                    .filter(|(key, _)| !NOT_SEARCHED.contains(&key.as_str()))
                    .map(|(_, v)| v),
            ),
            Value::Null | Value::Bool(_) | Value::Number(_) => {}
        }
    }
    false
}

/// Anonymise a raw JSONL game record identically to `baylee-gateway records export`.
pub fn anonymise(jsonl: &str, names: &[String]) -> Result<String, String> {
    let names: Vec<&str> = names
        .iter()
        .map(String::as_str)
        .filter(|n| n.chars().count() >= SHORTEST_SEARCHED)
        .collect();
    let mut out = String::with_capacity(jsonl.len());
    for (n, line) in jsonl.lines().enumerate() {
        if line.trim().is_empty() {
            continue;
        }
        let mut value: Value =
            serde_json::from_str(line).map_err(|e| format!("line {}: {e}", n + 1))?;
        let object = value
            .as_object_mut()
            .ok_or_else(|| format!("line {}: not an object", n + 1))?;
        let kind = object
            .get("kind")
            .and_then(Value::as_str)
            .ok_or_else(|| format!("line {}: no kind", n + 1))?
            .to_owned();
        if !KINDS.contains(&kind.as_str()) {
            return Err(format!("line {}: a line of kind `{kind}`", n + 1));
        }
        match kind.as_str() {
            "header" => forget_accounts(object),
            "declared_mind" => forget_addresses(object),
            _ => {}
        }
        if let Some(at) = object.get_mut("at") {
            *at = Value::from(0_u64);
        }
        if names.iter().any(|name| mentions(&value, name)) {
            return Err(format!("line {} names a seated account", n + 1));
        }
        out.push_str(&value.to_string());
        out.push('\n');
    }
    if out.is_empty() {
        return Err("an empty record".to_owned());
    }
    Ok(out)
}

/// Export and anonymise a single completed game record by `game_id` directly from PostgreSQL.
pub async fn export_game_record(
    db: &DatabaseConnection,
    game_id: &str,
    dest_path: &Path,
) -> anyhow::Result<()> {
    let record = baylee_db::records::for_export(db, game_id)
        .await
        .context("querying record for export")?
        .with_context(|| format!("game {game_id} has no complete record in PostgreSQL"))?;
    let names = baylee_db::records::seated_identities(db, game_id)
        .await
        .context("querying seated identities")?;
    let mut text = String::new();
    MultiGzDecoder::new(record.data.as_slice())
        .read_to_string(&mut text)
        .context("decompressing stored record")?;
    let clean = anonymise(&text, &names)
        .map_err(|e| anyhow::anyhow!("anonymising record for game {game_id} failed: {e}"))?;
    if let Some(parent) = dest_path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let tmp = dest_path.with_extension("gz.part");
    {
        let file = std::fs::File::create(&tmp)?;
        let mut gz = GzEncoder::new(file, Compression::default());
        gz.write_all(clean.as_bytes())?;
        gz.finish()?;
    }
    std::fs::rename(&tmp, dest_path)?;
    Ok(())
}

pub fn read_jsonl(path: &Path) -> anyhow::Result<Vec<Value>> {
    let raw = std::fs::read(path).with_context(|| format!("reading {}", path.display()))?;
    let text = if raw.starts_with(&[0x1f, 0x8b]) {
        let mut s = String::new();
        MultiGzDecoder::new(raw.as_slice())
            .read_to_string(&mut s)
            .with_context(|| format!("decompressing {}", path.display()))?;
        s
    } else {
        String::from_utf8(raw).with_context(|| format!("utf8 decoding {}", path.display()))?
    };
    let mut rows = Vec::new();
    for line in text.lines() {
        if !line.trim().is_empty() {
            rows.push(serde_json::from_str(line)?);
        }
    }
    Ok(rows)
}

#[derive(Clone, Debug)]
pub struct RecordProof {
    pub actual_seed: u64,
    pub terminal: Value,
    pub inputs: [Vec<Value>; 2],
    pub input_count: usize,
    /// What each seat declared, in order: `{"mind", "chain_index",
    /// "first_input_n"}`, the mind as the record has it, where it stands in
    /// the seat's requested chain (0 its profile's, 1 its first fallback,
    /// …), and the record `n` of the seat's first input after it (`null`
    /// where none came). A mind declared again on a new socket is not listed
    /// again.
    pub declared: [Vec<Value>; 2],
    input_ns: Vec<u64>,
    record: Vec<u8>,
}

/// Lifecycle proof from an official completed export, independent of model admission.
pub fn completed_record_end(path: &Path) -> anyhow::Result<Value> {
    let rows = read_jsonl(path)?;
    terminal_row(&rows).cloned()
}

fn terminal_row(rows: &[Value]) -> anyhow::Result<&Value> {
    ensure!(
        rows.first().is_some_and(|row| row["kind"] == "header"),
        "Missing official record header"
    );
    let end = rows.last().context("Empty official record")?;
    ensure!(end["kind"] == "end", "Official record has no terminal End");
    let winners = end
        .get("winners")
        .and_then(Value::as_array)
        .context("Missing terminal winners")?;
    ensure!(
        winners.len() <= 1
            && winners
                .iter()
                .all(|winner| winner.as_u64().is_some_and(|seat| seat < 2)),
        "Invalid terminal duel outcome"
    );
    Ok(end)
}

/// Preserve the engine's real format while verifying the requested room policy.
pub fn audit_record_setup(
    path: &Path,
    requested: &baylee_core::preset::RoomSetup,
) -> anyhow::Result<Value> {
    let rows = read_jsonl(path)?;
    terminal_row(&rows)?;
    let preset = rows[0].get("preset").context("Missing official preset")?;
    let format = preset
        .get("format")
        .and_then(Value::as_str)
        .context("Missing actual record format")?;
    let seats = preset
        .get("seats")
        .and_then(Value::as_array)
        .context("Missing actual seats")?;
    ensure!(
        seats.len() == 2
            && seats
                .iter()
                .all(|seat| seat.get("starting_life").and_then(Value::as_i64)
                    == Some(i64::from(requested.starting_life))),
        "Actual starting life differs from requested room setup"
    );
    let rules = preset
        .get("house_rules")
        .context("Missing actual house rules")?;
    ensure!(
        rules.get("free_mulligans").and_then(Value::as_u64)
            == Some(u64::from(requested.free_mulligans))
            && rules.get("mulligan_free_first").and_then(Value::as_bool)
                == Some(requested.free_mulligans > 0),
        "Actual mulligan rules differ from requested room setup"
    );
    Ok(json!({
        "actual_record_format": format,
        "seat_starting_life": seats.iter().map(|seat| seat["starting_life"].clone()).collect::<Vec<_>>(),
        "free_mulligans": rules["free_mulligans"],
        "mulligan_free_first": rules["mulligan_free_first"],
    }))
}

/// Where a seat's declaration stands in its requested chain: the first
/// is the profile's own mind; each later one is the mind playing now
/// again (a socket dialled again declares it again) or a fallback after
/// it, never one before it (a game never hands back). `None` where the
/// declaration is not in the chain there.
fn chain_index(
    chain: &[baylee_gamehost::record::Mind],
    from: Option<usize>,
    declared: &baylee_gamehost::record::Mind,
) -> Option<usize> {
    match from {
        None => (chain.first() == Some(declared)).then_some(0),
        Some(from) => (from..chain.len()).find(|&at| &chain[at] == declared),
    }
}

/// Audit the exported gzipped game record against the exact pinned
/// declarations: each seat's `chains[seat]`, the mind its profile declares
/// and then its fallbacks' in the order they take over. Each seat's first
/// declaration is its profile's, and each after it is the mind playing now
/// or a later one of its chain ([`chain_index`]); anything else fails.
#[allow(clippy::too_many_lines)]
pub fn audit_record(
    path: &Path,
    chains: &[Vec<baylee_gamehost::record::Mind>; 2],
) -> anyhow::Result<RecordProof> {
    let data = read_jsonl(path)?;
    let end = terminal_row(&data)?.clone();
    let header = &data[0];
    let seed = header
        .get("preset")
        .and_then(|p| p.get("seed"))
        .and_then(Value::as_u64)
        .context("Missing server seed")?;
    let seats_len = header
        .get("preset")
        .and_then(|p| p.get("seats"))
        .and_then(Value::as_array)
        .map_or(0, Vec::len);
    ensure!(seats_len == 2, "Wrong seat count");

    // Where each seat's last declaration stands in its chain, and what
    // each seat declared, a mind declared again folded into itself.
    let mut standing: [Option<usize>; 2] = [None, None];
    let mut declared: [Vec<Value>; 2] = [Vec::new(), Vec::new()];
    let mut inputs: [Vec<Value>; 2] = [Vec::new(), Vec::new()];
    for value in &data {
        let kind = value.get("kind").and_then(Value::as_str).unwrap_or("");
        ensure!(
            matches!(kind, "header" | "input" | "chair" | "declared_mind" | "end"),
            "Unknown official record row: {kind}"
        );
        if let Some(at) = value.get("at") {
            ensure!(at.as_u64() == Some(0), "Non-anonymized time");
        }
        if kind == "declared_mind" {
            let seat = value
                .get("seat")
                .and_then(Value::as_u64)
                .and_then(|s| usize::try_from(s).ok())
                .context("Missing declared seat")?;
            ensure!(seat < 2, "Unexpected declared seat");
            let mind = value.get("mind").context("Missing mind object")?;
            let mind: baylee_gamehost::record::Mind = serde_json::from_value(mind.clone())?;
            let index = chain_index(&chains[seat], standing[seat], &mind)
                .with_context(|| format!("Unexpected declared identity for seat {seat}"))?;
            if standing[seat] != Some(index) {
                declared[seat].push(json!({
                    "mind": mind,
                    "chain_index": index,
                    "first_input_n": null,
                }));
            }
            standing[seat] = Some(index);
        } else if kind == "input" {
            let seat = value
                .get("seat")
                .and_then(Value::as_u64)
                .and_then(|s| usize::try_from(s).ok())
                .context("Missing input seat")?;
            ensure!(seat < 2, "Unexpected input seat");
            if let Some(latest) = declared[seat].last_mut()
                && latest["first_input_n"].is_null()
            {
                latest["first_input_n"] = value.get("n").cloned().unwrap_or(Value::Null);
            }
            inputs[seat].push(value.clone());
        }
    }
    ensure!(!declared[0].is_empty(), "Missing seat 0 LLM declaration");
    ensure!(!declared[1].is_empty(), "Missing seat 1 LLM declaration");
    let winners = end
        .get("winners")
        .and_then(Value::as_array)
        .context("Missing winners array")?;
    ensure!(
        winners.len() <= 1 && winners.iter().all(|v| v.as_u64().is_some_and(|s| s < 2)),
        "Invalid duel outcome"
    );
    let winner_seat = winners.first().cloned().unwrap_or(Value::Null);
    let draw = winners.is_empty();
    let input_ns = data
        .iter()
        .filter(|v| v["kind"] == "input")
        .map(|v| {
            v.get("n")
                .and_then(Value::as_u64)
                .context("Missing record input n")
        })
        .collect::<anyhow::Result<Vec<_>>>()?;
    ensure!(
        !input_ns.is_empty() && input_ns.windows(2).all(|pair| pair[0] < pair[1]),
        "Record input numbers must increase"
    );
    let raw = std::fs::read(path)?;
    let record = if raw.starts_with(&[0x1f, 0x8b]) {
        let mut decoded = Vec::new();
        MultiGzDecoder::new(raw.as_slice()).read_to_end(&mut decoded)?;
        decoded
    } else {
        raw
    };
    let input_count = inputs[0].len() + inputs[1].len();
    Ok(RecordProof {
        actual_seed: seed,
        terminal: json!({
            "winner_seat": winner_seat,
            "draw": draw,
        }),
        inputs,
        input_count,
        declared,
        input_ns,
        record,
    })
}

/// Classify explicit rate-limit/cooldown signals in a seat transcript.
pub fn classify_quota_events(data: &[Value]) -> Value {
    let mut evidence = Vec::new();
    for (index, value) in data.iter().enumerate() {
        let mut cause = None;
        if value.get("event").and_then(Value::as_str) == Some("mind_failed")
            && let Some(error) = value.get("error").and_then(Value::as_str)
        {
            if error.starts_with("the mind is rate limited: ") {
                cause = Some("native_mind_error_limited");
            } else if error.starts_with("the mind is unavailable: rate limit: ") {
                cause = Some("native_cli_rate_limit_or_cooldown");
            }
        }
        if let Some(fb) = value.get("fallback").and_then(Value::as_object)
            && fb.get("cause").and_then(Value::as_str) == Some("limited")
        {
            cause = Some("native_fallback_cause_limited");
        }
        if let Some(c) = cause {
            evidence.push(json!({
                "row_index": index,
                "event": value.get("event").cloned().unwrap_or(Value::Null),
                "cause": c,
            }));
        }
    }
    json!({
        "schema_version": 1,
        "quota_limited": !evidence.is_empty(),
        "evidence": evidence,
        "scope": "Explicit native rate-limit/busy/cooldown signals for operational backoff only; not a claim that monetary quota was exhausted.",
    })
}

/// What a seat's transcript says of its fallback profiles
/// (`docs/llm-seat.md` §"A fallback profile"), in the order it happened:
/// each that took over (`took_over`: `{"profile", "turn", "cause", "why"}`,
/// `cause` `limited` or `failed`), each that could not (`refused`:
/// `{"profile", "why"}`), and how each that took over played (`played`:
/// `{"label", "turn", "answered"}`, the bridge's closing `fell_back` line).
pub fn transcript_fallbacks(data: &[Value]) -> Value {
    let notes = |key: &str| -> Vec<Value> {
        data.iter()
            .filter_map(|row| row.get(key))
            .filter(|note| note.is_object())
            .cloned()
            .collect()
    };
    json!({
        "took_over": notes("fallback"),
        "refused": notes("fallback_refused"),
        "played": data
            .iter()
            .rev()
            .find_map(|row| row.get("fell_back"))
            .cloned()
            .unwrap_or_else(|| json!([])),
    })
}

/// Audit a single seat's CLI transcript (`transport_audit.py::audit_transcript`).
pub fn audit_transcript(path: &Path, seat_inputs: &[Value]) -> anyhow::Result<(Value, bool)> {
    let data = read_jsonl(path)?;
    ensure!(
        data.iter()
            .any(|v| v.get("event").and_then(Value::as_str) == Some("over")),
        "Transcript lacks GameOver"
    );
    let mut llm = 0_u64;
    let mut house = 0_u64;
    let mut standing = 0_u64;
    let mut other = 0_u64;
    let mut answered: Vec<Value> = Vec::new();
    for value in &data {
        match value.get("event").and_then(Value::as_str) {
            Some("answered") => {
                match value.get("by").and_then(Value::as_str) {
                    Some("mind") => llm += 1,
                    Some("house") => house += 1,
                    Some("standing") => standing += 1,
                    _ => other += 1,
                }
                answered.push(value.get("action").cloned().unwrap_or(Value::Null));
            }
            Some("standing") => {
                standing += 1;
                answered.push(value.get("action").cloned().unwrap_or(Value::Null));
            }
            _ => {}
        }
    }
    ensure!(
        other == 0 && llm > 0,
        "Unknown provenance or no actual model answers"
    );
    for value in seat_inputs {
        if value.get("by").and_then(Value::as_str) == Some("seat") {
            let act = value.get("action").unwrap_or(&Value::Null);
            let pos = answered
                .iter()
                .position(|a| a == act)
                .context("Executed seat input missing transcript attribution")?;
            answered.swap_remove(pos);
        }
    }
    let quota = classify_quota_events(&data)
        .get("quota_limited")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    Ok((
        json!({
            "llm": llm,
            "house": house,
            "standing": standing,
            "other": other,
        }),
        quota,
    ))
}

/// Require full engine hashes, including inputs that produced no visible tape frame.
pub fn audit_tape(path: &Path, proof: &RecordProof) -> anyhow::Result<Value> {
    use baylee_gamehost::replay::{self, Check, Perspective, Stop, Verify};
    use std::ops::ControlFlow;

    let stop = audit_tape_rows(&read_jsonl(path)?, proof)?;
    let mut checked = Vec::new();
    let mut all_exact = true;
    let projection = replay::project(
        &proof.record,
        vec!["Seat 0".to_owned(), "Seat 1".to_owned()],
        Perspective::Public,
        Verify::Stop,
        |moment| {
            all_exact &= moment.check == Check::Exact;
            if let Some(n) = moment.input {
                checked.push(n);
            }
            if all_exact {
                ControlFlow::Continue(())
            } else {
                ControlFlow::Break(())
            }
        },
    )
    .map_err(|error| anyhow::anyhow!("Full engine replay failed: {error:?}"))?;
    ensure!(
        all_exact
            && checked == proof.input_ns
            && projection.stop == Stop::End
            && projection.diverged.is_none()
            && projection.inputs == u64::try_from(proof.input_count)?
            && projection.exact_to == proof.input_ns.last().copied(),
        "Replay did not verify every input with its full engine hash"
    );
    Ok(stop)
}

fn audit_tape_rows(data: &[Value], proof: &RecordProof) -> anyhow::Result<Value> {
    let stop = data
        .last()
        .filter(|v| v.get("kind").and_then(Value::as_str) == Some("stop"))
        .cloned()
        .context("Replay has no stop receipt")?;
    ensure!(
        stop.get("stop").and_then(Value::as_str) == Some("end")
            && stop.get("diverged").is_some_and(Value::is_null),
        "Replay refused/cut/diverged"
    );
    ensure!(
        stop.get("inputs").and_then(Value::as_u64) == Some(u64::try_from(proof.input_count)?),
        "Replay did not apply every input"
    );
    ensure!(
        stop.get("exact_to").and_then(Value::as_u64) == proof.input_ns.last().copied(),
        "Replay full hash proof does not reach the final record input"
    );
    for row in data {
        if row.get("kind").and_then(Value::as_str) == Some("view") {
            ensure!(
                row.get("check").and_then(Value::as_str) == Some("exact"),
                "Replay frame lacks full engine hash verification"
            );
        }
    }
    Ok(stop)
}

#[derive(Clone, Debug)]
struct SendItem {
    action: Value,
    source: &'static str,
    question: Option<u64>,
}

/// Build the conservative ordered provenance attribution for one seat (`transport_audit.py::ordered_attribution`).
#[allow(clippy::too_many_lines)]
pub fn ordered_attribution(transcript_path: &Path, seat_inputs: &[Value]) -> anyhow::Result<Value> {
    let tr_rows = read_jsonl(transcript_path)?;
    let mut sends: Vec<SendItem> = Vec::new();
    for value in &tr_rows {
        match value.get("event").and_then(Value::as_str) {
            Some("answered" | "standing") => {
                let source = if value.get("event").and_then(Value::as_str) == Some("standing") {
                    "standing"
                } else {
                    match value.get("by").and_then(Value::as_str) {
                        Some("mind") => "llm",
                        Some("house") => "house",
                        Some("standing") => "standing",
                        _ => "other",
                    }
                };
                sends.push(SendItem {
                    action: value.get("action").cloned().unwrap_or(Value::Null),
                    source,
                    question: value.get("question").and_then(Value::as_u64),
                });
            }
            Some("refused") => {
                let q = value.get("question").and_then(Value::as_u64);
                let act = value.get("action").unwrap_or(&Value::Null);
                if let Some(idx) = sends
                    .iter()
                    .rposition(|s| s.question == q && &s.action == act)
                {
                    sends.remove(idx);
                }
            }
            _ => {}
        }
    }

    let executed: Vec<&Value> = seat_inputs
        .iter()
        .filter(|v| v.get("by").and_then(Value::as_str) == Some("seat"))
        .collect();
    let mut earliest = Vec::with_capacity(executed.len());
    let mut cursor = 0_usize;
    for val in &executed {
        let act = val.get("action").unwrap_or(&Value::Null);
        while cursor < sends.len() && &sends[cursor].action != act {
            cursor += 1;
        }
        ensure!(
            cursor < sends.len(),
            "Executed inputs are not an ordered transcript subsequence"
        );
        earliest.push(cursor);
        cursor += 1;
    }

    let mut latest = vec![0_usize; executed.len()];
    let mut bcursor = sends.len();
    for (i, val) in executed.iter().enumerate().rev() {
        let act = val.get("action").unwrap_or(&Value::Null);
        while bcursor > 0 && &sends[bcursor - 1].action != act {
            bcursor -= 1;
        }
        ensure!(
            bcursor > 0,
            "Executed inputs lack ordered suffix attribution"
        );
        latest[i] = bcursor - 1;
        bcursor -= 1;
    }

    let mut proof = Vec::with_capacity(executed.len());
    for (i, val) in executed.iter().enumerate() {
        let lo = if i == 0 { 0 } else { earliest[i - 1] + 1 };
        let hi = if i + 1 < executed.len() {
            latest[i + 1]
        } else {
            sends.len()
        };
        ensure!(lo <= hi && hi <= sends.len(), "Invalid attribution window");
        let act = val.get("action").unwrap_or(&Value::Null);
        let matching: Vec<&SendItem> = sends[lo..hi].iter().filter(|s| &s.action == act).collect();
        let origins: Vec<&str> = ["llm", "house", "standing", "other"]
            .into_iter()
            .filter(|src| matching.iter().any(|m| m.source == *src))
            .collect();
        ensure!(!origins.is_empty(), "No compatible executed-input origin");
        proof.push(json!({
            "record_input_n": val.get("n").cloned().unwrap_or(Value::Null),
            "action": val.get("action").cloned().unwrap_or(Value::Null),
            "possible_sources": origins,
            "ambiguous": origins.len() > 1,
            "possible_send_count": matching.len(),
            "policy_target_admitted": false,
        }));
    }

    let mut nonseat = Map::new();
    for val in seat_inputs {
        let by = val.get("by").and_then(Value::as_str).unwrap_or("unknown");
        if by != "seat" {
            let prev = nonseat.get(by).and_then(Value::as_u64).unwrap_or(0);
            nonseat.insert(by.to_owned(), json!(prev + 1));
        }
    }

    Ok(json!({
        "scope": "Conservative ordered subsequence attribution after explicit refusals. No shared question identifier; withdrawn/unconfirmed sends may leave ambiguity.",
        "policy_targets": "none_until_review",
        "ppo_eligible": false,
        "inputs": proof,
        "nonseat_input_sources": nonseat,
    }))
}

/// A seat row's `declared_minds` hold what [`audit_record`] proved: the
/// profile's own mind first, then fallbacks of the seat's requested chain
/// (`requested_fallbacks`, none in an older request), each later than the
/// one before it.
fn declared_in_order(row: &Value, request: &Value, seat: usize) -> anyhow::Result<()> {
    let places = row
        .get("declared_minds")
        .and_then(Value::as_array)
        .context("Missing declared_minds")?
        .iter()
        .map(|declared| declared.get("chain_index").and_then(Value::as_u64))
        .collect::<Option<Vec<u64>>>()
        .context("A declared mind without its place in the chain")?;
    let fallbacks = request
        .get("requested_fallbacks")
        .and_then(|seats| seats.get(seat))
        .and_then(Value::as_array)
        .map_or(0, Vec::len);
    ensure!(
        places.first() == Some(&0)
            && places.windows(2).all(|pair| pair[0] < pair[1])
            && places
                .iter()
                .all(|&place| usize::try_from(place).is_ok_and(|place| place <= fallbacks)),
        "Declared minds out of seat {seat}'s requested chain"
    );
    Ok(())
}

/// Validate `technical-result.json` against `core.py::validate`.
#[allow(clippy::too_many_lines)]
pub fn validate_technical_result(
    result: &Value,
    request: &Value,
    work_dir: &Path,
) -> anyhow::Result<()> {
    let room_path = work_dir.join("room-state.json");
    ensure!(room_path.is_file(), "Missing room-state.json");
    let room: Value = serde_json::from_str(&std::fs::read_to_string(&room_path)?)?;
    ensure!(
        room.get("status").and_then(Value::as_str) == Some("closed"),
        "Owned room not confirmed closed"
    );
    ensure!(
        result.get("schema_version").and_then(Value::as_u64) == Some(1)
            && result.get("status").and_then(Value::as_str) == Some("complete"),
        "Incomplete technical result"
    );
    ensure!(
        result.get("actual_seed").and_then(Value::as_u64).is_some(),
        "Missing actual seed"
    );
    for k in [
        "connection",
        "view_boundary",
        "full_record",
        "replay",
        "provenance",
    ] {
        ensure!(
            result
                .get("checks")
                .and_then(|c| c.get(k))
                .and_then(Value::as_bool)
                == Some(true),
            "Technical check {k} not complete"
        );
    }
    let terminal = result.get("terminal").context("Missing terminal")?;
    let draw = terminal
        .get("draw")
        .and_then(Value::as_bool)
        .context("Missing draw bool")?;
    let winner = terminal.get("winner_seat").unwrap_or(&Value::Null);
    ensure!(
        (draw && winner.is_null()) || (!draw && winner.as_u64().is_some_and(|w| w == 0 || w == 1)),
        "Unknown/invalid terminal outcome"
    );
    let seats = result
        .get("seats")
        .and_then(Value::as_array)
        .context("Missing seats array")?;
    ensure!(seats.len() == 2, "Two seat receipts required");
    let req_models = request
        .get("models")
        .and_then(Value::as_array)
        .context("Missing request models")?;
    for (seat, row) in seats.iter().enumerate() {
        ensure!(
            row.get("seat").and_then(Value::as_u64) == Some(u64::try_from(seat).unwrap_or(0))
                && row.get("model") == req_models.get(seat)
                && row.get("declared_mind_verified").and_then(Value::as_bool) == Some(true),
            "Wrong seat/model provenance"
        );
        declared_in_order(row, request, seat)?;
        let counts = row
            .get("transcript_answer_counts")
            .and_then(Value::as_object)
            .context("Missing transcript_answer_counts")?;
        for k in ["llm", "house", "standing", "other"] {
            ensure!(
                counts.get(k).and_then(Value::as_u64).is_some(),
                "Missing count key {k}"
            );
        }
        ensure!(
            counts.get("other").and_then(Value::as_u64) == Some(0),
            "Unresolved answer provenance"
        );
    }
    let artifacts = result
        .get("artifacts")
        .and_then(Value::as_array)
        .context("Missing artifacts array")?;
    let work_canon = work_dir.canonicalize()?;
    let mut roles = BTreeSet::new();
    let mut record_matched = false;
    for item in artifacts {
        let p = Path::new(
            item.get("path")
                .and_then(Value::as_str)
                .context("Missing artifact path")?,
        );
        ensure!(p.is_absolute() && p.is_file(), "Artifact missing on disk");
        let p_canon = p.canonicalize()?;
        ensure!(
            p_canon.starts_with(&work_canon),
            "Artifact escapes attempt directory"
        );
        let digest = sha256_file(p)?;
        ensure!(
            item.get("sha256").and_then(Value::as_str) == Some(digest.as_str()),
            "Artifact hash mismatch"
        );
        let role = item
            .get("role")
            .and_then(Value::as_str)
            .context("Missing artifact role")?;
        roles.insert(role.to_owned());
        if role == "anonymized_record"
            && result.get("record_sha256").and_then(Value::as_str) == Some(digest.as_str())
        {
            if let Some(setup) = request.get("room_setup") {
                let requested: baylee_core::preset::RoomSetup =
                    serde_json::from_value(setup.clone())?;
                ensure!(
                    result.get("requested_room_setup") == Some(setup),
                    "Technical requested room setup differs"
                );
                let actual = audit_record_setup(p, &requested)?;
                ensure!(
                    result.get("actual_room_setup") == Some(&actual),
                    "Technical actual room setup differs from official record"
                );
            }
            record_matched = true;
        }
    }
    for req_role in [
        "anonymized_record",
        "transcript_seat0",
        "transcript_seat1",
        "replay_receipt",
    ] {
        ensure!(
            roles.contains(req_role),
            "Missing required artifact role {req_role}"
        );
    }
    ensure!(record_matched, "Record identity missing");
    Ok(())
}

#[cfg(test)]
pub(crate) mod tests;
