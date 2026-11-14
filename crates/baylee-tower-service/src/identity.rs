//! Online identity introspection. No JWT cache, alternate origin or credential logging.
use bytes::Bytes;
use central_api::{
    Endpoint, Token,
    tower_access::Identity,
    wire::{Request, Response},
};
use http_body_util::{BodyExt as _, Full, Limited};
use hyper::header::{AUTHORIZATION, CONTENT_TYPE, HeaderValue};
use hyper_rustls::{HttpsConnector, HttpsConnectorBuilder};
use hyper_util::{
    client::legacy::{Client, connect::HttpConnector},
    rt::{TokioExecutor, TokioTimer},
};
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use zeroize::Zeroizing;

use crate::player::Failure;

pub(super) struct Central {
    client: Client<HttpsConnector<HttpConnector>, Full<Bytes>>,
    url: String,
    own_origin: String,
}

impl Central {
    pub(super) fn new(endpoint: &Endpoint, own_origin: &str) -> Result<Self, crate::Error> {
        central_api::gateway::validate_origin(own_origin, endpoint.legacy_origin().is_none())
            .map_err(|_| crate::Error::Contract)?;
        if own_origin == endpoint.origin() {
            return Err(crate::Error::Contract);
        }
        let mut http = HttpConnector::new();
        http.enforce_http(false);
        http.set_connect_timeout(Some(Duration::from_secs(3)));
        http.set_nodelay(true);
        let tls = HttpsConnectorBuilder::new().with_webpki_roots();
        let connector = if endpoint.legacy_origin().is_some() {
            tls.https_only().enable_http1().wrap_connector(http)
        } else {
            tls.https_or_http().enable_http1().wrap_connector(http)
        };
        Ok(Self {
            client: Client::builder(TokioExecutor::new())
                .pool_timer(TokioTimer::new())
                .pool_idle_timeout(Duration::from_secs(60))
                .build(connector),
            url: endpoint.native_url(),
            own_origin: own_origin.into(),
        })
    }

    pub(super) async fn authenticate(&self, token: &Token) -> Result<Identity, Failure> {
        tokio::time::timeout(Duration::from_secs(5), self.send(token))
            .await
            .unwrap_or(Err(Failure::Unavailable))
    }

    async fn send(&self, token: &Token) -> Result<Identity, Failure> {
        let mut bearer =
            HeaderValue::from_str(&token.bearer()).map_err(|_| Failure::Unavailable)?;
        bearer.set_sensitive(true);
        let body =
            central_api::encode(&Request::TowerIdentity {}).map_err(|_| Failure::Unavailable)?;
        let request = hyper::Request::post(&self.url)
            .header(CONTENT_TYPE, "application/json")
            .header(central_api::PROTOCOL_HEADER, central_api::PROTOCOL_VERSION)
            .header(AUTHORIZATION, bearer)
            .body(Full::new(Bytes::from(body)))
            .map_err(|_| Failure::Unavailable)?;
        let response = self
            .client
            .request(request)
            .await
            .map_err(|_| Failure::Unavailable)?;
        let (parts, body) = response.into_parts();
        match parts.status.as_u16() {
            200 => {}
            401 => return Err(Failure::Unauthorized),
            403 => return Err(Failure::Terms),
            429 => return Err(Failure::Busy),
            _ => return Err(Failure::Unavailable),
        }
        if parts.headers.get(CONTENT_TYPE).map(HeaderValue::as_bytes) != Some(b"application/json") {
            return Err(Failure::Unavailable);
        }
        let bytes = Limited::new(body, 4096)
            .collect()
            .await
            .map_err(|_| Failure::Unavailable)?
            .to_bytes();
        let response: Response = central_api::decode(&Zeroizing::new(bytes.to_vec()))
            .map_err(|_| Failure::Unavailable)?;
        let Response::TowerIdentity { identity } = response else {
            return Err(Failure::Unavailable);
        };
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|_| Failure::Unavailable)?
            .as_secs();
        let expires = u64::try_from(identity.expires_at).map_err(|_| Failure::Unavailable)?;
        let account = uuid::Uuid::from_bytes(identity.account);
        if identity.origin != self.own_origin
            || expires <= now
            || account.get_version_num() != 7
            || account.get_variant() != uuid::Variant::RFC4122
        {
            return Err(Failure::Unauthorized);
        }
        Ok(identity)
    }
}
