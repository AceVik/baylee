//! The deck routes: a player's own decks, their history, and shared ones.

use crate::{
    Deck, DeckBody, ErrorBody, HeaderMap, Json, Path, Router, Shared, State, StatusCode, auth,
    authed, check_pictures, db_down, err, get, post, store, validate_deck,
};

/// `GET /decks`: the account's decks, newest save first, each with what the
/// list says about it beyond its name.
///
/// WG-3 added `updated_at` (unix seconds of the last save), `unplayable`
/// (main-deck copies this build cannot play), `signature` (a deck without a
/// commander's picture) and an `artist` on `signature` and every `leaders`
/// entry, from the catalog: one query for the whole list, and no artist at
/// all — never a failed list — without a catalog or when it is down.
pub(crate) async fn list_decks(
    State(state): State<Shared>,
    headers: HeaderMap,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ErrorBody>)> {
    let account_id = authed(&state, &headers).await?;
    // One indexed read on `(account_id, updated_at DESC)`, where this used
    // to walk every deck on the gateway to find one player's.
    let decks = store::decks_of(&state.db, &account_id)
        .await
        .map_err(|e| db_down(&e))?;
    let mut digests: Vec<_> = decks
        .iter()
        .map(|d| baylee_cards::digest::digest(&d.cards, &d.sideboard, &d.commanders))
        .collect();
    credit_artists(&state, &mut digests).await;
    let decks: Vec<_> = decks
        .iter()
        .zip(digests)
        .map(|(d, digest)| {
            serde_json::json!({
                "id": d.id,
                "name": d.name,
                "format": d.format,
                "cards": d.cards.len(),
                "sideboard": d.sideboard.len(),
                "copies": digest.copies,
                "side_copies": digest.side_copies,
                "identity": digest.identity,
                "commanders": d.commanders,
                "leaders": digest.leaders,
                "signature": digest.signature,
                "unplayable": digest.unplayable,
                "updated_at": d.updated_at,
                "sleeve": d.sleeve,
                "playmat": d.playmat,
            })
        })
        .collect();
    Ok(Json(serde_json::json!(decks)))
}

/// Fills in who painted every picture these digests name, from the
/// catalog. Left empty where it does not know, and wholly empty without a
/// catalog or when it fails: a deck list without credits is a list, and the
/// client then shows no art rather than uncredited art.
async fn credit_artists(state: &Shared, digests: &mut [baylee_core::deckdigest::Digest]) {
    let Some(catalog) = state.catalog.as_ref() else {
        return;
    };
    let mut ids: Vec<String> = digests
        .iter()
        .flat_map(|d| d.leaders.iter().chain(d.signature.as_ref()))
        .map(|l| l.scryfall_id.clone())
        .filter(|id| !id.is_empty())
        .collect();
    ids.sort_unstable();
    ids.dedup();
    if ids.is_empty() {
        return;
    }
    let artists = match catalog.artists(&ids).await {
        Ok(artists) => artists,
        Err(e) => {
            tracing::warn!("deck list without artists: {e:#}");
            return;
        }
    };
    for digest in digests {
        for picture in digest.leaders.iter_mut().chain(digest.signature.as_mut()) {
            if let Some(artist) = artists.get(&picture.scryfall_id) {
                picture.artist.clone_from(artist);
            }
        }
    }
}

pub(crate) async fn get_deck(
    State(state): State<Shared>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ErrorBody>)> {
    let account_id = authed(&state, &headers).await?;
    let deck = readable_deck(&state, &id, &account_id).await?;
    Ok(Json(serde_json::json!({
        "id": deck.id,
        "kind": deck.kind,
        "name": deck.name,
        "format": deck.format,
        "description": deck.description,
        "version": deck.version,
        "copied_from": deck.origin.as_ref().map(|(deck, _)| deck.clone()),
        "copied_version": deck.origin.as_ref().map(|(_, version)| *version),
        "cards": deck.cards,
        "sideboard": deck.sideboard,
        "commanders": deck.commanders,
        "sleeve": deck.sleeve,
        "playmat": deck.playmat,
    })))
}

