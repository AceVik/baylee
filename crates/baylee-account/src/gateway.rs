//! Admission-only transport. This type deliberately has no password-bearing operation.
use crate::{Error, SignedToken};
use std::io::Read as _;
use zeroize::Zeroize;

/// The most of a gateway's refusal that is read (`docs/protocol.md` §"Refusals").
const MOST_SAID: u64 = 1024;

/// A gateway turned a token away: the error, and the body it said that with,
/// for a caller that reads refusals. Read by nothing here.
#[derive(Debug)]
pub struct Turned {
    /// What the status comes to, or what failed before there was one.
    pub error: Error,
    /// The answer's body, at most [`MOST_SAID`] bytes and one more.
    pub said: Vec<u8>,
}

impl From<Error> for Turned {
    fn from(error: Error) -> Self {
        Self {
            error,
            said: Vec::new(),
        }
    }
}

/// What a sign-in or a refresh sends a gateway.
#[derive(serde::Serialize)]
struct Presented<'a> {
    token: &'a SignedToken,
}

/// Gateway-local session response. Central account credentials are never represented here.
#[derive(serde::Deserialize)]
pub struct Admission {
    /// Gateway bearer token; wiped on drop unless moved into the authenticated local session.
    pub token: String,
    /// Legacy private username, absent for a projection on a different gateway.
    pub username: Option<String>,
    /// Whether the gateway's terms in force still need this account's
    /// acceptance (at Central, for a Central account).
    pub terms_stale: Option<bool>,
    /// Seconds until this bearer wants renewing, which the gateway counts so
    /// that this machine's clock does not come into it.
    #[serde(default)]
    pub renew_in: Option<u64>,
}

impl Drop for Admission {
    fn drop(&mut self) {
        self.token.zeroize();
        self.username.zeroize();
    }
}

/// Strict transport for a centrally approved gateway audience.
/// Construction validates syntax; only Central can issue an admission ticket for the origin.
pub struct GatewayHttp {
    origin: String,
    agent: ureq::Agent,
}

/// A capability for the human seat of an official challenge, never Debug-printed.
#[derive(serde::Deserialize)]
pub struct ChallengeSeat {
    /// Claimed game `UUIDv7`.
    pub game_id: String,
    /// Human seat, always zero.
    pub seat: u32,
    /// Short-lived game capability.
    #[serde(deserialize_with = "seat_secret")]
    pub seat_token: zeroize::Zeroizing<String>,
}

fn seat_secret<'de, D: serde::Deserializer<'de>>(
    deserializer: D,
) -> Result<zeroize::Zeroizing<String>, D::Error> {
    <String as serde::Deserialize>::deserialize(deserializer).map(zeroize::Zeroizing::new)
}

impl GatewayHttp {
    /// Start or resume an owned Central reservation over the strict gateway transport.
    /// # Errors
    /// Invalid identities, transport failure, refused claim or malformed capability.
    pub fn challenge_play(&self, bearer: &str, attempt: [u8; 16]) -> Result<ChallengeSeat, Error> {
        let attempt = uuid::Uuid::from_bytes(attempt);
        if attempt.get_version_num() != 7 {
            return Err(Error::Rejected);
        }
        let response = self
            .agent
            .post(format!("{}/challenges/{attempt}/play", self.origin))
            .header("Authorization", &format!("Bearer {bearer}"))
            .send_empty()
            .map_err(|_| Error::Unavailable)?;
        if let Some(error) = crate::http::status_error(response.status().as_u16()) {
            return Err(error);
        }
        let seat: ChallengeSeat = crate::http::decode_response(response)?;
        if seat.seat != 0
            || uuid::Uuid::parse_str(&seat.game_id).is_err()
            || seat.seat_token.len() != 64
            || !seat.seat_token.bytes().all(|b| b.is_ascii_hexdigit())
        {
            return Err(Error::Rejected);
        }
        Ok(seat)
    }
    /// Validated HTTPS origin, with certificate verification and redirects disabled.
    ///
    /// # Errors
    /// Rejects noncanonical origins, credentials, paths and insecure schemes.
    pub fn live(origin: &str) -> Result<Self, Error> {
        Self::new(origin, false)
    }

    /// Explicit numeric-loopback development fixture, never inferred from a live URL.
    ///
    /// # Errors
    /// Rejects every origin outside the strict development endpoint policy.
    pub fn development(origin: &str) -> Result<Self, Error> {
        Self::new(origin, true)
    }

    fn new(origin: &str, development: bool) -> Result<Self, Error> {
        central_api::gateway::validate_origin(origin, development)?;
        Ok(Self {
            origin: origin.into(),
            agent: crate::http::agent(!development),
        })
    }

    /// Trade a gateway token for a session here. The caller obtains the token
    /// from the trusted Central session; it is good at this gateway alone.
    ///
    /// # Errors
    /// Rejects an invalid token, transport failure and non-success responses.
    pub fn admit(&self, token: &SignedToken) -> Result<Admission, Turned> {
        self.present("/auth/central/login", token)
    }

    /// Carry a session this gateway holds on with a later gateway token, for
    /// a new bearer. A gateway that does not hold it, as after its restart,
    /// refuses, and the client signs in with [`GatewayHttp::admit`].
    ///
    /// # Errors
    /// As [`GatewayHttp::admit`].
    pub fn refresh(&self, token: &SignedToken) -> Result<Admission, Error> {
        self.present("/auth/central/refresh", token)
            .map_err(|turned| turned.error)
    }

    fn present(&self, path: &str, token: &SignedToken) -> Result<Admission, Turned> {
        let bytes = zeroize::Zeroizing::new(central_api::encode(&Presented { token })?);
        let mut response = self
            .agent
            .post(format!("{}{path}", self.origin))
            .header("Content-Type", "application/json")
            .send(bytes.as_slice())
            .map_err(|_| Error::Unavailable)?;
        if let Some(error) = crate::http::status_error(response.status().as_u16()) {
            let mut said = Vec::new();
            let body = response.body_mut().as_reader();
            let _ = body.take(MOST_SAID + 1).read_to_end(&mut said);
            return Err(Turned { error, said });
        }
        let admission: Admission = crate::http::decode_response(response)?;
        if admission.token.len() != 64
            || !admission
                .token
                .bytes()
                .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
            || admission.username.as_ref().is_some_and(|username| {
                username.is_empty() || username.len() > 96 || username.chars().any(char::is_control)
            })
        {
            return Err(Error::Rejected.into());
        }
        Ok(admission)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn admission_origin_cannot_downgrade_or_smuggle_an_alternate_destination() {
        assert!(GatewayHttp::live("https://registered.example").is_ok());
        for origin in [
            "http://registered.example",
            "http://127.0.0.1:5555",
            "https://u@registered.example",
            "https://registered.example/path",
            "https://registered.example?next=x",
            "https://REGISTERED.example",
            "https://registered.example/",
        ] {
            assert!(GatewayHttp::live(origin).is_err(), "{origin}");
        }
        assert!(GatewayHttp::development("http://127.0.0.1:5555").is_ok());
        assert!(GatewayHttp::development("https://registered.example").is_err());
    }
}
