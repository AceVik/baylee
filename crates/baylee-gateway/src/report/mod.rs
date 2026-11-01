//! `POST /reports` (#307): a signed-in player's bug report, crash report or
//! feedback, passed on to the feedback service (`docs/feedback.md`).
//!
//! The gateway adds what only it knows: which gateway and build this is, a
//! pseudonym for the reporter that is the same for every report of one
//! account on this gateway and says nothing else about it, and, when the
//! report names a game the reporter sat at, that game's record (#315). It
//! never passes on a name, an address or an IP.
//!
//! A report about a game the client hosted itself (a local game against
//! the house, no engine of the gateway's) may carry that game's record as
//! the client wrote it (`local_record`). The gateway cannot vouch for it and
//! does not pretend to: it bounds its size, checks it is gzip, and passes it
//! on marked as the client's (`record_origin: "client"`), which the service
//! checks for a record's shape and keeps apart from the gateway's records.
//! Filed with Central, which never unpacks a part, the gateway checks that
//! shape itself first ([`checked`]). A report is about one game or the
//! other, never both.
//!
//! Configured by `BAYLEE_FEEDBACK_URL` (the service; unset = `503`),
//! `BAYLEE_FEEDBACK_TOKEN` (this gateway's intake token there) and
//! `BAYLEE_FEEDBACK_KEY` (the key the pseudonym is made with; unset = one
//! derived from the token, so rotating the token renames every reporter).
//!
//! `BAYLEE_REPORTS_TO=central` files them with Baylee Central instead, under
//! the gateway's own Central link (`central.rs`), in the shape Central takes
//! (`central.rs` here, baylee-central `docs/reports.md`); a reported chat
//! message still goes to the service, which Central does not take.

use std::time::Duration;

use axum::Json;
use axum::body::Bytes;
use axum::extract::State;
use axum::http::{HeaderMap, StatusCode};
use base64::Engine as _;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::{ErrorBody, Shared, err};

mod central;
pub mod direct;

/// The longest report text, in characters.
pub const MAX_TEXT_CHARS: usize = 20_000;

/// The longest `game_id`, in characters.
pub const MAX_GAME_ID_CHARS: usize = 128;

/// The largest `client` object, serialized.
pub const MAX_CLIENT_BYTES: usize = 2 * 1024 * 1024;

/// The largest record a client may attach of a game it hosted itself,
/// compressed (`docs/feedback.md`): the service's own bound for a client's
/// record (`baylee_feedback::direct::MAX_CLIENT_RECORD_BYTES`).
pub const MAX_LOCAL_RECORD_BYTES: usize = 4 * 1024 * 1024;

/// The largest request body the route reads: a full `client` object, a
/// full text and a full local record in base64, with room for the JSON
/// around them.
pub const MAX_BODY_BYTES: usize =
    MAX_CLIENT_BYTES + MAX_LOCAL_RECORD_BYTES.div_ceil(3) * 4 + 256 * 1024;

/// How many reports one account may send per [`REPORT_WINDOW`].
pub const REPORTS_PER_WINDOW: usize = 64;

/// The window [`REPORTS_PER_WINDOW`] counts in.
pub const REPORT_WINDOW: Duration = Duration::from_secs(3600);

/// How long the service has to take a report.
const FORWARD_TIMEOUT: Duration = Duration::from_secs(20);

/// How long a report about a game that goes on waits for the game's engine
/// to send its record as it stands and for that to be stored (#323), before
/// it attaches what is there. An engine answers in milliseconds; this bounds
/// one that is busy, stuck or from before the ask (which never answers), and
/// it is what a player filing such a report can wait at most for it.
pub const FLUSH_WAIT: Duration = Duration::from_secs(3);

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
    /// `BAYLEE_REPORTS_TO=central`: reports go to Central, not the service.
    to_central: bool,
    /// Reports on their way to Central at once ([`central::IN_FLIGHT`]).
    in_flight: std::sync::Arc<tokio::sync::Semaphore>,
    /// Parts uploading to Central at once ([`central::UPLOADS`]).
    uploads: std::sync::Arc<tokio::sync::Semaphore>,
    /// `BAYLEE_DIRECT_REPORT_KEY`, hashed: what a device's pseudonym is made
    /// with; none = no direct reports (`direct.rs`).
    direct_key: Option<[u8; 32]>,
    /// Direct reports per address ([`direct::PER_ADDRESS`]).
    direct_per_address: crate::auth::RateLimiter,
    /// Direct reports from everyone ([`direct::PER_GATEWAY`]).
    direct_in_all: crate::auth::RateLimiter,
}

