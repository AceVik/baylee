//! `POST /social/reports` (§4.7): one delivered message, revealed by its
//! recipient, checked and forwarded to the feedback service as
//! `kind: "chat"`.
//!
//! The reporter sends the message as it was delivered, its message key and
//! the body it decrypted to. The gateway looks the sending device's
//! identity key up, checks the signature and that the key opens the
//! ciphertext to exactly that body (`baylee_seal::verify_report`), and
//! forwards the text with `verified` saying whether both held. A device
//! removed since cannot be checked: the report goes on unverified. Revealing
//! the key reveals this one message and nothing else. Nothing of it is
//! logged.
//!
//! Who can be found from a report: the sender of a verified message, by its
//! public tag (`client.chat.sender_tag`), so a moderator can act on what was
//! written (`GET /admin/accounts?tag=`); never the reporter, who reaches the
//! service as a keyed pseudonym only.

use super::routes::caller;
use crate::{ErrorBody, Shared, err};
use axum::extract::State;
use axum::http::{HeaderMap, StatusCode};
use axum::response::Json;
use baylee_protocol::social::{self as wire, ChatReport, Conv, Header, ReportReceipt};
use baylee_seal::b64::url as b64;
use serde::Serialize;

type Answer<T> = Result<T, (StatusCode, Json<ErrorBody>)>;

/// The largest body this route reads.
pub(crate) const MAX_REPORT_BODY: usize = 64 * 1024;

#[derive(Serialize)]
struct Forward<'a> {
    gateway: crate::report::GatewayInfo<'a>,
    reporter: String,
    kind: &'static str,
    text: String,
    game_id: Option<String>,
    client: serde_json::Value,
    record: Option<()>,
    record_origin: &'static str,
}

fn bad(why: &'static str) -> (StatusCode, Json<ErrorBody>) {
    err(StatusCode::BAD_REQUEST, why)
}

/// What kind of conversation `conv` is, for the moderator.
fn conv_kind(conv: &Conv) -> &'static str {
    match conv {
        Conv::Dm { .. } => "whisper",
        Conv::Group { .. } => "group",
        Conv::Table { .. } => "table",
        Conv::Team { .. } => "team",
    }
}

/// The message's text out of its body, which is the sender's JSON; the
/// body as text when it has none.
fn message_text(body: &[u8]) -> String {
    serde_json::from_slice::<serde_json::Value>(body)
        .ok()
        .and_then(|v| v.get("text").and_then(|t| t.as_str()).map(str::to_owned))
        .unwrap_or_else(|| String::from_utf8_lossy(body).into_owned())
}

