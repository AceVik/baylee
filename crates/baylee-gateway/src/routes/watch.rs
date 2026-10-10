//! `/games/{id}/watch`: a spectator's socket (`docs/protocol.md`
//! §"Spectators").
//!
//! One stream per game, the engine's public view, handed to every watching
//! socket without being decoded. Nothing a spectator sends reaches the
//! engine: a spectator has nothing to say to a game. The gateway's count of
//! open watch sockets is the number the listing and the seats are told.

use super::seat_socket::{MAX_SEAT_FRAME, engine_ready, refuse_seat};
use crate::{
    ErrorBody, Json, LobbyState, Message, Path, Query, RecvError, Shared, State, StatusCode,
    TICKET_REFUSED, WebSocket, WebSocketUpgrade, WsParams, err, lobby, spend_ticket, v1, wsticket,
};

/// Why `account_id`'s session may not watch `game` now, as the status a
/// ticket or an upgrade is refused with. Shared by both doors.
pub(crate) fn may_watch(game: &lobby::LobbyGame) -> Result<(), (StatusCode, Json<ErrorBody>)> {
    if !game.allow_spectators {
        return Err(err(
            StatusCode::FORBIDDEN,
            "this table does not allow spectators",
        ));
    }
    if game.state != LobbyState::Playing {
        return Err(err(StatusCode::CONFLICT, "this table is not playing"));
    }
    Ok(())
}

pub(crate) async fn watch_ws(
    State(state): State<Shared>,
    Path(id): Path<String>,
    Query(params): Query<WsParams>,
    ws: WebSocketUpgrade,
) -> Result<axum::response::Response, (StatusCode, Json<ErrorBody>)> {
    let ticket = params
        .ticket
        .as_deref()
        .ok_or_else(|| err(StatusCode::UNAUTHORIZED, "missing ticket"))?;
    let wsticket::Grant::Watch { .. } = spend_ticket(
        &state,
        ticket,
        &wsticket::Door::Watch {
            game_id: id.clone(),
        },
    )?
    else {
        return Err(err(StatusCode::UNAUTHORIZED, TICKET_REFUSED));
    };
    {
        let lobby = state.lobby.lock();
        let game = lobby
            .games
            .get(&id)
            .ok_or_else(|| err(StatusCode::NOT_FOUND, "no such game"))?;
        // Asked again at the door: the host may have closed the table to
        // spectators since the ticket was bought.
        may_watch(game)?;
    }
    if let Some(why) = baylee_protocol::version_refusal("This client", params.protocol) {
        return Ok(ws.on_upgrade(move |socket| refuse_seat(socket, why)));
    }
    Ok(ws
        .max_message_size(MAX_SEAT_FRAME)
        .on_upgrade(move |socket| run_watch_socket(state, id, socket)))
}

/// Moves the game's spectator count by `by` and tells its engine; `joined`
/// asks the engine for a whole snapshot. Returns the engine's link.
fn recount(state: &Shared, game_id: &str, joined: bool) -> Option<lobby::EngineLink> {
    let link = {
        let mut lobby = state.lobby.lock();
        let game = lobby.games.get_mut(game_id)?;
        if joined {
            game.watching += 1;
        } else {
            game.watching = game.watching.saturating_sub(1);
        }
        let changed = v1::envelope::Msg::SpectatorsChanged(v1::SpectatorsChanged {
            count: game.watching,
            joined,
        });
        let link = game.engine.clone();
        if let Some(link) = &link {
            let _ = link.send(v1::Envelope { msg: Some(changed) });
        }
        link
    };
    state.lobby_moved();
    link
}

/// One spectator's socket: the game's public stream out, nothing in.
pub(crate) async fn run_watch_socket(state: Shared, game_id: String, mut socket: WebSocket) {
    let (mut rx, mut ready) = {
        let mut lobby = state.lobby.lock();
        let Some(game) = lobby.games.get_mut(&game_id) else {
            return;
        };
        (game.spectator_outbox.subscribe(), game.ready.subscribe())
    };
    if !engine_ready(&mut ready).await {
        return;
    }
    // Subscribed before the engine hears of us, so the snapshot it answers
    // with is not missed.
    if recount(&state, &game_id, true).is_none() {
        return;
    }
    tracing::info!(game_id, "spectator socket opened");
    loop {
        tokio::select! {
            frame = socket.recv() => match frame {
                // A spectator answers nothing: whatever it sends is dropped.
                Some(Ok(_)) => {}
                Some(Err(_)) | None => break,
            },
            update = rx.recv() => match update {
                Ok(bytes) => {
                    if futures_util::SinkExt::send(&mut socket, Message::Binary(bytes))
                        .await
                        .is_err()
                    {
                        break;
                    }
                }
                // Behind: a fresh snapshot rather than a gap. Every spectator
                // shares the stream, so they all get it; it is idempotent.
                Err(RecvError::Lagged(_)) => {
                    let lobby = state.lobby.lock();
                    if let Some(game) = lobby.games.get(&game_id)
                        && let Some(link) = &game.engine
                    {
                        let _ = link.send(v1::Envelope {
                            msg: Some(v1::envelope::Msg::SpectatorsChanged(
                                v1::SpectatorsChanged { count: game.watching, joined: true },
                            )),
                        });
                    }
                }
                Err(RecvError::Closed) => break,
            },
            over = ready.changed() => {
                if over.is_err() || !*ready.borrow() {
                    break;
                }
            }
        }
    }
    let _ = socket.send(Message::Close(None)).await;
    recount(&state, &game_id, false);
    tracing::info!(game_id, "spectator socket closed");
}
