//! `/games/{id}/ws`: one seat's socket, forwarded to and from its engine
//! without being decoded.

use crate::{
    Envelope, ErrorBody, Json, LobbyGame, Message, Path, Query, RecvError, Shared, State,
    StatusCode, TICKET_REFUSED, WebSocket, WebSocketUpgrade, WsParams, auth, err, legacy_token,
    lobby, seatrate, spend_ticket, v1, wsticket,
};

pub(crate) async fn game_ws(
    State(state): State<Shared>,
    Path(id): Path<String>,
    Query(params): Query<WsParams>,
    ws: WebSocketUpgrade,
) -> Result<axum::response::Response, (StatusCode, Json<ErrorBody>)> {
    // Spent before the game is looked up, so a ticket is gone after one
    // presentation whatever the answer (#294).
    let granted = match params.ticket.as_deref() {
        Some(ticket) => Some(spend_ticket(
            &state,
            ticket,
            &wsticket::Door::Seat {
                game_id: id.clone(),
            },
        )?),
        None => None,
    };
    let seat = {
        let lobby = state.lobby.lock();
        let game = lobby
            .games
            .get(&id)
            .ok_or_else(|| err(StatusCode::NOT_FOUND, "no such game"))?;
        match granted {
            // The seat the ticket names, and only while its token is the one
            // that asked: a seat handed out again since (`.../seat`) is not
            // opened by a ticket its old token bought.
            Some(wsticket::Grant::Seat {
                seat,
                seat_token_hash,
                ..
            }) => game
                .seats
                .iter()
                .find(|s| {
                    s.seat == seat
                        && s.seat_token_hash
                            .as_ref()
                            .is_some_and(|h| auth::ct_eq(h, &seat_token_hash))
                })
                .map(|s| s.seat)
                .ok_or_else(|| err(StatusCode::UNAUTHORIZED, TICKET_REFUSED))?,
            Some(wsticket::Grant::Lobby { .. } | wsticket::Grant::Chair { .. }) => {
                return Err(err(StatusCode::UNAUTHORIZED, TICKET_REFUSED));
            }
            None => {
                let token = legacy_token(&state, params.token.as_deref(), "/games/{id}/ws")?;
                seat_of_token(game, token)
                    .ok_or_else(|| err(StatusCode::UNAUTHORIZED, "invalid seat token"))?
            }
        }
    };
    // After the seat token, as at the agent's door, and before the engine
    // hears of the socket: a table this client cannot read is not joined at
    // all. Said on the socket rather than as an HTTP status, because a
    // browser's `WebSocket` shows a script neither (#271).
    if let Some(why) = baylee_protocol::version_refusal("This client", params.protocol) {
        tracing::warn!(game_id = id, seat, why, "seat socket refused");
        return Ok(ws.on_upgrade(move |socket| refuse_seat(socket, why)));
    }
    Ok(ws.on_upgrade(move |socket| run_game_socket(state, id, seat, socket)))
}

/// The one frame a seat socket of another protocol is sent before it closes
/// (#271): a `HelloAck` that is not compatible, carrying this gateway's
/// version and the sentence naming both.
pub(crate) async fn refuse_seat(mut socket: WebSocket, why: String) {
    use prost::Message as _;
    let ack = Envelope {
        msg: Some(v1::envelope::Msg::HelloAck(v1::HelloAck {
            protocol_version: baylee_protocol::PROTOCOL_VERSION,
            compatible: false,
            message: why,
        })),
    };
    if socket
        .send(Message::Binary(ack.encode_to_vec().into()))
        .await
        .is_ok()
    {
        let _ = socket.send(Message::Close(None)).await;
    }
}

/// The seat of `game` whose token is `token`, compared in constant time.
pub(crate) fn seat_of_token(game: &LobbyGame, token: &str) -> Option<usize> {
    let token_hash = auth::token_hash(token);
    game.seats
        .iter()
        .find(|s| {
            s.seat_token_hash
                .as_ref()
                .is_some_and(|h| auth::ct_eq(h, &token_hash))
        })
        .map(|s| s.seat)
}

/// How long a seat socket waits for its game's engine to attach.
///
/// A seat may open its socket the moment the lobby says "playing", which is
/// before the agent has finished starting the process. Generous, because the
/// alternative is a client that has to poll and guess.
pub(crate) const ENGINE_WAIT_SECS: u64 = 30;

/// The biggest frame a seat may send.
///
/// A player's frame is a `PlayerActionMsg` carrying a JSON action — hundreds
/// of bytes at most. The gateway forwards these without decoding them, so
/// this is the only bound on what one seat can make the engine read.
pub(crate) const MAX_SEAT_FRAME: usize = 64 * 1024;

/// Waits until the game's engine is attached.
pub(crate) async fn engine_ready(ready: &mut tokio::sync::watch::Receiver<bool>) -> bool {
    let wait = async {
        loop {
            if *ready.borrow_and_update() {
                return true;
            }
            if ready.changed().await.is_err() {
                return false;
            }
        }
    };
    tokio::time::timeout(std::time::Duration::from_secs(ENGINE_WAIT_SECS), wait)
        .await
        .unwrap_or(false)
}

/// Sends one frame to a game's engine. False when there is no engine to send
/// to, which is the end of this socket.
pub(crate) fn to_engine(state: &Shared, game_id: &str, msg: v1::envelope::Msg) -> bool {
    let lobby = state.lobby.lock();
    lobby.games.get(game_id).is_some_and(|game| {
        game.engine
            .as_ref()
            .is_some_and(|tx| tx.send(Envelope { msg: Some(msg) }).is_ok())
    })
}

