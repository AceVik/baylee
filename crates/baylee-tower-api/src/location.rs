use baylee_tower::v2::run::Phase;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// An execution operator's explicitly approved gateway destination.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Host {
    /// Public Tower operator identity; not a capability.
    pub id: Uuid,
    /// Operator-controlled public label.
    pub name: String,
    /// Canonical approved gateway origin. Obtain a separate gateway admission;
    /// never forward a Tower or Central session bearer to this address.
    pub origin: String,
}

/// Fresh owner-only routing, separate from immutable command retry receipts.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Location {
    /// The requested owned run.
    pub run: Uuid,
    /// Current authority revision, not an idempotency receipt's historical one.
    pub revision: u64,
    /// Current activity. Only Reserved/Playing can expose an execution destination.
    pub phase: Phase,
    /// Absent when no duel is attachable or its operator lacks an approved location.
    pub host: Option<Host>,
    /// Current execution's transport game, when Playing; still requires a seat grant.
    pub game: Option<Uuid>,
}