impl Feedback {
    /// Reads the variables.
    ///
    /// # Errors
    /// `BAYLEE_REPORTS_TO` other than `feedback` or `central`, or `central`
    /// without a key to make pseudonyms with (`BAYLEE_FEEDBACK_KEY`, or the
    /// token it is derived from): reporters must stay unlinkable to accounts;
    /// a `BAYLEE_DIRECT_REPORT_KEY` under 16 characters.
    pub fn from_env() -> Result<Self, &'static str> {
        let var = |name: &str| std::env::var(name).ok().filter(|v| !v.is_empty());
        let to_central = match var("BAYLEE_REPORTS_TO").as_deref() {
            None | Some("feedback") => false,
            Some("central") => true,
            Some(_) => return Err("BAYLEE_REPORTS_TO is `feedback` or `central`"),
        };
        if to_central
            && var("BAYLEE_FEEDBACK_KEY").is_none()
            && var("BAYLEE_FEEDBACK_TOKEN").is_none()
        {
            return Err(
                "BAYLEE_REPORTS_TO=central needs BAYLEE_FEEDBACK_KEY for the reporters' pseudonyms",
            );
        }
        let mut feedback = Self::new(
            var("BAYLEE_FEEDBACK_URL"),
            var("BAYLEE_FEEDBACK_TOKEN").unwrap_or_default(),
            var("BAYLEE_FEEDBACK_KEY").as_deref(),
            var("BAYLEE_PUBLIC_URL"),
        );
        feedback.to_central = to_central;
        feedback.direct_key = direct::key(var("BAYLEE_DIRECT_REPORT_KEY").as_deref())?;
        Ok(feedback)
    }

    /// The service at `url`, taking reports with `token`; pseudonyms made
    /// under `key`, or under one derived from `token` when there is none.
    fn new(
        url: Option<String>,
        token: String,
        key: Option<&str>,
        public_url: Option<String>,
    ) -> Self {
        let key = key.map_or_else(
            || {
                let mut h = Sha256::new();
                h.update(b"baylee reporter pseudonym\0");
                h.update(token.as_bytes());
                h.finalize().into()
            },
            |key| Sha256::digest(key.as_bytes()).into(),
        );
        Self {
            url: url.map(|u| u.trim_end_matches('/').to_owned()),
            token,
            key,
            public_url,
            limiter: crate::auth::RateLimiter::new(REPORT_WINDOW, REPORTS_PER_WINDOW),
            to_central: false,
            in_flight: std::sync::Arc::new(tokio::sync::Semaphore::new(central::IN_FLIGHT)),
            uploads: std::sync::Arc::new(tokio::sync::Semaphore::new(central::UPLOADS)),
            direct_key: None,
            direct_per_address: crate::auth::RateLimiter::new(REPORT_WINDOW, direct::PER_ADDRESS),
            direct_in_all: crate::auth::RateLimiter::new(REPORT_WINDOW, direct::PER_GATEWAY),
        }
    }

    /// Where a report goes and the token it is sent with, when reports are
    /// configured; for a reported chat message (`social::report`).
    pub(crate) fn target(&self) -> Option<(String, String)> {
        self.url.clone().map(|url| (url, self.token.clone()))
    }

    /// What this gateway calls its own address (`BAYLEE_PUBLIC_URL`).
    pub(crate) fn public_url(&self) -> Option<&str> {
        self.public_url.as_deref()
    }

    /// The reporter's pseudonym: HMAC-SHA256 of the account id under this
    /// gateway's key, in hex.
    pub(crate) fn pseudonym(&self, account_id: &str) -> String {
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
///
/// @description What a report is about: `bug` (something is broken),
/// `improvement`, `feedback`, `crash` (the client crashed; sent by the
/// client) or `other`.
/// @openapi
#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ReportKind {
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
///
/// @description A bug report, crash report or feedback. It is about one
/// game of this gateway's (`game_id`), one the client hosted itself
/// (`local_record`), or none; never both.
/// @openapi
#[derive(Deserialize)]
struct Report {
    /// @description What it is about.
    kind: ReportKind,
    /// @description What the player says, at most 20000 characters.
    text: String,
    /// @description The game of this gateway's it is about, at most 128
    /// characters; `null`, empty or left out for none. When the reporter sat
    /// at it, that game's record goes with the report.
    #[serde(default)]
    game_id: Option<String>,
    /// @description Anything the client wants to say: a JSON object, at most
    /// 2 MiB serialized.
    /// @openapi type: object
    client: serde_json::Value,
    /// @description The record of a game the client hosted itself, as it
    /// wrote it; never beside a `game_id`.
    #[serde(default)]
    local_record: Option<LocalRecord>,
}

/// A client's record of its own game, as the body carries it.
/// @openapi
#[derive(Deserialize)]
struct LocalRecord {
    /// @description Whether the game's end is in it.
    complete: bool,
    /// @description The record (the JSON Lines a game's engine writes),
    /// gzip, in standard base64: at most 4 MiB of gzip.
    gzip_base64: String,
}

/// A local record's bytes, or why not: base64 of gzip, at most
/// [`MAX_LOCAL_RECORD_BYTES`]. What is inside is checked by the service, or
/// before Central by [`checked`].
fn local_record(record: &LocalRecord) -> Result<Vec<u8>, (StatusCode, &'static str)> {
    if record.gzip_base64.len() > MAX_LOCAL_RECORD_BYTES.div_ceil(3) * 4 {
        return Err((StatusCode::PAYLOAD_TOO_LARGE, "the record is too large"));
    }
    let bytes = base64::engine::general_purpose::STANDARD
        .decode(&record.gzip_base64)
        .map_err(|_| (StatusCode::BAD_REQUEST, "the record is not base64"))?;
    if bytes.len() > MAX_LOCAL_RECORD_BYTES {
        return Err((StatusCode::PAYLOAD_TOO_LARGE, "the record is too large"));
    }
    if !bytes.starts_with(&[0x1f, 0x8b]) {
        return Err((StatusCode::BAD_REQUEST, "the record is not gzip"));
    }
    Ok(bytes)
}

/// A client's record as a game record, or why not, read as the feedback
/// service reads one (`baylee_account::client_record::check_record`): a
/// report filed with Central carries it unread, so the gateway is the one
/// that unpacks it, at most 32 MiB of it, off the request's thread.
async fn checked(
    record: central::RecordBytes,
) -> Result<central::RecordBytes, (StatusCode, Json<ErrorBody>)> {
    use baylee_account::client_record::{NotARecord, check_record};
    tokio::task::spawn_blocking(move || match check_record(&record.gzip) {
        Ok(()) => Ok(record),
        Err(NotARecord::TooLarge | NotARecord::UnpacksTooLarge) => {
            Err((StatusCode::PAYLOAD_TOO_LARGE, "the record is too large"))
        }
        Err(_) => Err((StatusCode::BAD_REQUEST, "the record is not a game record")),
    })
    .await
    .unwrap_or(Err((
        StatusCode::INTERNAL_SERVER_ERROR,
        "checking the record",
    )))
    .map_err(|(status, why)| err(status, why))
}

/// A request body as a report, and its `local_record`'s bytes; or why not
/// (`docs/feedback.md`, the table of answers). An empty `game_id` is no
/// game id.
fn validate(
    body: &[u8],
) -> Result<(Report, Option<central::RecordBytes>), (StatusCode, &'static str)> {
    if body.len() > MAX_BODY_BYTES {
        return Err((StatusCode::PAYLOAD_TOO_LARGE, "the report is too large"));
    }
    let mut report: Report =
        serde_json::from_slice(body).map_err(|_| (StatusCode::BAD_REQUEST, "not a report"))?;
    if !report.client.is_object() {
        return Err((StatusCode::BAD_REQUEST, "client must be an object"));
    }
    if report.text.chars().count() > MAX_TEXT_CHARS {
        return Err((StatusCode::PAYLOAD_TOO_LARGE, "the text is too long"));
    }
    let client_bytes = serde_json::to_vec(&report.client).map_or(usize::MAX, |v| v.len());
    if client_bytes > MAX_CLIENT_BYTES {
        return Err((
            StatusCode::PAYLOAD_TOO_LARGE,
            "the client details are too large",
        ));
    }
    report.game_id = report.game_id.filter(|g| !g.is_empty());
    if report
        .game_id
        .as_deref()
        .is_some_and(|g| g.chars().count() > MAX_GAME_ID_CHARS)
    {
        return Err((StatusCode::BAD_REQUEST, "not a game id"));
    }
    let local = match &report.local_record {
        // A record the client wrote never rides under a game of this
        // gateway's: whose record it would then be is not the client's to say.
        Some(_) if report.game_id.is_some() => {
            return Err((
                StatusCode::BAD_REQUEST,
                "a report is about a game of this gateway's or a local one, not both",
            ));
        }
        Some(record) => Some(central::RecordBytes {
            complete: record.complete,
            origin: baylee_account::reports::RecordOrigin::Client,
            gzip: local_record(record)?,
        }),
        None => None,
    };
    Ok((report, local))
}

/// What the service is sent.
#[derive(Serialize)]
struct Forward<'a> {
    gateway: GatewayInfo<'a>,
    reporter: String,
    kind: ReportKind,
    text: &'a str,
    game_id: Option<&'a str>,
    client: &'a serde_json::Value,
    record: Option<RecordOut>,
    /// Who wrote `record`: `"gateway"`, or `"client"` for a local game's.
    record_origin: &'static str,
}

