//! Copy original artifacts into a new, independently reverified collector root.
use super::{game::Receipt, hash, json_file, lines, read};
use anyhow::{Context, Result, ensure};
use baylee_gamehost::record::Line;
use serde_json::{Value, json};
use std::{fs, path::Path};

pub(super) fn publish(
    original: &Path,
    destination: &Path,
    audit: &Value,
    build: &str,
) -> Result<()> {
    let source = Receipt::load(original)?;
    ensure!(
        source.technical["status"] == "failed"
            && audit["full_engine_hashes_exact"] == true
            && audit["native_public_projector_verified"] == true
            && audit["game_id"] == source.game,
        "Recovery requires an independent exact replay"
    );
    let dest = std::path::absolute(destination)?;
    let root_name = dest
        .file_name()
        .context("Missing snapshot name")?
        .to_string_lossy();
    let staging = dest.with_file_name(format!("{root_name}.partial"));
    ensure!(
        !dest.exists() && !staging.exists(),
        "Recovery output or staging already exists"
    );
    let name = original
        .file_name()
        .context("Missing original attempt name")?;
    let attempt = Path::new("attempts").join(name);
    let temp = staging.join(&attempt);
    let final_path = dest.join(&attempt);
    fs::create_dir(&staging)?;
    fs::create_dir_all(&temp)?;
    let mut artifacts = Vec::new();
    for (role, path) in &source.artifacts {
        let relative = path.strip_prefix(original)?;
        let raw = read(path)?;
        pin(&temp, &final_path, relative, &raw, role, &mut artifacts)?;
    }
    for seat in 0..2 {
        let relative = format!("transcripts/{}-seat{seat}-mind.jsonl", source.game);
        let bytes = read(&original.join(&relative))?;
        ensure!(
            hash(&bytes) == audit["seats"][seat]["provider_transcript_sha256"],
            "Raw provider data changed after verification"
        );
        pin(
            &temp,
            &final_path,
            Path::new(&relative),
            &bytes,
            &format!("provider_transcript_seat{seat}"),
            &mut artifacts,
        )?;
    }
    let original_receipt = read(&original.join("receipt.json"))?;
    let original_result = read(&original.join("technical-result.json"))?;
    let lineage = json!({
        "kind":"independent_offline_recovery",
        "original_attempt":original, "original_receipt_sha256":hash(&original_receipt),
        "original_technical_result_sha256":hash(&original_result),
        "original_error":source.technical["error_message"],"supported_loop_policy":"Off",
        "engine_commit":build,"provider_calls":0,
        "record_sha256":audit["record_artifact_sha256"],"public_tape_sha256":audit["public_tape_sha256"],
    });
    pin(
        &temp,
        &final_path,
        Path::new("recovery-receipt.json"),
        &serde_json::to_vec_pretty(&lineage)?,
        "offline_recovery_receipt",
        &mut artifacts,
    )?;
    let replay = json!({
        "status":"verified", "method":"independent_exact_standard_replay_and_public_tape",
        "gateway_info":{"commit":build}, "stop":audit["public_tape_stop"],
        "record_sha256":audit["record_artifact_sha256"],"tape_sha256":audit["public_tape_sha256"],
    });
    pin(
        &temp,
        &final_path,
        Path::new("replay-receipt.json"),
        &serde_json::to_vec_pretty(&replay)?,
        "replay_receipt",
        &mut artifacts,
    )?;
    let record: Vec<Line> = lines(&read(source.artifact("anonymized_record")?)?)?;
    let technical = technical(&source, audit, &record, &artifacts)?;
    fs::write(
        temp.join("request.json"),
        read(&original.join("request.json"))?,
    )?;
    fs::write(
        temp.join("technical-result.json"),
        serde_json::to_vec_pretty(&technical)?,
    )?;
    fs::write(
        temp.join("receipt.json"),
        serde_json::to_vec_pretty(&json!({
            "status":"admitted","admission":"independent offline technical BC-only recovery",
            "request":source.request,"technical":technical
        }))?,
    )?;
    fs::rename(staging, dest)?;
    Ok(())
}

fn pin(
    temp: &Path,
    final_path: &Path,
    relative: &Path,
    bytes: &[u8],
    role: &str,
    artifacts: &mut Vec<Value>,
) -> Result<()> {
    let staging_file = temp.join(relative);
    fs::create_dir_all(staging_file.parent().context("Invalid pin path")?)?;
    fs::write(staging_file, bytes)?;
    artifacts.push(json!({"path":final_path.join(relative),"role":role,"sha256":hash(bytes)}));
    Ok(())
}

fn technical(
    source: &Receipt,
    audit: &Value,
    record: &[Line],
    artifacts: &[Value],
) -> Result<Value> {
    let Some(Line::Header { preset, .. }) = record.first() else {
        anyhow::bail!("Missing record header");
    };
    let Some(Line::End { winners, .. }) = record.last() else {
        anyhow::bail!("Missing record end");
    };
    let bindings = json_file(source.artifact("input_receipt")?)?;
    let mut seats = Vec::new();
    for seat in 0..2 {
        let provenance = &audit["seats"][seat];
        let origins = &provenance["accepted_input_origins"];
        let count = |key| origins[key].as_u64().unwrap_or(0);
        ensure!(
            count("ambiguous") == 0 && count("other") == 0 && count("provider_uncertain") == 0,
            "Recovery requires unambiguous source per accepted input"
        );
        let declared: Vec<_> = record
            .iter()
            .filter_map(|line| match line {
                Line::DeclaredMind {
                    n, seat: s, mind, ..
                } if usize::from(*s) == seat => Some(json!({"record_n":n,"mind":mind})),
                _ => None,
            })
            .collect();
        let house = count("house");
        let mind = count("local_mind") + count("provider");
        seats.push(json!({
            "seat":seat,"model":source.request["models"][seat],"declared_mind_verified":true,"declared_minds":declared,
            "transcript_answer_counts":{"house":house,"llm":mind,"other":0,"standing":count("standing")},
            "executed_input_sources":{"seat":provenance["accepted_socket_inputs"]},
            "profile_fallbacks":{"refused":[],"played":[],"took_over":[]},
        }));
    }
    Ok(json!({
        "schema_version":1,"status":"complete","game_id":source.game,
        "actual_seed":preset.seed,"requested_room_setup":bindings["requested_room_setup"],
        "actual_room_setup":{"actual_record_format":preset.format,"free_mulligans":preset.house_rules.free_mulligans,
            "mulligan_free_first":preset.house_rules.mulligan_free_first,
            "seat_starting_life":preset.seats.iter().map(|seat| seat.starting_life).collect::<Vec<_>>()},
        "terminal":{"draw":winners.is_empty(),"winner_seat":winners.first()},
        "seats":seats,"checks":{"connection":true,"view_boundary":true,"full_record":true,"replay":true,"provenance":true},
        "artifacts":artifacts,"record_sha256":audit["record_artifact_sha256"],
        "bc_only":true,"ppo_eligible":false,"policy_targets":"none_until_ordered_provenance_review",
        "offline_recovery":true
    }))
}
