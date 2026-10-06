//! The card catalog's routes: card text in a player's language and search.

use crate::{
    AppState, Deserialize, ErrorBody, HeaderMap, Json, Query, Shared, State, StatusCode, authed,
    err,
};

/// How many printings or cards one text request may ask for.
///
/// A commander table's whole print table is a few hundred entries, so this
/// covers a full game in one round trip and still bounds what a single request
/// can cost.
pub(crate) const MAX_TEXT_IDS: usize = 500;

/// Connects the card catalog when `DATABASE_URL` is configured.
///
/// A failure here is logged and otherwise ignored: card text is presentation,
/// and a gateway that cannot reach Postgres should still host games.
pub(crate) async fn connect_catalog(
    db: sea_orm::DatabaseConnection,
) -> Option<baylee_catalog::Catalog> {
    // The pool the gateway already opened, not a second one against the same
    // URL: two pools are twice the backend processes for no more concurrency,
    // and one `search_path` is what lets a test put the whole gateway in a
    // schema of its own.
    let catalog = baylee_catalog::Catalog::from_connection(db);
    if let Err(err) = catalog.migrate().await {
        // Not fatal, unlike the account tables. A gateway with no card
        // catalog serves no card text and plays every game; the client draws
        // faces from what the engine projects.
        tracing::error!(%err, "card catalog schema could not be applied");
        return None;
    }
    let count = catalog.count().await.unwrap_or(0);
    tracing::info!(printings = count, "card catalog connected");
    Some(catalog)
}

/// Query for `/catalog/text`.
#[derive(Deserialize)]
pub(crate) struct CatalogTextQuery {
    /// Comma-separated Scryfall oracle ids: text by card.
    oracle_ids: Option<String>,
    /// Comma-separated Scryfall printing ids: text by printing, answered
    /// under the id asked for. Kept for clients that predate `oracle_ids`.
    ids: Option<String>,
    /// Preferred language; English is the fallback.
    lang: Option<String>,
}

/// The well-formed ids in a comma-separated list, lowercased, at most
/// [`MAX_TEXT_IDS`].
///
/// They are bound parameters, so this is not about injection: one malformed
/// id would fail the cast for the whole batch and cost every other card its
/// text.
pub(crate) fn text_ids(list: &str) -> Vec<String> {
    list.split(',')
        .map(str::trim)
        .filter(|id| uuid::Uuid::parse_str(id).is_ok())
        .map(str::to_lowercase)
        .take(MAX_TEXT_IDS)
        .collect()
}

/// Card text for a set of cards (`oracle_ids=`) or printings (`ids=`).
///
/// For a signed-in session only (#270), asked before anything else, as
/// `/art` is (#273). The catalog is Scryfall's data, and Scryfall's terms
/// say "You may not simply repackage, republish, or proxy Scryfall data":
/// a route anyone could call served it to whoever asked. A player
/// signs in with a free account, a guest's included, which the same terms
/// allow ("end-users should be able to access card data anonymously or
/// with free accounts"). A client that is not signed in asks Scryfall
/// itself, and has the English Oracle compiled in.
///
/// A card or printing the catalog lacks is simply not answered. Asked by
/// printing, the gateway used to fetch a missing one from Scryfall and keep
/// it; that made it a proxy for whoever asked, and nothing asked it any
/// more, so it asks Scryfall on nobody's behalf (#270).
pub(crate) async fn catalog_text(
    State(state): State<Shared>,
    headers: HeaderMap,
    Query(params): Query<CatalogTextQuery>,
) -> Result<Json<Vec<baylee_catalog::CardTextEntry>>, (StatusCode, Json<ErrorBody>)> {
    authed(&state, &headers).await?;
    let catalog = state.catalog.as_ref().ok_or_else(|| {
        err(
            StatusCode::SERVICE_UNAVAILABLE,
            "card catalog not configured",
        )
    })?;
    let lang = params.lang.as_deref().unwrap_or("en").to_lowercase();

    if let Some(list) = params.oracle_ids.as_deref() {
        let found = card_text(&state, catalog, &text_ids(list), &lang)
            .await
            .map_err(|e| catalog_error("looking up card text", &e))?;
        return Ok(Json(found));
    }

    let ids = text_ids(params.ids.as_deref().unwrap_or_default());
    if ids.is_empty() {
        return Ok(Json(Vec::new()));
    }
    let asked = catalog
        .cards_of(&ids)
        .await
        .map_err(|e| catalog_error("looking up card text", &e))?;

    let mut cards: Vec<String> = asked.iter().map(|(_, card)| card.clone()).collect();
    cards.sort_unstable();
    cards.dedup();
    let by_card: std::collections::HashMap<String, baylee_catalog::CardTextEntry> =
        card_text(&state, catalog, &cards, &lang)
            .await
            .map_err(|e| catalog_error("looking up card text", &e))?
            .into_iter()
            .map(|entry| (entry.oracle_id.clone(), entry))
            .collect();
    Ok(Json(
        asked
            .into_iter()
            .filter_map(|(scryfall_id, card)| {
                Some(baylee_catalog::CardTextEntry {
                    scryfall_id,
                    ..by_card.get(&card)?.clone()
                })
            })
            .collect(),
    ))
}

