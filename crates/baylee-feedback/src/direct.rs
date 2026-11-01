//! `POST /client/reports`: a report straight from a client that is signed
//! in to no gateway (`docs/feedback.md` §"Straight from a client").
//!
//! A shipped client holds no secret, so this route has none to ask for. It
//! stands on its limits instead: a small body, a bounded record that must
//! read as a game record, a per-address and a service-wide allowance per
//! hour (in memory, never written down), and a reporter that is the
//! service's HMAC of a random id the client made for its device, so a
//! report names neither an account nor an address. What it takes is kept
//! apart from what gateways pass on: `channel = 'direct'`, its gateway
//! `(direct)` (a name no gateway token can carry), its record marked as the
//! client's (`record_origin = 'client'`).
//!
//! Off until `FEEDBACK_DIRECT_KEY` is set (`503` before then).

use std::collections::{HashMap, VecDeque};

use axum::Json;
use axum::body::Bytes;
use axum::extract::{ConnectInfo, State};
use axum::http::{HeaderMap, StatusCode};
use base64::Engine as _;
use serde::Deserialize;
use sha2::{Digest, Sha256};
use time::OffsetDateTime;

use crate::{Created, Kind, MAX_TEXT_CHARS, NewReport, Refusal, Shared, refuse, store};

/// The largest `client` object a direct report carries, serialized: half
/// what a gateway may pass on, because nobody vouches for this one.
pub const MAX_CLIENT_BYTES: usize = 1_048_576;

/// A client's record and its check, shared with the official gateway,
/// which checks one before it files it with Central.
pub use baylee_account::client_record::{
    MAX_CLIENT_RECORD_BYTES, MAX_CLIENT_RECORD_UNPACKED, NotARecord, check_record,
};

/// The largest body the route reads: a whole client object, a whole record
/// in base64 and a whole text, with room for the JSON around them.
pub const MAX_BODY_BYTES: usize =
    MAX_CLIENT_BYTES + MAX_CLIENT_RECORD_BYTES.div_ceil(3) * 4 + 128 * 1024;

/// Direct reports one address may send per [`WINDOW`].
pub const PER_ADDRESS: usize = 10;

/// Direct reports the service takes per [`WINDOW`] from everyone together:
/// what a crowd of addresses can cost it at most.
pub const PER_SERVICE: usize = 600;

/// The window the allowances count in.
pub const WINDOW: time::Duration = time::Duration::HOUR;

/// What the gateway column says for a direct report. No gateway can be
/// configured under it: a gateway name holds no parentheses.
pub const DIRECT_GATEWAY: &str = "(direct)";

/// The longest a client's own device id is: 32 hex digits, exactly.
const DEVICE_CHARS: usize = 32;

/// Direct reports in the last [`WINDOW`], per address and in all.
///
/// Memory only: a restart forgets it, and no address is ever written down.
/// Every request is counted, a refused one too, so probing costs as much
/// as sending.
#[derive(Default)]
pub struct Allowance {
    by_address: HashMap<String, VecDeque<OffsetDateTime>>,
    all: VecDeque<OffsetDateTime>,
}

impl Allowance {
    /// The most addresses it holds before it sweeps out every stale one.
    const SWEEP_AT: usize = 4096;

    /// Whether a request from `address` may be taken at `now`; counted if so.
    pub fn take(&mut self, address: &str, now: OffsetDateTime) -> bool {
        self.take_within(address, now, PER_ADDRESS, PER_SERVICE)
    }

    /// [`Allowance::take`] under other bounds: `per_address` and
    /// `per_service` requests per [`WINDOW`].
    pub fn take_within(
        &mut self,
        address: &str,
        now: OffsetDateTime,
        per_address: usize,
        per_service: usize,
    ) -> bool {
        while self.all.front().is_some_and(|t| now - *t >= WINDOW) {
            self.all.pop_front();
        }
        if self.by_address.len() >= Self::SWEEP_AT {
            self.by_address
                .retain(|_, times| times.back().is_some_and(|t| now - *t < WINDOW));
        }
        let times = self.by_address.entry(address.to_owned()).or_default();
        while times.front().is_some_and(|t| now - *t >= WINDOW) {
            times.pop_front();
        }
        if times.len() >= per_address || self.all.len() >= per_service {
            return false;
        }
        times.push_back(now);
        self.all.push_back(now);
        true
    }
}

