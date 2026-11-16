//! Correlated native WebSocket transport for the existing owner-choice protocol.
//! Authentication is a header on the versioned upgrade, never part of these frames.
use crate::{Request, Response};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Version is part of the route so incompatible clients fail before authentication.
pub const PATH: &str = "/v2/player/ws/7";
/// Envelope allowance in addition to the bounded ordinary request.
pub const MAX_CLIENT_BYTES: usize = crate::MAX_REQUEST_BYTES + 128;
/// Envelope allowance in addition to a bounded ordinary response.
pub const MAX_SERVER_BYTES: usize = crate::MAX_RESPONSE_BYTES + 128;

/// One work identity correlates an answer; mutation identities remain inside Request.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Client {
    /// Fresh local work identity, including when retrying an old durable mutation.
    pub serial: Uuid,
    /// An ordinary read or owner choice, under the same authorization boundaries.
    pub request: Request,
}

/// Closed error categories. Never contains diagnostics, credentials or private data.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Refusal {
    /// Malformed request.
    Invalid,
    /// Expired or revoked Tower scope.
    Unauthorized,
    /// Current terms must be accepted.
    Terms,
    /// Missing or foreign state.
    Missing,
    /// Stale revision or rejected domain choice.
    Conflict,
    /// Bounded capacity exhausted; no mutation was dispatched.
    Busy,
    /// Indeterminate service failure; preserve any mutation identity.
    Unavailable,
}

/// Committed responses and owner-authorized invalidation snapshots.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "event", rename_all = "snake_case", deny_unknown_fields)]
pub enum Server {
    /// Upgrade completed with the exact shared contract.
    Ready {
        /// Current protocol version.
        version: String,
    },
    /// Answer to precisely one client work item.
    Reply {
        /// Echo of the client work identity, never an account identity.
        serial: Uuid,
        /// Same response as HTTP, or a value-free refusal.
        result: Result<Response, Refusal>,
    },
    /// New state for the socket's most recently read run or ranking.
    Changed {
        /// No mutation receipt: this cannot acknowledge an uncertain purchase.
        response: Response,
    },
}

/// Decode before authenticating/dispatching work; allocations remain bounded.
/// # Errors
/// Overlong frames, extra fields, malformed JSON or non-v7 work identity.
pub fn decode(bytes: &[u8]) -> Result<Client, Refusal> {
    if bytes.len() > MAX_CLIENT_BYTES {
        return Err(Refusal::Invalid);
    }
    let frame: Client = serde_json::from_slice(bytes).map_err(|_| Refusal::Invalid)?;
    if frame.serial.get_version_num() != 7 || frame.serial.get_variant() != uuid::Variant::RFC4122 {
        return Err(Refusal::Invalid);
    }
    Ok(frame)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strict_socket_envelopes_keep_version_and_mutation_identity() {
        assert!(PATH.ends_with(&format!("/{}", crate::VERSION)));
        let frame = Client {
            serial: Uuid::now_v7(),
            request: Request::Cohorts { before: None },
        };
        let bytes = serde_json::to_vec(&frame).unwrap();
        assert_eq!(decode(&bytes), Ok(frame));
        assert!(decode(&vec![b' '; MAX_CLIENT_BYTES + 1]).is_err());
        let invalid = br#"{"serial":"00000000-0000-0000-0000-000000000000","request":{"operation":"runs","before":null}}"#;
        assert!(decode(invalid).is_err());
        let mut extended: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
        extended["account"] = serde_json::json!(Uuid::now_v7());
        assert!(decode(&serde_json::to_vec(&extended).unwrap()).is_err());
    }
}
