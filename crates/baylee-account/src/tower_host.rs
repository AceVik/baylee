//! Async host transport; dedicated credentials, exact response bindings, no blind retries.
use crate::{Error, Token};
use baylee_tower_api::{
    self as api,
    host::{self, Acknowledged, Batch, Request, Response},
};
use bytes::Bytes;
use http_body_util::{BodyExt as _, Full, Limited};
use hyper::header::{AUTHORIZATION, CONTENT_TYPE, HeaderValue};
use hyper_rustls::{HttpsConnector, HttpsConnectorBuilder};
use hyper_util::{
    client::legacy::{Client, connect::HttpConnector},
    rt::{TokioExecutor, TokioTimer},
};
use std::time::Duration;

/// A trusted operator-configured Tower destination. Never selected by a player,
/// package, public gateway advertisement or HTTP redirect. Deliberately not printable.
pub struct TowerHost {
    client: Client<HttpsConnector<HttpConnector>, Full<Bytes>>,
    origin: String,
    credential: Token,
}

impl TowerHost {
    /// Configure verified HTTPS, or explicitly selected numeric-loopback development.
    /// No requests or credential checks occur until an operation is sent.
    /// # Errors
    /// Noncanonical or wrong-environment destination.
    pub fn new(origin: &str, credential: Token, development: bool) -> Result<Self, Error> {
        central_api::gateway::validate_origin(origin, development)?;
        let mut http = HttpConnector::new();
        http.enforce_http(false);
        http.set_connect_timeout(Some(Duration::from_secs(3)));
        http.set_nodelay(true);
        let tls = HttpsConnectorBuilder::new().with_webpki_roots();
        let connector = if development {
            tls.https_or_http().enable_http1().wrap_connector(http)
        } else {
            tls.https_only().enable_http1().wrap_connector(http)
        };
        Ok(Self {
            client: Client::builder(TokioExecutor::new())
                .pool_timer(TokioTimer::new())
                .pool_idle_timeout(Duration::from_secs(60))
                .build(connector),
            origin: origin.into(),
            credential,
        })
    }

    /// Send one control operation. Keep the claim ID unchanged after a lost response.
    /// # Errors
    /// Refused scope/assignment, stale state, capacity, malformed reply or unavailable transport.
    pub async fn control(&self, request: &Request) -> Result<Response, Error> {
        let body = serde_json::to_vec(request).map_err(|_| Error::Rejected)?;
        if body.len() > api::MAX_REQUEST_BYTES {
            return Err(Error::Rejected);
        }
        let response = self
            .exchange(host::CONTROL_PATH, "application/json", body)
            .await?;
        let matching = match (request, &response) {
            (Request::Advertise { .. }, Response::Advertised {})
            | (Request::Renew { .. }, Response::Renewed { .. }) => true,
            (Request::Claim { claim, account, .. }, Response::Claimed { game }) => {
                game.lease.claim == *claim && game.account == *account
            }
            (Request::Read { index, .. }, Response::Chunk { chunk }) => chunk.index == *index,
            (Request::Seal { lease, expected }, Response::Acknowledged { acknowledgement }) => {
                acknowledgement.fence == lease.fence && acknowledgement.prefix == *expected
            }
            _ => false,
        };
        if !matching {
            return Err(Error::Rejected);
        }
        Ok(response)
    }

    /// Commit one complete operation. Preserve these exact chunks for ambiguous retries.
    /// The native table must additionally check the returned content commitment before
    /// releasing views; a successful HTTP status alone cannot release its barrier.
    /// # Errors
    /// Incomplete/oversized input, wrong/stale lease, refused write or mismatched ACK.
    pub async fn append(&self, batch: &Batch) -> Result<Acknowledged, Error> {
        let body = batch.encode().map_err(|_| Error::Rejected)?;
        let Response::Acknowledged { acknowledgement } = self
            .exchange(host::RECORD_PATH, "application/octet-stream", body)
            .await?
        else {
            return Err(Error::Rejected);
        };
        let chunks = batch
            .expected
            .chunks
            .checked_add(batch.chunks.len() as u64)
            .ok_or(Error::Rejected)?;
        let size: u64 = batch.chunks.iter().map(|chunk| chunk.len() as u64).sum();
        let bytes = batch
            .expected
            .bytes
            .checked_add(size)
            .ok_or(Error::Rejected)?;
        if acknowledgement.fence != batch.lease.fence
            || acknowledgement.prefix.chunks != chunks
            || acknowledgement.prefix.bytes != bytes
        {
            return Err(Error::Rejected);
        }
        Ok(acknowledgement)
    }

    async fn exchange(&self, path: &str, kind: &str, body: Vec<u8>) -> Result<Response, Error> {
        tokio::time::timeout(Duration::from_secs(15), self.send(path, kind, body))
            .await
            .unwrap_or(Err(Error::Unavailable))
    }

    async fn send(&self, path: &str, kind: &str, body: Vec<u8>) -> Result<Response, Error> {
        let mut bearer = HeaderValue::from_str(self.credential.bearer().as_str())
            .map_err(|_| Error::Rejected)?;
        bearer.set_sensitive(true);
        let request = hyper::Request::post(format!("{}{path}", self.origin))
            .header(CONTENT_TYPE, kind)
            .header(api::VERSION_HEADER, api::VERSION)
            .header(AUTHORIZATION, bearer)
            .body(Full::new(Bytes::from(body)))
            .map_err(|_| Error::Rejected)?;
        let response = self
            .client
            .request(request)
            .await
            .map_err(|_| Error::Unavailable)?;
        let (parts, body) = response.into_parts();
        if let Some(error) = crate::http::status_error(parts.status.as_u16()) {
            return Err(error);
        }
        if parts.headers.get(CONTENT_TYPE).map(HeaderValue::as_bytes) != Some(b"application/json")
            || parts
                .headers
                .get(api::VERSION_HEADER)
                .and_then(|v| v.to_str().ok())
                != Some(api::VERSION)
        {
            return Err(Error::Rejected);
        }
        let bytes = Limited::new(body, host::MAX_RESPONSE_BYTES)
            .collect()
            .await
            .map_err(|_| Error::Rejected)?
            .to_bytes();
        serde_json::from_slice(&bytes).map_err(|_| Error::Rejected)
    }
}

#[cfg(test)]
mod tests;
