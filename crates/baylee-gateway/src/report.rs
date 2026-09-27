//! `POST /reports` (#307): a signed-in player's bug report, crash report or
//! feedback, passed on to the feedback service (`docs/feedback.md`).
//!
//! The gateway adds what only it knows: which gateway and build this is, a
//! pseudonym for the reporter that is the same for every report of one
//! account on this gateway and says nothing else about it, and, when the
//! report names a game the reporter sat at, that game's record (#315). It
//! never passes on a name, an address or an IP.
//!
//! Configured by `BAYLEE_FEEDBACK_URL` (the service; unset = `503`),
//! `BAYLEE_FEEDBACK_TOKEN` (this gateway's intake token there) and
//! `BAYLEE_FEEDBACK_KEY` (the key the pseudonym is made with; unset = one
//! derived from the token, so rotating the token renames every reporter).

use std::time::Duration;

use axum::Json;
use axum::body::Bytes;
use axum::extract::State;
use axum::http::{HeaderMap, StatusCode};
use base64::Engine as _;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::{ErrorBody, Shared, err};

/// The longest report text, in characters.
pub const MAX_TEXT_CHARS: usize = 20_000;

/// The largest `client` object, serialized.
pub const MAX_CLIENT_BYTES: usize = 2 * 1024 * 1024;

/// The largest request body the route reads: a full `client` object and a
/// full text, with room for the JSON around them.
pub const MAX_BODY_BYTES: usize = MAX_CLIENT_BYTES + 256 * 1024;

/// How many reports one account may send per [`REPORT_WINDOW`].
pub const REPORTS_PER_WINDOW: usize = 20;

/// The window [`REPORTS_PER_WINDOW`] counts in.
pub const REPORT_WINDOW: Duration = Duration::from_secs(3600);

/// How long the service has to take a report.
const FORWARD_TIMEOUT: Duration = Duration::from_secs(20);

/// Where reports go, from the environment.
pub struct Feedback {
    /// `BAYLEE_FEEDBACK_URL` without a trailing slash; `None` = not
    /// configured.
    url: Option<String>,
    /// `BAYLEE_FEEDBACK_TOKEN`.
    token: String,
    /// What the pseudonyms are made with.
    key: [u8; 32],
    /// `BAYLEE_PUBLIC_URL`, what the gateway calls its own address.
    public_url: Option<String>,
    /// Reports per account ([`REPORTS_PER_WINDOW`]). A limiter of its own,
    /// so a client sending crash reports never costs its player a sign-in.
    limiter: crate::auth::RateLimiter,
}

impl Feedback {
    /// Reads the three variables.
    pub fn from_env() -> Self {
        let var = |name: &str| std::env::var(name).ok().filter(|v| !v.is_empty());
        let token = var("BAYLEE_FEEDBACK_TOKEN").unwrap_or_default();
        let key = var("BAYLEE_FEEDBACK_KEY").map_or_else(
            || {
                let mut h = Sha256::new();
                h.update(b"baylee reporter pseudonym\0");
                h.update(token.as_bytes());
                h.finalize().into()
            },
            |key| Sha256::digest(key.as_bytes()).into(),
        );
        Self {
            url: var("BAYLEE_FEEDBACK_URL").map(|u| u.trim_end_matches('/').to_owned()),
            token,
            key,
            public_url: var("BAYLEE_PUBLIC_URL"),
            limiter: crate::auth::RateLimiter::new(REPORT_WINDOW, REPORTS_PER_WINDOW),
        }
    }

    /// The reporter's pseudonym: HMAC-SHA256 of the account id under this
    /// gateway's key, in hex.
    fn pseudonym(&self, account_id: &str) -> String {
        hex(&hmac_sha256(&self.key, account_id.as_bytes()))
    }
}

