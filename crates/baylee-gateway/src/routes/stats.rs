//! How busy the gateway is, in three numbers (`GET /lobby/stats`, WG-0).

use crate::{ErrorBody, HeaderMap, Json, Serialize, Shared, State, StatusCode, authed};

/// `{ players_online, tables_waiting, games_running }` for the lobby's
/// header pill.
///
/// Signed in only, as the listing is: a stranger learns nothing from the
/// front door, not even how busy it is. Numbers and nothing else — no ids,
/// no names, nothing per table — and exact rather than rounded.
///
/// - `players_online`: distinct accounts with a socket open — the lobby's,
///   or their own chair's at a running game ([`crate::presence`]) —
///   together with those whose seat bridge plays a chair of one. A player
///   whose connection is gone is not online, though the chair is held. A session row is not
///   presence: a guest's lasts thirty days after its last request.
/// - `tables_waiting`, `games_running`: the lobby's tables in those two
///   states, counted as `/health` counts them ([`crate::Lobby::waiting`],
///   [`crate::Lobby::running`]); a finished table is neither.
///
/// @route GET /lobby/stats
/// @scope public
/// @summary How busy this gateway is
/// @description Three exact numbers for a lobby's header, read from memory
/// as the request is answered: no ids, no names, nothing per table. For
/// signed-in sessions only, as the listing is.
/// @tag Lobby
/// @security bearer
/// @insert `SessionRefusals`
pub(crate) async fn lobby_stats(
    State(state): State<Shared>,
    headers: HeaderMap,
) -> Result<Json<LobbyStats>, (StatusCode, Json<ErrorBody>)> {
    authed(&state, &headers).await?;
    let (mut online, tables_waiting, games_running) = {
        let lobby = state.lobby.lock();
        (
            lobby.bridging_accounts(),
            lobby.waiting().count(),
            lobby.running().count(),
        )
    };
    online.extend(state.presence.accounts());
    Ok(Json(LobbyStats {
        players_online: online.len(),
        tables_waiting,
        games_running,
    }))
}

/// `GET /lobby/stats`.
/// @openapi
#[derive(Serialize)]
pub(crate) struct LobbyStats {
    // In key order: the bytes the `json!` it replaced wrote (serde_json sorts keys).
    /// @description Games being played; a finished table is neither.
    games_running: usize,
    /// @description Distinct accounts with a socket open (the lobby feed,
    /// or their own chair's at a running game), together with those whose
    /// seat bridge plays a chair of a running game. A chair held for a
    /// player whose connection is gone does not count them.
    players_online: usize,
    /// @description Tables waiting for players.
    tables_waiting: usize,
}
