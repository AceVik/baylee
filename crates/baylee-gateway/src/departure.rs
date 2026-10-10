//! A player whose client went away (`docs/protocol.md` §"Leaving, and
//! losing the connection").
//!
//! A waiting room has no engine to hold a chair, so the gateway does: when
//! an account's last socket closes ([`crate::presence`]) a departure is
//! armed, and if the account has opened none by the time it falls due it
//! stands up from every room it waits in, exactly as its own `Leave` would
//! (`LobbyGame::leave`): the room passes to whoever has been there longest
//! or closes, and the lobby feed tells everyone still looking. A client that
//! quits on purpose says so (`POST /lobby/depart`) and is not waited for.
//!
//! A running game is not this module's: its engine holds the chair
//! (`Deadline::StandIn`, the pause), and is told about a deliberate
//! departure by [`depart`].

use crate::{
    ErrorBody, HeaderMap, Json, Shared, State, StatusCode, auth, authed, chair, routes, v1,
};

/// How long a waiting room keeps the chair of a player whose client went
/// away without saying so, in seconds (`BAYLEE_ROOM_GRACE_SECS`).
///
/// A minute: long enough for a restart into an update, which walks back to
/// the room it left (`docs/client.md` §"Restarting into an update"), and for
/// a dropped Wi-Fi; short enough that a room whose host closed the lid is
/// gone before anyone wonders why it never starts. The same number as the
/// minute a client restarting into an update has always been given.
pub(crate) const ROOM_GRACE_SECS: u64 = 60;

/// `account_id`'s socket closed: if it was the last, arm its departure.
pub(crate) fn socket_closed(state: &Shared, account_id: &str) {
    if state.presence.here(account_id) {
        return;
    }
    let visits = state.presence.visits(account_id);
    let state = state.clone();
    let account_id = account_id.to_owned();
    tokio::spawn(async move {
        tokio::time::sleep(state.room_grace).await;
        // Back in between, whether or not it has gone again since: the
        // later closing armed its own departure, and that one decides.
        if state.presence.here(&account_id) || state.presence.visits(&account_id) != visits {
            return;
        }
        let left = leave_waiting_rooms(&state, &account_id);
        if left > 0 {
            tracing::info!(
                rooms = left,
                "a player whose client went away left its rooms"
            );
        }
    });
}

/// Stands `account_id` up from every room still waiting; how many.
pub(crate) fn leave_waiting_rooms(state: &Shared, account_id: &str) -> usize {
    let left = state
        .lobby
        .lock()
        .leave_waiting_rooms(account_id, auth::now_secs());
    for id in &left {
        chair::revoke(state, id, account_id);
    }
    if !left.is_empty() {
        state.lobby_moved();
    }
    left.len()
}

/// `POST /lobby/depart`: the client is quitting on purpose.
///
/// Every waiting room it sits in is left at once, and every running game it
/// has a chair in is told the player left (`SeatDetached { left: true }`):
/// the house takes the chair now if another player is at the table, and the
/// game ends if none is. A crash says none of this and is waited for
/// instead. Answers `204` whether or not there was anything to leave.
pub(crate) async fn depart(
    State(state): State<Shared>,
    headers: HeaderMap,
) -> Result<StatusCode, (StatusCode, Json<ErrorBody>)> {
    let account_id = authed(&state, &headers).await?;
    leave_waiting_rooms(&state, &account_id);
    let links: Vec<(crate::lobby::EngineLink, usize)> = {
        let lobby = state.lobby.lock();
        lobby
            .running()
            .filter_map(|game| {
                let link = game.engine.clone()?;
                let seat = game
                    .seats
                    .iter()
                    .find(|s| s.account_id.as_deref() == Some(account_id.as_str()))?;
                Some((link, seat.seat))
            })
            .collect()
    };
    for (link, seat) in links {
        routes::to_engine(
            &link,
            v1::envelope::Msg::SeatDetached(v1::SeatDetached {
                seat: seat as u32,
                left: true,
            }),
        );
    }
    Ok(StatusCode::NO_CONTENT)
}