#[derive(Serialize)]
pub(crate) struct GatewayInfo<'a> {
    pub(crate) name: Option<&'a str>,
    pub(crate) url: Option<&'a str>,
    pub(crate) version: &'static str,
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
///
/// @route POST /reports
/// @scope public
/// @operation-id `send_report`
/// @summary Send a bug report or feedback
/// @description A signed-in player's bug report, crash report or feedback,
/// guests included. The gateway passes it on to the feedback service, or
/// files it with Baylee Central where `BAYLEE_REPORTS_TO=central`, with its
/// own name and build, a pseudonym for the reporter, and the record of the
/// game it names when the reporter sat at it (or the client's own record of
/// a local game); never a name, address or IP. 64 reports an hour per
/// account; one that was not taken is not counted.
/// @tag Reports
/// @security bearer
/// @insert `SessionRefusals`
/// @body `$Report`
/// @return 201: `$ReportReceipt` "The feedback service took it, or Central filed it (its parts follow)"
/// @return 400: `$ErrorBody` "Not a report: not JSON, an unknown `kind`, no `text` (for Central: none left once what it does not show is taken out), `client` not an object, a `game_id` over 128 characters (for Central: or not printable ASCII), or a `local_record` beside a `game_id` or not base64 of gzip (for Central: or not a game record)"
/// @return 413: `$ErrorBody` "The text is over 20000 characters, `client` over 2 MiB, or the `local_record` over 4 MiB (for Central: or over 32 MiB unpacked); a body over 7.58 MiB is refused before it is read, in plain text"
/// @return 429: `$ErrorBody` "64 reports in the last hour from this account"
/// @return 500: `$ErrorBody` "The report could not be encoded"
/// @return 502: `$ErrorBody` "The feedback service or Central did not take it"
/// @return 503: `$ErrorBody` "Reports are not configured here (`BAYLEE_FEEDBACK_URL` unset, or with `BAYLEE_REPORTS_TO=central` no link to Central), too many reports are on their way to Central or it is past its budget for this gateway, or the account store, the database or Baylee Central is unavailable"
pub async fn post_report(
    State(state): State<Shared>,
    headers: HeaderMap,
    body: Bytes,
) -> Result<(StatusCode, Json<Created>), (StatusCode, Json<ErrorBody>)> {
    let session = crate::authed_session(&state, &headers).await?;
    let feedback = &state.feedback;
    let not_configured = || {
        err(
            StatusCode::SERVICE_UNAVAILABLE,
            "reports are not configured",
        )
    };
    // Where the report goes, settled before anything is read or counted.
    let to = if feedback.to_central {
        To::Central(state.central.get().ok_or_else(not_configured)?)
    } else {
        To::Service(feedback.url.clone().ok_or_else(not_configured)?)
    };
    let (report, local) = validate(&body).map_err(|(status, why)| err(status, why))?;
    let game_id = report.game_id.as_deref();
    // Checked after everything else, so a report refused for what it is
    // costs nothing of the budget, and before anything is sent; and never
    // given back unless this took it (`give_back` returns the latest).
    if !feedback.limiter.allow(&session.account_id) {
        return Err(err(StatusCode::TOO_MANY_REQUESTS, "too many reports"));
    }
    // A report that was not taken was not sent, and costs the player
    // nothing of the budget.
    let give_back = || feedback.limiter.give_back(&session.account_id);
    // Central's parts are held in memory on their way: only so many at once.
    let slot = match &to {
        To::Central(_) => Some(
            tokio::time::timeout(CENTRAL_QUEUE, feedback.in_flight.clone().acquire_owned())
                .await
                .ok()
                .and_then(Result::ok)
                .ok_or_else(|| {
                    give_back();
                    err(
                        StatusCode::SERVICE_UNAVAILABLE,
                        "too many reports on their way; try again",
                    )
                })?,
        ),
        To::Service(_) => None,
    };
    if let Some(game) = game_id {
        flush_record(&state, game, &session.account_id).await;
    }
    let record = match local {
        Some(local) => Some(local),
        None => gateway_record(&state, game_id, &session.account_id)
            .await
            .inspect_err(|_| give_back())?,
    };
    let out = Outgoing {
        gateway: GatewayInfo {
            name: state.display_name.as_deref(),
            url: feedback.public_url.as_deref(),
            version: baylee_build::short(),
        },
        reporter: feedback.pseudonym(&session.account_id),
        report: &report,
        game_id,
        record,
    };
    let sent = match to {
        To::Service(url) => to_service(feedback, url, out).await,
        To::Central(bridge) => to_central(feedback, bridge, slot, out).await,
    };
    let report_id = sent.inspect_err(|_| give_back())?;
    Ok((StatusCode::CREATED, Json(Created { report_id })))
}

