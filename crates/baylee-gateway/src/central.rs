//! Opt-in migration bridge. Secrets are read from an operator-owned file and never logged.
#![forbid(unsafe_code)]

use crate::Shared;
use axum::{
    extract::{Request, State},
    http::StatusCode,
    middleware::Next,
    response::{IntoResponse, Response},
};
use baylee_account::{CentralHttp, Endpoint, Token, wire};
use serde::Deserialize;
use std::{io::Read, sync::Arc, time::Duration};
use zeroize::Zeroizing;

mod routes;
pub(crate) use routes::router;

pub(crate) struct Bridge {
    http: CentralHttp,
    credential: Token,
    workers: Arc<tokio::sync::Semaphore>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Settings {
    /// Exactly "live" or an explicitly selected numeric-loopback development origin.
    endpoint: String,
    bridge: Token,
}

impl Bridge {
    pub(crate) fn from_env() -> Result<Option<Arc<Self>>, ()> {
        let Some(path) = std::env::var_os("BAYLEE_CENTRAL_CONFIG_FILE") else {
            return Ok(None);
        };
        let mut bytes = Zeroizing::new(Vec::new());
        std::fs::File::open(path)
            .map_err(|_| ())?
            .take(4097)
            .read_to_end(&mut bytes)
            .map_err(|_| ())?;
        if bytes.len() > 4096 {
            return Err(());
        }
        let settings: Settings = serde_json::from_slice(&bytes).map_err(|_| ())?;
        let endpoint = if settings.endpoint == "live" {
            Endpoint::live()
        } else {
            Endpoint::development(&settings.endpoint).map_err(|_| ())?
        };
        Ok(Some(Arc::new(Self {
            http: CentralHttp::new(endpoint),
            credential: settings.bridge,
            workers: Arc::new(tokio::sync::Semaphore::new(4)),
        })))
    }

    async fn exchange(
        self: &Arc<Self>,
        request: wire::Request,
    ) -> Result<wire::Response, StatusCode> {
        let permit = self
            .workers
            .clone()
            .try_acquire_owned()
            .map_err(|_| StatusCode::TOO_MANY_REQUESTS)?;
        let bridge = self.clone();
        tokio::task::spawn_blocking(move || {
            let _permit = permit;
            bridge
                .http
                .exchange(&request, Some(&bridge.credential))
                .map_err(|error| match error {
                    baylee_account::Error::Conflict => StatusCode::CONFLICT,
                    baylee_account::Error::Busy => StatusCode::TOO_MANY_REQUESTS,
                    baylee_account::Error::Rejected => StatusCode::UNAUTHORIZED,
                    baylee_account::Error::Unavailable => StatusCode::SERVICE_UNAVAILABLE,
                })
        })
        .await
        .map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?
    }
}

/// Drain in-flight account mutations before cutover and revoke old sessions under the same barrier.
/// Central export/link routes are outside this layer and acquire its exclusive guard themselves.
pub(crate) async fn barrier(State(state): State<Shared>, request: Request, next: Next) -> Response {
    if state.central.is_none() {
        return next.run(request).await;
    }
    let Ok(_guard) =
        tokio::time::timeout(Duration::from_secs(5), state.central_barrier.read()).await
    else {
        return StatusCode::SERVICE_UNAVAILABLE.into_response();
    };
    next.run(request).await
}
