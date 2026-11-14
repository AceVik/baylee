//! Native owner API; human choices never enter the operator/verification namespace.
use crate::identity::Central;
use baylee_tower_api::{Request, Response};
use baylee_tower_store::{Snapshot, Store};
use central_api::Endpoint;
use std::sync::{Arc, Mutex};
use tokio::sync::Semaphore;
use uuid::Uuid;

mod dispatch;
mod http;
mod limits;

/// Authenticated player service. Clone the Arc, not an account authorization result.
pub struct Players {
    store: Store,
    central: Central,
    slots: Semaphore,
    limits: Mutex<limits::Limits>,
    record_bytes: u64,
}

impl Players {
    /// Configure one trusted Central and one canonical Tower origin. Does not listen.
    /// The listener must be private behind verified HTTPS in production; no forwarded
    /// header can replace this explicit origin or select an identity endpoint.
    /// # Errors
    /// Invalid destination or private-record budget.
    pub fn new(
        store: Store,
        endpoint: &Endpoint,
        own_origin: &str,
        record_bytes: u64,
    ) -> Result<Self, crate::Error> {
        if record_bytes == 0 || record_bytes > baylee_tower::v2::execution::MAX_RECORD_BYTES {
            return Err(crate::Error::Contract);
        }
        Ok(Self {
            store,
            central: Central::new(endpoint, own_origin)?,
            slots: Semaphore::new(16),
            limits: Mutex::new(limits::Limits::new()),
            record_bytes,
        })
    }

    /// Only the native player endpoint. No browser CORS, operator routes or state override.
    pub fn router(self: Arc<Self>) -> axum::Router {
        http::router(self)
    }
}

fn project(snapshot: Snapshot) -> baylee_tower_api::Run {
    baylee_tower_api::Run::project(snapshot.id, snapshot.cohort, snapshot.state)
}

fn response(snapshot: Snapshot) -> Response {
    Response::Run {
        run: Box::new(project(snapshot)),
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Failure {
    Invalid,
    Unauthorized,
    Terms,
    Missing,
    Conflict,
    Busy,
    Unavailable,
    Version,
    Oversized,
}

impl From<baylee_tower_store::Error> for Failure {
    fn from(value: baylee_tower_store::Error) -> Self {
        use baylee_tower::Refusal;
        use baylee_tower_store::Error;
        match value {
            Error::NotFound => Self::Missing,
            Error::Conflict
            | Error::Refused(
                Refusal::Stale | Refusal::Unavailable | Refusal::Gold | Refusal::Capacity,
            ) => Self::Conflict,
            Error::Refused(Refusal::Invalid) => Self::Invalid,
            Error::Schema | Error::Unavailable => Self::Unavailable,
        }
    }
}
