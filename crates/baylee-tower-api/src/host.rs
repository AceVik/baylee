//! Private host contracts; never serialize these on the player endpoint.
pub use baylee_tower::v2::execution::Prefix;
use baylee_tower::{cards::Catalog, v2::package::Package};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

mod batch;
mod wire;
pub use batch::{Batch, MAX_FRAME_BYTES};
pub use wire::{CONTROL_PATH, RECORD_PATH, Request, Response};

/// Worst-case JSON expansion of a full private chunk, including bounded metadata.
pub const MAX_RESPONSE_BYTES: usize = 2 * 1024 * 1024;

/// Identifies a claimed worker; authentication still comes from the dedicated host.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Lease {
    /// Database-assigned immutable duel reservation.
    pub reservation: Uuid,
    /// Host's unique worker-instance request, not an authorization token.
    pub claim: Uuid,
    /// Monotonic authority generation.
    pub fence: u64,
}

/// Private execution material, delivered only to the assigned live host.
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Claimed {
    /// Central owner identity. The gateway must bind its human seat to this account.
    pub account: Uuid,
    /// Worker binding used for every append, renewal and seal.
    pub lease: Lease,
    /// Exact frozen build, opponent and independent seeds.
    pub package: Package,
    /// Frozen reviewed card interpretation and prices.
    pub catalog: Catalog,
    /// Every acknowledged chunk must be replayed before continuing/revealing.
    pub prefix: Prefix,
    /// Authority Unix milliseconds, not a game or player clock.
    pub lease_until: u64,
}

/// Returned only after the chunk and run head transaction commits.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Acknowledged {
    /// The authorized generation; old generations cannot receive new ACKs.
    pub fence: u64,
    /// Head immediately after this exact append, including on retries.
    pub prefix: Prefix,
}

/// One bounded private record row. This intentionally has no Debug implementation.
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StoredChunk {
    /// Zero-based stream position.
    pub index: u64,
    /// Exact preceding acknowledged head.
    pub previous: Prefix,
    /// Exact resulting acknowledged head.
    pub head: Prefix,
    /// Uncompressed canonical record bytes; adapters may compress bounded transport.
    pub data: Vec<u8>,
}
