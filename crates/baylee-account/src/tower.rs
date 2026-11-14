//! The account's dedicated Tower connection; strict TLS and no arbitrary destination.
use crate::{Endpoint, Error};
use baylee_tower_api::{self as api, Request, Response};
use central_api::tower_access::Grant;
use std::io::Read;

/// Short-lived native Tower access, obtained only through [`crate::Account::tower`].
/// Never a Central session or gateway credential; deliberately not printable.
pub struct TowerHttp {
    grant: Grant,
    agent: ureq::Agent,
}

impl TowerHttp {
    pub(crate) fn new(central: &Endpoint, grant: Grant) -> Result<Self, Error> {
        central_api::gateway::validate_origin(&grant.origin, central.legacy_origin().is_none())?;
        if grant.origin == central.origin() {
            return Err(Error::Rejected);
        }
        Ok(Self {
            grant,
            agent: crate::http::agent(central.legacy_origin().is_some()),
        })
    }

    /// Canonical service origin, learned directly from the authenticated Central response.
    #[must_use]
    pub fn origin(&self) -> &str {
        &self.grant.origin
    }

    /// Unix seconds; refresh the handle through Central before expiry.
    #[must_use]
    pub const fn expires_at(&self) -> i64 {
        self.grant.expires_at
    }

    /// Send a bounded owner choice. Keep its request UUID after an ambiguous failure;
    /// asking again under a renewed credential must use the same exact request.
    /// No automatic retry, redirect, cookie or alternate-origin fallback occurs.
    /// # Errors
    /// Revoked access, changed state, current terms, capacity or transport failure.
    pub fn exchange(&self, request: &Request) -> Result<Response, Error> {
        let body = serde_json::to_vec(request).map_err(|_| Error::Rejected)?;
        if body.len() > api::MAX_REQUEST_BYTES {
            return Err(Error::Rejected);
        }
        let mut response = self
            .agent
            .post(format!("{}{}", self.grant.origin, api::PLAYER_PATH))
            .header("Content-Type", "application/json")
            .header(api::VERSION_HEADER, api::VERSION)
            .header("Authorization", self.grant.token.bearer().as_str())
            .send(body.as_slice())
            .map_err(|_| Error::Unavailable)?;
        if let Some(error) = crate::http::status_error(response.status().as_u16()) {
            return Err(error);
        }
        if response
            .headers()
            .get("content-type")
            .and_then(|v| v.to_str().ok())
            != Some("application/json")
            || response
                .headers()
                .get(api::VERSION_HEADER)
                .and_then(|v| v.to_str().ok())
                != Some(api::VERSION)
        {
            return Err(Error::Rejected);
        }
        let mut bytes = Vec::new();
        response
            .body_mut()
            .as_reader()
            .take((api::MAX_RESPONSE_BYTES + 1) as u64)
            .read_to_end(&mut bytes)
            .map_err(|_| Error::Unavailable)?;
        if bytes.len() > api::MAX_RESPONSE_BYTES {
            return Err(Error::Rejected);
        }
        let response: Response = serde_json::from_slice(&bytes).map_err(|_| Error::Rejected)?;
        if !matches!(
            (request, &response),
            (Request::Cohorts { .. }, Response::Cohorts { .. })
                | (Request::Runs { .. }, Response::Runs { .. })
                | (
                    Request::Run { .. }
                        | Request::Start { .. }
                        | Request::Command { .. }
                        | Request::Reserve { .. },
                    Response::Run { .. }
                )
                | (Request::WithdrawEchoes {}, Response::Withdrawn {})
        ) {
            return Err(Error::Rejected);
        }
        Ok(response)
    }
}

#[cfg(test)]
mod tests;
