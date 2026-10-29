//! Shared idle-priority classification for external behavior-cloning labels.

use baylee_engine::choice::{Pending, PlayerAction};

pub(super) fn is_idle_pass(pending: &Pending, action: &PlayerAction) -> bool {
    matches!(action, PlayerAction::PassPriority)
        && matches!(pending, Pending::Priority { legal, .. }
            if legal.lands.is_empty()
                && legal.castable.is_empty()
                && legal.suspendable.is_empty()
                && legal.payable.is_empty())
}
