//! Release retries abandoned by the core without disturbing uncertain sends.
use super::{wire, worker::Retries};
use baylee_client_core::social::Social;

pub(super) fn retain_pending(retries: &Retries, machine: &Social) {
    let Ok(mut retries) = retries.lock() else {
        return;
    };
    // Membership loss can end a send locally, without a worker refusal. Prune
    // before new personal sends so these entries cannot exhaust the 64 slots.
    retries.retain(|encoded, _| {
        let Ok(bytes) = baylee_seal::b64::url::decode(encoded) else {
            return false;
        };
        serde_json::from_slice::<wire::Header>(&bytes)
            .is_ok_and(|header| machine.has_pending_message(&header.id))
    });
}