/// Text for `cards` in `lang`, in oracle-id order: a pool card's from the
/// language held in memory, any other from the catalog.
pub(crate) async fn card_text(
    state: &AppState,
    catalog: &baylee_catalog::Catalog,
    cards: &[String],
    lang: &str,
) -> anyhow::Result<Vec<baylee_catalog::CardTextEntry>> {
    if cards.is_empty() {
        return Ok(Vec::new());
    }
    let language = state.texts.get(catalog, lang).await?;
    let mut found = Vec::with_capacity(cards.len());
    let mut rest = Vec::new();
    for card in cards {
        match language.entry(card) {
            Some(entry) => found.push(entry.clone()),
            None => rest.push(card.clone()),
        }
    }
    if !rest.is_empty() {
        found.extend(catalog.text_by_card(&rest, language.lang()).await?);
    }
    found.sort_by(|a, b| a.oracle_id.cmp(&b.oracle_id));
    found.dedup_by(|a, b| a.oracle_id == b.oracle_id);
    Ok(found)
}

/// Query for `/catalog/search`.
#[derive(Deserialize)]
pub(crate) struct CatalogSearchQuery {
    /// Search terms.
    q: String,
    /// Preferred language.
    lang: Option<String>,
    /// Maximum hits.
    limit: Option<u64>,
}

/// Searches the card catalog — the entry point the deck builder will use.
///
/// For a signed-in session only, for the reason `catalog_text` gives: open,
/// it was a search over Scryfall's data for anyone who asked (#270).
pub(crate) async fn catalog_search(
    State(state): State<Shared>,
    headers: HeaderMap,
    Query(params): Query<CatalogSearchQuery>,
) -> Result<Json<Vec<baylee_catalog::SearchHit>>, (StatusCode, Json<ErrorBody>)> {
    authed(&state, &headers).await?;
    let catalog = state.catalog.as_ref().ok_or_else(|| {
        err(
            StatusCode::SERVICE_UNAVAILABLE,
            "card catalog not configured",
        )
    })?;
    let lang = params.lang.as_deref().unwrap_or("en").to_lowercase();
    let limit = params.limit.unwrap_or(50).clamp(1, 200);
    let hits = catalog
        .search(params.q.trim(), &lang, limit)
        .await
        .map_err(|e| catalog_error("searching the catalog", &e))?;
    Ok(Json(hits))
}

/// Logs a catalog failure and returns a response that leaks nothing.
///
/// The error text can carry connection strings and SQL, neither of which
/// belongs in a client response.
pub(crate) fn catalog_error(
    what: &str,
    error: &impl std::fmt::Display,
) -> (StatusCode, Json<ErrorBody>) {
    tracing::error!(%error, "{what} failed");
    err(StatusCode::BAD_GATEWAY, "card catalog request failed")
}
