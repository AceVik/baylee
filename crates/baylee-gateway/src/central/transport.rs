//! The bridge's exchange with Central, on the runtime.
//!
//! Every session check goes through here, so an exchange holds a task and no
//! thread: `baylee_account::CentralHttp`, which a report's upload still uses,
//! blocks a thread for as long as Central takes. The settings are that
//! transport's, and they are the point of it: TLS verified against the `WebPKI`
//! roots, no proxy, no redirect followed, `https` only on the live endpoint,
//! and every wait bounded.
//!
//! **One thing is deliberately stricter.** `401` is Central refusing, and the
//! only answer that means a session is gone; a socket closes on it at once
//! (`standing.rs`). So nothing else may read as `Rejected` here: a status
//! Central does not send, the wrong content type and a body that does not
//! decode are what something in the way sends, and read as `Unavailable`.
//! Otherwise a proxy's error page would end every session behind it.

use baylee_account::{Endpoint, Error, Token, wire};
use bytes::Bytes;
use http_body_util::{BodyExt as _, Full, Limited};
use hyper::header::{AUTHORIZATION, CONTENT_TYPE, HeaderValue};
use hyper_rustls::{HttpsConnector, HttpsConnectorBuilder};
use hyper_util::client::legacy::Client;
use hyper_util::client::legacy::connect::HttpConnector;
use hyper_util::rt::{TokioExecutor, TokioTimer};
use std::time::Duration;
use zeroize::Zeroizing;

/// How long one attempt may take, as `CentralHttp`'s.
const ONE_ATTEMPT: Duration = Duration::from_secs(10);

/// How long a connection may take to open, as `CentralHttp`'s.
const CONNECTING: Duration = Duration::from_secs(3);

/// How long a connection stays open unused.
const IDLE: Duration = Duration::from_secs(60);

/// The waits before asking again after Central said it was busy before it
/// started, as `CentralHttp`'s.
const BUSY_WAITS: [Duration; 3] = [
    Duration::from_millis(100),
    Duration::from_millis(250),
    Duration::from_millis(700),
];

/// One trusted Central endpoint, and the connections kept open to it.
pub(super) struct Transport {
    client: Client<HttpsConnector<HttpConnector>, Full<Bytes>>,
    url: String,
}

impl Transport {
    /// Opens nothing yet.
    pub(super) fn new(endpoint: &Endpoint) -> Self {
        let mut http = HttpConnector::new();
        http.enforce_http(false);
        http.set_connect_timeout(Some(CONNECTING));
        http.set_nodelay(true);
        let tls = HttpsConnectorBuilder::new().with_webpki_roots();
        let connector = if endpoint.legacy_origin().is_some() {
            tls.https_only().enable_http1().wrap_connector(http)
        } else {
            tls.https_or_http().enable_http1().wrap_connector(http)
        };
        Self {
            client: Client::builder(TokioExecutor::new())
                .pool_timer(TokioTimer::new())
                .pool_idle_timeout(IDLE)
                .build(connector),
            url: endpoint.native_url(),
        }
    }

    /// One request under `bearer`, asked again while Central is busy.
    ///
    /// # Errors
    /// `Rejected` only for Central's own `401` (the module says why), `Busy`,
    /// `Conflict` and `TermsRequired` as it says them, `Unavailable` otherwise.
    pub(super) async fn exchange(
        &self,
        request: &wire::Request,
        bearer: Option<&Token>,
    ) -> Result<wire::Response, Error> {
        // Wiped when the last attempt lets go of it: a request can carry a lease.
        let body = Bytes::from_owner(Zeroizing::new(central_api::encode(request)?));
        let bearer = bearer
            .map(|token| {
                HeaderValue::from_str(token.bearer().as_str()).map(|mut value| {
                    value.set_sensitive(true);
                    value
                })
            })
            .transpose()
            .map_err(|_| Error::Unavailable)?;
        for wait in BUSY_WAITS {
            match self.attempt(body.clone(), bearer.as_ref()).await {
                Err(Error::Busy) => tokio::time::sleep(wait).await,
                answer => return answer,
            }
        }
        self.attempt(body, bearer.as_ref()).await
    }

    async fn attempt(
        &self,
        body: Bytes,
        bearer: Option<&HeaderValue>,
    ) -> Result<wire::Response, Error> {
        tokio::time::timeout(ONE_ATTEMPT, self.send(body, bearer))
            .await
            .unwrap_or(Err(Error::Unavailable))
    }

    async fn send(
        &self,
        body: Bytes,
        bearer: Option<&HeaderValue>,
    ) -> Result<wire::Response, Error> {
        let mut request = hyper::Request::post(self.url.as_str())
            .header(CONTENT_TYPE, "application/json")
            .body(Full::new(body))
            .map_err(|_| Error::Unavailable)?;
        if let Some(bearer) = bearer {
            request.headers_mut().insert(AUTHORIZATION, bearer.clone());
        }
        let response = self
            .client
            .request(request)
            .await
            .map_err(|_| Error::Unavailable)?;
        let (parts, body) = response.into_parts();
        match parts.status.as_u16() {
            200 => {}
            401 => return Err(Error::Rejected),
            403 => return Err(Error::TermsRequired),
            409 => return Err(Error::Conflict),
            429 => return Err(Error::Busy),
            _ => return Err(Error::Unavailable),
        }
        if parts.headers.get(CONTENT_TYPE).map(HeaderValue::as_bytes) != Some(b"application/json") {
            return Err(Error::Unavailable);
        }
        let bytes = Limited::new(body, central_api::MAX_WIRE_BYTES)
            .collect()
            .await
            .map_err(|_| Error::Unavailable)?
            .to_bytes();
        central_api::decode(&Zeroizing::new(bytes.to_vec())).map_err(|_| Error::Unavailable)
    }
}
