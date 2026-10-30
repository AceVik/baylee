//! Strict evidence admission for one bounded Flash reset probe.
use super::{attempts, json, pins, text, verify_closed};
use crate::files;
use anyhow::{Context, Result, ensure};
use serde_json::{Value, json};
use std::{
    collections::BTreeMap,
    fs::File,
    io::{BufRead, BufReader, Read},
    path::{Path, PathBuf},
};
const MODEL: &str = "gemini-3.8-flash-low";
fn jsonl(path: &Path) -> Result<Vec<Value>> {
    let mut source = File::open(path)?;
    let mut magic = [0; 2];
    let count = source.read(&mut magic)?;
    let source = File::open(path)?;
    let reader: Box<dyn Read> = if count == 2 && magic == [31, 139] {
        Box::new(flate2::read::GzDecoder::new(source))
    } else {
        Box::new(source)
    };
    BufReader::new(reader)
        .lines()
        .filter_map(|line| match line {
            Ok(line) if line.trim().is_empty() => None,
            other => Some(
                other
                    .map_err(anyhow::Error::from)
                    .and_then(|line| Ok(serde_json::from_str(&line)?)),
            ),
        })
        .collect()
}
/// Check immutable probe evidence. `false` means not finished; malformed or rejected evidence errors.
/// # Errors
/// Refuses incomplete identity, setup, transcript, executed provenance or exact replay evidence.
pub fn validate_clean_probe(config: &Value) -> Result<bool> {
    let Some(root) = config["clean_probe_root"].as_str() else {
        return Ok(true);
    };
    let root = Path::new(root);
    if !root.join("status.json").exists() {
        return Ok(false);
    }
    let status = json(root.join("status.json"))?;
    if matches!(
        status["phase"].as_str(),
        Some("collecting" | "setup" | "running" | "draining")
    ) {
        return Ok(false);
    }
    validate_status(&status)?;
    verify_closed(root)?;
    let receipts = attempts(root, "receipt.json")?;
    let requests = attempts(root, "request.json")?;
    ensure!(
        receipts.len() == 1 && requests.len() == 1 && receipts[0].parent() == requests[0].parent(),
        "probe must contain exactly one receipt and request"
    );
    let attempt = receipts[0]
        .parent()
        .context("attempt directory")?
        .canonicalize()?;
    let receipt = json(&receipts[0])?;
    let request = json(&requests[0])?;
    let technical = json(attempt.join("technical-result.json"))?;
    ensure!(
        receipt["status"] == "admitted"
            && receipt["request"] == request
            && receipt["technical"] == technical,
        "probe admission evidence differs"
    );
    validate_request(config, &request, &technical)?;
    ensure!(
        technical["schema_version"].as_u64() == Some(1)
            && technical["status"] == "complete"
            && technical["quota_limited"] == false,
        "probe incomplete or quota limited"
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
        .all(|k| technical["checks"][k] == true),
        "probe checks incomplete"
    );
    let seats = technical["seats"].as_array().context("probe seats")?;
    ensure!(seats.len() == 2, "probe requires two seats");
    let mut artifacts: BTreeMap<String, (PathBuf, String)> = BTreeMap::new();
    for item in technical["artifacts"]
        .as_array()
        .context("probe artifacts")?
    {
        let path = Path::new(text(item, "path")?);
        ensure!(
            path.is_absolute() && path.canonicalize()?.starts_with(&attempt),
            "artifact escapes attempt"
        );
        let digest = files::sha256(path)?;
        ensure!(
            item["sha256"] == digest
                && artifacts
                    .insert(text(item, "role")?.to_owned(), (path.to_owned(), digest))
                    .is_none(),
            "artifact hash differs or role duplicated"
        );
    }
    for role in [
        "anonymized_record",
        "replay_tape",
        "replay_receipt",
        "transcript_seat0",
        "transcript_seat1",
        "executed_provenance_seat0",
        "executed_provenance_seat1",
    ] {
        ensure!(
            artifacts.contains_key(role),
            "probe artifact missing: {role}"
        );
    }
    ensure!(
        technical["record_sha256"] == artifacts["anonymized_record"].1,
        "record hash differs"
    );
    validate_transcripts(seats, &artifacts)?;
    validate_record(&artifacts, &technical["actual_room_setup"])
}
fn validate_record(
    artifacts: &BTreeMap<String, (PathBuf, String)>,
    actual: &Value,
) -> Result<bool> {
    let record = jsonl(&artifacts["anonymized_record"].0)?;
    ensure!(
        record.first().is_some_and(|v| v["kind"] == "header")
            && record.last().is_some_and(|v| v["kind"] == "end"),
        "official record incomplete"
    );
    let preset = &record[0]["preset"];
    let seats = preset["seats"].as_array().context("record seats")?;
    ensure!(
        seats.len() == 2
            && seats.iter().all(|s| s["starting_life"] == 20)
            && preset["house_rules"]["free_mulligans"].as_u64() == Some(0)
            && preset["house_rules"]["mulligan_free_first"] == false,
        "record setup differs"
    );
    ensure!(
        preset["format"].is_string() && actual["actual_record_format"] == preset["format"],
        "record format differs"
    );
    let declarations: Vec<_> = record
        .iter()
        .filter(|v| v["kind"] == "declared_mind")
        .collect();
    ensure!(
        [0, 1]
            .iter()
            .all(|n| declarations.iter().any(|v| v["seat"] == *n))
            && declarations
                .iter()
                .all(|v| matches!(v["seat"].as_u64(), Some(0 | 1))
                    && v["mind"]
                        == json!({"kind":"llm_cli","provider":"agy","model":MODEL,"effort":"low"})),
        "record declarations differ"
    );
    let inputs: Vec<_> = record.iter().filter(|v| v["kind"] == "input").collect();
    let ns: Vec<u64> = inputs
        .iter()
        .map(|v| v["n"].as_u64().context("record input n"))
        .collect::<Result<_>>()?;
    ensure!(
        !ns.is_empty()
            && ns.windows(2).all(|w| w[0] < w[1])
            && inputs
                .iter()
                .all(|v| v["by"] == "seat" && matches!(v["seat"].as_u64(), Some(0 | 1))),
        "record input identity invalid"
    );
    for seat in 0..2 {
        let provenance = json(&artifacts[&format!("executed_provenance_seat{seat}")].0)?;
        let rows = provenance["inputs"].as_array().context("executed inputs")?;
        let executed: Vec<_> = inputs.iter().filter(|v| v["seat"] == seat).collect();
        ensure!(
            rows.len() == executed.len()
                && rows
                    .iter()
                    .zip(executed)
                    .all(|(a, b)| a["record_input_n"] == b["n"] && a["action"] == b["action"]),
            "executed provenance differs"
        );
        ensure!(
            rows.iter().all(|v| v["possible_sources"]
                .as_array()
                .is_some_and(|s| !s.is_empty() && s.iter().all(|v| v == "llm" || v == "standing")))
                && rows.iter().any(|v| v["possible_sources"] == json!(["llm"])),
            "executed fallback or no definite LLM input"
        );
    }
    let replay = json(&artifacts["replay_receipt"].0)?;
    let tape = jsonl(&artifacts["replay_tape"].0)?;
    let stop = &replay["stop"];
    ensure!(
        replay["status"] == "verified"
            && replay["record_sha256"] == artifacts["anonymized_record"].1
            && replay["tape_sha256"] == artifacts["replay_tape"].1,
        "replay pins differ"
    );
    ensure!(
        stop["kind"] == "stop"
            && stop["stop"] == "end"
            && stop["diverged"].is_null()
            && stop["inputs"].as_u64() == Some(u64::try_from(ns.len())?)
            && stop["exact_to"].as_u64() == ns.last().copied()
            && tape.last() == Some(stop),
        "replay does not verify final input"
    );
    ensure!(
        tape.iter()
            .filter(|v| v["kind"] == "view")
            .all(|v| v["check"] == "exact"),
        "replay inexact frame"
    );
    Ok(true)
}
#[cfg(test)]
#[path = "probe_tests.rs"]
pub(super) mod tests;

