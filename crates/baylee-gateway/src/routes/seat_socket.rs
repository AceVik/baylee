//! `/games/{id}/ws`: one seat's socket, forwarded to and from its engine
//! without being decoded.

use crate::{
    Envelope, ErrorBody, Json, LobbyGame, Message, Path, Query, RecvError, Shared, State,
    StatusCode, TICKET_REFUSED, WebSocket, WebSocketUpgrade, WsParams, auth, err, legacy_token,
    lobby, seatrate, spend_ticket, v1, wsticket,
};

mod ownership;
use ownership::{Connection, Ownership};

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
    let (seat, hash) = {
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
                .map(|s| (s.seat, seat_token_hash.clone()))
                .ok_or_else(|| err(StatusCode::UNAUTHORIZED, TICKET_REFUSED))?,
            Some(
                wsticket::Grant::Lobby { .. }
                | wsticket::Grant::Chair { .. }
                | wsticket::Grant::Watch { .. }
                | wsticket::Grant::Social { .. },
            ) => {
                return Err(err(StatusCode::UNAUTHORIZED, TICKET_REFUSED));
            }
            None => {
                let token = legacy_token(&state, params.token.as_deref(), "/games/{id}/ws")?;
                seat_of_token(game, token)
                    .map(|seat| (seat, auth::token_hash(token)))
                    .ok_or_else(|| err(StatusCode::UNAUTHORIZED, "invalid seat token"))?
            }
        }
    };
    let authorization = SeatAuthorization::read(&state, &id, seat, &hash)
        .ok_or_else(|| err(StatusCode::UNAUTHORIZED, TICKET_REFUSED))?;
    // Reserve before the first asynchronous check: an older upgrade may finish
    // its Central request after a newer socket has already attached.
    let ownership = state
        .lobby
        .lock()
        .games
        .get_mut(&id)
        .and_then(|game| Ownership::reserve(game, seat, &hash))
        .ok_or_else(|| err(StatusCode::UNAUTHORIZED, TICKET_REFUSED))?;
    if !authorization.valid(&state, &id, seat).await {
        return Err(err(StatusCode::UNAUTHORIZED, TICKET_REFUSED));
    }
    // After the seat token, as at the agent's door, and before the engine
    // hears of the socket: a table this client cannot read is not joined at
    // all. Said on the socket rather than as an HTTP status, because a
    // browser's `WebSocket` shows a script neither (#271).
    if let Some(why) = baylee_protocol::version_refusal("This client", params.protocol) {
        tracing::warn!(game_id = id, seat, why, "seat socket refused");
        return Ok(ws.on_upgrade(move |socket| refuse_seat(socket, why)));
    }
    Ok(ws
        .max_message_size(MAX_SEAT_FRAME)
        .max_frame_size(MAX_SEAT_FRAME)
        .read_buffer_size(SEAT_READ_BUFFER)
        .on_upgrade(move |socket| {
            run_game_socket(state, id, seat, authorization, ownership, socket)
        }))
}

/// Frozen authorization of the socket, never replaced by a later device reclaiming a seat.
pub(crate) struct SeatAuthorization {
    hash: String,
    account: Option<String>,
    session: Option<Vec<u8>>,
}

impl SeatAuthorization {
    pub(crate) fn read(state: &Shared, game: &str, seat: usize, hash: &str) -> Option<Self> {
        let lobby = state.lobby.lock();
        let chair = lobby.games.get(game)?.seats.get(seat)?;
        if chair.seat_token_hash.as_deref() != Some(hash) {
            return None;
        }
        Some(Self {
            hash: hash.into(),
            account: chair
                .account_id
                .clone()
                .or_else(|| chair.delegate.as_ref().map(|d| d.by.clone())),
            session: chair.central_session.clone(),
        })
    }

    pub(crate) async fn valid(&self, state: &Shared, game: &str, seat: usize) -> bool {
        let same = state
            .lobby
            .lock()
            .games
            .get(game)
            .and_then(|g| g.seats.get(seat))
            .is_some_and(|chair| chair.seat_token_hash.as_deref() == Some(self.hash.as_str()));
        if !same {
            return false;
        }
        match &self.account {
            Some(account) => {
                account_capability_valid(state, account, self.session.as_deref()).await
            }
            None => false,
        }
    }
}