/// HMAC-SHA256 (RFC 2104), from the hash the gateway already links.
fn hmac_sha256(key: &[u8; 32], message: &[u8]) -> [u8; 32] {
    const BLOCK: usize = 64;
    let mut inner = [0x36_u8; BLOCK];
    let mut outer = [0x5c_u8; BLOCK];
    for (i, k) in key.iter().enumerate() {
        inner[i] ^= k;
        outer[i] ^= k;
    }
    let inside = Sha256::new()
        .chain_update(inner)
        .chain_update(message)
        .finalize();
    Sha256::new()
        .chain_update(outer)
        .chain_update(inside)
        .finalize()
        .into()
}

fn hex(bytes: &[u8]) -> String {
    use std::fmt::Write as _;
    bytes
        .iter()
        .fold(String::with_capacity(bytes.len() * 2), |mut s, b| {
            let _ = write!(s, "{b:02x}");
            s
        })
}

/// What a report is about.
#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Kind {
    /// Something is broken.
    Bug,
    /// Something could be better.
    Improvement,
    /// Anything the player wants to say.
    Feedback,
    /// The client crashed; sent by the client.
    Crash,
    /// None of these.
    Other,
}

/// The request body (frozen: `docs/feedback.md`).
#[derive(Deserialize)]
struct Report {
    kind: Kind,
    text: String,
    #[serde(default)]
    game_id: Option<String>,
    client: serde_json::Value,
}

/// What the service is sent.
#[derive(Serialize)]
struct Forward<'a> {
    gateway: GatewayInfo<'a>,
    reporter: String,
    kind: Kind,
    text: &'a str,
    game_id: Option<&'a str>,
    client: &'a serde_json::Value,
    record: Option<RecordOut>,
}

#[derive(Serialize)]
struct GatewayInfo<'a> {
    name: Option<&'a str>,
    url: Option<&'a str>,
    version: &'static str,
}

/// A game's record as the service is sent it.
#[derive(Serialize)]
struct RecordOut {
    /// Whether the last piece arrived before the engine went.
    complete: bool,
    /// The record, gzip, in standard base64.
    gzip_base64: String,
}

#[derive(Serialize)]
pub struct Created {
    report_id: String,
}

#[derive(Deserialize)]
struct ServiceCreated {
    report_id: String,
}

/// The route, with a body limit of its own: a whole `client` object is
/// larger than axum's default.
pub fn routes() -> axum::Router<Shared> {
    axum::Router::new().route(
        "/reports",
        axum::routing::post(post_report)
            .layer(axum::extract::DefaultBodyLimit::max(MAX_BODY_BYTES)),
    )
}