fn validate_request(config: &Value, request: &Value, technical: &Value) -> Result<()> {
    ensure!(
        request["models"] == json!([MODEL, MODEL]),
        "probe must request Flash both seats"
    );
    let decks = request["decks"].as_array().context("probe decks")?;
    ensure!(
        decks.len() == 2 && decks.iter().all(|v| v["format"] == "Constructed"),
        "probe decks must be Constructed"
    );
    let setup = &request["room_setup"];
    let fields = setup.as_object().context("probe setup")?;
    ensure!(
        setup["starting_life"] == 20
            && setup["free_mulligans"].as_u64() == Some(0)
            && setup["seats"] == json!([])
            && setup.get("format").is_none_or(|v| v == "host")
            && fields
                .keys()
                .all(|k| ["starting_life", "free_mulligans", "seats", "format"]
                    .contains(&k.as_str())),
        "probe must explicitly request normal duel setup"
    );
    ensure!(
        technical["requested_room_setup"] == *setup,
        "technical requested setup differs"
    );
    let actual = &technical["actual_room_setup"];
    ensure!(
        actual["seat_starting_life"] == json!([20, 20])
            && actual["free_mulligans"].as_u64() == Some(0)
            && actual["mulligan_free_first"] == false,
        "actual room setup differs"
    );
    pins(&config["clean_probe_sha256"])?;
    for (key, field, digest) in [
        (
            "clean_probe_profiles_config",
            "profiles_config",
            "profiles_config_sha256",
        ),
        (
            "clean_probe_pairings_file",
            "pairings_file",
            "pairings_sha256",
        ),
    ] {
        let path = text(config, key)?;
        ensure!(
            request[field] == path
                && request[digest] == config["clean_probe_sha256"][path]
                && request[digest].is_string(),
            "probe input identity differs"
        );
    }
    let profiles = json(text(config, "clean_probe_profiles_config")?)?;
    let pairings = json(text(config, "clean_probe_pairings_file")?)?;
    let pair = request["profiles"]
        .as_array()
        .context("probe profile pair")?;
    ensure!(
        pair.len() == 2
            && pairings
                .as_array()
                .context("pairings array")?
                .contains(&request["profiles"]),
        "probe pairing differs"
    );
    for name in pair {
        let profile = &profiles["profiles"][name.as_str().context("profile name")?];
        ensure!(
            profile["provider"] == "cli"
                && profile["model"] == format!("agy:{MODEL}")
                && profile["effort"] == "low",
            "probe model identity differs"
        );
    }
    Ok(())
}

