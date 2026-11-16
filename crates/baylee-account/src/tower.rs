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
    development: bool,
    central: Endpoint,
    account: [u8; 16],
}

impl TowerHttp {
    pub(crate) fn new(central: &Endpoint, account: [u8; 16], grant: Grant) -> Result<Self, Error> {
        central_api::gateway::validate_origin(&grant.origin, central.legacy_origin().is_none())?;
        if grant.origin == central.origin() {
            return Err(Error::Rejected);
        }
        Ok(Self {
            grant,
            agent: crate::http::agent(central.legacy_origin().is_some()),
            development: central.legacy_origin().is_none(),
            central: central.clone(),
            account,
        })
    }

    pub(crate) fn belongs_to(&self, central: &Endpoint, account: [u8; 16]) -> bool {
        &self.central == central && self.account == account
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
        validate_response(request, &response, self.development)?;
        Ok(response)
    }
}

fn validate_response(
    request: &Request,
    response: &Response,
    development: bool,
) -> Result<(), Error> {
    if !matches!(
        (request, response),
        (Request::Cohorts { .. }, Response::Cohorts { .. })
            | (Request::Ranking { .. }, Response::Ranking { .. })
            | (Request::Listing { .. }, Response::Listing { .. })
            | (Request::Cohort { .. }, Response::Cohort { .. })
            | (Request::Locate { .. }, Response::Location { .. })
            | (Request::Hosts { .. }, Response::Hosts { .. })
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
    if let (Request::Ranking { cohort, lane }, Response::Ranking { board }) = (request, response)
        && (board.cohort != *cohort || board.lane != *lane || !uuid7(*cohort) || !board.valid())
    {
        return Err(Error::Rejected);
    }
    if let (Request::Listing { revision, listed }, Response::Listing { listing }) =
        (request, response)
        && (revision.checked_add(1) != Some(listing.revision) || listing.listed != *listed)
    {
        return Err(Error::Rejected);
    }
    if let (Request::Locate { run }, Response::Location { location }) = (request, response) {
        validate_location(*run, location, development)?;
    }
    if let (Request::Cohort { cohort: expected }, Response::Cohort { cohort }) = (request, response)
        && (cohort.id != *expected || !uuid7(*expected))
    {
        return Err(Error::Rejected);
    }
    if let (
        Request::Hosts { run, before },
        Response::Hosts {
            run: returned,
            hosts,
        },
    ) = (request, response)
    {
        if run != returned || !uuid7(*run) || hosts.len() > api::PAGE_SIZE {
            return Err(Error::Rejected);
        }
        let mut prior = *before;
        for host in hosts {
            validate_host(host, development)?;
            if prior.is_some_and(|prior| host.id >= prior) {
                return Err(Error::Rejected);
            }
            prior = Some(host.id);
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests;

fn validate_location(
    run: uuid::Uuid,
    location: &api::Location,
    development: bool,
) -> Result<(), Error> {
    use api::Phase;
    if location.run != run || !uuid7(run) || location.game.is_some_and(|id| !uuid7(id)) {
        return Err(Error::Rejected);
    }
    if let Some(host) = &location.host {
        validate_host(host, development)?;
        if !matches!(location.phase, Phase::Reserved | Phase::Playing)
            || (location.phase == Phase::Playing) != location.game.is_some()
        {
            return Err(Error::Rejected);
        }
    } else if location.game.is_some() {
        return Err(Error::Rejected);
    }
    Ok(())
}

fn uuid7(id: uuid::Uuid) -> bool {
    id.get_version_num() == 7 && id.get_variant() == uuid::Variant::RFC4122
}

fn validate_host(host: &api::Host, development: bool) -> Result<(), Error> {
    central_api::gateway::validate_origin(&host.origin, development)?;
    if !uuid7(host.id)
        || host.name.is_empty()
        || host.name.len() > 160
        || host.name.trim() != host.name
        || host.name.chars().any(char::is_control)
    {
        return Err(Error::Rejected);
    }
    Ok(())
}
