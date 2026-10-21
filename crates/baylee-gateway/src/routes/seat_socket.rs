//! `/games/{id}/ws`: one seat's socket, forwarded to and from its engine
//! without being decoded.

use crate::{
    Envelope, ErrorBody, Json, LobbyGame, Message, Path, Query, RecvError, Shared, State,
    StatusCode, TICKET_REFUSED, WebSocket, WebSocketUpgrade, WsParams, auth, err, legacy_token,
    lobby, seatrate, spend_ticket, v1, wsticket,
};

mod ownership;
use ownership::{Connection, Ownership};

/// `GET /games/{id}/ws`: one chair's socket at a running game.
///
/// @channel seat /games/{id}/ws
/// @scope public
/// @summary A seat's socket
/// @description One chair's socket at a running game. Every frame is one
/// protobuf `Envelope` (`baylee.v1`) in a binary frame: the gateway wraps
/// each frame the client sends in a `SeatFrame` with this seat's number for
/// the game's engine, and hands back each frame the engine addresses to the
/// seat, decoding neither. Buy a ticket with the seat token
/// (`POST /ws-ticket` with `{"socket":"seat","game":"<id>"}` and the seat
/// token as `Authorization: Bearer`), then open the socket with it and the
/// protocol the client speaks (`baylee_protocol::seat_socket_path` builds
/// the address). The upgrade is refused as an HTTP answer, not a frame:
/// with `404` and `no such game`, and with `401` and
/// `ticket expired or used` for a ticket that is unknown, spent, expired or
/// for another socket or game, one bought by a seat token that is not the
/// chair's any more, and a chair whose player is out of the game or whose
/// authorization no longer holds. A client of another protocol is let in,
/// sent one `HelloAck` and closed, and the engine never hears of it.
/// Otherwise the socket waits up to 30 seconds for the game's engine (and
/// closes if none attaches), and the engine is told the seat is here. Its
/// first frames are the opening payload (`GameStaticMsg`) and the seat's
/// view (`StateDelta`). Until the table opens nothing is asked, no clock
/// runs and actions are dropped (the curtain): the client sends
/// `SeatReady` once its table is drawn, the engine counts the human seats
/// that are in `TableLoading`, and when the last is ready it names a shared
/// departure 1.5 seconds ahead for a 1.65-second entrance, after which each
/// seat is sent its view and question and `Curtain` last. A table not ready
/// within 120 seconds is sent an `Error` and ends; a socket that drops
/// while the table loads cancels the departure for everyone. From then on
/// the engine asks with `ChoiceRequest`, shows with `StateDelta`, and the
/// client answers with `PlayerActionMsg`; a refused answer is sent an
/// `Error` and the question again. The end of the game is a `ChoiceRequest`
/// whose `Pending` is `GameOver` (or a `GameClosed` when an admin ended
/// it), and then the socket closes; `GameEnded` never reaches a seat. A
/// frame the client sends is at most 64 KiB (`MAX_SEAT_FRAME`), and a
/// larger one ends the connection. A socket may send 512 frames at once
/// and 240 a second after that (`seatrate`, frames of every kind counted),
/// and past that it is closed with 1008 and `too many frames`. Text frames
/// are passed over. A socket that falls behind the engine is not dropped:
/// the engine is asked to send it a fresh snapshot. A newer socket for the
/// same chair replaces this one, which closes. Before each frame is
/// forwarded the seat token is checked, and the chair's whole
/// authorization too when the gateway itself has withdrawn something of
/// its account since the last check (moved to Baylee Central, password
/// changed or reset, banned, deleted). The whole authorization is also
/// checked every 5 seconds, so a Central session that lapsed, was signed
/// out or was revoked at Central stops the chair's frames within 5 seconds
/// (`docs/protocol.md`, Once a socket is open). The socket closes, with a
/// close frame without a code, once it no longer holds (the seat token was
/// replaced or ended, or the Central session behind it lapsed or was
/// banned), once the engine's link ends, and at the
/// first frame the client sends after the engine said the seat is out of
/// the game. While a chair has no socket, its frames are dropped and the
/// engine holds its place for the reconnect window (`docs/protocol.md`,
/// Leaving, and losing the connection).
/// @ws-query ticket: String required "A single-use ticket bought for this seat with `POST /ws-ticket` and the seat token"
/// @ws-query protocol: u32 required "The protocol the client speaks (`PROTOCOL_VERSION`, as `GET /info` answers it); another, or none, is answered with a `HelloAck` that is not compatible"
/// @ws-query token: String deprecated "Instead of a ticket, the seat token itself, as clients from before tickets send it; accepted until 2026-10-31 (UTC) and then refused with `401`. A ticket wins when both are sent."
/// parameters:
///   id:
///     description: The game's id, as the lobby lists it.
/// @send `HelloAck`: proto(baylee.v1.Envelope) application/x-protobuf "Envelope case `hello_ack`, from the gateway itself: the client's protocol is not the gateway's. `compatible` is false, `protocol_version` is the gateway's and `message` is a sentence naming both. The only frame; the socket closes after it."
/// @send `GameStaticMsg`: proto(baylee.v1.Envelope) application/x-protobuf "Envelope case `game_static`: the opening payload, always first on the socket, and sent again before a view that shows a printing the seat had not been shown. `view_version` is the view schema's version, outside the payload so a client that cannot render it can say so; `static_json` is a `GameStatic` (the roster and the seat's print table) as JSON, see `docs/protocol.md`."
/// @send `StateDelta`: proto(baylee.v1.Envelope) application/x-protobuf "Envelope case `state_delta`: the seat's view after something changed. `seq` is what a `ResumeGame` names; `view_json` is a `PlayerView` as JSON; `log_json` is a `LogTail` as JSON (the seat's game log lines since the last tail, appended by its `from`), empty when there is none. Several frames may repeat a view and `seq` to carry a long log, so read `log_json` from every frame."
/// @send `ChoiceRequest`: proto(baylee.v1.Envelope) application/x-protobuf "Envelope case `choice_request`: a question to this seat. `seq` as in `StateDelta`; `pending_json` is the engine's `Pending` as JSON, which names every answer the engine takes. A `Pending` of `GameOver` is the end of the game."
/// @send `TableLoading`: proto(baylee.v1.Envelope) application/x-protobuf "Envelope case `table_loading`, before the curtain: how many human seats are ready (`ready` of `total`) and, once every one is, the shared departure `enter_at_ms` in server Unix milliseconds; 0 cancels a departure."
/// @send `Curtain`: proto(baylee.v1.Envelope) application/x-protobuf "Envelope case `curtain`, empty: the entrance has finished and play may begin. Sent after each seat's view and question; a seat that attaches later is sent it with its snapshot."
/// @send `ClockProbeReply`: proto(baylee.v1.Envelope) application/x-protobuf "Envelope case `clock_probe`: the engine's echo of a `ClockProbe`, with `client_time_ms` as it was sent and `server_time_ms` the server's Unix milliseconds."
/// @send `Spectators`: proto(baylee.v1.Envelope) application/x-protobuf "Envelope case `spectators`: how many people watch the table (`count`), sent when it moves and when the seat attaches."
/// @send `Error`: proto(baylee.v1.Envelope) application/x-protobuf "Envelope case `error` (`code` 1, `message` a sentence): an answer or a seat setting the engine refused, followed by the question again when the seat holds one, or a table that could not start because not every player finished loading in 120 seconds."
/// @send `GameClosed`: proto(baylee.v1.Envelope) application/x-protobuf "Envelope case `game_closed`: an admin ended the game, and nobody won or lost; `reason` is the admin's words. The last frame of the game."
/// @send `AiLogForwarded`: proto(baylee.v1.Envelope) application/x-protobuf "Envelope case `ai_log`, from a debug engine only: what another seat's language model said beside an answer (`note`, `thinking`, and the `seat` that said it). A release engine forwards none."
/// @receive `PlayerActionMsg`: proto(baylee.v1.Envelope) application/x-protobuf "Envelope case `player_action`: the seat's answer. `action_json` is a `PlayerAction` as JSON, one the question's `Pending` names; `game_id` and `seat_token` are not read, since the socket is bound to its seat. Dropped before the curtain."
/// @receive `ResumeGame`: proto(baylee.v1.Envelope) application/x-protobuf "Envelope case `resume`: send what this seat missed after `last_seq`. Read-only, it moves nothing in the game; a seat that is current is sent nothing, and before the curtain the seat is sent its table without a question."
/// @receive `SeatReady`: proto(baylee.v1.Envelope) application/x-protobuf "Envelope case `seat_ready`, empty: this seat's opening table, art and render pipelines are prepared. Counts once per attach; a repeat, or one after the curtain, changes nothing."
/// @receive `SeatSettingMsg`: proto(baylee.v1.Envelope) application/x-protobuf "Envelope case `seat_setting`: a setting about the seat itself rather than a move, such as showing its hand to a teammate or asking to be shown one. `setting_json` is a `SeatSetting` as JSON; a refusal is an `Error`. Taken before the curtain too."
/// @receive `ClockProbe`: proto(baylee.v1.Envelope) application/x-protobuf "Envelope case `clock_probe` with the client's monotonic `client_time_ms` and `server_time_ms` 0: asks for the server's time, answered by a `ClockProbeReply`."
/// @receive `SeatMind`: proto(baylee.v1.Envelope) application/x-protobuf "Envelope case `seat_mind`: what answers this seat, as its own client declares it (a person, the house, a script, a language model by API or CLI), for the game's record only. Sent once on every socket before the seat's first answer and again when it changes; shown to no seat, and one that is malformed is dropped without a word."
/// @receive `AiLog`: proto(baylee.v1.Envelope) application/x-protobuf "Envelope case `ai_log`, from a seat bridge in a debug build: what its model said beside an answer. A debug engine forwards it to every other attached seat; a release engine drops it."
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

    /// Whether `account` is the one this capability answers for: the
    /// player's own, or the host's for a chair it handed to a seat bridge.
    fn answers_for(&self, account: &str) -> bool {
        self.account.as_deref() == Some(account)
    }

    /// Whether a frame read now may go on to the engine: at once, unless
    /// the gateway withdrew something of this capability's account since
    /// `departed` was last read, or lost count of what it withdrew; then as
    /// [`Self::valid`] answers. Its seat token is `Ownership::send`'s to check.
    async fn admits_frame(
        &self,
        departed: &mut tokio::sync::broadcast::Receiver<String>,
        state: &Shared,
        game: &str,
        seat: usize,
    ) -> bool {
        !self.heard_withdrawal(departed) || self.valid(state, game, seat).await
    }

    /// Whether this capability outlives what `departed` just said: unchanged
    /// when it named another account, else as [`Self::valid`] answers. A
    /// closed channel can no longer say anything, and is taken for no.
    async fn outlives(
        &self,
        gone: Result<String, RecvError>,
        state: &Shared,
        game: &str,
        seat: usize,
    ) -> bool {
        match gone {
            Ok(account) if !self.answers_for(&account) => true,
            Ok(_) | Err(RecvError::Lagged(_)) => self.valid(state, game, seat).await,
            Err(RecvError::Closed) => false,
        }
    }

    /// Whether `departed` named this capability's account since it was last
    /// read, or lost count. Reads only what is already there; never waits.
    fn heard_withdrawal(&self, departed: &mut tokio::sync::broadcast::Receiver<String>) -> bool {
        use tokio::sync::broadcast::error::TryRecvError;
        let mut heard = false;
        loop {
            match departed.try_recv() {
                Ok(account) => heard |= self.answers_for(&account),
                Err(TryRecvError::Lagged(_)) => heard = true,
                Err(TryRecvError::Empty) => return heard,
                Err(TryRecvError::Closed) => return true,
            }
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

/// How often an open seat socket checks its capability in full, whether or
/// not it sends: the longest a withdrawal this gateway does not make itself
/// (Central revoking a lease, a session running out) goes unheard by the
/// socket. The lobby, watch and social sockets recheck on the same beat.
const RECHECK: std::time::Duration = std::time::Duration::from_secs(5);

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
///
/// What a frame is authorized against (`docs/protocol.md` §"Once a socket
/// is open"): the seat token on every frame, and the capability behind it
/// in full when the socket opens, every [`RECHECK`], and before the next
/// frame whenever `departed` names its account or lost count.
///
/// So no frame reaches the engine after its seat token went, or after a
/// withdrawal this gateway made and named on `departed` (moved to Baylee
/// Central, password changed or reset, banned, deleted). A withdrawal it
/// does not make itself is heard only at the next full check, so frames
/// still reach the engine for up to one [`RECHECK`] and the length of that
/// check after it: a lease Central revoked (a newer login elsewhere, a ban
/// at Central), a lease or session that ran out, a Central-bound session
/// signed out (`POST /auth/logout` names nobody on `departed`), a change
/// another process made in the database. And a frame already past its
/// check when a withdrawal lands still goes on, as it always did. Until
/// 21.10.2026 every frame was checked in full: one database query a frame,
/// and for a Central-bound seat a call to Central besides, which held a
/// flooded socket's reading below its allowance (#284) on a loaded machine.
#[allow(clippy::too_many_lines)] // one select over all a seat socket waits on, read top to bottom
async fn run_game_socket(
    state: Shared,
    game_id: String,
    seat: usize,
    authorization: SeatAuthorization,
    ownership: Ownership,
    mut socket: WebSocket,
) {
    // Before the first full check, so nothing withdrawn after it is missed.
    let mut departed = state.departed.subscribe();
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
    let mut recheck = tokio::time::interval(RECHECK);
    loop {
        tokio::select! {
            _ = recheck.tick() => {
                if !connection.active() || !authorization.valid(&state, &game_id, seat).await { break; }
            }
            // An idle socket hears a withdrawal at once too, rather than at
            // the next recheck.
            gone = departed.recv() => {
                if !authorization.outlives(gone, &state, &game_id, seat).await { break; }
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
                        // Every action is authorized after reception: its
                        // seat token by `connection.send`, under the lobby
                        // lock, and its capability in full first if the
                        // gateway withdrew anything of its account since
                        // the last check. Read here and not only in the
                        // arm above, which `select!` may not have polled.
                        // No stale action reaches an engine after a
                        // withdrawal this gateway made; one it learns only
                        // by asking leaves up to a `RECHECK` (see above).
                        if !authorization.admits_frame(&mut departed, &state, &game_id, seat).await { break; }
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

#[cfg(test)]
mod tests {
    use super::SeatAuthorization;
    use tokio::sync::broadcast;

    fn answering_for(account: &str) -> SeatAuthorization {
        SeatAuthorization {
            hash: "seat token hash".into(),
            account: Some(account.into()),
            session: None,
        }
    }

    /// A frame waits for the full check only after the gateway withdrew
    /// something of its own account, or lost count of what it withdrew;
    /// every other frame goes on without asking the database.
    #[test]
    fn a_frame_waits_for_the_full_check_only_after_a_withdrawal_of_its_account() {
        let (departed, mut heard) = broadcast::channel(4);
        let seat = answering_for("me");
        assert!(!seat.heard_withdrawal(&mut heard), "nothing said");
        departed.send("someone else".to_owned()).unwrap();
        assert!(!seat.heard_withdrawal(&mut heard), "another account");
        departed.send("me".to_owned()).unwrap();
        departed.send("someone else".to_owned()).unwrap();
        assert!(seat.heard_withdrawal(&mut heard), "its own, among others");
        assert!(!seat.heard_withdrawal(&mut heard), "and only once");
        for _ in 0..5 {
            departed.send("someone else".to_owned()).unwrap();
        }
        assert!(seat.heard_withdrawal(&mut heard), "lagged: it cannot tell");
        assert!(!seat.heard_withdrawal(&mut heard), "caught up again");
        drop(departed);
        assert!(
            seat.heard_withdrawal(&mut heard),
            "a channel that can say nothing more"
        );
    }
}
