//! `POST /client/reports` (31.10.2026): a report from a client signed in to
//! no gateway, filed with Baylee Central marked `direct`. Only the official
//! gateway takes one, and only where its reports go to Central
//! (`BAYLEE_REPORTS_TO=central`): Central files a direct report under the
//! bridge's credential alone.
//!
//! The body, its bounds and its allowance are the feedback service's own
//! (`docs/feedback.md` §"Straight from a client"), so a client sends here
//! what it sent there. A shipped client holds no secret, so the route asks
//! for none and stands on its limits: every report it reads counts against
//! its address (an IPv6 one by its /64) and against everyone, one refused
//! for what it says too; a body over the route's bound is refused before it
//! is counted, and one refused for the count is not counted. Everyone's
//! count is a third of what Central files for one gateway in an hour, which
//! the official gateway's signed-in players draw on too. The reporter is the
//! service's HMAC of the device's random id; under the same key
//! (`BAYLEE_DIRECT_REPORT_KEY` set to the service's `FEEDBACK_DIRECT_KEY`) a
//! device's reports line up across the move. Neither the id nor the address
//! is kept or logged.

use std::net::IpAddr;

use axum::Json;
use axum::body::Bytes;
use axum::extract::{ConnectInfo, State};
use axum::http::{HeaderMap, StatusCode};
use baylee_account::reports::{self as intake, Build, Channel, GatewaySays, RecordOrigin};
use serde::Deserialize;
use sha2::{Digest as _, Sha256};

use super::{Created, LocalRecord, MAX_LOCAL_RECORD_BYTES, MAX_TEXT_CHARS, ReportKind, central};
use crate::{ErrorBody, Shared, err};

/// The largest `client` object of a direct report, serialized: half a
/// signed-in player's, because nobody vouches for this one.
pub const MAX_DIRECT_CLIENT_BYTES: usize = 1_048_576;

/// The largest body the route reads: a full `client` object, a full record
/// in base64 and room for the rest.
pub const MAX_DIRECT_BODY_BYTES: usize =
    MAX_DIRECT_CLIENT_BYTES + MAX_LOCAL_RECORD_BYTES.div_ceil(3) * 4 + 128 * 1024;

/// Direct reports one address may send in [`super::REPORT_WINDOW`].
pub const PER_ADDRESS: usize = 10;

/// Direct reports everyone together may send in [`super::REPORT_WINDOW`]:
/// a third of the 120 Central files for one gateway in an hour (baylee-central
/// `central_api::reports::PER_GATEWAY_HOUR`), which this gateway's signed-in
/// players' reports draw on too. However many addresses send them, 80 stay
/// for the players.
pub const PER_GATEWAY: usize = 40;

/// The least a key for the devices' pseudonyms may hold.
const MIN_KEY_CHARS: usize = 16;

/// The key a device's pseudonym is made with: `BAYLEE_DIRECT_REPORT_KEY`
/// hashed, as the feedback service hashes its `FEEDBACK_DIRECT_KEY`.
///
/// # Errors
/// A key under 16 characters.
pub(super) fn key(value: Option<&str>) -> Result<Option<[u8; 32]>, &'static str> {
    value
        .map(|key| {
            if key.chars().count() < MIN_KEY_CHARS {
                return Err("BAYLEE_DIRECT_REPORT_KEY needs at least 16 characters");
            }
            Ok(Sha256::digest(key.as_bytes()).into())
        })
        .transpose()
}

/// What an address is counted under: an IPv6 address by its /64, which one
/// subscriber usually holds whole, so a single line cannot pass for many;
/// an IPv4 address, or one mapped into IPv6, as itself.
fn bucket(address: &str) -> String {
    match address.parse::<IpAddr>().map(|a| a.to_canonical()) {
        Ok(IpAddr::V6(v6)) => {
            let s = v6.segments();
            format!("{:x}:{:x}:{:x}:{:x}::/64", s[0], s[1], s[2], s[3])
        }
        Ok(v4) => v4.to_string(),
        Err(_) => address.to_owned(),
    }
}

/// The reporter a device's id is filed under: never the id itself.
pub(super) fn pseudonym(key: &[u8; 32], device: &str) -> String {
    super::hex(&super::hmac_sha256(
        key,
        format!("device\0{device}").as_bytes(),
    ))
}

/// The client's build as the body names it.
/// @openapi
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct BuildIn {
    /// @description The client's release.
    version: String,
    /// @description The commit it was built from, if it knows; one that is
    /// not hex digits, or over 64 of them, is left out, not refused.
    #[serde(default)]
    commit: Option<String>,
}

/// What a client signed in nowhere sends.
///
/// @description A report from a client signed in to no gateway; every
/// field named, no other taken.
/// @openapi
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Direct {
    /// @description What the report is about.
    kind: ReportKind,
    /// @description What the player wrote, at most 20000 characters; empty
    /// only for a crash.
    text: String,
    /// @description The client's build: its version and commit together at
    /// most 128 characters, not empty.
    build: BuildIn,
    /// @description A random id the client made for its reports: 32
    /// lowercase hex digits. The report is filed under this gateway's HMAC of
    /// it, never the id.
    /// @openapi pattern: '^[0-9a-f]{32}$'
    device: String,
    /// @description What the client says about itself: a JSON object of at
    /// most 1 MiB serialized.
    /// @openapi type: object
    client: serde_json::Value,
    /// @description The record of a game the client hosted itself, if one
    /// is attached.
    #[serde(default)]
    record: Option<LocalRecord>,
}