/// A report on its way, once it was read and counted.
struct Outgoing<'a> {
    gateway: GatewayInfo<'a>,
    reporter: String,
    report: &'a Report,
    game_id: Option<&'a str>,
    record: Option<central::RecordBytes>,
}

/// Sends `out` to the feedback service at `url`, its record in base64; the
/// service's id for it.
async fn to_service(
    feedback: &Feedback,
    url: String,
    out: Outgoing<'_>,
) -> Result<String, (StatusCode, Json<ErrorBody>)> {
    let record_origin = out.record.as_ref().map_or("gateway", |r| match r.origin {
        baylee_account::reports::RecordOrigin::Client => "client",
        baylee_account::reports::RecordOrigin::Gateway => "gateway",
    });
    let forward = Forward {
        gateway: out.gateway,
        reporter: out.reporter,
        kind: out.report.kind,
        text: &out.report.text,
        game_id: out.game_id,
        client: &out.report.client,
        record: out.record.map(|r| RecordOut {
            complete: r.complete,
            gzip_base64: base64::engine::general_purpose::STANDARD.encode(r.gzip),
        }),
        record_origin,
    };
    let payload = serde_json::to_vec(&forward)
        .map_err(|_| err(StatusCode::INTERNAL_SERVER_ERROR, "encoding the report"))?;
    let token = feedback.token.clone();
    tokio::task::spawn_blocking(move || send(&url, &token, &payload))
        .await
        .unwrap_or_else(|_| Err("the sender stopped".to_owned()))
        .map_err(|why| {
            tracing::warn!("the feedback service did not take a report: {why}");
            err(
                StatusCode::BAD_GATEWAY,
                "the feedback service did not take the report",
            )
        })
}

