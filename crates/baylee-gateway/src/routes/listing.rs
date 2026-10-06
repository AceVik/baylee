//! The lobby listing, as one account sees it.

use crate::{ErrorBody, HeaderMap, Json, Query, Shared, State, StatusCode, authed, lobby};

/// The lobby listing as one account sees it, whole.
///
/// What the mutating routes answer with: a player who just arranged a chair
/// is looking at one room, not at a page of the lobby, and handing them back
/// a page would make the room they are in vanish from their own screen if it
/// happened to fall off the end of it.
pub(crate) async fn listing(state: &Shared, account_id: &str) -> serde_json::Value {
    let names = seated_names(state).await;
    let agents_available = state.agents.lock().any_connected();
    let lobby = state.lobby.lock();
    serde_json::json!({
        "agents_available": agents_available,
        "games": lobby.list_for(account_id, &names),
    })
}

/// One page of the listing, searched and counted.
pub(crate) async fn listing_page(
    state: &Shared,
    account_id: &str,
    query: &lobby::LobbyQuery,
) -> serde_json::Value {
    let names = seated_names(state).await;
    // Whether a game could start at all: with no agent connected the lobby
    // still lists rooms, and creating or starting one answers 503. Said here
    // so a client can say so first, and pushed again whenever it changes.
    let agents_available = state.agents.lock().any_connected();
    let lobby = state.lobby.lock();
    let (games, total) = lobby.page_for(account_id, &names, query);
    serde_json::json!({
        "agents_available": agents_available,
        "games": games,
        "total": total,
        "offset": query.offset,
        "limit": query.page(),
    })
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
pub(crate) async fn list_games(
    State(state): State<Shared>,
    headers: HeaderMap,
    Query(query): Query<lobby::LobbyQuery>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ErrorBody>)> {
    let account_id = authed(&state, &headers).await?;
    Ok(Json(listing_page(&state, &account_id, &query).await))
}
