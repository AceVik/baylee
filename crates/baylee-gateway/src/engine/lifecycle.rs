//! Transport ownership and the bounded gap before an engine attaches.

use super::{Shared, auth, stop_engine};
use crate::lobby::LobbyState;

/// A process that never identifies itself cannot keep a playing room forever.
/// This is independent of player sockets and of their reconnect/hold clocks.
pub(super) fn watch_start(state: Shared, game_id: String, agent_id: String) {
    tokio::spawn(async move {
        tokio::time::sleep(std::time::Duration::from_secs(
            crate::routes::seat_socket::ENGINE_WAIT_SECS,
        ))
        .await;
        end_unattached(&state, &game_id, &agent_id);
    });
}

/// Close only the still-unattached engine assignment that was observed.
pub(super) fn end_unattached(state: &Shared, game_id: &str, agent_id: &str) {
    let ended = {
        let mut lobby = state.lobby.lock();
        lobby.games.get_mut(game_id).is_some_and(|game| {
            if game.state != LobbyState::Playing
                || game.agent_id.as_deref() != Some(agent_id)
                || game.engine.is_some()
            {
                return false;
            }
            game.finish(auth::now_secs());
            true
        })
    };
    if ended {
        tracing::warn!(game_id, "engine did not attach; game closed");
        state.lobby_moved();
        stop_engine(state, game_id);
    }
}

/// Agent reconnection cannot reclaim engines assigned to its previous link.
pub(super) fn owner_lost(state: &Shared, game_id: &str) {
    if game_id.starts_with(crate::replays::JOB_PREFIX) {
        crate::replays::failed(state, game_id);
    } else {
        super::end_game(state, game_id);
    }
}