/// `POST /social/reports`.
#[allow(clippy::too_many_lines)] // the checks, in their order, read top to bottom
pub(crate) async fn post_report(
    State(state): State<Shared>,
    headers: HeaderMap,
    body: axum::body::Bytes,
) -> Answer<(StatusCode, Json<ReportReceipt>)> {
    let caller = caller(&state, &headers).await?;
    if body.len() > MAX_REPORT_BODY {
        return Err(err(
            StatusCode::PAYLOAD_TOO_LARGE,
            "the report is too large",
        ));
    }
    let report: ChatReport = serde_json::from_slice(&body).map_err(|_| bad("not a report"))?;
    let conv: Conv = report.conv.parse().map_err(|_| bad("not a conversation"))?;
    if report.note.chars().count() > wire::MAX_REPORT_NOTE {
        return Err(bad("the note is too long"));
    }
    let hdr = b64::decode(&report.sealed.hdr).map_err(|_| bad("not a message"))?;
    let n = b64::decode(&report.sealed.n).map_err(|_| bad("not a message"))?;
    let ct = b64::decode(&report.sealed.ct).map_err(|_| bad("not a message"))?;
    let epk = b64::decode(&report.sealed.epk).map_err(|_| bad("not a message"))?;
    let sig = b64::decode(&report.sealed.sig).map_err(|_| bad("not a message"))?;
    let mk: [u8; 32] = b64::decode_array(&report.mk).map_err(|_| bad("not a message key"))?;
    let plain = b64::decode(&report.body).map_err(|_| bad("not a body"))?;
    if hdr.len() > wire::MAX_HEADER_BYTES || plain.len() > wire::MAX_BODY_BYTES {
        return Err(bad("not a message"));
    }
    let header: Header = serde_json::from_slice(&hdr).map_err(|_| bad("not a message"))?;
    if header.conv != report.conv {
        return Err(bad("the message is of another conversation"));
    }
    let Some((url, token)) = state.feedback.target() else {
        return Err(err(
            StatusCode::SERVICE_UNAVAILABLE,
            "reports are not configured",
        ));
    };
    if !state.social.reports.allow(&caller.account) {
        return Err(err(StatusCode::TOO_MANY_REQUESTS, "too many reports"));
    }
    // The sending device, if it is still there, and only if it is the
    // account the header names.
    let device = baylee_db::social::device_by_id(&state.db, &header.dev)
        .await
        .map_err(|e| crate::db_down(&e.into()))?;
    let sender = match &device {
        Some(device) => baylee_db::social::who_by_ids(&state.db, &[device.account_id])
            .await
            .map_err(|e| crate::db_down(&e.into()))?
            .into_iter()
            .next()
            .filter(|who| crate::handle::tag_text(who.tag) == header.from),
        None => None,
    };
    let verified = match (&device, &sender) {
        (Some(device), Some(_)) => <[u8; 32]>::try_from(device.ik.as_slice()).is_ok_and(|ik| {
            let envelope = baylee_seal::Envelope {
                hdr: &hdr,
                n: &n,
                ct: &ct,
                epk: &epk,
                sig: &sig,
            };
            baylee_seal::verify_report(&ik, &envelope, &mk, &plain).is_ok()
        }),
        _ => false,
    };
    let feedback = &state.feedback;
    let message = message_text(&plain);
    let text = if report.note.trim().is_empty() {
        format!("Reported chat message:\n{message}")
    } else {
        format!("{}\n\nReported chat message:\n{message}", report.note)
    };
    let forward = Forward {
        gateway: crate::report::GatewayInfo {
            name: state.display_name.as_deref(),
            url: feedback.public_url(),
            version: baylee_build::short(),
        },
        reporter: feedback.pseudonym(&caller.account),
        kind: "chat",
        text,
        game_id: conv.game().map(str::to_owned),
        client: serde_json::json!({
            "chat": {
                "verified": verified,
                "conversation": conv_kind(&conv),
                // The sender's public tag, so a moderator can find the
                // account and act on what it wrote: only when the check
                // held, because an unverified report could name anyone.
                // No sender pseudonym beside it: it is the same keyed hash
                // a reporter is known by, and paired with the tag it would
                // resolve every report that account ever filed. The
                // reporter stays a pseudonym and is never resolvable.
                "sender_tag": sender
                    .as_ref()
                    .filter(|_| verified)
                    .map(|who| crate::handle::tag_text(who.tag)),
                "sent_at": header.ts,
                "message": message,
                "note": report.note,
            }
        }),
        record: None,
        record_origin: "gateway",
    };
    let payload = serde_json::to_vec(&forward)
        .map_err(|_| err(StatusCode::INTERNAL_SERVER_ERROR, "encoding the report"))?;
    let sent = tokio::task::spawn_blocking(move || crate::report::send(&url, &token, &payload))
        .await
        .unwrap_or_else(|_| Err("the sender stopped".to_owned()));
    if let Ok(report_id) = sent {
        tracing::info!(verified, "a chat message was reported");
        Ok((StatusCode::CREATED, Json(ReportReceipt { report_id })))
    } else {
        tracing::warn!("the feedback service did not take a chat report");
        Err(err(
            StatusCode::BAD_GATEWAY,
            "the feedback service did not take the report",
        ))
    }
}
