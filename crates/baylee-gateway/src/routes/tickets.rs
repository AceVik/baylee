//! Opening a socket: the single-use tickets a session or seat token buys
//! (#294), and the old token in the address until its end.

use crate::{
    AppState, Deserialize, ErrorBody, HeaderMap, Json, Shared, State, StatusCode, auth,
    authed_session, err, lobby, seat_of_token, wsticket,
};

/// What a seat socket is opened with (`/games/{id}/ws`).
///
/// Never logged, and deliberately no `Debug`: both secrets ride here.
#[derive(Deserialize)]
pub(crate) struct WsParams {
    /// A ticket from `POST /ws-ticket` (#294): what a client dials with.
    #[serde(default)]
    pub(crate) ticket: Option<String>,
    /// The seat token itself, as clients from before #294 send it; accepted
    /// until [`wsticket::LEGACY_UNTIL`] and then refused.
    #[serde(default)]
    pub(crate) token: Option<String>,
    /// The protocol the dialler speaks (#271). A client from before it was
    /// sent says nothing, which reads as 0 and is refused.
    #[serde(default)]
    pub(crate) protocol: u32,
}

/// What `/lobby/ws` is opened with: a ticket, and the same search a
/// `GET /lobby/games` would carry.
#[derive(Deserialize)]
pub(crate) struct LobbyWsParams {
    /// A ticket from `POST /ws-ticket` (#294), bound to the session that
    /// asked for it.
    #[serde(default)]
    pub(crate) ticket: Option<String>,
    /// The account's session token, as clients from before #294 send it;
    /// accepted until [`wsticket::LEGACY_UNTIL`] and then refused.
    #[serde(default)]
    pub(crate) token: Option<String>,
    /// The page and search this reader wants, flattened so the socket URL and
    /// the HTTP route take the identical parameters.
    #[serde(flatten)]
    pub(crate) query: lobby::LobbyQuery,
}

/// What an upgrade whose ticket opened nothing is answered, whatever the
/// reason: unknown, used, expired, or for another socket. One sentence, so a
/// stranger learns nothing from it, and a fixed one, so a client can tell it
/// from every other `401` and fetch a fresh ticket.
pub(crate) const TICKET_REFUSED: &str = baylee_protocol::TICKET_REFUSED;

/// Which socket a ticket is asked for (`POST /ws-ticket`).
#[derive(Deserialize)]
#[serde(tag = "socket", rename_all = "snake_case")]
pub(crate) enum TicketFor {
    /// `/lobby/ws`, proven by the session in `Authorization`.
    Lobby,
    /// `/games/{game}/ws`, proven by that seat's token in `Authorization`.
    Seat {
        /// The game.
        game: String,
    },
    /// `/games/{game}/watch`, proven by the session; refused `403` at a
    /// table that allows no spectators.
    Watch {
        /// The game.
        game: String,
    },
}

/// `POST /ws-ticket` — trade a bearer token for a ticket to open one socket
/// with (#294).
///
/// The bearer is what proves the socket today: the session for the lobby
/// feed, the seat token for a seat. The ticket is bound to it and to the
/// socket it names; see `wsticket.rs`. Neither the bearer nor the ticket is
/// ever logged.
pub(crate) async fn ws_ticket(
    State(state): State<Shared>,
    headers: HeaderMap,
    Json(body): Json<TicketFor>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ErrorBody>)> {
    let grant = match body {
        TicketFor::Lobby => {
            let session = authed_session(&state, &headers).await?;
            let bearer = bearer_token(&headers).unwrap_or_default();
            wsticket::Grant::Lobby {
                account_id: session.account_id,
                session: auth::token_digest(bearer),
            }
        }
        TicketFor::Watch { game } => {
            authed_session(&state, &headers).await?;
            let bearer = bearer_token(&headers).unwrap_or_default();
            {
                let lobby = state.lobby.lock();
                let table = lobby
                    .games
                    .get(&game)
                    .ok_or_else(|| err(StatusCode::NOT_FOUND, "no such game"))?;
                super::may_watch(table)?;
            }
            wsticket::Grant::Watch {
                game_id: game,
                session: auth::token_digest(bearer),
            }
        }
        TicketFor::Seat { game } => {
            let bearer = bearer_token(&headers)
                .ok_or_else(|| err(StatusCode::UNAUTHORIZED, "missing bearer token"))?;
            let token_hash = auth::token_hash(bearer);
            let lobby = state.lobby.lock();
            let table = lobby
                .games
                .get(&game)
                .ok_or_else(|| err(StatusCode::NOT_FOUND, "no such game"))?;
            let seated = seat_of_token(table, bearer)
                .ok_or_else(|| err(StatusCode::UNAUTHORIZED, "invalid seat token"))?;
            wsticket::Grant::Seat {
                game_id: game,
                seat: seated,
                seat_token_hash: token_hash,
            }
        }
    };
    let ticket = state
        .tickets
        .issue(grant, std::time::Instant::now())
        .map_err(|wsticket::Full| {
            tracing::warn!("the socket ticket store is full");
            err(
                StatusCode::SERVICE_UNAVAILABLE,
                "too many sockets are being opened",
            )
        })?;
    Ok(Json(serde_json::json!({
        "ticket": ticket,
        "expires_in": state.tickets.ttl().as_secs(),
    })))
}

/// The bearer token of a request, if it names one.
pub(crate) fn bearer_token(headers: &HeaderMap) -> Option<&str> {
    headers
        .get("authorization")
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.strip_prefix("Bearer "))
}

/// Spends an upgrade's ticket on `door`, or says why it opened nothing.
///
/// The reason goes to the debug log for whoever runs the gateway; the
/// caller gets [`TICKET_REFUSED`] whichever it was.
pub(crate) fn spend_ticket(
    state: &AppState,
    ticket: &str,
    door: &wsticket::Door,
) -> Result<wsticket::Grant, (StatusCode, Json<ErrorBody>)> {
    state
        .tickets
        .consume(ticket, door, std::time::Instant::now())
        .map_err(|why| {
            tracing::debug!(?why, "a socket ticket opened nothing");
            err(StatusCode::UNAUTHORIZED, TICKET_REFUSED)
        })
}

/// A token in the query string, from a client older than #294, while the
/// window for those is still open ([`wsticket::LEGACY_UNTIL`]).
///
/// Said at `info` each time, naming the route and never the token, so an
/// operator can see when the last old client stopped dialling.
pub(crate) fn legacy_token<'a>(
    state: &AppState,
    token: Option<&'a str>,
    route: &'static str,
) -> Result<&'a str, (StatusCode, Json<ErrorBody>)> {
    let token = token.ok_or_else(|| err(StatusCode::UNAUTHORIZED, "missing ticket"))?;
    if !wsticket::legacy_open(state.legacy_until, auth::now_secs()) {
        return Err(err(
            StatusCode::UNAUTHORIZED,
            "a token in the address is no longer accepted; update the client",
        ));
    }
    tracing::info!(
        route,
        until = wsticket::LEGACY_UNTIL_DATE,
        "opened with a token in the query string (#294)"
    );
    Ok(token)
}
