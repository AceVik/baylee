//! Admission-only transport. This type deliberately has no password-bearing operation.
use crate::{Error, Token, migration::Capability};
use zeroize::Zeroize;

/// Gateway-local session response. Central account credentials are never represented here.
#[derive(serde::Deserialize)]
pub struct Admission {
    /// Gateway bearer token; wiped on drop unless moved into the authenticated local session.
    pub token: String,
    /// Legacy private username, absent for a projection on a different gateway.
    pub username: Option<String>,
    /// Whether the local gateway terms need acknowledgement.
    pub terms_stale: Option<bool>,
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

impl GatewayHttp {
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

    /// Exchange a short-lived, origin-bound capability for a gateway session.
    /// The caller must first obtain this capability from the trusted Central session.
    ///
    /// # Errors
    /// Rejects invalid capabilities, transport failure and non-success responses.
    pub fn admit(&self, ticket: Token) -> Result<Admission, Error> {
        let bytes = zeroize::Zeroizing::new(central_api::encode(&Capability { ticket })?);
        let response = self
            .agent
            .post(format!("{}/auth/central/login", self.origin))
            .header("Content-Type", "application/json")
            .send(bytes.as_slice())
            .map_err(|_| Error::Unavailable)?;
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
            return Err(Error::Rejected);
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