/// A capability minted before migration cannot silently gain Central authorization.
pub(crate) async fn account_capability_valid(
    state: &Shared,
    account: &str,
    session: Option<&[u8]>,
) -> bool {
    if let Some(digest) = session {
        return matches!(crate::central::resolve_digest(state, digest.to_vec()).await,
            Ok(Some(session)) if session.account_id == account && crate::moderation::admit(&session).is_ok());
    }
    matches!(
        baylee_db::central::linked(&state.db, account).await,
        Ok(false)
    )
}

/// Apply the same frozen seat/session checks to HTTP routes using a seat bearer.
pub(crate) async fn authorize_seat_bearer(
    state: &Shared,
    game_id: &str,
    headers: &axum::http::HeaderMap,
) -> Result<(), (StatusCode, Json<ErrorBody>)> {
    let token = crate::bearer_token(headers)
        .ok_or_else(|| err(StatusCode::UNAUTHORIZED, "missing seat token"))?;
    authorize_seat_token(state, game_id, token).await
}

pub(crate) async fn authorize_seat_token(
    state: &Shared,
    game_id: &str,
    token: &str,
) -> Result<(), (StatusCode, Json<ErrorBody>)> {
    let seat = {
        let lobby = state.lobby.lock();
        let game = lobby
            .games
            .get(game_id)
            .ok_or_else(|| err(StatusCode::NOT_FOUND, "no such game"))?;
        seat_of_token(game, token).ok_or_else(|| err(StatusCode::UNAUTHORIZED, TICKET_REFUSED))?
    };
    let grant = SeatAuthorization::read(state, game_id, seat, &auth::token_hash(token))
        .ok_or_else(|| err(StatusCode::UNAUTHORIZED, TICKET_REFUSED))?;
    if !grant.valid(state, game_id, seat).await {
        return Err(err(StatusCode::UNAUTHORIZED, TICKET_REFUSED));
    }
    Ok(())
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
///
/// Set on the socket itself, so a bigger message is refused as it arrives
/// and the socket closes; before, the transport's own 64 MiB was read whole
/// and only then dropped, which made the bound one on the engine and not on
/// the gateway's memory.
pub(crate) const MAX_SEAT_FRAME: usize = 64 * 1024;

/// How much a seat socket reads from the network at once.
///
/// tungstenite zeroes its whole read buffer before every read, 128 KiB by
/// default, and a seat's frames are hundreds of bytes: profiling a gateway
/// forwarding seat frames found that memset beside `recvfrom` itself, and
/// the buffer's pages resident for every socket. A frame bigger than this
/// is read in several calls.
pub(crate) const SEAT_READ_BUFFER: usize = 8 << 10;

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

/// Sends one frame to a game's engine down the link the socket took when it
/// attached. False once that engine's link is gone, which is the end of this
/// socket: the game is over or its engine has left.
pub(crate) fn to_engine(link: &lobby::EngineLink, msg: v1::envelope::Msg) -> bool {
    link.send(Envelope { msg: Some(msg) }).is_ok()
}

/// Whether chair `seat` of `game` still holds a seat token. A seated
/// player's chair loses it only when their account is deleted (#292).
pub(crate) fn holds_token(game: &lobby::LobbyGame, seat: usize) -> bool {
    game.seats
        .get(seat)
        .is_some_and(|chair| chair.seat_token_hash.is_some())
}

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
    // Finish the closing handshake instead of resetting TCP with unread flood
    // data, which can erase the close reason on Windows. No drained message
    // reaches the engine. Bound time, work and bytes even for an uncooperative
    // peer, and yield so a buffered flood cannot starve the deadline.
    let _ = tokio::time::timeout(std::time::Duration::from_secs(1), async {
        if socket.send(Message::Close(Some(over))).await.is_err() {
            return;
        }
        let mut bytes_left: usize = 1024 * 1024;
        for _ in 0..16_384 {
            let length = match socket.recv().await {
                Some(Ok(Message::Binary(data) | Message::Ping(data) | Message::Pong(data))) => {
                    data.len()
                }
                Some(Ok(Message::Text(text))) => text.len(),
                Some(Ok(Message::Close(_)) | Err(_)) | None => break,
            };
            let Some(remaining) = bytes_left.checked_sub(length) else {
                break;
            };
            bytes_left = remaining;
            tokio::task::yield_now().await;
        }
    })
    .await;
}

