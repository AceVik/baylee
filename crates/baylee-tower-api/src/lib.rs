//! Player choices and projections. No credentials, private packages or record transport.
#![forbid(unsafe_code)]
#![warn(missing_docs)]

use baylee_tower::{
    cards::{Catalog, Collection},
    v2::run::Command,
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

pub mod engine;
pub mod host;
mod location;
mod view;
pub use baylee_tower::v2::run::Phase;
pub use location::{Host, Location};
pub use view::{Duel, Run};

/// Distinct wire contract; incompatible versions are refused before authentication.
pub const VERSION: &str = "2";
/// Header carrying [`VERSION`].
pub const VERSION_HEADER: &str = "baylee-tower-version";
/// Sole player endpoint. The origin is delegated by Central, never a gateway.
pub const PLAYER_PATH: &str = "/v2/player";
/// Bounds parsing/allocation, independently of individual domain limits.
pub const MAX_REQUEST_BYTES: usize = 16 * 1024;
/// Bounds every player projection, independently of private record sizes.
pub const MAX_RESPONSE_BYTES: usize = 1024 * 1024;
/// Fixed descending UUID page size; clients continue with the last returned ID.
pub const PAGE_SIZE: usize = 8;

/// Only owner choices. Account, seed, prices, outcomes and executable paths are absent.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "operation", rename_all = "snake_case", deny_unknown_fields)]
pub enum Request {
    /// Published enrollment choices, newest first.
    Cohorts {
        /// Exclusive cursor from a previous page.
        before: Option<Uuid>,
    },
    /// This account's retained runs, newest first.
    Runs {
        /// Exclusive cursor from a previous page.
        before: Option<Uuid>,
    },
    /// One owned run, including its saved shop.
    Run {
        /// Owned run ID.
        run: Uuid,
    },
    /// Fresh routing for an owned outstanding duel, including another gateway.
    Locate {
        /// Owned run, never an arbitrary account or destination.
        run: Uuid,
    },
    /// Explicit enrollment after displaying the current echo-consent explanation.
    Start {
        /// Idempotency identity retained until a definitive answer.
        request: Uuid,
        /// Published cohort.
        cohort: Uuid,
        /// Zero-based starter choice.
        starter: u8,
        /// Explicitly accepted explanation revision.
        consent: u16,
    },
    /// One revision-checked owner choice.
    Command {
        /// Idempotency identity retained across ambiguous transport failures.
        request: Uuid,
        /// Owned run.
        run: Uuid,
        /// Last observed revision.
        revision: u64,
        /// Shop/deck choice, never a replacement state.
        command: Command,
    },
    /// Commit the next opponent and builds before opening a gateway seat.
    Reserve {
        /// Idempotency identity.
        request: Uuid,
        /// Owned run.
        run: Uuid,
        /// Last observed shop revision.
        revision: u64,
        /// Approved Tower host identity, not an address or credential.
        host: Uuid,
    },
    /// Withdraw future echo use/attribution; does not award or undo a duel.
    WithdrawEchoes {},
}

/// Public enrollment material. No historical opponent builds or shuffle seeds.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Cohort {
    /// Database-assigned identity, used to enroll.
    pub id: Uuid,
    /// Exact registry/rules identity; clients must interpret card indices using it.
    pub contract: baylee_tower::v2::package::Cohort,
    /// Reviewed cards and immutable prices for deck construction.
    pub catalog: Catalog,
    /// Exactly three starting choices.
    pub starters: [Collection; 3],
}

/// Successful bounded player responses. Failures use closed HTTP status codes.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "result", rename_all = "snake_case", deny_unknown_fields)]
pub enum Response {
    /// Current owned execution routing; contains no token, package or record head.
    Location {
        /// Re-read after technical recovery; an earlier game ID can be retired.
        location: Location,
    },
    /// Up to [`PAGE_SIZE`] enrollment choices, with the consent revision to display.
    Cohorts {
        /// Current required explanation revision.
        consent: u16,
        /// Next page starts strictly below the last ID.
        cohorts: Vec<Cohort>,
    },
    /// Up to [`PAGE_SIZE`] owned run projections.
    Runs {
        /// Next page starts strictly below the last ID.
        runs: Vec<Run>,
    },
    /// Exact committed response to a command, including on a repeated request.
    Run {
        /// Owner-safe state.
        run: Box<Run>,
    },
    /// Future use withdrawn; existing rules evidence remains private.
    Withdrawn {},
}

/// Parse bounded untrusted owner choices; no unknown fields are accepted.
/// # Errors
/// Oversized, malformed or extended requests.
pub fn decode(bytes: &[u8]) -> Result<Request, baylee_tower::Refusal> {
    if bytes.len() > MAX_REQUEST_BYTES {
        return Err(baylee_tower::Refusal::Capacity);
    }
    serde_json::from_slice(bytes).map_err(|_| baylee_tower::Refusal::Invalid)
}

#[cfg(test)]
mod tests;
