//! Separately authenticated execution plane. Hosts submit evidence, never outcomes.
use crate::http::{self, Failure};
use axum::{
    Router,
    body::to_bytes,
    extract::{Request, State},
    response::Response,
    routing::post,
};
use baylee_tower_api::{
    self as api,
    host::{self as wire, Batch},
};
use baylee_tower_store::{Principal, Role, Store};
use std::{sync::Arc, time::Duration};
use tokio::sync::{OwnedSemaphorePermit, Semaphore};

/// Configured renewal window, independent of human time or duel length.
const LEASE_MS: u64 = 60_000;

/// Dedicated host transport, sharing the durable authority but no player credential cache.
pub struct Hosts {
    store: Store,
    slots: Arc<Semaphore>,
    records: Arc<Semaphore>,
}

impl Hosts {
    /// Bound control and expensive record-body concurrency independently.
    #[must_use]
    pub fn new(store: Store) -> Self {
        Self {
            store,
            slots: Arc::new(Semaphore::new(16)),
            records: Arc::new(Semaphore::new(4)),
        }
    }
    /// Merge with the player router; every host operation has its own credential scope.
    pub fn router(self: Arc<Self>) -> Router {
        Router::new()
            .route(wire::CONTROL_PATH, post(control))
            .route(wire::RECORD_PATH, post(record))
            .with_state(self)
    }

    async fn admit(&self, request: Request, binary: bool) -> Result<Admitted, Failure> {
        let slot = self
            .slots
            .clone()
            .try_acquire_owned()
            .map_err(|_| Failure::Busy)?;
        let record = if binary {
            Some(
                self.records
                    .clone()
                    .try_acquire_owned()
                    .map_err(|_| Failure::Busy)?,
            )
        } else {
            None
        };
        if request.uri().query().is_some() {
            return Err(Failure::Invalid);
        }
        let (parts, body) = request.into_parts();
        let token = http::credential(
            &parts.headers,
            if binary {
                "application/octet-stream"
            } else {
                "application/json"
            },
        )?;
        // Authenticate before reading a potentially eight-MiB record from a slow peer.
        let principal = self
            .store
            .authenticate_operator(token.digest())
            .await
            .map_err(|error| {
                if error == baylee_tower_store::Error::NotFound {
                    Failure::Unauthorized
                } else {
                    Failure::Unavailable
                }
            })?;
        if principal.role() != Role::Host {
            return Err(Failure::Unauthorized);
        }
        let bound = if binary {
            wire::MAX_FRAME_BYTES
        } else {
            api::MAX_REQUEST_BYTES
        };
        let bytes = tokio::time::timeout(Duration::from_secs(10), to_bytes(body, bound))
            .await
            .map_err(|_| Failure::Invalid)?
            .map_err(|_| Failure::Oversized)?;
        Ok(Admitted {
            principal,
            bytes,
            _slot: slot,
            _record: record,
        })
    }

    async fn control(&self, request: Request) -> Result<Vec<u8>, Failure> {
        let admitted = self.admit(request, false).await?;
        let request: wire::Request =
            serde_json::from_slice(&admitted.bytes).map_err(|_| Failure::Invalid)?;
        let principal = &admitted.principal;
        let response = match request {
            wire::Request::Advertise { engines } => {
                self.store.advertise(principal, &engines).await?;
                wire::Response::Advertised {}
            }
            wire::Request::Claim { run, claim } => wire::Response::Claimed {
                game: Box::new(self.store.claim(principal, run, claim, LEASE_MS).await?),
            },
            wire::Request::Renew { lease } => wire::Response::Renewed {
                until: self.store.renew(principal, lease, LEASE_MS).await?,
            },
            wire::Request::Read { lease, index } => wire::Response::Chunk {
                chunk: self.store.read_chunk(principal, lease, index).await?,
            },
            wire::Request::Seal { lease, expected } => wire::Response::Acknowledged {
                acknowledgement: self.store.seal(principal, lease, expected).await?,
            },
        };
        encode(&response)
    }

    async fn record(&self, request: Request) -> Result<Vec<u8>, Failure> {
        let admitted = self.admit(request, true).await?;
        let batch = Batch::decode(&admitted.bytes).map_err(|_| Failure::Invalid)?;
        let acknowledgement = self
            .store
            .append_batch(
                &admitted.principal,
                batch.lease,
                batch.expected,
                &batch.chunks,
            )
            .await?;
        encode(&wire::Response::Acknowledged { acknowledgement })
    }
}

struct Admitted {
    principal: Principal,
    bytes: bytes::Bytes,
    _slot: OwnedSemaphorePermit,
    _record: Option<OwnedSemaphorePermit>,
}

fn encode(response: &wire::Response) -> Result<Vec<u8>, Failure> {
    let bytes = serde_json::to_vec(response).map_err(|_| Failure::Unavailable)?;
    if bytes.len() > wire::MAX_RESPONSE_BYTES {
        return Err(Failure::Unavailable);
    }
    Ok(bytes)
}

async fn control(State(hosts): State<Arc<Hosts>>, request: Request) -> Response {
    http::reply(hosts.control(request).await)
}
async fn record(State(hosts): State<Arc<Hosts>>, request: Request) -> Response {
    http::reply(hosts.record(request).await)
}
