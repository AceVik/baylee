//! Where a report goes: the gateway the client is signed in to, or else
//! straight to the feedback service, or nowhere.
//!
//! Signed in, a report goes to that gateway with that session, exactly as
//! before (`POST {gateway}/reports`). Signed in nowhere, a client that
//! knows a feedback service sends it there itself (`POST
//! {service}/client/reports`, `docs/feedback.md` §"Straight from a
//! client"): no session, no account, a random id this device made instead.
//! Knowing none, it says so and sends nothing anywhere, least of all to a
//! third party.

use serde::Serialize;

use super::{BugReport, Build, Kind, LocalRecord};

/// The most the `client` object of a direct report may weigh, serialised:
/// the service's own bound (`baylee_feedback::direct::MAX_CLIENT_BYTES`),
/// half a gateway's, because nobody vouches for this one.
pub const MAX_DIRECT_CLIENT_BYTES: usize = 1_048_576;

/// The path a feedback service takes direct reports at.
pub const DIRECT_PATH: &str = "/client/reports";

/// Where a report would go now.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Route {
    /// To this gateway, with the session the client is signed in with.
    Gateway(String),
    /// Straight to the feedback service, at this URL.
    Direct(String),
    /// Nowhere: signed in to no gateway and knowing no service.
    Nowhere,
}

impl Route {
    /// Whether a report can be sent at all.
    #[must_use]
    pub fn sends(&self) -> bool {
        !matches!(self, Self::Nowhere)
    }

    /// Whether it goes straight to the service.
    #[must_use]
    pub fn is_direct(&self) -> bool {
        matches!(self, Self::Direct(_))
    }
}

/// The route for a client signed in at `gateway` (if anywhere) that knows
/// the feedback service at `service` (if any; [`feedback_service`]). The
/// gateway wins whenever there is one: it vouches for the reporter, and it
/// attaches a networked game's record itself.
#[must_use]
pub fn route(gateway: Option<&str>, service: Option<&str>) -> Route {
    match (gateway, service) {
        (Some(gateway), _) => Route::Gateway(gateway.to_string()),
        (None, Some(service)) => {
            Route::Direct(format!("{}{DIRECT_PATH}", service.trim_end_matches('/')))
        }
        (None, None) => Route::Nowhere,
    }
}

/// The feedback service this client knows: the settings' own address when
/// it has one, else the one the build was given
/// (`BAYLEE_FEEDBACK_PUBLIC_URL`). An empty setting turns it off whatever
/// the build says. Either must be `https://`, or `http://` to this machine
/// (a service in development): a report is not sent in the clear across a
/// network. Anything else is no service.
#[must_use]
pub fn feedback_service(setting: Option<&str>, built: Option<&str>) -> Option<String> {
    let chosen = match setting.map(str::trim) {
        Some("") => return None,
        Some(set) => set,
        None => built.map(str::trim).filter(|b| !b.is_empty())?,
    };
    let base = chosen.trim_end_matches('/');
    let (secure, rest) = if let Some(rest) = base.strip_prefix("https://") {
        (true, rest)
    } else {
        (false, base.strip_prefix("http://")?)
    };
    let authority = rest.split('/').next().unwrap_or_default();
    let host = if authority.starts_with("[::1]") {
        "[::1]"
    } else {
        authority.split(':').next().unwrap_or_default()
    };
    let loopback = matches!(host, "localhost" | "127.0.0.1" | "[::1]");
    let fine = !host.is_empty()
        && (secure || loopback)
        && base.len() <= 200
        && !base
            .chars()
            .any(|c| c.is_whitespace() || c.is_control() || matches!(c, '?' | '#' | '@'));
    fine.then(|| base.to_string())
}

/// How long a device id is: 32 lowercase hex digits, as the service takes
/// one and no other way.
pub const DEVICE_ID_CHARS: usize = 32;

/// Whether `id` is a device id: [`DEVICE_ID_CHARS`] lowercase hex digits.
#[must_use]
pub fn is_device_id(id: &str) -> bool {
    id.len() == DEVICE_ID_CHARS && id.bytes().all(|b| matches!(b, b'0'..=b'9' | b'a'..=b'f'))
}

