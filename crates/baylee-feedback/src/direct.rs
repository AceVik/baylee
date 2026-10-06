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
use std::io::Read as _;

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

/// The largest record a client may attach, compressed (either route).
pub const MAX_CLIENT_RECORD_BYTES: usize = 4 * 1024 * 1024;

/// The most a client's record may unpack to: a bound on what reading it
/// costs, so a small stream that unpacks to gigabytes is refused, not read.
pub const MAX_CLIENT_RECORD_UNPACKED: u64 = 32 * 1024 * 1024;

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
        if times.len() >= PER_ADDRESS || self.all.len() >= PER_SERVICE {
            return false;
        }
        times.push_back(now);
        self.all.push_back(now);
        true
    }
}

/// HMAC-SHA256 (RFC 2104) under a 32-byte key, as the gateway makes its
/// reporter pseudonyms.
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

/// The reporter a device's id is filed under: never the id itself, so the
/// table cannot be matched against a device that kept its settings file.
#[must_use]
pub fn pseudonym(key: &[u8; 32], device: &str) -> String {
    hex(&hmac_sha256(key, format!("device\0{device}").as_bytes()))
}

/// Why a client's record is refused.
#[derive(Debug, PartialEq, Eq)]
pub enum NotARecord {
    /// Over [`MAX_CLIENT_RECORD_BYTES`] compressed.
    TooLarge,
    /// Over [`MAX_CLIENT_RECORD_UNPACKED`] unpacked.
    UnpacksTooLarge,
    /// Not gzip, or not text.
    Unreadable,
    /// It does not read as a game record (`docs/protocol.md` §"The game
    /// record"): a header first and only there, every line one of the
    /// record's lines.
    NotARecord,
}

/// Checks that `gzip` reads as a game record, without the engine: a
/// header of the record version this service knows first, and only once,
/// then inputs, chair changes and an end, each a JSON object. Whether it
/// replays is for the build its header names (`baylee_gamehost::record`),
/// and nothing here claims it does.
///
/// # Errors
///
/// [`NotARecord`] says which way it is not one.
pub fn check_record(gzip: &[u8]) -> Result<(), NotARecord> {
    if gzip.len() > MAX_CLIENT_RECORD_BYTES {
        return Err(NotARecord::TooLarge);
    }
    let mut text = String::new();
    flate2::read::GzDecoder::new(gzip)
        .take(MAX_CLIENT_RECORD_UNPACKED + 1)
        .read_to_string(&mut text)
        .map_err(|_| NotARecord::Unreadable)?;
    if text.len() as u64 > MAX_CLIENT_RECORD_UNPACKED {
        return Err(NotARecord::UnpacksTooLarge);
    }
    let mut lines = text.lines().filter(|l| !l.trim().is_empty());
    let header: serde_json::Value = lines
        .next()
        .and_then(|l| serde_json::from_str(l).ok())
        .ok_or(NotARecord::NotARecord)?;
    let is_header = header.get("kind").and_then(serde_json::Value::as_str) == Some("header")
        && header.get("record").and_then(serde_json::Value::as_u64) == Some(1)
        && header
            .get("build")
            .is_some_and(serde_json::Value::is_string);
    if !is_header {
        return Err(NotARecord::NotARecord);
    }
    for line in lines {
        let value: serde_json::Value =
            serde_json::from_str(line).map_err(|_| NotARecord::NotARecord)?;
        let kind = value.get("kind").and_then(serde_json::Value::as_str);
        if !matches!(kind, Some("input" | "chair" | "end")) {
            return Err(NotARecord::NotARecord);
        }
    }
    Ok(())
}

/// A record as a client attaches it.
#[derive(Deserialize)]
pub(crate) struct ClientRecord {
    pub(crate) complete: bool,
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
#[derive(Deserialize)]
struct BuildIn {
    version: String,
    #[serde(default)]
    commit: Option<String>,
}

/// What a client sends (`docs/feedback.md` §"Straight from a client").
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Direct {
    kind: Kind,
    text: String,
    build: BuildIn,
    device: String,
    client: serde_json::Value,
    #[serde(default)]
    record: Option<ClientRecord>,
}

/// `POST /client/reports`.
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
    use std::io::Write as _;

    fn gz(text: &str) -> Vec<u8> {
        let mut out = flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::fast());
        out.write_all(text.as_bytes()).unwrap();
        out.finish().unwrap()
    }

    const HEADER: &str = r#"{"kind":"header","record":1,"build":"0.1.0","preset":{},"hash":"00"}"#;

    /// A record's shape is read without the engine: a header first, then
    /// only the record's own lines.
    #[test]
    fn a_record_is_its_header_and_its_own_lines() {
        let input =
            r#"{"kind":"input","n":0,"at":0,"seat":0,"by":"seat","action":"Pass","hash":"01"}"#;
        assert_eq!(check_record(&gz(&format!("{HEADER}\n{input}\n"))), Ok(()));
        assert_eq!(check_record(&gz(HEADER)), Ok(()));
        for bad in [
            String::new(),
            input.to_owned(),
            format!("{HEADER}\n{HEADER}\n"),
            format!("{HEADER}\nnot json\n"),
            r#"{"kind":"header","record":2,"build":"x"}"#.to_owned(),
        ] {
            assert_eq!(
                check_record(&gz(&bad)),
                Err(NotARecord::NotARecord),
                "{bad}"
            );
        }
        assert_eq!(check_record(b"plain"), Err(NotARecord::Unreadable));
    }

    /// A small stream that unpacks past the bound is refused before it is
    /// read whole.
    #[test]
    fn a_record_that_unpacks_past_its_bound_is_refused() {
        let line = format!("{HEADER}\n");
        let mut body = line.clone();
        let filler = " ".repeat(1024 * 1024);
        while (body.len() as u64) <= MAX_CLIENT_RECORD_UNPACKED {
            body.push_str(&filler);
        }
        let packed = gz(&body);
        assert!(packed.len() < MAX_CLIENT_RECORD_BYTES);
        assert_eq!(check_record(&packed), Err(NotARecord::UnpacksTooLarge));
    }

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