/// What a seat socket holds for its life, taken once as it attaches.
pub(crate) struct Attached {
    /// Everything the engine addresses to this seat.
    rx: tokio::sync::broadcast::Receiver<bytes::Bytes>,
    /// The lobby's changes, for a chair that loses its seat token.
    changed: tokio::sync::broadcast::Receiver<()>,
    /// Exclusive engine ownership, including cancellation-safe departure.
    connection: Connection,
}

/// Subscribes seat `seat` of `game_id`, waits for its engine and tells it
/// the seat is here. `None` when there is no such seat or no engine.
async fn attach(
    state: &Shared,
    game_id: &str,
    seat: usize,
    authorization: &SeatAuthorization,
    ownership: Ownership,
) -> Option<Attached> {
    // Subscribe BEFORE announcing the seat, so this socket cannot miss its
    // own first view; every envelope addressed to this seat arrives here,
    // including the ones produced by the opponent's actions.
    //
    // The lobby's changes too, under the same lock the chair is read with:
    // a chair loses its seat token when its player's account is deleted
    // (#292), which the socket hears about as a change and then reads.
    let (rx, mut ready, changed) = {
        let mut lobby = state.lobby.lock();
        let game = lobby
            .games
            .get_mut(game_id)
            .filter(|g| holds_token(g, seat))?;
        (
            game.outbox(seat).subscribe(),
            game.ready.subscribe(),
            state.lobby_changed.subscribe(),
        )
    };
    if !engine_ready(&mut ready).await {
        tracing::warn!(game_id, seat, "no engine attached; seat socket closing");
        return None;
    }
    if !authorization.valid(state, game_id, seat).await {
        return None;
    }
    let link = ownership.attach(state.lobby.lock().games.get_mut(game_id)?, seat)?;
    let connection = Connection {
        state: state.clone(),
        game: game_id.into(),
        seat,
        ownership,
        link,
    };
    Some(Attached {
        rx,
        changed,
        connection,
    })
}

/// Sends `first` and up to 64 immediately queued frames from `rx` in a single
/// WebSocket flush. Returns `Ok(Some(n))` if `rx` reported lagging by `n`
/// messages after the batch, `Ok(None)` if flushed normally, or `Err(())` if
/// the socket failed.
pub(crate) async fn send_broadcast_batch(
    socket: &mut WebSocket,
    first: bytes::Bytes,
    rx: &mut tokio::sync::broadcast::Receiver<bytes::Bytes>,
) -> Result<Option<u64>, ()> {
    use futures_util::SinkExt as _;
    use tokio::sync::broadcast::error::TryRecvError;
    socket.feed(Message::Binary(first)).await.map_err(|_| ())?;
    let mut lagged = None;
    for _ in 0..64 {
        match rx.try_recv() {
            Ok(more) => {
                socket.feed(Message::Binary(more)).await.map_err(|_| ())?;
            }
            Err(TryRecvError::Lagged(n)) => {
                lagged = Some(n);
                break;
            }
            Err(TryRecvError::Empty | TryRecvError::Closed) => break,
        }
    }
    socket.flush().await.map_err(|_| ())?;
    Ok(lagged)
}