/// Files `out` with Central through `bridge` (`central.rs`), holding `slot`
/// until its parts are sent; Central's id for it.
async fn to_central(
    feedback: &Feedback,
    bridge: std::sync::Arc<crate::central::Bridge>,
    slot: Option<tokio::sync::OwnedSemaphorePermit>,
    out: Outgoing<'_>,
) -> Result<String, (StatusCode, Json<ErrorBody>)> {
    use baylee_account::reports::{GatewaySays, said};
    let says = central::Said {
        gateway: GatewaySays {
            name: out.gateway.name.and_then(said),
            url: out.gateway.url.and_then(said),
            version: said(out.gateway.version),
        },
        channel: baylee_account::reports::Channel::Gateway,
        reporter: out.reporter,
        kind: out.report.kind,
        text: &out.report.text,
        game_id: out.game_id,
        client: &out.report.client,
        build: None,
    };
    file_with_central(feedback, bridge, &says, out.record, slot).await
}

/// Files `said` with Central through `bridge`, for either route: a client's
/// record checked ([`checked`]), the metadata filed, and Central's id
/// answered; the parts follow behind the answer, holding `slot`
/// ([`central::send_parts`]).
async fn file_with_central(
    feedback: &Feedback,
    bridge: std::sync::Arc<crate::central::Bridge>,
    said: &central::Said<'_>,
    record: Option<central::RecordBytes>,
    slot: Option<tokio::sync::OwnedSemaphorePermit>,
) -> Result<String, (StatusCode, Json<ErrorBody>)> {
    use baylee_account::reports::RecordOrigin;
    let record = match record {
        Some(r) if r.origin == RecordOrigin::Client => Some(checked(r).await?),
        other => other,
    };
    let filing = central::prepare(said, record).map_err(|why| err(StatusCode::BAD_REQUEST, why))?;
    let filer = bridge.clone();
    let (filed, filing) = tokio::task::spawn_blocking(move || {
        let filed = filer.file_report(&filing.intake);
        (filed, filing)
    })
    .await
    .map_err(|_| err(StatusCode::BAD_GATEWAY, "Central did not take the report"))?;
    let id = filed.map_err(|why| {
        tracing::warn!(?why, "Central did not take a report");
        match why {
            baylee_account::Error::Busy => err(
                StatusCode::SERVICE_UNAVAILABLE,
                "Central takes no more reports from this gateway for now",
            ),
            _ => err(StatusCode::BAD_GATEWAY, "Central did not take the report"),
        }
    })?;
    tokio::spawn(central::send_parts(
        bridge,
        id,
        filing,
        feedback.uploads.clone(),
        slot,
    ));
    Ok(baylee_account::reports::id_hex(&id))
}

