//! How busy the gateway is, in three numbers (`GET /lobby/stats`, WG-0).

use crate::{ErrorBody, HeaderMap, Json, Shared, State, StatusCode, authed};

/// `{ players_online, tables_waiting, games_running }` for the lobby's
/// header pill.
///
/// Signed in only, as the listing is: a stranger learns nothing from the
/// front door, not even how busy it is. Numbers and nothing else — no ids,
/// no names, nothing per table — and exact rather than rounded.
///
/// - `players_online`: distinct accounts with a lobby socket open
///   ([`crate::presence`]) together with those in a chair of a running game
///   (their own, or one their seat bridge plays). A session row is not
///   presence: a guest's lasts thirty days after its last request.
/// - `tables_waiting`, `games_running`: the lobby's tables in those two
///   states, counted as `/health` counts them ([`crate::Lobby::waiting`],
///   [`crate::Lobby::running`]); a finished table is neither.
pub(crate) async fn lobby_stats(
    State(state): State<Shared>,
    headers: HeaderMap,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ErrorBody>)> {
    authed(&state, &headers).await?;
    let (mut online, tables_waiting, games_running) = {
        let lobby = state.lobby.lock();
        (
            lobby.playing_accounts(),
            lobby.waiting().count(),
            lobby.running().count(),
        )
    };
    online.extend(state.presence.accounts());
    Ok(Json(serde_json::json!({
        "players_online": online.len(),
        "tables_waiting": tables_waiting,
        "games_running": games_running,
    })))
}