/// HMAC-SHA256 (RFC 2104) under a 32-byte key, as the gateway makes its
/// reporter pseudonyms.
pub(crate) fn hmac_sha256(key: &[u8; 32], message: &[u8]) -> [u8; 32] {
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

/// The reporter a device's id is filed under: never the id itself, so the
/// table cannot be matched against a device that kept its settings file.
#[must_use]
pub fn pseudonym(key: &[u8; 32], device: &str) -> String {
    hex(&hmac_sha256(key, format!("device\0{device}").as_bytes()))
}

/// A record as a client attaches it.
///
/// @description A game record a client attaches: at most 4 MiB of gzip
/// that unpacks to at most 32 MiB of JSON Lines, a header of record version
/// 1 first and nothing after it but input, chair and end lines.
/// @openapi
#[derive(Deserialize)]
pub(crate) struct ClientRecord {
    /// @description Whether the record reaches the game's end.
    pub(crate) complete: bool,
    /// @description The record: gzip of JSON Lines, in standard base64.
    pub(crate) gzip_base64: String,
}

impl ClientRecord {
    /// The record's bytes, checked ([`check_record`]), or the refusal.
    pub(crate) fn bytes(self) -> Result<(Vec<u8>, bool), Refusal> {
        if self.gzip_base64.len() > MAX_CLIENT_RECORD_BYTES.div_ceil(3) * 4 {
            return Err(refuse(
                StatusCode::PAYLOAD_TOO_LARGE,
                "the record is too large",
            ));
        }
        let bytes = base64::engine::general_purpose::STANDARD
            .decode(self.gzip_base64)
            .map_err(|_| refuse(StatusCode::BAD_REQUEST, "the record is not base64"))?;
        match check_record(&bytes) {
            Ok(()) => Ok((bytes, self.complete)),
            Err(NotARecord::TooLarge | NotARecord::UnpacksTooLarge) => Err(refuse(
                StatusCode::PAYLOAD_TOO_LARGE,
                "the record is too large",
            )),
            Err(_) => Err(refuse(
                StatusCode::BAD_REQUEST,
                "the record is not a game record",
            )),
        }
    }
}

/// The build a client says it is.
/// @openapi
#[derive(Deserialize)]
struct BuildIn {
    /// @description The client's release.
    version: String,
    /// @description The commit it was built from, if it knows.
    #[serde(default)]
    commit: Option<String>,
}

/// What a client sends (`docs/feedback.md` §"Straight from a client").
///
/// @description A report from a client signed in to no gateway; every
/// field named, no other taken.
/// @openapi
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Direct {
    /// @description What the report is about; never `chat`, which goes
    /// through the message's gateway.
    kind: Kind,
    /// @description What the player wrote, at most 20000 characters; empty
    /// only for a crash.
    text: String,
    /// @description The client's build: its version and commit together at
    /// most 128 characters, not empty.
    build: BuildIn,
    /// @description A random id the client made for its reports: 32
    /// lowercase hex digits. The report is filed under this service's HMAC of
    /// it, never the id.
    /// @openapi pattern: '^[0-9a-f]{32}$'
    device: String,
    /// @description What the client says about itself: a JSON object of at
    /// most 1 MiB serialized.
    client: serde_json::Value,
    /// @description The game's record, if one is attached.
    #[serde(default)]
    record: Option<ClientRecord>,
}

/// Any page may post here: a browser build of the client is served from
/// its own origin, never this service's. Only on this route, and never
/// with `Allow-Credentials`: the UI's session cookie is this origin's own
/// and stays so, and a direct report carries no credential to send.
pub(crate) async fn any_origin(mut response: axum::response::Response) -> axum::response::Response {
    response.headers_mut().insert(
        axum::http::header::ACCESS_CONTROL_ALLOW_ORIGIN,
        axum::http::HeaderValue::from_static("*"),
    );
    response
}

/// The browser's preflight for [`post`]: a JSON body is what makes one.
///
/// @route OPTIONS /client/reports
/// @route OPTIONS /client/alive
/// @scope feedback
/// @summary A browser's preflight for a direct report or a beat
/// @description Says yes to any origin: `POST`, `Content-Type`, for a day.
/// @tag Clients
/// @security none
/// @return 204: "Allowed"
/// @return-header 204 Access-Control-Allow-Origin: String "`*`"
/// @return-header 204 Access-Control-Allow-Methods: String "`POST, OPTIONS`"
/// @return-header 204 Access-Control-Allow-Headers: String "`content-type`"
/// @return-header 204 Access-Control-Max-Age: String "`86400`"
pub(crate) async fn preflight() -> axum::response::Response {
    use axum::http::{HeaderValue, header};
    let mut response = axum::response::Response::new(axum::body::Body::empty());
    *response.status_mut() = StatusCode::NO_CONTENT;
    let headers = response.headers_mut();
    headers.insert(
        header::ACCESS_CONTROL_ALLOW_METHODS,
        HeaderValue::from_static("POST, OPTIONS"),
    );
    headers.insert(
        header::ACCESS_CONTROL_ALLOW_HEADERS,
        HeaderValue::from_static("content-type"),
    );
    headers.insert(
        header::ACCESS_CONTROL_MAX_AGE,
        HeaderValue::from_static("86400"),
    );
    response
}

/// `POST /client/reports`.
///
/// @route POST /client/reports
/// @scope feedback
/// @summary Take a report straight from a client
/// @description A report from a client signed in to no gateway. It asks
/// for no credential and stands on its limits: a bounded body, a record that
/// must read as a game record, and 10 reports per address and 600 from
/// everyone per hour, every request counted, a refused one too. Off until
/// `FEEDBACK_DIRECT_KEY` is set. Any origin may post (`Access-Control-Allow-Origin: *`,
/// never with credentials).
/// @tag Clients
/// @operation-id `postDirectReport`
/// @security none
/// @body `$Direct`
/// @return 201: `$Created` "Taken"
/// @return 400: `$ErrorBody` "Not a report: malformed JSON, an unknown field or `kind`, a `chat` report, no text (but for a crash), `client` not an object, a malformed `device` or `build`, or a record that is not base64 or not a game record"
/// @return 413: `$ErrorBody` "The text is over 20000 characters, `client` over 1 MiB, or the record over 4 MiB (32 MiB unpacked)"
/// @return 413: String text/plain "The text, `client` or the record is over its bound (an `ErrorBody`), or the whole body is over the route's limit, which is refused in plain text before anything else is checked"
/// @return 429: `$ErrorBody` "Too many reports from this address or from everyone in the last hour"
/// @return 503: `$ErrorBody` "Direct reports are not taken here (`FEEDBACK_DIRECT_KEY` is unset), or the database is unavailable"
pub(crate) async fn post(
    State(shared): State<Shared>,
    peer: Option<axum::Extension<ConnectInfo<std::net::SocketAddr>>>,
    headers: HeaderMap,
    body: Bytes,
) -> Result<(StatusCode, Json<Created>), Refusal> {
    let Shared { state, ui } = shared;
    let Some(key) = state.config.direct_key() else {
        return Err(refuse(
            StatusCode::SERVICE_UNAVAILABLE,
            "direct reports are not taken here",
        ));
    };
    let address = ui.address(peer.map(|axum::Extension(ConnectInfo(a))| a.ip()), &headers);
    if !ui.allow_direct(&address) {
        return Err(refuse(StatusCode::TOO_MANY_REQUESTS, "too many reports"));
    }
    if body.len() > MAX_BODY_BYTES {
        return Err(refuse(
            StatusCode::PAYLOAD_TOO_LARGE,
            "the report is too large",
        ));
    }
    let bad = |why| refuse(StatusCode::BAD_REQUEST, why);
    let report: Direct = serde_json::from_slice(&body).map_err(|_| bad("not a report"))?;
    if !report.client.is_object() {
        return Err(bad("client must be an object"));
    }
    // A chat report is checked by the gateway the message went through;
    // a client signed in nowhere has none to send.
    if report.kind == Kind::Chat {
        return Err(bad("a chat message is reported through its gateway"));
    }
    if report.text.trim().is_empty() && report.kind != Kind::Crash {
        return Err(bad("a report says something"));
    }
    if report.text.chars().count() > MAX_TEXT_CHARS {
        return Err(refuse(
            StatusCode::PAYLOAD_TOO_LARGE,
            "the text is too long",
        ));
    }
    let client = serde_json::to_string(&report.client).map_err(|_| bad("client"))?;
    if client.len() > MAX_CLIENT_BYTES {
        return Err(refuse(
            StatusCode::PAYLOAD_TOO_LARGE,
            "the client details are too large",
        ));
    }
    let device_fine = report.device.len() == DEVICE_CHARS
        && report
            .device
            .chars()
            .all(|c| c.is_ascii_digit() || ('a'..='f').contains(&c));
    let version = match &report.build.commit {
        Some(commit) => format!("{} ({commit})", report.build.version),
        None => report.build.version.clone(),
    };
    if !device_fine || !crate::fits(Some(&version), 128) || version.is_empty() {
        return Err(bad("a field is malformed"));
    }
    let record = report.record.map(ClientRecord::bytes).transpose()?;
    let id = store(
        &state.db,
        NewReport {
            gateway: DIRECT_GATEWAY.to_owned(),
            gateway_name: None,
            gateway_url: None,
            gateway_version: version,
            reporter: pseudonym(key, &report.device),
            kind: report.kind,
            text: report.text,
            game_id: None,
            client,
            record,
            record_origin: crate::Origin::Client,
            channel: crate::Channel::Direct,
        },
    )
    .await?;
    tracing::info!(report = %id, kind = report.kind.name(), "direct report taken");
    Ok((
        StatusCode::CREATED,
        Json(Created {
            report_id: id.to_string(),
        }),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    /// An address has [`PER_ADDRESS`] an hour and the service [`PER_SERVICE`]
    /// from everyone; an hour later both are whole again.
    #[test]
    fn the_allowance_counts_per_address_and_in_all() {
        let mut shelf = Allowance::default();
        let now = OffsetDateTime::UNIX_EPOCH + time::Duration::days(20_000);
        for _ in 0..PER_ADDRESS {
            assert!(shelf.take("a", now));
        }
        assert!(!shelf.take("a", now), "one address is held to its share");
        assert!(shelf.take("b", now), "another is not");
        assert!(shelf.take("a", now + WINDOW), "an hour later it may again");
        let mut crowd = Allowance::default();
        for n in 0..PER_SERVICE {
            assert!(crowd.take(&format!("x{n}"), now));
        }
        assert!(!crowd.take("fresh", now), "the service's own bound holds");
    }

    /// A device's reporter is the same each time, differs per device and
    /// per key, and is never the id.
    #[test]
    fn a_device_is_filed_under_a_pseudonym() {
        let key = [7_u8; 32];
        let device = "0123456789abcdef0123456789abcdef";
        let one = pseudonym(&key, device);
        assert_eq!(one, pseudonym(&key, device));
        assert_ne!(one, pseudonym(&[8_u8; 32], device));
        assert_ne!(one, pseudonym(&key, "fedcba9876543210fedcba9876543210"));
        assert!(!one.contains(device));
    }
}