/// One seat's socket: everything it says goes to the engine tagged with its
/// seat, and everything the engine addresses to that seat comes back.
///
/// The gateway never decodes either direction. It cannot: it does not link the
/// rules kernel, and the whole point of the engine plane is that it does not
/// have to.
///
/// Ownership checks and synchronous engine enqueues share the lobby lock, so
/// an old socket cannot act or detach after a replacement attaches. Network I/O
/// and Central authorization always happen outside that critical section.
async fn run_game_socket(
    state: Shared,
    game_id: String,
    seat: usize,
    authorization: SeatAuthorization,
    ownership: Ownership,
    mut socket: WebSocket,
) {
    if !authorization.valid(&state, &game_id, seat).await {
        return;
    }
    let Some(Attached {
        mut rx,
        mut changed,
        connection,
    }) = attach(&state, &game_id, seat, &authorization, ownership).await
    else {
        return;
    };
    // What this seat sends, for the line logged when it goes, and how much
    // it may (#284).
    // The player's own chair counts them as here (`presence.rs`); a seat
    // bridge's does not, its host is counted by `Lobby::bridging_accounts`.
    let account = player_account(&state, &game_id, seat);
    let present = account.as_deref().map(|id| state.presence.enter(id));
    let opened = std::time::Instant::now();
    let mut meter = seatrate::Meter::new(opened);
    let mut allowance = seatrate::Allowance::new(opened, seatrate::RATE, seatrate::BURST);
    let mut recheck = tokio::time::interval(std::time::Duration::from_secs(5));
    loop {
        tokio::select! {
            _ = recheck.tick() => {
                if !connection.active() || !authorization.valid(&state, &game_id, seat).await { break; }
            }
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
                    // At most `MAX_SEAT_FRAME`: the socket refuses a bigger
                    // message before it is read (`game_ws`).
                    Some(Ok(Message::Binary(data))) => {
                        // Every action is authorized after reception. Periodic checks
                        // also close idle sockets, while no stale action reaches an engine.
                        if !authorization.valid(&state, &game_id, seat).await { break; }
                        let tagged = v1::envelope::Msg::SeatFrame(v1::SeatFrame {
                            seat: seat as u32,
                            envelope: data,
                        });
                        if !connection.send(tagged) {
                            break;
                        }
                    }
                    // Pings are answered by axum; ignore other frame kinds.
                    Some(Ok(_)) => {}
                    Some(Err(_)) | None => break,
                }
            }
            update = rx.recv() => {
                let lagged = match update {
                    Ok(bytes) => {
                        if !connection.active() { break; }
                        let Ok(lagged) = send_broadcast_batch(&mut socket, bytes, &mut rx).await
                        else {
                            break;
                        };
                        lagged
                    }
                    Err(RecvError::Lagged(n)) => Some(n),
                    Err(RecvError::Closed) => break,
                };
                if let Some(n) = lagged {
                    // Dropping the player was the old answer. Now that a
                    // seat's whole state can be rebuilt on demand, ask for
                    // it instead: the gap in the stream stops mattering.
                    tracing::warn!(game_id, seat, n, "seat socket lagged; resyncing");
                    let resync = v1::envelope::Msg::SeatAttached(v1::SeatAttached {
                        seat: seat as u32,
                        resync: true,
                    });
                    if !authorization.valid(&state, &game_id, seat).await || !connection.send(resync) {
                        break;
                    }
                }
            }
            // Its player's account was deleted (#292): the chair is the
            // house's once the engine hears the seat has gone, and the
            // socket goes now.
            news = changed.recv() => {
                if matches!(news, Err(RecvError::Closed)) || !connection.active() {
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
    // hears departure before network closing can wait. A replaced socket's
    // guard cannot detach its successor; cancellation drops the guard too.
    drop(connection);
    drop(present);
    if let Some(account) = account {
        crate::departure::socket_closed(&state, &account);
    }
    let _ = tokio::time::timeout(
        std::time::Duration::from_secs(1),
        socket.send(Message::Close(None)),
    )
    .await;
}

/// Delegated bridge sockets do not count as a player's own online presence.
fn player_account(state: &Shared, game: &str, seat: usize) -> Option<String> {
    state
        .lobby
        .lock()
        .games
        .get(game)?
        .seats
        .get(seat)?
        .account_id
        .clone()
}