pub(crate) async fn create_deck(
    State(state): State<Shared>,
    headers: HeaderMap,
    Json(body): Json<DeckBody>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ErrorBody>)> {
    let account_id = authed(&state, &headers).await?;
    validate_deck(&body)?;
    check_pictures(&state, &account_id, &body).await?;
    let commanders = body.commander_names();
    let format = body
        .format
        .clone()
        .unwrap_or_else(|| baylee_cards::decks::format_of(&commanders).to_string());
    let id = store::create_deck(
        &state.db,
        store::NewDeck {
            account_id,
            name: body.name,
            format,
            description: body.description.filter(|d| !d.is_empty()),
            origin: None,
            cards: body.cards,
            sideboard: body.sideboard,
            commanders,
            sleeve: body.sleeve,
            playmat: body.playmat,
            updated_at: auth::now_secs(),
        },
    )
    .await
    .map_err(|e| db_down(&e))?
    .ok_or_else(|| err(StatusCode::INTERNAL_SERVER_ERROR, "account gone"))?;
    Ok(Json(serde_json::json!({ "deck_id": id })))
}

pub(crate) async fn update_deck(
    State(state): State<Shared>,
    headers: HeaderMap,
    Path(id): Path<String>,
    Json(body): Json<DeckBody>,
) -> Result<StatusCode, (StatusCode, Json<ErrorBody>)> {
    let account_id = authed(&state, &headers).await?;
    validate_deck(&body)?;
    let deck = store::deck(&state.db, &id)
        .await
        .map_err(|e| db_down(&e))?
        .ok_or_else(|| err(StatusCode::NOT_FOUND, "no such deck"))?;
    if deck.account_id != account_id {
        return Err(err(StatusCode::FORBIDDEN, "not your deck"));
    }
    check_pictures(&state, &account_id, &body).await?;
    let commanders = body.commander_names();
    let format = body.format.clone().unwrap_or_else(|| deck.format.clone());
    let description = match body.description {
        // An empty string is somebody clearing the field; leaving it out is
        // somebody saving a deck without touching it.
        Some(text) if text.is_empty() => None,
        Some(text) => Some(text),
        None => deck.description.clone(),
    };
    store::put_deck(
        &state.db,
        Deck {
            name: body.name,
            format,
            description,
            cards: body.cards,
            sideboard: body.sideboard,
            commanders,
            sleeve: body.sleeve,
            playmat: body.playmat,
            updated_at: auth::now_secs(),
            ..deck
        },
        body.summary.filter(|s| !s.is_empty()),
    )
    .await
    .map_err(|e| db_down(&e))?;
    Ok(StatusCode::NO_CONTENT)
}

/// Everything a deck is reached through, in one place.
///
/// Split off the router `main` builds because there are now eight of them —
/// the deck itself, the shared decks anybody may take, and the history. It
/// is also the one place where the order matters: `/decks/shared` is a
/// static segment and has to be matched as one rather than as a deck whose
/// id is the word "shared".
pub(crate) fn deck_routes() -> Router<Shared> {
    Router::new()
        .route("/decks", get(list_decks).post(create_deck))
        .route("/decks/shared", get(list_shared_decks))
        .route(
            "/decks/{id}",
            get(get_deck).put(update_deck).delete(delete_deck),
        )
        .route("/decks/{id}/copy", post(copy_deck))
        .route("/decks/{id}/history", get(deck_history))
        .route("/decks/{id}/versions/{version}", get(deck_version))
        .route("/decks/{id}/versions/{version}/revert", post(revert_deck))
}