/// Where a report goes.
enum To {
    /// The feedback service at this address.
    Service(String),
    /// Central, through the gateway's link.
    Central(std::sync::Arc<crate::central::Bridge>),
}

/// How long a report waits for one of [`central::IN_FLIGHT`] places before
/// the player is told to try again.
const CENTRAL_QUEUE: Duration = Duration::from_secs(10);

/// The record a report without a local one carries: the gateway's of the
/// game it names when `account_id` sat at it; none else.
async fn gateway_record(
    state: &Shared,
    game_id: Option<&str>,
    account_id: &str,
) -> Result<Option<central::RecordBytes>, (StatusCode, Json<ErrorBody>)> {
    use baylee_account::reports::RecordOrigin;
    let (Some(game), Ok(account)) = (game_id, uuid::Uuid::parse_str(account_id)) else {
        return Ok(None);
    };
    let stored = baylee_db::records::for_seated(&state.db, game, account)
        .await
        .map_err(|e| crate::db_down(&anyhow::Error::from(e)))?;
    Ok(stored.map(|r| central::RecordBytes {
        complete: r.complete,
        origin: RecordOrigin::Gateway,
        gzip: r.data,
    }))
}

/// Asks the engine of `game_id` for its record as it stands and waits, up to
/// [`FLUSH_WAIT`], until what it sent is stored (#323), when `account_id`
/// sits at that game and it goes on; otherwise asks nothing. Whether it was
/// stored in time.
///
/// Never an error: a report is not refused because its game's engine is
/// slow or gone. The record attached is then what was stored before, and
/// `complete` says what it said before: whether the game's end is in it.
async fn flush_record(state: &Shared, game_id: &str, account_id: &str) -> bool {
    // The link is cloned out, so no lock is held across the wait. A game
    // has one exactly while it goes on: it is set when the engine attaches
    // and cleared when the game ends (`LobbyGame::finish`), after the last
    // piece.
    let engine = {
        let lobby = state.lobby.lock();
        lobby
            .games
            .get(game_id)
            .filter(|game| {
                game.seats
                    .iter()
                    .any(|seat| seat.account_id.as_deref() == Some(account_id))
            })
            .and_then(|game| game.engine.clone())
    };
    let Some(engine) = engine else {
        return false;
    };
    let flushes = &state.record_flushes;
    // Forgotten when this ends, however it ends: a report whose request
    // is cancelled while it waits leaves no ask behind.
    let (asked, stored) = flushes.ask(game_id);
    let ask = baylee_protocol::v1::Envelope {
        msg: Some(baylee_protocol::v1::envelope::Msg::FlushRecord(
            baylee_protocol::v1::FlushRecord {
                game_id: game_id.to_owned(),
                nonce: asked.nonce,
            },
        )),
    };
    if engine.send(ask).is_err() {
        return false;
    }
    if let Ok(Ok(())) = tokio::time::timeout(FLUSH_WAIT, stored).await {
        return true;
    }
    tracing::warn!(
        game_id,
        "the engine did not send its record in time; the report carries what was stored"
    );
    false
}

