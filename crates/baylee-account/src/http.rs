use crate::{
    Endpoint, Error, Token,
    wire::{Request, Response},
};
use std::{io::Read, time::Duration};
use zeroize::Zeroizing;

/// Strict native credential transport, bound to one trusted Central endpoint.
pub struct CentralHttp {
    endpoint: Endpoint,
    agent: ureq::Agent,
}

impl CentralHttp {
    /// Construct without network activity. TLS verification is always enabled.
    #[must_use]
    pub fn new(endpoint: Endpoint) -> Self {
        let agent = agent(endpoint.legacy_origin().is_some());
        Self { endpoint, agent }
    }

    /// Trusted endpoint selected locally, never from a gateway response.
    #[must_use]
    pub const fn endpoint(&self) -> &Endpoint {
        &self.endpoint
    }

    /// Exchange a bounded native request. Remote errors are deliberately discarded.
    ///
    /// # Errors
    /// Reports generic auth/transport/conflict/capacity failures; never follows a redirect.
    pub fn exchange(&self, request: &Request, bearer: Option<&Token>) -> Result<Response, Error> {
        let bytes = Zeroizing::new(central_api::encode(request)?);
        // A legacy password crosses once (`docs/central-login.md` §"Legacy
        // migration"): a refusal before execution is answered, not retried.
        let attempts = if matches!(request, Request::LegacyLogin { .. }) {
            &[][..]
        } else {
            &[100, 250, 700][..]
        };
        retry_busy(
            attempts,
            || {
                let mut asked = self
                    .agent
                    .post(self.endpoint.native_url())
                    .header("Content-Type", "application/json");
                if let Some(token) = bearer {
                    asked = asked.header("Authorization", token.bearer().as_str());
                }
                let response = asked
                    .send(bytes.as_slice())
                    .map_err(|_| Error::Unavailable)?;
                decode_response(response)
            },
            std::thread::sleep,
        )
    }
}

// 429 means rejection before execution. No other error, including an ambiguous
// connection loss after a mutating operation, permits a blind transport retry.
fn retry_busy<T>(
    waits: &[u64],
    mut operation: impl FnMut() -> Result<T, Error>,
    mut wait: impl FnMut(Duration),
) -> Result<T, Error> {
    for &millis in waits {
        match operation() {
            Err(Error::Busy) => wait(Duration::from_millis(millis)),
            result => return result,
        }
    }
    operation()
}

pub(crate) fn agent(https_only: bool) -> ureq::Agent {
    ureq::Agent::config_builder()
        .tls_config(
            ureq::tls::TlsConfig::builder()
                .root_certs(ureq::tls::RootCerts::WebPki)
                .disable_verification(false)
                .build(),
        )
        .https_only(https_only)
        .proxy(None)
        .max_redirects(0)
        .http_status_as_error(false)
        .timeout_global(Some(Duration::from_secs(10)))
        .timeout_connect(Some(Duration::from_secs(3)))
        .timeout_recv_body(Some(Duration::from_secs(5)))
        .build()
        .new_agent()
}

pub(crate) fn decode_response<T: serde::de::DeserializeOwned>(
    mut response: ureq::http::Response<ureq::Body>,
) -> Result<T, Error> {
    match response.status().as_u16() {
        200 => {}
        403 => return Err(Error::TermsRequired),
        409 => return Err(Error::Conflict),
        429 => return Err(Error::Busy),
        500..=599 => return Err(Error::Unavailable),
        _ => return Err(Error::Rejected),
    }
    if response
        .headers()
        .get("Content-Type")
        .and_then(|v| v.to_str().ok())
        != Some("application/json")
    {
        return Err(Error::Rejected);
    }
    let mut bytes = Zeroizing::new(Vec::new());
    response
        .body_mut()
        .as_reader()
        .take((central_api::MAX_WIRE_BYTES + 1) as u64)
        .read_to_end(&mut bytes)
        .map_err(|_| Error::Unavailable)?;
    central_api::decode(&bytes)
}

#[cfg(feature = "client")]
impl central_client::Transport for CentralHttp {
    fn endpoint(&self) -> &Endpoint {
        &self.endpoint
    }
    fn exchange(&self, request: &Request, bearer: Option<&Token>) -> Result<Response, Error> {
        Self::exchange(self, request, bearer)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn only_explicit_preexecution_capacity_rejections_are_retried() {
        let mut attempts = 0;
        let mut waits = Vec::new();
        let result = retry_busy(
            &[100, 250, 700],
            || {
                attempts += 1;
                if attempts < 3 {
                    Err(Error::Busy)
                } else {
                    Ok(())
                }
            },
            |delay| waits.push(delay),
        );
        assert!(result.is_ok());
        assert_eq!(attempts, 3);
        assert_eq!(
            waits,
            [Duration::from_millis(100), Duration::from_millis(250)]
        );
        attempts = 0;
        assert!(matches!(
            retry_busy::<()>(
                &[100, 250, 700],
                || {
                    attempts += 1;
                    Err(Error::Unavailable)
                },
                |_| panic!("ambiguous writes must not retry")
            ),
            Err(Error::Unavailable)
        ));
        assert_eq!(attempts, 1);
        attempts = 0;
        let mut waited = Duration::ZERO;
        assert!(matches!(
            retry_busy::<()>(
                &[100, 250, 700],
                || {
                    attempts += 1;
                    Err(Error::Busy)
                },
                |delay| waited += delay
            ),
            Err(Error::Busy)
        ));
        assert_eq!(attempts, 4);
        assert!(
            waited > Duration::from_secs(1),
            "the last attempt crosses the server's one-second rate window"
        );
        // The legacy sign-in's waits: none, so its password crosses once.
        attempts = 0;
        assert!(matches!(
            retry_busy::<()>(
                &[],
                || {
                    attempts += 1;
                    Err(Error::Busy)
                },
                |_| panic!("a legacy sign-in is not retried")
            ),
            Err(Error::Busy)
        ));
        assert_eq!(attempts, 1);
    }
}