/// A device id made of `random`, sixteen bytes the caller drew from the
/// platform's generator. It is never derived from anything: not the
/// machine, not an account, not an address.
#[must_use]
pub fn device_id(random: [u8; 16]) -> String {
    use std::fmt::Write as _;
    random.iter().fold(String::with_capacity(32), |mut out, b| {
        let _ = write!(out, "{b:02x}");
        out
    })
}

/// The device id to send under: `kept` when it is one, else one made of
/// `random`. The caller keeps the answer; a kept value that is not an id
/// (a hand-edited settings file) is replaced, never sent.
#[must_use]
pub fn kept_device_id(kept: Option<&str>, random: impl FnOnce() -> [u8; 16]) -> String {
    match kept {
        Some(id) if is_device_id(id) => id.to_string(),
        _ => device_id(random()),
    }
}

/// The body of `POST {service}/client/reports`, field for field: the
/// service refuses any field it does not name.
#[derive(Clone, Debug, Serialize)]
pub struct DirectSubmission {
    /// What kind of report.
    pub kind: Kind,
    /// What the player wrote.
    pub text: String,
    /// Which build.
    pub build: Build,
    /// This device's random id: the service keeps only a keyed hash of it.
    pub device: String,
    /// Everything else, as far as the player allowed it.
    pub client: BugReport,
    /// A local game's record, when the player ticked it for this report.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub record: Option<LocalRecord>,
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Signed in, the gateway; otherwise the service, if there is one;
    /// otherwise nowhere at all.
    #[test]
    fn a_report_goes_to_the_gateway_then_the_service_then_nowhere() {
        let service = Some("https://feedback.example");
        assert_eq!(
            route(Some("https://gw.example"), service),
            Route::Gateway("https://gw.example".into())
        );
        assert_eq!(
            route(Some("https://gw.example"), None),
            Route::Gateway("https://gw.example".into())
        );
        assert_eq!(
            route(None, Some("https://feedback.example/")),
            Route::Direct("https://feedback.example/client/reports".into())
        );
        assert_eq!(route(None, None), Route::Nowhere);
        assert!(!Route::Nowhere.sends());
        assert!(route(None, service).is_direct());
        assert!(!route(Some("x"), service).is_direct());
    }

    /// The setting wins, an empty one turns it off, and only an address a
    /// report can safely go to is one.
    #[test]
    fn the_service_is_the_setting_then_the_build_and_only_a_safe_address() {
        let built = Some("https://feedback.example");
        assert_eq!(
            feedback_service(None, built),
            Some("https://feedback.example".into())
        );
        assert_eq!(
            feedback_service(Some("https://mine.example/fb/"), built),
            Some("https://mine.example/fb".into())
        );
        assert_eq!(feedback_service(Some(""), built), None, "off");
        assert_eq!(feedback_service(Some("  "), built), None, "off");
        assert_eq!(feedback_service(None, None), None);
        assert_eq!(feedback_service(None, Some("")), None);
        for local in [
            "http://localhost:28780",
            "http://127.0.0.1:28780",
            "http://[::1]:28780",
        ] {
            assert_eq!(feedback_service(Some(local), None), Some(local.into()));
        }
        for bad in [
            "http://feedback.example",
            "http://localhost.evil.example",
            "ftp://feedback.example",
            "feedback.example",
            "https://",
            "https://user@feedback.example",
            "https://feedback.example/?q=1",
            "https://feed back.example",
        ] {
            assert_eq!(feedback_service(Some(bad), built), None, "{bad}");
        }
    }

    /// A device id is 32 lowercase hex digits drawn at random; a kept one
    /// is used, and anything else in its place is replaced.
    #[test]
    fn a_device_id_is_random_hex_and_a_bad_one_is_replaced() {
        let made = device_id([0xab; 16]);
        assert_eq!(made, "ab".repeat(16));
        assert!(is_device_id(&made));
        assert_eq!(kept_device_id(Some(&made), || unreachable!()), made);
        for bad in [
            "",
            "AB".repeat(16).as_str(),
            &"ab".repeat(15),
            &"ab".repeat(17),
            &format!("{}g", "a".repeat(31)),
        ] {
            assert!(!is_device_id(bad), "{bad}");
            assert_eq!(kept_device_id(Some(bad), || [1; 16]), "01".repeat(16));
        }
        assert_eq!(kept_device_id(None, || [2; 16]), "02".repeat(16));
    }
}
