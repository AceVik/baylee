//! The expiry sweep (§7): slots past their `expires_at`, then messages with
//! no slot left. Run as the gateway starts and in its ten-minute cleanup;
//! its log line carries counts only.

use crate::Shared;

/// Sweeps once.
pub(crate) async fn sweep(state: &Shared) {
    if !state.social.on() {
        return;
    }
    match baylee_db::social::sweep(&state.db, time::OffsetDateTime::now_utc()).await {
        Ok(swept) if swept.slots > 0 || swept.messages > 0 => tracing::info!(
            slots = swept.slots,
            messages = swept.messages,
            "undelivered chat messages expired"
        ),
        Ok(_) => {}
        Err(e) => tracing::warn!("social: the sweep failed: {e}"),
    }
}
