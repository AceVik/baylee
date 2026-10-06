//! What a table shows besides the game: its cosmetics and its names.

use crate::{
    Deserialize, ErrorBody, HeaderMap, Json, Path, Query, Shared, State, StatusCode, bearer_token,
    cosmetics, err, legacy_token, seat_of_token, store,
};

/// What `GET /games/{id}/cosmetics` may still carry in its query: the seat
/// token, from a client older than #294.
#[derive(Deserialize)]
pub(crate) struct CosmeticsParams {
    #[serde(default)]
    token: Option<String>,
}

/// `GET /games/{id}/cosmetics` — every seat's sleeve and playmat, for
/// `Authorization: Bearer <seat token>`.
///
/// The one route decorations travel on, and the reason they do not travel with
/// the game: `GameStatic` is rules data and a view is what a seat is entitled
/// to know, while a sleeve is a fact about a *deck*. The gateway is the layer
/// that knows which deck sits in which chair, so answering here costs no
/// `VIEW_VERSION` and leaves the engine as ignorant of decoration as it is of
/// card text.
///
/// Authorised by the same seat token as the game socket, which is the honest
/// bound: this says nothing a player will not see the moment the first card is
/// drawn face-down in front of them, and nothing at all about a deck's
/// contents.
pub(crate) async fn game_cosmetics(
    State(state): State<Shared>,
    Path(id): Path<String>,
    headers: HeaderMap,
    Query(params): Query<CosmeticsParams>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ErrorBody>)> {
    // A header since #294: this is an ordinary request, so nothing forces
    // the secret into its address.
    let token = match (bearer_token(&headers), params.token.as_deref()) {
        (Some(token), _) => token,
        (None, Some(old)) => legacy_token(&state, Some(old), "/games/{id}/cosmetics")?,
        (None, None) => return Err(err(StatusCode::UNAUTHORIZED, "missing bearer token")),
    };
    let lobby = state.lobby.lock();
    let game = lobby
        .games
        .get(&id)
        .ok_or_else(|| err(StatusCode::NOT_FOUND, "no such game"))?;
    if seat_of_token(game, token).is_none() {
        return Err(err(StatusCode::UNAUTHORIZED, "invalid seat token"));
    }
    // Seats with nothing set are left out rather than sent as empty objects:
    // a table where nobody uploaded anything is `{}`, and the client's answer
    // to "no entry" and to "an entry with no sleeve" has to be the same thing
    // anyway — draw the generated back.
    let mut seats = cosmetics::TableCosmetics::new();
    for seat in &game.seats {
        let Some(deck) = seat.deck.as_ref() else {
            continue;
        };
        let worn = cosmetics::SeatCosmetics {
            sleeve: deck.sleeve.clone(),
            playmat: deck.playmat.clone(),
        };
        if !worn.is_empty() {
            seats.insert(seat.seat, worn);
        }
    }
    Ok(Json(serde_json::json!(seats)))
}

/// The names shown at each seat of a game, in seat order.
///
/// The rules kernel has never heard of an account, so the roster is assembled
/// here and handed to the engine with the preset. The two locks are taken one
/// after the other rather than nested: the lobby says which account sits
/// where, the store says what that account is called, and nothing in between
/// needs both at once.
pub(crate) async fn seat_names(state: &Shared, game_id: &str) -> Vec<String> {
    let (accounts, delegates): (Vec<Option<String>>, Vec<Option<String>>) = {
        let lobby = state.lobby.lock();
        match lobby.games.get(game_id) {
            Some(game) => game
                .seats
                .iter()
                .map(|s| {
                    (
                        s.account_id.clone(),
                        s.delegate.as_ref().map(|d| d.name.clone()),
                    )
                })
                .unzip(),
            None => return Vec::new(),
        }
    };
    let names = match store::display_names(&state.db, accounts.iter().flatten().cloned()).await {
        Ok(names) => names,
        Err(e) => {
            tracing::error!("{e:#}");
            std::collections::HashMap::new()
        }
    };
    accounts
        .into_iter()
        .zip(delegates)
        .map(|(id, delegate)| match (id, delegate) {
            (Some(id), _) => names
                .get(&id)
                .cloned()
                .unwrap_or_else(|| "Unknown player".to_string()),
            // A host's seat bridge sits under the name it chose, which says
            // what plays (`LLM-…`); the listing says whose it is.
            (None, Some(name)) => name,
            // An empty chair in a running game is the house playing it.
            (None, None) => "House AI".to_string(),
        })
        .collect()
}
