//! Bind the independently replayed Astra report to actual provider and bridge logs.
use super::{
    json_lines,
    schema::{FilePin, Manifest, SeatManifest},
};
use anyhow::{Context, Result, ensure};
use baylee_train_ops::corpus::llm_freeze::verify_provider_audit_identity;
use serde::Deserialize;
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};

#[derive(Deserialize)]
pub(super) struct AuditedGame {
    game_id: String,
    record_artifact_sha256: String,
    seats: [AuditedSeat; 2],
}

#[derive(Deserialize)]
struct AuditedSeat {
    seat: u8,
    provider_transcript_sha256: String,
    seat_transcript_sha256: String,
    responses: Vec<Response>,
}

#[derive(Deserialize)]
struct Response {
    reply_line: usize,
    question: u64,
    direct_label_candidate: bool,
    execution: Option<Execution>,
}

#[derive(Deserialize)]
struct Execution {
    input_n: u64,
    send_line: usize,
    ask_line: usize,
    ask_seq: u64,
    send_seq: u64,
    direct: bool,
    same_context: bool,
    action: Value,
    menu_sha256: String,
    seat_view_sha256: String,
}

impl AuditedGame {
    pub(super) fn load(
        pin: &FilePin,
        manifest: &Manifest,
        technical: &Value,
        requested_models: &Value,
        input_receipt: Option<&Value>,
    ) -> Result<Self> {
        let raw = pin.json()?;
        verify_provider_audit_identity(&raw, requested_models, input_receipt)?;
        let audit: Self = serde_json::from_value(raw)?;
        ensure!(
            technical["game_id"] == audit.game_id
                && manifest.record_artifact_sha256 == audit.record_artifact_sha256,
            "Provider audit game or record mismatch"
        );
        Ok(audit)
    }

    #[allow(
        clippy::too_many_lines,
        reason = "One pass binds every audited reply to its executed input"
    )]
    pub(super) fn direct(
        &self,
        seat: usize,
        manifest: &SeatManifest,
        transcript: &[Value],
        inputs: &[Value],
        provenance: &Value,
    ) -> Result<BTreeSet<u64>> {
        let audited = &self.seats[seat];
        let pin = manifest
            .provider_transcript
            .as_ref()
            .context("Missing provider transcript")?;
        ensure!(
            usize::from(audited.seat) == seat
                && audited.provider_transcript_sha256 == pin.sha256
                && audited.seat_transcript_sha256 == manifest.transcript.sha256,
            "Provider audit seat or transcript identity mismatch"
        );
        let provider = json_lines(&pin.bytes()?)?;
        let replies: BTreeSet<_> = provider
            .iter()
            .enumerate()
            .filter(|(_, row)| row["reply"].is_object())
            .map(|(i, _)| i + 1)
            .collect();
        let mut registered = BTreeSet::new();
        let mut executed = BTreeSet::new();
        let mut direct = BTreeSet::new();
        let origins = provenance["inputs"]
            .as_array()
            .context("Missing input provenance")?;
        let input_map = inputs
            .iter()
            .zip(origins)
            .map(|(input, origin)| {
                let n = input["n"].as_u64().context("Missing record input number")?;
                Ok((n, (input, origin)))
            })
            .collect::<Result<BTreeMap<_, _>>>()?;
        for row in &audited.responses {
            ensure!(
                replies.contains(&row.reply_line) && registered.insert(row.reply_line),
                "Provider audit repeats or invents a response"
            );
            let provider_row = &provider[row.reply_line - 1];
            ensure!(
                provider_row["question"] == row.question,
                "Provider response question mismatch"
            );
            let Some(execution) = &row.execution else {
                ensure!(
                    !row.direct_label_candidate,
                    "Missing executed action for policy label"
                );
                continue;
            };
            let bridge_row = |line: usize| {
                line.checked_sub(1)
                    .and_then(|i| transcript.get(i))
                    .context("Provider audit bridge line missing")
            };
            let send = bridge_row(execution.send_line)?;
            let asked = bridge_row(execution.ask_line)?;
            let (input, origin) = input_map
                .get(&execution.input_n)
                .context("Audited input absent")?;
            ensure!(
                executed.insert(execution.input_n)
                    && execution.ask_line < execution.send_line
                    && send["event"] == "answered"
                    && send["by"] == "mind"
                    && asked["event"] == "asked"
                    && asked["seq"] == execution.ask_seq
                    && send["seq"] == execution.send_seq
                    && asked["question"] == row.question
                    && send["question"] == row.question
                    && send["action"] == input["action"]
                    && execution.action == input["action"]
                    && provider_row["error"].is_null()
                    && provider_row["reply"]["answer"]["ask"] == format!("q{}", row.question)
                    && provider_row["reply"]["text"].is_string()
                    && provider_row["reply"]["text"] == send["thinking"]
                    && origin["possible_send_count"] == 1
                    && origin["ambiguous"] == false
                    && origin["possible_sources"] == serde_json::json!(["llm"]),
                "Provider audit response, send or accepted input mismatch"
            );
            let note: Value = serde_json::from_str(
                send["note"]
                    .as_str()
                    .context("Missing provider choice note")?,
            )?;
            ensure!(
                note["tokens"] == provider_row["reply"]["usage"],
                "Provider usage mismatch"
            );
            let prompt = provider_row["message"]
                .as_str()
                .context("Missing provider prompt")?;
            let (_, menu) = prompt
                .split_once("\nQUESTION:")
                .context("Missing legal menu")?;
            ensure!(
                super::sha(menu.trim_end().as_bytes()) == execution.menu_sha256
                    && execution.seat_view_sha256.len() == 64,
                "Provider legal menu or seat view mismatch"
            );
            if row.direct_label_candidate {
                ensure!(
                    execution.ask_seq == execution.send_seq
                        && execution.direct
                        && execution.same_context,
                    "Direct label must retain the answering seat's original information context"
                );
                direct.insert(execution.input_n);
            }
        }
        ensure!(registered == replies, "Provider audit omitted a response");
        Ok(direct)
    }
}