/// Posts one report to the service; its id, or why not.
pub(crate) fn send(url: &str, token: &str, payload: &[u8]) -> Result<String, String> {
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

    #[allow(clippy::needless_pass_by_value)] // the callers build the values in place
    fn body(
        kind: &str,
        text: &str,
        game_id: serde_json::Value,
        client: serde_json::Value,
    ) -> Vec<u8> {
        serde_json::json!({ "kind": kind, "text": text, "game_id": game_id, "client": client })
            .to_string()
            .into_bytes()
    }

    fn status(body: &[u8]) -> Option<StatusCode> {
        validate(body).err().map(|(status, _)| status)
    }

    /// Every field's bound from both sides: the longest the gateway takes
    /// and one more (`docs/feedback.md`).
    #[test]
    fn every_bound_holds_at_its_edge() {
        let none = serde_json::Value::Null;
        let empty = serde_json::json!({});
        // Characters, not bytes: `é` is two.
        let text = |n: usize| "é".repeat(n);
        assert_eq!(
            status(&body(
                "bug",
                &text(MAX_TEXT_CHARS),
                none.clone(),
                empty.clone()
            )),
            None
        );
        assert_eq!(
            status(&body(
                "bug",
                &text(MAX_TEXT_CHARS + 1),
                none.clone(),
                empty.clone()
            )),
            Some(StatusCode::PAYLOAD_TOO_LARGE)
        );
        let game = |n: usize| serde_json::json!("g".repeat(n));
        assert_eq!(
            status(&body("bug", "", game(MAX_GAME_ID_CHARS), empty.clone())),
            None
        );
        assert_eq!(
            status(&body("bug", "", game(MAX_GAME_ID_CHARS + 1), empty.clone())),
            Some(StatusCode::BAD_REQUEST)
        );
        // `{"l":"…"}` is 8 bytes around the value.
        let client = |n: usize| serde_json::json!({ "l": "x".repeat(n - 8) });
        assert_eq!(
            status(&body("crash", "", none.clone(), client(MAX_CLIENT_BYTES))),
            None
        );
        assert_eq!(
            status(&body(
                "crash",
                "",
                none.clone(),
                client(MAX_CLIENT_BYTES + 1)
            )),
            Some(StatusCode::PAYLOAD_TOO_LARGE)
        );
        let mut huge = body("bug", "", none, empty);
        huge.resize(MAX_BODY_BYTES + 1, b' ');
        assert_eq!(status(&huge), Some(StatusCode::PAYLOAD_TOO_LARGE));
    }

    #[test]
    fn a_report_is_read_as_the_contract_says() {
        let empty = serde_json::json!({});
        for kind in ["bug", "improvement", "feedback", "crash", "other"] {
            let (report, _) = validate(&body(kind, "t", serde_json::Value::Null, empty.clone()))
                .unwrap_or_else(|e| panic!("{kind}: {e:?}"));
            assert_eq!(serde_json::to_value(report.kind).unwrap(), kind);
        }
        for bad in [
            body("rant", "t", serde_json::Value::Null, empty.clone()),
            body("Bug", "t", serde_json::Value::Null, empty.clone()),
            body("bug", "t", serde_json::Value::Null, serde_json::json!([])),
            body("bug", "t", serde_json::Value::Null, serde_json::json!("x")),
            body("bug", "t", serde_json::json!(7), empty.clone()),
            br#"{"kind":"bug","game_id":null,"client":{}}"#.to_vec(),
            br#"{"kind":"bug","text":"t","game_id":null}"#.to_vec(),
            b"not json".to_vec(),
        ] {
            assert_eq!(
                status(&bad),
                Some(StatusCode::BAD_REQUEST),
                "{}",
                String::from_utf8_lossy(&bad)
            );
        }
        // No game, said three ways.
        for game in [
            br#"{"kind":"bug","text":"","client":{}}"#.to_vec(),
            body("bug", "", serde_json::Value::Null, empty.clone()),
            body("bug", "", serde_json::json!(""), empty.clone()),
        ] {
            assert_eq!(validate(&game).map(|(r, _)| r.game_id).ok(), Some(None));
        }
        assert_eq!(
            validate(&body("bug", "", serde_json::json!("g1"), empty))
                .map(|(r, _)| r.game_id)
                .ok(),
            Some(Some("g1".to_owned()))
        );
    }

    /// Without a key the pseudonym is made under one derived from the
    /// token, so rotating the token renames every reporter; with one, the
    /// token can change and nobody is renamed.
    #[test]
    fn a_pseudonym_follows_the_key_and_not_the_token() {
        let account = "0190a1b2-0000-7000-8000-000000000001";
        let at = |token: &str, key: Option<&str>| {
            Feedback::new(None, token.to_owned(), key, None).pseudonym(account)
        };
        assert_eq!(at("token-one", None), at("token-one", None));
        assert_ne!(at("token-one", None), at("token-two", None));
        assert_eq!(at("token-one", Some("key")), at("token-two", Some("key")));
        assert_ne!(
            at("token-one", Some("key")),
            at("token-one", Some("other key"))
        );
        assert_ne!(at("token-one", Some("key")), at("token-one", None));
        // HMAC under SHA-256 of the key, which a reader can check by hand.
        let key: [u8; 32] = Sha256::digest(b"key").into();
        assert_eq!(
            at("anything", Some("key")),
            hex(&hmac_sha256(&key, account.as_bytes()))
        );
    }

    /// The budget: [`REPORTS_PER_WINDOW`] in a window, per account, a slot
    /// given back is spendable again, and the window slides.
    #[test]
    fn the_budget_is_per_account_and_slides() {
        let feedback = Feedback::new(Some("http://x".into()), "t".into(), None, None);
        let limiter = &feedback.limiter;
        let start = std::time::Instant::now();
        for i in 0..REPORTS_PER_WINDOW {
            assert!(limiter.allow_at("a", start), "report {i}");
        }
        assert!(!limiter.allow_at("a", start), "one past the budget");
        assert!(limiter.allow_at("b", start), "another account");
        limiter.give_back("a");
        assert!(limiter.allow_at("a", start), "a slot given back");
        assert!(!limiter.allow_at("a", start));
        let almost = (start + REPORT_WINDOW)
            .checked_sub(Duration::from_millis(1))
            .unwrap();
        assert!(!limiter.allow_at("a", almost), "the window has not passed");
        assert!(
            limiter.allow_at("a", start + REPORT_WINDOW),
            "and now it has"
        );

        // The same edge where the periodic sweep cannot have cleared it: the
        // budget spent half a window in, a sweep at a full window, and the
        // edge half a window after that, which the key's own count decides.
        let spent = start + REPORT_WINDOW / 2;
        for _ in 0..REPORTS_PER_WINDOW {
            assert!(limiter.allow_at("c", spent));
        }
        assert!(limiter.allow_at("d", start + REPORT_WINDOW), "a sweep");
        let edge = spent + REPORT_WINDOW;
        let before = edge.checked_sub(Duration::from_millis(1)).unwrap();
        assert!(!limiter.allow_at("c", before));
        assert!(limiter.allow_at("c", edge), "a window after it was spent");
    }

    #[test]
    fn a_pseudonym_is_stable_and_names_nobody() {
        let feedback = Feedback {
            url: None,
            token: "t".into(),
            key: [7; 32],
            public_url: None,
            limiter: crate::auth::RateLimiter::new(REPORT_WINDOW, 1),
            to_central: false,
            in_flight: std::sync::Arc::new(tokio::sync::Semaphore::new(1)),
            uploads: std::sync::Arc::new(tokio::sync::Semaphore::new(1)),
            direct_key: None,
            direct_per_address: crate::auth::RateLimiter::new(REPORT_WINDOW, 1),
            direct_in_all: crate::auth::RateLimiter::new(REPORT_WINDOW, 1),
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
