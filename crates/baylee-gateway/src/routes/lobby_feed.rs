//! `/lobby/ws`: the listing, pushed whole whenever the lobby moves.

use crate::{
    ErrorBody, Json, LobbyWsParams, Message, Query, RecvError, Shared, State, StatusCode,
    TICKET_REFUSED, WebSocket, WebSocketUpgrade, auth, err, legacy_token, listing_page, lobby,
    spend_ticket, store, wsticket,
};

/// The lobby's push channel.
///
/// The listing used to be re-read every two seconds by every client sitting
/// in the lobby, because nothing could tell them a chair had moved. This can:
/// it sends the listing on connect and again whenever anything in the lobby
/// changes, rendered for *this* reader with *their* search.
///
/// @channel lobby /lobby/ws
/// @scope public
/// @summary The lobby feed
/// @description The lobby listing, pushed: the page this socket asked for,
/// sent as the socket opens and again, whole, whenever anything in the lobby
/// changes (a table opened or closed, a chair taken, freed, arranged or
/// readied, a game started or ended, the first engine agent arriving or the
/// last one leaving). It is the object `GET /lobby/games` answers for the
/// same query, rendered for the caller. A reader that falls behind is sent
/// the newest page, never the ones it missed. A change of search or page is
/// a new subscription, so close the socket and open another with the new
/// query. Buy a ticket with the session (`POST /ws-ticket` with
/// `{"socket":"lobby"}`) and open the socket with it and the search of
/// `GET /lobby/games` (`q`, `offset`, `limit`, `waiting_only`). The
/// upgrade is refused as an HTTP answer, not a frame: with `401` and
/// `ticket expired or used` for a ticket that is unknown, spent, expired or
/// for another socket, or whose session has since signed out, lapsed, been
/// banned or must accept new terms, and with `503` when the account store
/// or Baylee Central cannot be asked. The client sends nothing: the gateway
/// reads the socket only to notice it closing. It pings every 20 seconds
/// and drops a socket that has said nothing for 50 (a pong counts). It
/// checks the session again every 5 seconds, and once it is signed out,
/// lapsed or banned, or the account is deleted, it sends a close frame
/// without a code. An open lobby socket counts its account as online
/// (`GET /lobby/stats`) and keeps its waiting-room chairs.
/// @ws-query ticket: String required "A single-use ticket bought for the lobby feed with `POST /ws-ticket`"
/// @ws-query token: String deprecated "Instead of a ticket, the session token itself, as clients from before tickets send it; accepted until 2026-10-31 (UTC) and then refused with `401`. A ticket wins when both are sent."
/// @ws-query q: String "Text to find, case-insensitively and anywhere, in a table's name or its host's display name; a table's exact id also matches. Blank matches every table."
/// @ws-query offset: u32 "How many matching tables to skip; 0 when absent."
/// @ws-query limit: u32 default=25 "How many tables the page holds: 25 when absent, and anything outside 1 to 100 is taken as the nearer of the two."
/// @ws-query `waiting_only`: bool "Whether to leave out the games already being played; `true`, `1` and `yes` say so."
/// @send Page: `$LobbyPage` application/json "The page this socket asked for, whole: on connect and after every change to the lobby"
pub(crate) async fn lobby_ws(
    State(state): State<Shared>,
    Query(params): Query<LobbyWsParams>,
    ws: WebSocketUpgrade,
) -> Result<axum::response::Response, (StatusCode, Json<ErrorBody>)> {
    let (account_id, session_digest) = if let Some(ticket) = params.ticket.as_deref() {
        let wsticket::Grant::Lobby {
            account_id,
            session,
        } = spend_ticket(&state, ticket, &wsticket::Door::Lobby)?
        else {
            return Err(err(StatusCode::UNAUTHORIZED, TICKET_REFUSED));
        };
        // The session that asked must still stand: signed out, lapsed, or
        // its account deleted since, the ticket opens nothing.
        match crate::central::resolve_digest(&state, session.clone())
            .await
            .map_err(|status| err(status, TICKET_REFUSED))?
        {
            Some(resolved)
                if resolved.account_id == account_id
                    && crate::moderation::admit(&resolved).is_ok()
                    && crate::terms_admit(&resolved).is_ok() =>
            {
                (account_id, session)
            }
            _ => {
                tracing::debug!("a lobby ticket outlived its session");
                return Err(err(StatusCode::UNAUTHORIZED, TICKET_REFUSED));
            }
        }
    } else {
        let token = legacy_token(&state, params.token.as_deref(), "/lobby/ws")?;
        let digest = auth::token_digest(token);
        let session = crate::central::resolve_digest(&state, digest.clone())
            .await
            .map_err(|status| err(status, TICKET_REFUSED))?
            .ok_or_else(|| err(StatusCode::UNAUTHORIZED, "invalid or expired token"))?;
        crate::moderation::admit(&session)?;
        crate::terms_admit(&session)?;
        (session.account_id, digest)
    };
    Ok(ws.on_upgrade(move |socket| {
        run_lobby_socket(state, account_id, session_digest, params.query, socket)
    }))
}