/// The route, with a body limit of its own.
pub fn routes() -> axum::Router<Shared> {
    axum::Router::new().route(
        "/client/reports",
        axum::routing::post(post_direct)
            .layer(axum::extract::DefaultBodyLimit::max(MAX_DIRECT_BODY_BYTES)),
    )
}

/// A direct report read and checked as the feedback service checks one, and
/// its record's bytes; or why not. The record's shape is checked later, off
/// the request's thread ([`super::checked`]).
fn read(body: &[u8]) -> Result<(Direct, Option<central::RecordBytes>), (StatusCode, &'static str)> {
    let bad = |why| (StatusCode::BAD_REQUEST, why);
    let report: Direct = serde_json::from_slice(body).map_err(|_| bad("not a report"))?;
    if !report.client.is_object() {
        return Err(bad("client must be an object"));
    }
    if report.text.trim().is_empty() && report.kind != ReportKind::Crash {
        return Err(bad("a report says something"));
    }
    if report.text.chars().count() > MAX_TEXT_CHARS {
        return Err((StatusCode::PAYLOAD_TOO_LARGE, "the text is too long"));
    }
    if serde_json::to_vec(&report.client).map_or(usize::MAX, |v| v.len()) > MAX_DIRECT_CLIENT_BYTES
    {
        return Err((
            StatusCode::PAYLOAD_TOO_LARGE,
            "the client details are too large",
        ));
    }
    let device = report.device.len() == 32
        && report
            .device
            .bytes()
            .all(|b| matches!(b, b'0'..=b'9' | b'a'..=b'f'));
    let build = report.build.version.chars().count()
        + report
            .build
            .commit
            .as_deref()
            .map_or(0, |c| c.chars().count() + 3);
    if !device || report.build.version.is_empty() || build > 128 {
        return Err(bad("a field is malformed"));
    }
    let record = report
        .record
        .as_ref()
        .map(|r| {
            super::local_record(r).map(|gzip| central::RecordBytes {
                complete: r.complete,
                origin: RecordOrigin::Client,
                gzip,
            })
        })
        .transpose()?;
    Ok((report, record))
}