/// Whether chair `seat` of `game` still holds a seat token. A seated
/// player's chair loses it only when their account is deleted (#292).
pub(crate) fn holds_token(game: &lobby::LobbyGame, seat: usize) -> bool {
    game.seats
        .get(seat)
        .is_some_and(|chair| chair.seat_token_hash.is_some())
}

/// [`holds_token`] for a game named by id, which is gone once reaped.
pub(crate) fn still_seated(state: &Shared, game_id: &str, seat: usize) -> bool {
    let lobby = state.lobby.lock();
    lobby
        .games
        .get(game_id)
        .is_some_and(|game| holds_token(game, seat))
}

/// One seat's socket: everything it says goes to the engine tagged with its
/// seat, and everything the engine addresses to that seat comes back.
///
/// The gateway never decodes either direction. It cannot: it does not link the
/// rules kernel, and the whole point of the engine plane is that it does not
/// have to.
/// Closes a seat socket that sent more than its allowance (#284), with 1008
/// and a line in the log.
///
/// Closed rather than dropped: a client whose answer vanished would wait on
/// a question it thinks it answered, and a closed one dials again on its own
/// schedule.
pub(crate) async fn close_over_allowance(
    socket: &mut WebSocket,
    game_id: &str,
    seat: usize,
    meter: &seatrate::Meter,
) {
    tracing::warn!(
        game_id,
        seat,
        frames = meter.frames,
        largest_burst = meter.largest_burst(),
        "seat socket over its frame allowance; closing"
    );
    let over = axum::extract::ws::CloseFrame {
        code: axum::extract::ws::close_code::POLICY,
        reason: "too many frames".into(),
    };
    let _ = socket.send(Message::Close(Some(over))).await;
}

pub(crate) async fn run_game_socket(
    state: Shared,
    game_id: String,
    seat: usize,
    mut socket: WebSocket,
) {
    // Subscribe BEFORE announcing the seat, so this socket cannot miss its
    // own first view; every envelope addressed to this seat arrives here,
    // including the ones produced by the opponent's actions.
    //
    // The lobby's changes too, under the same lock the chair is read with:
    // a chair loses its seat token when its player's account is deleted
    // (#292), which the socket hears about as a change and then reads.
    let (mut rx, mut ready, mut changed) = {
        let lobby = state.lobby.lock();
        let Some(game) = lobby.games.get(&game_id).filter(|g| holds_token(g, seat)) else {
            return;
        };
        (
            game.updates.subscribe(),
            game.ready.subscribe(),
            state.lobby_changed.subscribe(),
        )
    };
    if !engine_ready(&mut ready).await {
        tracing::warn!(game_id, seat, "no engine attached; seat socket closing");
        return;
    }
    let attach = v1::envelope::Msg::SeatAttached(v1::SeatAttached {
        seat: seat as u32,
        resync: false,
    });
    if !to_engine(&state, &game_id, attach) {
        return;
    }
    // What this seat sends, for the line logged when it goes, and how much
    // it may (#284).
    let opened = std::time::Instant::now();
    let mut meter = seatrate::Meter::new(opened);
    let mut allowance = seatrate::Allowance::new(opened, seatrate::RATE, seatrate::BURST);
    loop {
        tokio::select! {
            frame = socket.recv() => {
                if let Some(Ok(_)) = &frame {
                    let now = std::time::Instant::now();
                    meter.note(now);
                    if !allowance.admit(now) {
                        close_over_allowance(&mut socket, &game_id, seat, &meter).await;
                        break;
                    }
                }
                match frame {
                    Some(Ok(Message::Binary(data))) => {
                        if data.len() > MAX_SEAT_FRAME {
                            tracing::warn!(game_id, seat, len = data.len(), "oversized seat frame");
                            continue;
                        }
                        let tagged = v1::envelope::Msg::SeatFrame(v1::SeatFrame {
                            seat: seat as u32,
                            envelope: data.into(),
                        });
                        if !to_engine(&state, &game_id, tagged) {
                            break;
                        }
                    }
                    // Pings are answered by axum; ignore other frame kinds.
                    Some(Ok(_)) => {}
                    Some(Err(_)) | None => break,
                }
            }
            update = rx.recv() => {
                match update {
                    Ok((p, bytes)) => {
                        if p == seat as u8
                            && futures_util::SinkExt::send(&mut socket, Message::Binary(bytes.into()))
                                .await
                                .is_err()
                        {
                            break;
                        }
                    }
                    Err(tokio::sync::broadcast::error::RecvError::Lagged(n)) => {
                        // Dropping the player was the old answer. Now that a
                        // seat's whole state can be rebuilt on demand, ask for
                        // it instead: the gap in the stream stops mattering.
                        tracing::warn!(game_id, seat, n, "seat socket lagged; resyncing");
                        let resync = v1::envelope::Msg::SeatAttached(v1::SeatAttached {
                            seat: seat as u32,
                            resync: true,
                        });
                        if !to_engine(&state, &game_id, resync) {
                            break;
                        }
                    }
                    Err(tokio::sync::broadcast::error::RecvError::Closed) => break,
                }
            }
            // Its player's account was deleted (#292): the chair is the
            // house's once the engine hears the seat has gone, and the
            // socket goes now.
            news = changed.recv() => {
                if matches!(news, Err(RecvError::Closed)) || !still_seated(&state, &game_id, seat) {
                    break;
                }
            }
        }
    }
    tracing::info!(
        game_id,
        seat,
        frames = meter.frames,
        busiest_second = meter.busiest_second(),
        largest_burst = meter.largest_burst(),
        "seat socket closed"
    );
    // The engine runs a decision clock only for a seat that can answer, so it
    // has to be told when one walks away.
    to_engine(
        &state,
        &game_id,
        v1::envelope::Msg::SeatDetached(v1::SeatDetached { seat: seat as u32 }),
    );
}