/// The decks anybody may play: what this project publishes and what came in
/// a box.
///
/// Readable by any signed-in account and owned by none of them, which is
/// what makes them the thing a copy starts from.
pub(crate) async fn list_shared_decks(
    State(state): State<Shared>,
    headers: HeaderMap,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ErrorBody>)> {
    let _ = authed(&state, &headers).await?;
    let decks: Vec<_> = store::shared_decks(&state.db)
        .await
        .map_err(|e| db_down(&e))?
        .iter()
        .map(|d| {
            serde_json::json!({
                "id": d.id,
                "kind": d.kind,
                "name": d.name,
                "format": d.format,
                "description": d.description,
                "cards": d.cards.len(),
                "sideboard": d.sideboard.len(),
                "commanders": d.commanders,
                "version": d.version,
            })
        })
        .collect();
    Ok(Json(serde_json::json!(decks)))
}

/// One deck's history: every state it no longer holds, newest first.
///
/// The current cards are **not** in the list — they are the deck, and
/// `version` says which number they carry. A caller drawing a timeline puts
/// the deck at the top and these underneath it.
pub(crate) async fn deck_history(
    State(state): State<Shared>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ErrorBody>)> {
    let account_id = authed(&state, &headers).await?;
    let deck = readable_deck(&state, &id, &account_id).await?;
    let past: Vec<_> = store::deck_history(&state.db, &id)
        .await
        .map_err(|e| db_down(&e))?
        .iter()
        .map(|v| {
            serde_json::json!({
                "version": v.version,
                "cards": v.cards.len(),
                "sideboard": v.sideboard.len(),
                "commanders": v.commanders,
                "summary": v.summary,
                "superseded_at": v.superseded_at,
            })
        })
        .collect();
    Ok(Json(serde_json::json!({
        "version": deck.version,
        "updated_at": deck.updated_at,
        "past": past,
    })))
}

/// One superseded state, in full.
pub(crate) async fn deck_version(
    State(state): State<Shared>,
    headers: HeaderMap,
    Path((id, version)): Path<(String, i32)>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ErrorBody>)> {
    let account_id = authed(&state, &headers).await?;
    let deck = readable_deck(&state, &id, &account_id).await?;
    if version == deck.version {
        // The current state is not in the history table and never will be.
        // Answering it from the deck itself is what makes "show me version
        // N" a question the caller can ask about any N it was given.
        return Ok(Json(serde_json::json!({
            "version": deck.version,
            "cards": deck.cards,
            "sideboard": deck.sideboard,
            "commanders": deck.commanders,
            "current": true,
        })));
    }
    let past = store::deck_at_version(&state.db, &id, version)
        .await
        .map_err(|e| db_down(&e))?
        .ok_or_else(|| err(StatusCode::NOT_FOUND, "no such version"))?;
    Ok(Json(serde_json::json!({
        "version": past.version,
        "cards": past.cards,
        "sideboard": past.sideboard,
        "commanders": past.commanders,
        "summary": past.summary,
        "superseded_at": past.superseded_at,
        "current": false,
    })))
}

/// Put an earlier state back, as a new change.
///
/// Not a rewind: the deck's present is archived exactly as any other save
/// archives it, and the old lists become the new head one version higher.
/// So a revert can itself be reverted, and nothing in the history is ever
/// removed or rewritten.
pub(crate) async fn revert_deck(
    State(state): State<Shared>,
    headers: HeaderMap,
    Path((id, version)): Path<(String, i32)>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ErrorBody>)> {
    let account_id = authed(&state, &headers).await?;
    let deck = store::deck(&state.db, &id)
        .await
        .map_err(|e| db_down(&e))?
        .ok_or_else(|| err(StatusCode::NOT_FOUND, "no such deck"))?;
    if deck.account_id != account_id {
        return Err(err(StatusCode::FORBIDDEN, "not your deck"));
    }
    if version == deck.version {
        return Err(err(
            StatusCode::CONFLICT,
            "that is the deck's current state",
        ));
    }
    let past = store::deck_at_version(&state.db, &id, version)
        .await
        .map_err(|e| db_down(&e))?
        .ok_or_else(|| err(StatusCode::NOT_FOUND, "no such version"))?;
    let next = deck.version + 1;
    store::put_deck(
        &state.db,
        Deck {
            cards: past.cards,
            sideboard: past.sideboard,
            commanders: past.commanders,
            updated_at: auth::now_secs(),
            ..deck
        },
        Some(format!("zurück auf Version {version}")),
    )
    .await
    .map_err(|e| db_down(&e))?;
    Ok(Json(serde_json::json!({ "version": next })))
}

