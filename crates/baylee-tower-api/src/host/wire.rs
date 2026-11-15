use super::{Acknowledged, Claimed, Lease, Prefix, StoredChunk, Uuid};
use baylee_protocol::execution::Identity;
use serde::{Deserialize, Serialize};

/// Operator-only control route, under a separate Tower host credential.
pub const CONTROL_PATH: &str = "/v2/host";
/// Whole-operation binary record batch; never accepts individual partial ACK writes.
pub const RECORD_PATH: &str = "/v2/host/record";

/// Host choices. Lease duration is authority policy, not a host input.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "operation", rename_all = "snake_case", deny_unknown_fields)]
pub enum Request {
    /// Publish the local agents' exact, approved executable inventory.
    Advertise {
        /// Installed exact identities, including busy agents; empty when disconnected.
        engines: Vec<Identity>,
        /// Subset with currently free capacity. Empty does not revoke compatibility.
        available: Vec<Identity>,
    },
    /// Claim this host's assigned reservation. Retries retain the same claim ID.
    Claim {
        /// Central identity from the gateway's verified native session, never the
        /// caller's body. Checked before the reservation can change phase.
        account: Uuid,
        /// Reserved run.
        run: Uuid,
        /// Execution-instance `UUIDv7`.
        claim: Uuid,
    },
    /// Extend the existing generation before expiry.
    Renew {
        /// Exact currently assigned generation.
        lease: Lease,
    },
    /// Recover one retained chunk from this generation's acknowledged prefix.
    Read {
        /// Exact currently assigned generation.
        lease: Lease,
        /// Zero-based chunk position.
        index: u64,
    },
    /// Submit the retained terminal candidate to independent verification.
    Seal {
        /// Exact currently assigned generation.
        lease: Lease,
        /// Exact durable record head.
        expected: Prefix,
    },
}

/// Private host response. Deliberately not printable; never a player projection.
#[derive(Serialize, Deserialize)]
#[serde(tag = "result", rename_all = "snake_case", deny_unknown_fields)]
pub enum Response {
    /// Capabilities accepted; no additional authority granted.
    Advertised {
        /// Authority-resolved operator identity; never selected by the caller.
        host: Uuid,
    },
    /// Immutable private execution material and human owner binding.
    Claimed {
        /// All acknowledged bytes must be reconstructed before continuation.
        game: Box<Claimed>,
    },
    /// Authority Unix milliseconds of the renewed lease expiry.
    Renewed {
        /// Not a game clock or an automatic rules deadline.
        until: u64,
    },
    /// One bounded private record row for reconstruction.
    Chunk {
        /// Raw retained bytes with their preceding/resulting commitments.
        chunk: StoredChunk,
    },
    /// Whole-operation commit or terminal seal; neither asserts a rules outcome.
    Acknowledged {
        /// Exact final committed head, including on a retry.
        acknowledgement: Acknowledged,
    },
}
