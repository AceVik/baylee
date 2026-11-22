//! Bounded read-only export from a linked player's original gateway.

use super::DeckDraft;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Exactly one save from an owned deck. Historical rows use the head's
/// metadata because the old store only versioned the card lists.
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GatewayDeckVersion {
    /// Central account authenticated by the gateway session.
    pub central: Uuid,
    /// Linked local account whose rows were selected.
    pub owner: Uuid,
    /// Existing source deck.
    pub deck: Uuid,
    /// Commit to the complete head to detect in-place saves and renames.
    pub generation: [u8; 32],
    /// Head and returned revision numbers.
    pub head_revision: u64,
    /// Returned historical revision or the head itself.
    pub revision: u64,
    /// Source timestamp in Unix seconds.
    pub saved_at: i64,
    /// Source of a copy at this gateway, when recorded.
    pub copied: Option<(Uuid, u64)>,
    /// Contents. The client supplies the authenticated source origin.
    pub draft: DeckDraft,
}