/// `POST /client/reports`.
///
/// @route POST /client/reports
/// @scope public
/// @operation-id `send_direct_report`
/// @summary Send a report from a client signed in nowhere
/// @description A report from a client signed in to no gateway, filed with
/// Baylee Central marked `direct`; only the official gateway takes one,
/// where `BAYLEE_REPORTS_TO=central` and `BAYLEE_DIRECT_REPORT_KEY` are set.
/// It asks for no credential and stands on its limits: a bounded body, a
/// record that must read as a game record, and 10 reports per address (an
/// IPv6 one by its /64) and 40 from everyone an hour. Every report read
/// counts, one refused for what it says too; a body over the route's bound,
/// or one refused for the count, does not. The 40 are a third of what
/// Central files for this gateway in an hour, which its signed-in players
/// draw on too. Central's id is answered once the metadata is filed; the
/// client's details and the record follow. Any origin may post.
/// @tag Reports
/// @security none
/// @body `$Direct`
/// @return 201: `$ReportReceipt` "Central filed it; its parts follow"
/// @return 400: `$ErrorBody` "Not a report: malformed JSON, an unknown field or `kind`, no text (but for a crash), `client` not an object, a malformed `device` or `build`, a record that is not base64 of gzip or not a game record, or nothing left Central would take"
/// @return 413: `$ErrorBody` "The text is over 20000 characters, `client` over 1 MiB, or the record over 4 MiB (32 MiB unpacked); a body over 6.46 MiB is refused before it is read, in plain text"
/// @return 429: `$ErrorBody` "Too many reports from this address or from everyone in the last hour"
/// @return 502: `$ErrorBody` "Central did not take it"
/// @return 503: `$ErrorBody` "Direct reports are not taken here (not the official gateway, not filing with Central, or no `BAYLEE_DIRECT_REPORT_KEY`), too many reports are on their way to Central, or it is past its budget for this gateway"
pub async fn post_direct(
    State(state): State<Shared>,
    ConnectInfo(addr): ConnectInfo<std::net::SocketAddr>,
    headers: HeaderMap,
    body: Bytes,
) -> Result<(StatusCode, Json<Created>), (StatusCode, Json<ErrorBody>)> {
    let feedback = &state.feedback;
    let bridge = state
        .central
        .get()
        .filter(|b| feedback.to_central && b.official());
    let (Some(bridge), Some(key)) = (bridge, feedback.direct_key) else {
        return Err(err(
            StatusCode::SERVICE_UNAVAILABLE,
            "direct reports are not taken here",
        ));
    };
    // Its parts are held in memory on their way: no waiting for a place
    // here, where nobody vouches for the sender.
    let Ok(slot) = feedback.in_flight.clone().try_acquire_owned() else {
        return Err(err(
            StatusCode::SERVICE_UNAVAILABLE,
            "too many reports on their way; try again",
        ));
    };
    let address = bucket(&crate::rate_limit_ip(
        &state.trusted_proxies,
        addr.ip(),
        &headers,
    ));
    if !feedback.direct_per_address.allow(&address) {
        return Err(err(StatusCode::TOO_MANY_REQUESTS, "too many reports"));
    }
    if !feedback.direct_in_all.allow("all") {
        // Refused for everyone's count, so not counted against the address
        // either, as at the service.
        feedback.direct_per_address.give_back(&address);
        return Err(err(StatusCode::TOO_MANY_REQUESTS, "too many reports"));
    }
    let (report, record) = read(&body).map_err(|(status, why)| err(status, why))?;
    let said = central::Said {
        gateway: GatewaySays {
            name: state.display_name.as_deref().and_then(intake::said),
            url: feedback.public_url.as_deref().and_then(intake::said),
            version: intake::said(baylee_build::short()),
        },
        channel: Channel::Direct,
        reporter: pseudonym(&key, &report.device),
        kind: report.kind,
        text: &report.text,
        game_id: None,
        client: &report.client,
        // A commit that is not hex is left out, as Central would refuse it.
        build: intake::said(&report.build.version).map(|version| Build {
            version,
            commit: report.build.commit.clone().filter(|c| {
                !c.is_empty()
                    && c.len() <= intake::MAX_COMMIT
                    && c.bytes().all(|b| b.is_ascii_hexdigit())
            }),
        }),
    };
    let report_id = super::file_with_central(feedback, bridge, &said, record, Some(slot)).await?;
    Ok((StatusCode::CREATED, Json(Created { report_id })))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn body(kind: &str, text: &str, device: &str, extra: &str) -> Vec<u8> {
        format!(
            r#"{{"kind":"{kind}","text":"{text}","build":{{"version":"0.1.1","commit":"abc"}},"device":"{device}","client":{{}}{extra}}}"#
        )
        .into_bytes()
    }

    const DEVICE: &str = "0123456789abcdef0123456789abcdef";

    #[test]
    fn a_direct_report_is_read_as_the_service_reads_one() {
        assert!(read(&body("bug", "x", DEVICE, "")).is_ok());
        assert!(
            read(&body("crash", "", DEVICE, "")).is_ok(),
            "a crash may say nothing"
        );
        let refused = [
            body("bug", " ", DEVICE, ""),
            body("chat", "x", DEVICE, ""),
            body("bug", "x", "0123456789ABCDEF0123456789ABCDEF", ""),
            body("bug", "x", "short", ""),
            body("bug", "x", DEVICE, r#","game_id":"g""#),
            body(
                "bug",
                "x",
                DEVICE,
                r#","record":{"complete":true,"gzip_base64":"AAAA"}"#,
            ),
        ];
        for one in refused {
            assert_eq!(
                read(&one).err().map(|e| e.0),
                Some(StatusCode::BAD_REQUEST),
                "{}",
                String::from_utf8_lossy(&one)
            );
        }
        let long = body("bug", &"x".repeat(MAX_TEXT_CHARS + 1), DEVICE, "");
        assert_eq!(
            read(&long).err().map(|e| e.0),
            Some(StatusCode::PAYLOAD_TOO_LARGE)
        );
    }

    /// An IPv6 address is counted by its /64, so one line cannot pass for
    /// many; an IPv4 one, mapped or not, by itself.
    #[test]
    fn an_address_is_counted_by_what_one_subscriber_holds() {
        assert_eq!(
            bucket("2001:db8:1:2:aaaa::1"),
            bucket("2001:db8:1:2:ffff:ffff:ffff:ffff")
        );
        assert_eq!(bucket("2001:db8:1:2::9"), "2001:db8:1:2::/64");
        assert_ne!(bucket("2001:db8:1:2::1"), bucket("2001:db8:1:3::1"));
        assert_eq!(bucket("198.51.100.7"), "198.51.100.7");
        assert_eq!(bucket("::ffff:198.51.100.7"), "198.51.100.7");
        assert_ne!(bucket("198.51.100.7"), bucket("198.51.100.8"));
    }

    /// The feedback service's pseudonym (`baylee_feedback::direct::pseudonym`
    /// under `hash(FEEDBACK_DIRECT_KEY)`), so a device's reports line up
    /// across the move when the keys are the same.
    #[test]
    fn a_device_is_filed_under_the_services_pseudonym() {
        let key = key(Some("a key of sixteen chars")).unwrap().unwrap();
        let expected: [u8; 32] = Sha256::digest(b"a key of sixteen chars").into();
        assert_eq!(key, expected);
        let one = pseudonym(&key, DEVICE);
        assert_eq!(one.len(), 64);
        assert!(!one.contains(DEVICE));
        assert_ne!(one, pseudonym(&key, "fedcba9876543210fedcba9876543210"));
        assert!(super::key(Some("too short")).is_err());
        assert_eq!(super::key(None), Ok(None));
    }
}