/// `POST /reports`.
pub async fn post_report(
    State(state): State<Shared>,
    headers: HeaderMap,
    body: Bytes,
) -> Result<(StatusCode, Json<Created>), (StatusCode, Json<ErrorBody>)> {
    let session = crate::authed_session(&state, &headers).await?;
    let feedback = &state.feedback;
    let Some(url) = feedback.url.clone() else {
        return Err(err(
            StatusCode::SERVICE_UNAVAILABLE,
            "reports are not configured",
        ));
    };
    if body.len() > MAX_BODY_BYTES {
        return Err(err(
            StatusCode::PAYLOAD_TOO_LARGE,
            "the report is too large",
        ));
    }
    let report: Report =
        serde_json::from_slice(&body).map_err(|_| err(StatusCode::BAD_REQUEST, "not a report"))?;
    if !report.client.is_object() {
        return Err(err(StatusCode::BAD_REQUEST, "client must be an object"));
    }
    if report.text.chars().count() > MAX_TEXT_CHARS {
        return Err(err(StatusCode::PAYLOAD_TOO_LARGE, "the text is too long"));
    }
    let client_bytes = serde_json::to_vec(&report.client).map_or(usize::MAX, |v| v.len());
    if client_bytes > MAX_CLIENT_BYTES {
        return Err(err(
            StatusCode::PAYLOAD_TOO_LARGE,
            "the client details are too large",
        ));
    }
    if !feedback.limiter.allow(&session.account_id) {
        return Err(err(StatusCode::TOO_MANY_REQUESTS, "too many reports"));
    }

    let game_id = report
        .game_id
        .as_deref()
        .filter(|g| !g.is_empty() && g.len() <= 128);
    let record = match (game_id, uuid::Uuid::parse_str(&session.account_id)) {
        (Some(game), Ok(account)) => baylee_db::records::for_seated(&state.db, game, account)
            .await
            .map_err(|e| crate::db_down(&anyhow::Error::from(e)))?
            .map(|r| RecordOut {
                complete: r.complete,
                gzip_base64: base64::engine::general_purpose::STANDARD.encode(r.data),
            }),
        _ => None,
    };
    let forward = Forward {
        gateway: GatewayInfo {
            name: state.display_name.as_deref(),
            url: feedback.public_url.as_deref(),
            version: baylee_build::short(),
        },
        reporter: feedback.pseudonym(&session.account_id),
        kind: report.kind,
        text: &report.text,
        game_id,
        client: &report.client,
        record,
    };
    let payload = serde_json::to_vec(&forward)
        .map_err(|_| err(StatusCode::INTERNAL_SERVER_ERROR, "encoding the report"))?;
    let token = feedback.token.clone();
    let sent = tokio::task::spawn_blocking(move || send(&url, &token, &payload))
        .await
        .unwrap_or_else(|_| Err("the sender stopped".to_owned()));
    match sent {
        Ok(report_id) => Ok((StatusCode::CREATED, Json(Created { report_id }))),
        Err(why) => {
            tracing::warn!("the feedback service did not take a report: {why}");
            Err(err(
                StatusCode::BAD_GATEWAY,
                "the feedback service did not take the report",
            ))
        }
    }
}

/// Posts one report to the service; its id, or why not.
fn send(url: &str, token: &str, payload: &[u8]) -> Result<String, String> {
    let agent: ureq::Agent = ureq::Agent::config_builder()
        .timeout_global(Some(FORWARD_TIMEOUT))
        .build()
        .into();
    let mut response = agent
        .post(format!("{url}/intake/reports"))
        .header("Authorization", format!("Bearer {token}"))
        .header("Content-Type", "application/json")
        .send(payload)
        .map_err(|e| e.to_string())?;
    let created: ServiceCreated = response
        .body_mut()
        .with_config()
        .limit(64 * 1024)
        .read_json()
        .map_err(|e| e.to_string())?;
    Ok(created.report_id)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// RFC 4231, test case 2, with its key padded the way a 32-byte key is.
    /// The key there is 4 bytes; HMAC pads short keys with zeros, which is
    /// what a 32-byte array holding them does too.
    #[test]
    fn hmac_matches_rfc_4231() {
        let mut key = [0_u8; 32];
        key[..4].copy_from_slice(b"Jefe");
        assert_eq!(
            hex(&hmac_sha256(&key, b"what do ya want for nothing?")),
            "5bdcc146bf60754e6a042426089575c75a003f089d2739839dec58b964ec3843"
        );
    }

    #[test]
    fn a_pseudonym_is_stable_and_names_nobody() {
        let feedback = Feedback {
            url: None,
            token: "t".into(),
            key: [7; 32],
            public_url: None,
            limiter: crate::auth::RateLimiter::new(REPORT_WINDOW, 1),
        };
        let a = feedback.pseudonym("0190a1b2-0000-7000-8000-000000000001");
        assert_eq!(
            a,
            feedback.pseudonym("0190a1b2-0000-7000-8000-000000000001")
        );
        assert_ne!(
            a,
            feedback.pseudonym("0190a1b2-0000-7000-8000-000000000002")
        );
        assert!(!a.contains("0190a1b2"));
        assert_eq!(a.len(), 64);
    }
}