/// Take a copy of a deck anybody may play, as an account's own.
///
/// The copy is an ordinary account deck from the moment it exists — its own
/// history starts at version 1 — and it remembers which deck and which of
/// that deck's versions it came from, so "this is the Kess precon as it was"
/// survives the original moving on.
pub(crate) async fn copy_deck(
    State(state): State<Shared>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ErrorBody>)> {
    let account_id = authed(&state, &headers).await?;
    let source = readable_deck(&state, &id, &account_id).await?;
    // A withdrawn precon stays readable, so the copies taken of it can still
    // say what they came from, but it is no longer something to start from:
    // this build does not play every card in it.
    if !source.offered {
        return Err(err(StatusCode::GONE, "that deck is no longer offered"));
    }
    let copy = store::create_deck(
        &state.db,
        store::NewDeck {
            account_id,
            name: source.name.clone(),
            format: source.format.clone(),
            description: source.description.clone(),
            origin: Some((source.id.clone(), source.version)),
            cards: source.cards.clone(),
            sideboard: source.sideboard.clone(),
            commanders: source.commanders.clone(),
            // A copy starts with the generated back and is dressed like any
            // other deck ([`check_pictures`]).
            sleeve: None,
            playmat: None,
            updated_at: auth::now_secs(),
        },
    )
    .await
    .map_err(|e| db_down(&e))?
    .ok_or_else(|| err(StatusCode::INTERNAL_SERVER_ERROR, "account gone"))?;
    Ok(Json(serde_json::json!({ "deck_id": copy })))
}

/// A deck this account may read: their own, or one that belongs to nobody.
///
/// The two are one question because every route that shows a deck asks it,
/// and a route that asked only the first would make a house deck invisible
/// to the player it was published for.
pub(crate) async fn readable_deck(
    state: &Shared,
    id: &str,
    account_id: &str,
) -> Result<store::Deck, (StatusCode, Json<ErrorBody>)> {
    let deck = store::deck(&state.db, id)
        .await
        .map_err(|e| db_down(&e))?
        .ok_or_else(|| err(StatusCode::NOT_FOUND, "no such deck"))?;
    if deck.kind == baylee_db::entity::deck::KIND_ACCOUNT && deck.account_id != account_id {
        return Err(err(StatusCode::FORBIDDEN, "not your deck"));
    }
    Ok(deck)
}

pub(crate) async fn delete_deck(
    State(state): State<Shared>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> Result<StatusCode, (StatusCode, Json<ErrorBody>)> {
    let account_id = authed(&state, &headers).await?;
    // The ownership is inside the `DELETE`, so the row can only ever be
    // removed by the account that owns it. Telling "not yours" from "not
    // there" still takes a read, and the two answers are worth keeping
    // apart: one is a bug in the client and the other is a stale list.
    if store::delete_deck(&state.db, &id, &account_id)
        .await
        .map_err(|e| db_down(&e))?
    {
        return Ok(StatusCode::NO_CONTENT);
    }
    match store::deck(&state.db, &id).await.map_err(|e| db_down(&e))? {
        Some(_) => Err(err(StatusCode::FORBIDDEN, "not your deck")),
        None => Err(err(StatusCode::NOT_FOUND, "no such deck")),
    }
}
