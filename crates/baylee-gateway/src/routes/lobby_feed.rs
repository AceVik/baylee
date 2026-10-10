//! `/lobby/ws`: the listing, pushed whole whenever the lobby moves.

use crate::{
    ErrorBody, Json, LobbyWsParams, Message, Query, RecvError, Shared, State, StatusCode,
    TICKET_REFUSED, WebSocket, WebSocketUpgrade, auth, db_down, err, legacy_token, listing_page,
    lobby, spend_ticket, store, wsticket,
};

/// The lobby's push channel.
///
/// The listing used to be re-read every two seconds by every client sitting
/// in the lobby, because nothing could tell them a chair had moved. This can:
/// it sends the listing on connect and again whenever anything in the lobby
/// changes, rendered for *this* reader with *their* search.
pub(crate) async fn lobby_ws(
    State(state): State<Shared>,
    Query(params): Query<LobbyWsParams>,
    ws: WebSocketUpgrade,
) -> Result<axum::response::Response, (StatusCode, Json<ErrorBody>)> {
    let account_id = if let Some(ticket) = params.ticket.as_deref() {
        let wsticket::Grant::Lobby {
            account_id,
            session,
        } = spend_ticket(&state, ticket, &wsticket::Door::Lobby)?
        else {
            return Err(err(StatusCode::UNAUTHORIZED, TICKET_REFUSED));
        };
        // The session that asked must still stand: signed out, lapsed, or
        // its account deleted since, the ticket opens nothing.
        match store::resolve_digest(&state.db, session, auth::now_secs())
            .await
            .map_err(|e| db_down(&e))?
        {
            Some(session) if session.account_id == account_id => account_id,
            _ => {
                tracing::debug!("a lobby ticket outlived its session");
                return Err(err(StatusCode::UNAUTHORIZED, TICKET_REFUSED));
            }
        }
    } else {
        let token = legacy_token(&state, params.token.as_deref(), "/lobby/ws")?;
        store::resolve_token(&state.db, token, auth::now_secs())
            .await
            .map_err(|e| db_down(&e))?
            .ok_or_else(|| err(StatusCode::UNAUTHORIZED, "invalid or expired token"))?
            .account_id
    };
    Ok(ws.on_upgrade(move |socket| run_lobby_socket(state, account_id, params.query, socket)))
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
    loop {
        let payload = listing_page(state, account_id, query).await.to_string();
        if socket.send(Message::Text(payload.into())).await.is_err() {
            return;
        }
        loop {
            tokio::select! {
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