/// How often a quiet lobby socket is pinged.
const PING_EVERY: std::time::Duration = std::time::Duration::from_secs(20);

/// How long a lobby socket may go without a word (a pong counts) before it
/// is taken for dead: two pings unanswered and some slack.
const QUIET: std::time::Duration = std::time::Duration::from_secs(50);

/// Pushes the listing to one reader until they go away.
pub(crate) async fn run_lobby_socket(
    state: Shared,
    account_id: String,
    session_digest: Vec<u8>,
    query: lobby::LobbyQuery,
    mut socket: WebSocket,
) {
    // Subscribed before the first send, so a change that lands while the
    // opening listing is being rendered is not lost between the two.
    let mut changed = state.lobby_changed.subscribe();
    // And before asking whether the account is still there (#292): one
    // deleted after the door read its session is refused here, and one
    // deleted later is named on `departed`.
    let mut departed = state.departed.subscribe();
    if !matches!(store::account(&state.db, &account_id).await, Ok(Some(_))) {
        let _ = socket.send(Message::Close(None)).await;
        return;
    }
    // Counted as present for as long as this socket is open, and no longer:
    // every return below drops it (`GET /lobby/stats`).
    let present = state.presence.enter(&account_id);
    serve_listing(
        &state,
        &account_id,
        &session_digest,
        &query,
        &mut socket,
        &mut changed,
        &mut departed,
    )
    .await;
    drop(present);
    crate::departure::socket_closed(&state, &account_id);
}

/// The listing, sent and re-sent until the socket or the account goes.
async fn serve_listing(
    state: &Shared,
    account_id: &str,
    session_digest: &[u8],
    query: &lobby::LobbyQuery,
    socket: &mut WebSocket,
    changed: &mut tokio::sync::broadcast::Receiver<()>,
    departed: &mut tokio::sync::broadcast::Receiver<String>,
) {
    // A socket whose far end vanished without a word (a lid closed, a
    // network gone) is never read as closed by `recv`; a ping it cannot
    // answer is how it is found (#B7). Every client answers pings on its
    // own (tungstenite, browsers), so silence past `QUIET` is a dead line.
    let mut ping = tokio::time::interval(PING_EVERY);
    ping.tick().await;
    let mut heard = tokio::time::Instant::now();
    let mut authorization = tokio::time::interval(std::time::Duration::from_secs(5));
    loop {
        let payload = listing_page(state, account_id, query).await.to_string();
        if socket.send(Message::Text(payload.into())).await.is_err() {
            return;
        }
        loop {
            tokio::select! {
                _ = authorization.tick() => {
                    if !matches!(crate::central::resolve_digest(state, session_digest.to_vec()).await,
                        Ok(Some(session)) if session.account_id == account_id && crate::moderation::admit(&session).is_ok()) {
                        let _ = socket.send(Message::Close(None)).await;
                        return;
                    }
                }
                // A reader that fell behind is sent the state of the world,
                // not the history it missed: the payload is the whole
                // listing every time, so one late send says everything the
                // skipped ones would have.
                news = changed.recv() => match news {
                    Ok(()) | Err(RecvError::Lagged(_)) => break,
                    Err(RecvError::Closed) => return,
                },
                gone = departed.recv() => match gone {
                    Ok(id) if id != account_id => {}
                    // Its own account is gone, or so many went at once that
                    // this reader lost count and cannot tell. A reader that
                    // is still signed in dials again and is let back in.
                    _ => {
                        let _ = socket.send(Message::Close(None)).await;
                        return;
                    }
                },
                // The client says nothing on this socket, so the one thing
                // reading it learns is that it has gone. Without this arm a
                // closed socket was noticed only at the next lobby change,
                // and its reader counted as present until then
                // (`GET /lobby/stats`).
                said = socket.recv() => match said {
                    None | Some(Err(_) | Ok(Message::Close(_))) => return,
                    Some(Ok(_)) => heard = tokio::time::Instant::now(),
                },
                _ = ping.tick() => {
                    if heard.elapsed() > QUIET
                        || socket.send(Message::Ping(Vec::new().into())).await.is_err()
                    {
                        return;
                    }
                }
            }
        }
    }
}
