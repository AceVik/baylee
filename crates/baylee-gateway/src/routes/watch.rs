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

/// `GET /games/{id}/watch`: one spectator's socket.
///
/// @channel watch /games/{id}/watch
/// @scope public
/// @summary A spectator's socket
/// @description A spectator's socket at a running table whose host allows
/// spectators. It carries the game's public stream, the same for every
/// spectator, as one protobuf `Envelope` (`baylee.v1`) per binary frame,
/// which the gateway hands on without decoding; nothing a spectator sends
/// reaches the engine. Buy a ticket with the session (`POST /ws-ticket`
/// with `{"socket":"watch","game":"<id>"}`), then open the socket with it
/// and the protocol the client speaks (`baylee_protocol::watch_socket_path`
/// builds the address). The upgrade asks the ticket's questions again and
/// is refused as an HTTP answer, not a frame: with `401` for no ticket, or
/// with `ticket expired or used` for one that is unknown, spent, expired
/// or for another socket or game, or whose session has since signed out or
/// lapsed; with `404` and `no such game`; with `403` when the table allows
/// no spectators (any more) and `409` when it is not being played; with
/// `403` and `ban` or `terms_required` when the account is banned or must
/// accept new terms, `401` when it must choose a new password, and `503`
/// when the account store or Baylee Central cannot be asked. A client of
/// another protocol is let in, sent one `HelloAck` and closed. Otherwise
/// the socket waits up to 30 seconds for the game's engine (and closes if
/// none attaches), the engine is told the spectator count, and every
/// spectator is sent a whole snapshot (`GameStaticMsg`, `StateDelta` with
/// the whole public log, and `Curtain`); a socket that falls behind asks for
/// the same. A spectator is never waited for by the curtain, never on a
/// clock, and may leave at any time. A frame the spectator sends is dropped,
/// and one over 64 KiB ends the connection; there is no rate bound. The session is checked again
/// every 5 seconds; once it is signed out, lapsed or banned, or the game
/// ends, the socket closes with a close frame without a code.
/// @ws-query ticket: String required "A single-use ticket bought for watching this game with `POST /ws-ticket` and the session"
/// @ws-query protocol: u32 required "The protocol the client speaks (`PROTOCOL_VERSION`, as `GET /info` answers it); another, or none, is answered with a `HelloAck` that is not compatible"
/// parameters:
///   id:
///     description: The game's id, as the lobby lists it.
/// @send `HelloAck`: proto(baylee.v1.Envelope) application/x-protobuf "Envelope case `hello_ack`, from the gateway itself: the client's protocol is not the gateway's. `compatible` is false, `protocol_version` is the gateway's and `message` is a sentence naming both. The only frame; the socket closes after it."
/// @send `GameStaticMsg`: proto(baylee.v1.Envelope) application/x-protobuf "Envelope case `game_static`: the public opening payload, first in every snapshot and sent again before a view that shows a printing for the first time. `static_json` is a `GameStatic` as JSON whose print table holds only what was shown in public, never a decklist; `your_seat` is seat 0, the side the table is drawn from."
/// @send `StateDelta`: proto(baylee.v1.Envelope) application/x-protobuf "Envelope case `state_delta`: the public table. `view_json` is a `SpectatorView` as JSON (no private hand, question, looked-at card, clock or policy, and a face-down card is a blank); `log_json` is a `LogTail` as JSON, the game log as nobody in particular may know it, appended by its `from`."
/// @send `ChoiceRequest`: proto(baylee.v1.Envelope) application/x-protobuf "Envelope case `choice_request`, only at the end: `pending_json` is a `Pending` of `GameOver` as JSON."
/// @send `Curtain`: proto(baylee.v1.Envelope) application/x-protobuf "Envelope case `curtain`, empty: sent after every snapshot, so a spectator's table is open at once."
/// @send `GameClosed`: proto(baylee.v1.Envelope) application/x-protobuf "Envelope case `game_closed`: an admin ended the game, and nobody won or lost; `reason` is the admin's words. The last frame of the game."
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
    let wsticket::Grant::Watch { session, .. } = spend_ticket(
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
    // Whose socket it is, for the friends list's "watching" (`social/`):
    // the session that bought the ticket, if it still stands.
    let watcher = crate::central::resolve_digest(&state, session.clone())
        .await
        .map_err(|status| err(status, TICKET_REFUSED))?
        .ok_or_else(|| err(StatusCode::UNAUTHORIZED, TICKET_REFUSED))?;
    crate::moderation::admit(&watcher)?;
    crate::terms_admit(&watcher)?;
    let watcher = watcher.account_id;
    Ok(ws
        .max_message_size(MAX_SEAT_FRAME)
        .on_upgrade(move |socket| run_watch_socket(state, id, Some(watcher), session, socket)))
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
pub(crate) async fn run_watch_socket(
    state: Shared,
    game_id: String,
    watcher: Option<String>,
    session_digest: Vec<u8>,
    mut socket: WebSocket,
) {
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
    if let Some(account) = &watcher {
        state.social.watch_start(account, &game_id);
    }
    tracing::info!(game_id, "spectator socket opened");
    let mut authorization = tokio::time::interval(std::time::Duration::from_secs(5));
    loop {
        tokio::select! {
            _ = authorization.tick() => {
                if !matches!(crate::central::resolve_digest(&state, session_digest.clone()).await,
                    Ok(Some(session)) if watcher.as_deref() == Some(session.account_id.as_str()) && crate::moderation::admit(&session).is_ok()) {
                    break;
                }
            }
            frame = socket.recv() => match frame {
                // A spectator answers nothing: whatever it sends is dropped.
                Some(Ok(_)) => {}
                Some(Err(_)) | None => break,
            },
            update = rx.recv() => {
                let lagged = match update {
                    Ok(bytes) => {
                        let Ok(lagged) =
                            super::seat_socket::send_broadcast_batch(&mut socket, bytes, &mut rx)
                                .await
                        else {
                            break;
                        };
                        lagged.is_some()
                    }
                    Err(RecvError::Lagged(_)) => true,
                    Err(RecvError::Closed) => break,
                };
                // Behind: a fresh snapshot rather than a gap. Every spectator
                // shares the stream, so they all get it; it is idempotent.
                if lagged {
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
            },
            over = ready.changed() => {
                if over.is_err() || !*ready.borrow() {
                    break;
                }
            }
        }
    }
    let _ = socket.send(Message::Close(None)).await;
    if let Some(account) = &watcher {
        state.social.watch_end(account, &game_id);
    }
    recount(&state, &game_id, false);
    tracing::info!(game_id, "spectator socket closed");
}
