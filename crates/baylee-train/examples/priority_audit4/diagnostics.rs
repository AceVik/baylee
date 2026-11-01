//! Failure context may contain only the audited seat's authorized information.
use baylee_core::ids::PlayerId;
use baylee_engine::choice::{Pending, PlayerAction};
use baylee_gamehost::Session;
use serde_json::json;

pub fn refused(
    session: &Session,
    selected: PlayerId,
    actor: u8,
    n: u64,
    action: &PlayerAction,
    progress: (usize, usize, &[u64]),
    reason: &str,
) -> anyhow::Error {
    let (inputs, fallback, fallback_inputs) = progress;
    let view = session.observe_view(selected);
    let pending = session.view_for(selected).map(|(pending, _)| pending);
    let global_kind = match session.pending() {
        Pending::Priority { .. } => "Priority",
        Pending::GameOver(_) => "GameOver",
        _ => "Other private question (details omitted)",
    };
    let action = if actor == selected.get() {
        crate::views::action(&view, action)
    } else {
        json!({"kind":"other_seat_action_omitted"})
    };
    anyhow::anyhow!(
        "replay input refused: {}",
        json!({
            "n":n,"actor":actor,"reason":reason,"inputs_verified":inputs,
            "hash_checks_before_failure":inputs+1,"seen_fallback_checks":fallback,
            "exact_hash_checks":inputs+1-fallback,"seen_fallback_input_n":fallback_inputs,
            "pending_kind":global_kind,"selected_seat_pending":pending,
            "selected_seat_snapshot":crate::views::snapshot(&view),"actual":action
        })
    )
}
