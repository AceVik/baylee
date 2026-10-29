use super::{Input, LlmBc4, alignment, artifact, json_lines, schema::Manifest, sha};
use anyhow::{Context, ensure};
use baylee_engine::choice::PlayerAction;
use serde_json::Value;
use std::collections::BTreeMap;

#[allow(
    clippy::too_many_lines,
    reason = "Keep game-level LLM provenance validation in one linear pass"
)]
pub(super) fn load(
    run: &[u8],
    manifest: &[u8],
    record: &[u8],
    game: &str,
) -> anyhow::Result<LlmBc4> {
    let run: Value = serde_json::from_slice(run)?;
    ensure!(
        run["llm_bc_version"] == 1,
        "Missing explicit LLM run contract"
    );
    let manifest: Manifest = serde_json::from_slice(manifest)?;
    ensure!(
        manifest.schema_version == 1
            && manifest.contract == "admitted_executed_llm_v1"
            && manifest.source_game == game,
        "LLM manifest contract/game mismatch"
    );
    ensure!(
        manifest.record_sha256 == sha(record),
        "LLM record hash mismatch"
    );
    let request = manifest.request.json()?;
    let receipt = manifest.admitted_receipt.json()?;
    let technical = manifest.technical_result.json()?;
    ensure!(
        receipt["status"] == "admitted"
            && receipt["request"] == request
            && receipt["technical"] == technical,
        "LLM admission receipt mismatch"
    );
    ensure!(
        technical["schema_version"] == 1
            && technical["status"] == "complete"
            && technical["record_sha256"] == manifest.record_artifact_sha256,
        "Incomplete LLM technical receipt"
    );
    ensure!(
        request["attempt_id"] == game && request["models"].as_array().is_some_and(|v| v.len() == 2),
        "LLM request source/model count mismatch"
    );
    ensure!(
        request["bc_only"] == true
            && request["ppo_eligible"] == false
            && request["semantic_quality_gate"] == false,
        "LLM run must be technical BC-only"
    );
    for check in [
        "connection",
        "view_boundary",
        "full_record",
        "replay",
        "provenance",
    ] {
        ensure!(
            technical["checks"][check] == true,
            "LLM technical check missing"
        );
    }
    artifact(
        &technical,
        "anonymized_record",
        &manifest.record_artifact_sha256,
    )?;
    let rows = json_lines(record)?;
    ensure!(
        rows.first().is_some_and(|v| v["kind"] == "header")
            && rows.last().is_some_and(|v| v["kind"] == "end"),
        "Incomplete LLM record"
    );
    let header = &rows[0];
    ensure!(
        header["preset"]["seats"]
            .as_array()
            .is_some_and(|v| v.len() == 2)
            && technical["actual_seed"] == header["preset"]["seed"],
        "LLM seat count/actual seed mismatch"
    );
    ensure!(
        manifest.seats.len() == 2 && technical["seats"].as_array().is_some_and(|v| v.len() == 2),
        "Two LLM seats required"
    );
    let winners = rows.last().context("LLM End")?["winners"]
        .as_array()
        .context("LLM winners")?;
    ensure!(
        winners.len() <= 1 && winners.iter().all(|v| v.as_u64().is_some_and(|s| s < 2)),
        "Invalid duel terminal outcome"
    );
    ensure!(
        technical["terminal"]["draw"] == winners.is_empty()
            && technical["terminal"]["winner_seat"]
                == winners.first().cloned().unwrap_or(Value::Null),
        "LLM terminal receipt mismatch"
    );
    let input_receipt = if let Some(pin) = &manifest.input_receipt {
        artifact(&technical, "input_receipt", &pin.sha256)?;
        let value = pin.json()?;
        ensure!(
            value["requested_models"] == request["models"]
                && value["requested_minds"]
                    .as_array()
                    .is_some_and(|v| v.len() == 2),
            "LLM requested identity receipt mismatch"
        );
        Some(value)
    } else {
        None
    };
    let mut remaining = BTreeMap::new();
    for (seat, evidence) in manifest.seats.iter().enumerate() {
        ensure!(
            usize::from(evidence.seat) == seat && request["models"][seat] == evidence.model,
            "LLM requested model/seat mismatch"
        );
        ensure!(
            technical["seats"][seat]["seat"] == seat
                && technical["seats"][seat]["model"] == evidence.model
                && technical["seats"][seat]["declared_mind_verified"] == true,
            "LLM technical model binding mismatch"
        );
        let expected_provider = if evidence.model == "gpt-6.1-terra" {
            "codex"
        } else {
            "agy"
        };
        ensure!(
            expected_provider != "codex" || input_receipt.is_some(),
            "Codex provenance requires the bound requested identity receipt"
        );
        if let Some(input) = &input_receipt {
            let mind = &input["requested_minds"][seat];
            ensure!(
                mind["kind"] == "llm_cli"
                    && mind["provider"] == expected_provider
                    && mind["model"] == evidence.model
                    && mind["effort"] == "low"
                    && mind.get("level").is_none_or(|v| v == ""),
                "LLM requested provider/model identity mismatch"
            );
        }
        let declarations: Vec<_> = rows
            .iter()
            .filter(|v| v["kind"] == "declared_mind" && v["seat"] == seat)
            .collect();
        ensure!(
            !declarations.is_empty()
                && declarations.iter().all(|v| v["mind"]["kind"] == "llm_cli"
                    && v["mind"]["provider"] == expected_provider
                    && v["mind"]["model"] == evidence.model
                    && v["mind"]["effort"] == "low"
                    && v["mind"].get("level").is_none_or(|v| v == "")
                    && input_receipt
                        .as_ref()
                        .is_none_or(|input| input["requested_minds"][seat] == v["mind"])),
            "LLM actual declared model mismatch"
        );
        let inputs: Vec<_> = rows
            .iter()
            .filter(|v| v["kind"] == "input" && v["seat"] == seat && v["by"] == "seat")
            .cloned()
            .collect();
        let transcript = evidence.transcript.bytes()?;
        artifact(
            &technical,
            &format!("transcript_seat{seat}"),
            &evidence.transcript.sha256,
        )?;
        artifact(
            &technical,
            &format!("executed_provenance_seat{seat}"),
            &evidence.origins.sha256,
        )?;
        let origins = evidence.origins.json()?;
        let eligible = alignment::verify(&json_lines(&transcript)?, &inputs, &origins)?;
        for (row, eligible) in inputs.into_iter().zip(eligible) {
            let action: PlayerAction = serde_json::from_value(row["action"].clone())?;
            if action.is_automation_setting() {
                continue;
            }
            let n = row["n"].as_u64().context("LLM input number")?;
            ensure!(
                remaining
                    .insert(
                        n,
                        Input {
                            seat: evidence.seat,
                            action,
                            model: evidence.model.clone(),
                            eligible
                        }
                    )
                    .is_none(),
                "Duplicate LLM record input"
            );
        }
    }
    Ok(LlmBc4 { remaining })
}