fn validate_transcripts(
    seats: &[Value],
    artifacts: &BTreeMap<String, (PathBuf, String)>,
) -> Result<()> {
    for (number, seat) in seats.iter().enumerate() {
        ensure!(
            seat["seat"].as_u64() == Some(u64::try_from(number)?)
                && seat["model"] == MODEL
                && seat["declared_mind_verified"] == true,
            "seat identity differs"
        );
        let rows = jsonl(&artifacts[&format!("transcript_seat{number}")].0)?;
        ensure!(
            rows.iter().any(|v| v["event"] == "over"),
            "transcript lacks GameOver"
        );
        let mut counts = json!({"llm":0,"house":0,"other":0,"standing":0});
        for row in rows {
            let source = if row["event"] == "answered" {
                Some(match row["by"].as_str() {
                    Some("mind") => "llm",
                    Some("house") => "house",
                    Some("standing") => "standing",
                    _ => "other",
                })
            } else if row["event"] == "standing" {
                Some("standing")
            } else {
                None
            };
            if let Some(source) = source {
                counts[source] = json!(counts[source].as_u64().context("answer count")? + 1);
            }
            let limited = row["event"] == "mind_failed"
                && row["error"].as_str().is_some_and(|s| {
                    s.starts_with("the mind is rate limited: ")
                        || s.starts_with("the mind is unavailable: rate limit: ")
                });
            ensure!(
                !limited && row["fallback"]["cause"] != "limited",
                "transcript rate limited"
            );
            if let Some(fallbacks) = row["summary"]["fallbacks"].as_object() {
                ensure!(
                    fallbacks
                        .values()
                        .all(|v| v == 0 || v == false || v.is_null()),
                    "transcript summary fallback"
                );
            }
        }
        ensure!(
            counts == seat["transcript_answer_counts"]
                && counts["llm"].as_u64().is_some_and(|n| n > 0)
                && counts["house"] == 0
                && counts["other"] == 0,
            "transcript counts differ or contain fallback"
        );
    }
    Ok(())
}

fn validate_status(status: &Value) -> Result<()> {
    ensure!(
        status["phase"] == "stopped"
            && status["max_attempts"].as_u64() == Some(1)
            && status["assigned_attempts"].as_u64() == Some(1),
        "probe must stop after exactly one attempt"
    );
    ensure!(
        status.get("worker_failures").is_none_or(|v| v.is_null()
            || v == &json!([])
            || v == &json!({})
            || v == &json!(0)
            || v == &json!(false)),
        "probe worker failures"
    );
    Ok(())
}
