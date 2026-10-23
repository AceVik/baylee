//! The lobby listing, as one account sees it.

use crate::{
    ErrorBody, HeaderMap, Json, Query, Serialize, Shared, State, StatusCode, authed, lobby,
};

/// The whole listing as one account sees it.
///
/// @description Every table the lobby lists, as the caller sees it.
/// @openapi
#[derive(Serialize)]
pub(crate) struct LobbyListing {
    // In key order: the bytes the `json!` it replaced wrote (serde_json sorts keys).
    /// @description Whether any engine agent is connected, so whether a
    /// game can start at all: without one the lobby still lists rooms, and
    /// opening or starting a game answers `503`.
    agents_available: bool,
    /// @description The tables: rooms waiting for players and games being
    /// played. A finished game is in no listing.
    games: Vec<lobby::LobbyTable>,
}

/// One page of the listing, searched and counted.
///
/// @description One page of the lobby, as the caller sees it: rooms still
/// waiting first, then the newest, the id breaking a tie, so paging never
/// hands a table out twice. `GET /lobby/ws` pushes the same object on every
/// change.
/// @openapi
#[derive(Serialize)]
pub(crate) struct LobbyPage {
    // In key order: the bytes the `json!` it replaced wrote (serde_json sorts keys).
    #[serde(flatten)]
    listing: LobbyListing,
    /// @description How many tables a page holds (1 to 100).
    limit: usize,
    /// @description How many matching tables were skipped before this page.
    offset: usize,
    /// @description How many tables the search matched, over every page.
    total: usize,
}

/// The lobby listing as one account sees it, whole.
///
/// What the mutating routes answer with: a player who just arranged a chair
/// is looking at one room, not at a page of the lobby, and handing them back
/// a page would make the room they are in vanish from their own screen if it
/// happened to fall off the end of it.
pub(crate) async fn listing(state: &Shared, account_id: &str) -> LobbyListing {
    let names = seated_names(state).await;
    let agents_available = state.agents.lock().any_connected();
    let lobby = state.lobby.lock();
    LobbyListing {
        agents_available,
        games: lobby.tables(account_id, &names),
    }
}

/// One page of the listing, searched and counted.
pub(crate) async fn lobby_page(
    state: &Shared,
    account_id: &str,
    query: &lobby::LobbyQuery,
) -> LobbyPage {
    let names = seated_names(state).await;
    // Whether a game could start at all: with no agent connected the lobby
    // still lists rooms, and creating or starting one answers 503. Said here
    // so a client can say so first, and pushed again whenever it changes.
    let agents_available = state.agents.lock().any_connected();
    let lobby = state.lobby.lock();
    let (games, total) = lobby.page(account_id, &names, query);
    LobbyPage {
        listing: LobbyListing {
            agents_available,
            games,
        },
        total,
        offset: query.offset,
        limit: query.page(),
    }
}

/// [`lobby_page`] as JSON, for the lobby feed (`GET /lobby/ws`), which
/// sends it as text. Nothing in it can fail to serialise (every key is a
/// string), so `null` is never sent.
pub(crate) async fn listing_page(
    state: &Shared,
    account_id: &str,
    query: &lobby::LobbyQuery,
) -> serde_json::Value {
    serde_json::to_value(lobby_page(state, account_id, query).await).unwrap_or_default()
}

/// Display names for every account sitting at a visible table.
///
/// The lobby guard is taken, read and dropped before the database is asked
/// anything, and that order is now load-bearing rather than tidy: a
/// `parking_lot` guard held across an `.await` makes the whole future
/// `!Send`, which axum refuses to accept as a handler. The ordering was
/// already right — it is the reason this conversion was small.
///
/// Read through the name book (`namebook.rs`): one query for the accounts that
/// sat down since the last render, and none at all when nobody did.
pub(crate) async fn seated_names(
    state: &Shared,
) -> std::sync::Arc<std::collections::HashMap<String, String>> {
    let wanted = state.lobby.lock().seated_accounts();
    state.names.names(&state.db, wanted).await
}

/// The lobby, searched and paged.
///
/// @route GET /lobby/games
/// @scope public
/// @summary The lobby, searched and paged
/// @description One page of the tables this gateway lists: rooms waiting
/// for players and games being played, each described completely (its
/// chairs, who sits in them, what they brought, the pace). A client with no
/// lobby socket polls this; `GET /lobby/ws` pushes the same page on every
/// change.
/// @tag Lobby
/// @security bearer
/// @insert `SessionRefusals`
/// @query-params `$LobbyQuery`
pub(crate) async fn list_games(
    State(state): State<Shared>,
    headers: HeaderMap,
    Query(query): Query<lobby::LobbyQuery>,
) -> Result<Json<LobbyPage>, (StatusCode, Json<ErrorBody>)> {
    let account_id = authed(&state, &headers).await?;
    Ok(Json(lobby_page(&state, &account_id, &query).await))
}

/// The authenticated account's resumable chairs. No public query, names,
/// decks, account IDs or credentials are accepted or returned.
///
/// @route GET /lobby/seats
/// @scope public
/// @summary Your chairs you may take back
/// @description The chairs the caller holds at running games and waiting
/// rooms, at most 32, running games first, then the newest. Each is taken
/// back with `POST /lobby/games/{id}/seat`. A rematch chair not yet claimed
/// is not one.
/// @tag Lobby
/// @security bearer
/// @insert `SessionRefusals`
pub(crate) async fn list_seats(
    State(state): State<Shared>,
    headers: HeaderMap,
) -> Result<Json<ResumableSeats>, (StatusCode, Json<ErrorBody>)> {
    let account_id = authed(&state, &headers).await?;
    Ok(Json(ResumableSeats {
        seats: state.lobby.lock().resumable(&account_id),
    }))
}

/// `GET /lobby/seats`.
/// @openapi
#[derive(Serialize)]
pub(crate) struct ResumableSeats {
    /// @description The caller's chairs, running games first, then the
    /// newest.
    seats: Vec<lobby::ResumableSeat>,
}
