//! The deck routes: a player's own decks, their history, and shared ones.

use crate::{
    Deck, DeckBody, ErrorBody, HeaderMap, Json, Path, Router, Serialize, Shared, State, StatusCode,
    auth, authed, check_pictures, db_down, err, get, post, store, validate_deck,
};
use baylee_core::deckdiff::{self, Delta};
use baylee_core::deckdigest::Leader;

/// `GET /decks`: the account's decks, newest save first, each with what the
/// list says about it beyond its name.
///
/// WG-3 added `updated_at` (unix seconds of the last save), `unplayable`
/// (main-deck copies this build cannot play), `signature` (a deck without a
/// commander's picture) and an `artist` on `signature` and every `leaders`
/// entry, from the catalog: one query for the whole list, and no artist at
/// all — never a failed list — without a catalog or when it is down.
///
/// @route GET /decks
/// @scope public
/// @summary The account's decks
/// @description Every deck of the signed-in account, newest save first,
/// each with what a player picks a deck by: its counts, colour identity,
/// commanders' pictures, the cards this gateway blocks or this build cannot
/// play, and its sleeve and playmat.
/// @tag Decks
/// @security bearer
/// @insert `SessionRefusals`
pub(crate) async fn list_decks(
    State(state): State<Shared>,
    headers: HeaderMap,
) -> Result<Json<Vec<DeckRow>>, (StatusCode, Json<ErrorBody>)> {
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
    let now = time::OffsetDateTime::now_utc();
    let decks: Vec<_> = decks
        .into_iter()
        .zip(digests)
        .map(|(d, digest)| DeckRow {
            blocked: blocked_in(&state, &d, now),
            cards: d.cards.len(),
            sideboard: d.sideboard.len(),
            id: d.id,
            name: d.name,
            format: d.format,
            copies: digest.copies,
            side_copies: digest.side_copies,
            identity: digest.identity,
            commanders: d.commanders,
            leaders: digest.leaders,
            signature: digest.signature,
            unplayable: digest.unplayable,
            updated_at: d.updated_at,
            sleeve: d.sleeve,
            playmat: d.playmat,
        })
        .collect();
    Ok(Json(decks))
}

/// One row of `GET /decks`.
/// @openapi
#[derive(Serialize)]
pub(crate) struct DeckRow {
    /// @description The deck's id.
    id: String,
    /// @description The cards of this deck blocked on this gateway now; a
    /// deck with any does not sit down at a table here. Empty for a deck
    /// this gateway plays.
    blocked: Vec<BlockedCard>,
    /// @description The deck's name.
    name: String,
    /// @description What it plays (`commander`, `freeform`, …).
    format: String,
    /// @description How many main-deck lines it has (`4 Lightning Bolt` is
    /// one).
    cards: usize,
    /// @description How many sideboard lines it has.
    sideboard: usize,
    /// @description How many cards the main deck holds, counting copies.
    copies: u32,
    /// @description How many cards the sideboard holds, counting copies.
    side_copies: u32,
    /// @description Its colour identity as `WUBRG` letters, empty for
    /// colourless: its commanders' together, else every main-deck card's.
    identity: String,
    /// @description Its commanders: none, one, or two under the partner
    /// rule.
    commanders: Vec<String>,
    /// @description The picture of each commander, in the order of
    /// `commanders`.
    leaders: Vec<Leader>,
    /// @description The picture of a deck without commanders (its most
    /// expensive non-land card); `null` for a deck with commanders or of
    /// lands alone.
    signature: Option<Leader>,
    /// @description How many main-deck cards, counting copies, this build
    /// cannot play.
    unplayable: u32,
    /// @description When it was last saved, in unix seconds.
    updated_at: u64,
    /// @description The image id of its sleeve (`GET /images/{id}`), `null`
    /// for the generated card back.
    sleeve: Option<String>,
    /// @description The image id of its playmat, `null` for none.
    playmat: Option<String>,
}

/// A card blocked on this gateway, as a player is told it
/// ([`crate::blocks::Block::json`], the same four fields).
///
/// @description A card the operator has taken out of play on this gateway.
/// @openapi
#[derive(Serialize)]
pub(crate) struct BlockedCard {
    /// @description The card's `CardIndex`.
    index: u32,
    /// @description Its English name.
    name: String,
    /// @description Why, in the operator's words.
    reason: String,
    /// @description When the block ends by itself (RFC 3339, UTC), `null`
    /// for a block without an end.
    /// @openapi format: date-time
    until: Option<String>,
}

impl From<&crate::blocks::Block> for BlockedCard {
    fn from(block: &crate::blocks::Block) -> Self {
        Self {
            index: block.index,
            name: block.name.clone(),
            reason: block.reason.clone(),
            until: block.until.map(crate::blocks::iso),
        }
    }
}

/// The cards of `deck` blocked on this gateway (`blocks.rs`), as a list
/// row's `blocked`: `[{"index", "name", "reason", "until"}]`, empty for a
/// deck this gateway plays.
fn blocked_in(state: &Shared, deck: &Deck, now: time::OffsetDateTime) -> Vec<BlockedCard> {
    state
        .blocks
        .in_lines(&deck.cards, &deck.sideboard, &deck.commanders, now)
        .iter()
        .map(BlockedCard::from)
        .collect()
}

/// `GET /cards/blocked`: every card blocked on this gateway now, for a
/// signed-in session, as `{"blocked": [{"index", "name", "reason",
/// "until"}]}` (`docs/protocol.md` §"Blocked cards"). The deck builder marks
/// them; a local game ignores them.
///
/// @route GET /cards/blocked
/// @scope public
/// @summary The cards blocked on this gateway
/// @description Every card the operator has blocked on this gateway now,
/// with the reason and when the block ends. A deck holding one does not
/// sit down at a table here; the deck builder marks them.
/// @tag Decks
/// @security bearer
/// @insert `SessionRefusals`
pub(crate) async fn blocked_cards(
    State(state): State<Shared>,
    headers: HeaderMap,
) -> Result<Json<BlockedCards>, (StatusCode, Json<ErrorBody>)> {
    let _ = authed(&state, &headers).await?;
    let blocked: Vec<_> = state
        .blocks
        .now_in_force(time::OffsetDateTime::now_utc())
        .iter()
        .map(BlockedCard::from)
        .collect();
    Ok(Json(BlockedCards { blocked }))
}

/// `GET /cards/blocked`.
/// @openapi
#[derive(Serialize)]
pub(crate) struct BlockedCards {
    /// @description Every card blocked now.
    blocked: Vec<BlockedCard>,
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

/// @route GET /decks/{id}
/// @scope public
/// @summary One deck
/// @description One deck in full: the account's own, or a shared deck
/// (house or preconstructed, also one no longer offered), with the deck it
/// was copied from.
/// @tag Decks
/// @security bearer
/// @insert `SessionRefusals`
/// @path-param id: String "The deck's id"
/// @return 403: `$ErrorBody` "Another account's deck; or the account is banned (`ban`) or must accept the terms in force first (`terms_required`)"
/// @return 404: `$ErrorBody` "No such deck"
pub(crate) async fn get_deck(
    State(state): State<Shared>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> Result<Json<DeckDetail>, (StatusCode, Json<ErrorBody>)> {
    let account_id = authed(&state, &headers).await?;
    let deck = readable_deck(&state, &id, &account_id).await?;
    let (copied_from, copied_version) = deck.origin.unzip();
    Ok(Json(DeckDetail {
        id: deck.id,
        kind: deck.kind,
        name: deck.name,
        format: deck.format,
        description: deck.description,
        version: deck.version,
        copied_from,
        copied_version,
        cards: deck.cards,
        sideboard: deck.sideboard,
        commanders: deck.commanders,
        sleeve: deck.sleeve,
        playmat: deck.playmat,
    }))
}

/// `GET /decks/{id}`.
/// @openapi
#[derive(Serialize)]
pub(crate) struct DeckDetail {
    /// @description The deck's id.
    id: String,
    /// @description Whose it is: `account` (a player's own), `preconstructed`
    /// (a retail product) or `house` (published to be played with).
    /// @openapi enum: [account, preconstructed, house]
    kind: String,
    /// @description The deck's name.
    name: String,
    /// @description What it plays (`commander`, `freeform`, …).
    format: String,
    /// @description What the deck is for, in its owner's words; `null` for
    /// none.
    description: Option<String>,
    /// @description The version its cards are; every save that changes
    /// them raises it by one.
    version: i32,
    /// @description The deck it was copied from (`POST /decks/{id}/copy`),
    /// `null` for a deck that is no copy.
    copied_from: Option<String>,
    /// @description Which version of that deck was copied, `null` for a
    /// deck that is no copy.
    copied_version: Option<i32>,
    /// @description Main-deck lines (`N Card Name`, a printing optional).
    cards: Vec<String>,
    /// @description Sideboard lines, in the same form.
    sideboard: Vec<String>,
    /// @description Its commanders: none, one, or two under the partner
    /// rule.
    commanders: Vec<String>,
    /// @description The image id of its sleeve, `null` for the generated
    /// card back.
    sleeve: Option<String>,
    /// @description The image id of its playmat, `null` for none.
    playmat: Option<String>,
}

/// `POST /decks` and `POST /decks/{id}/copy`.
/// @openapi
#[derive(Serialize)]
pub(crate) struct DeckCreated {
    /// @description The new deck's id.
    deck_id: String,
}

/// @route POST /decks
/// @scope public
/// @summary Save a new deck
/// @description Stores a deck for the signed-in account and answers its
/// id. Every line is read against this build's pool: a card it cannot play,
/// more than four copies of a card that is not a basic land, an illegal
/// commander or a picture the account did not upload refuses the deck, and
/// the error says which.
/// @tag Decks
/// @security bearer
/// @insert `SessionRefusals`
/// @return 400: `$ErrorBody` "The deck is refused: its name, a card line, a commander or the deck's size (the error names it)"
/// @return 403: `$ErrorBody` "A sleeve or playmat the account did not upload (`that picture is not yours`); or the account is banned (`ban`) or must accept the terms in force first (`terms_required`)"
/// @return 415: String text/plain "The body is not sent as `application/json` (axum's rejection)"
/// @return 422: String text/plain "The body is not a deck (axum's rejection)"
/// @return 500: `$ErrorBody` "The account was deleted while the deck was saved"
pub(crate) async fn create_deck(
    State(state): State<Shared>,
    headers: HeaderMap,
    Json(body): Json<DeckBody>,
) -> Result<Json<DeckCreated>, (StatusCode, Json<ErrorBody>)> {
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
    Ok(Json(DeckCreated { deck_id: id }))
}

/// @route PUT /decks/{id}
/// @scope public
/// @summary Save a deck again
/// @description Replaces one of the account's decks with the body, under
/// the rules `POST /decks` checks. A save that changes a list keeps the
/// state it replaces in the deck's history (`summary` names the change);
/// one that leaves `format` or `description` out keeps them.
/// @tag Decks
/// @security bearer
/// @insert `SessionRefusals`
/// @path-param id: String "The deck's id"
/// @return 204: "Saved"
/// @return 400: `$ErrorBody` "The deck is refused: its name, a card line, a commander or the deck's size (the error names it)"
/// @return 403: `$ErrorBody` "Another account's deck or a shared one, or a sleeve or playmat the account did not upload; or the account is banned (`ban`) or must accept the terms in force first (`terms_required`)"
/// @return 404: `$ErrorBody` "No such deck"
/// @return 415: String text/plain "The body is not sent as `application/json` (axum's rejection)"
/// @return 422: String text/plain "The body is not a deck (axum's rejection)"
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
        .route("/cards/blocked", get(blocked_cards))
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
///
/// @route GET /decks/shared
/// @scope public
/// @summary The decks anybody may play
/// @description The house decks and preconstructed decks this gateway
/// offers, owned by no account, each with what a player picks a deck by.
/// `POST /decks/{id}/copy` makes one the account's own.
/// @tag Decks
/// @security bearer
/// @insert `SessionRefusals`
pub(crate) async fn list_shared_decks(
    State(state): State<Shared>,
    headers: HeaderMap,
) -> Result<Json<Vec<SharedDeckRow>>, (StatusCode, Json<ErrorBody>)> {
    let _ = authed(&state, &headers).await?;
    let decks = store::shared_decks(&state.db)
        .await
        .map_err(|e| db_down(&e))?;
    // What `GET /decks` says about a deck beyond its name, said here too
    // (WG-3b): a house tile pictures its deck and credits the painter by
    // the same rule as one of the player's own.
    let mut digests: Vec<_> = decks
        .iter()
        .map(|d| baylee_cards::digest::digest(&d.cards, &d.sideboard, &d.commanders))
        .collect();
    credit_artists(&state, &mut digests).await;
    let now = time::OffsetDateTime::now_utc();
    let decks: Vec<_> = decks
        .into_iter()
        .zip(digests)
        .map(|(d, digest)| SharedDeckRow {
            blocked: blocked_in(&state, &d, now),
            cards: d.cards.len(),
            sideboard: d.sideboard.len(),
            id: d.id,
            kind: d.kind,
            name: d.name,
            format: d.format,
            description: d.description,
            commanders: d.commanders,
            version: d.version,
            copies: digest.copies,
            identity: digest.identity,
            leaders: digest.leaders,
            signature: digest.signature,
            unplayable: digest.unplayable,
        })
        .collect();
    Ok(Json(decks))
}

/// One row of `GET /decks/shared`.
/// @openapi
#[derive(Serialize)]
pub(crate) struct SharedDeckRow {
    /// @description The deck's id.
    id: String,
    /// @description The cards of this deck blocked on this gateway now; a
    /// deck with any does not sit down at a table here.
    blocked: Vec<BlockedCard>,
    /// @description `preconstructed` (a retail product) or `house`
    /// (published to be played with).
    /// @openapi enum: [preconstructed, house]
    kind: String,
    /// @description The deck's name.
    name: String,
    /// @description What it plays (`commander`, `freeform`, …).
    format: String,
    /// @description What the deck is for, `null` for nothing said.
    description: Option<String>,
    /// @description How many main-deck lines it has.
    cards: usize,
    /// @description How many sideboard lines it has.
    sideboard: usize,
    /// @description Its commanders: none, one, or two under the partner
    /// rule.
    commanders: Vec<String>,
    /// @description The version its cards are, which a copy remembers.
    version: i32,
    /// @description How many cards the main deck holds, counting copies.
    copies: u32,
    /// @description Its colour identity as `WUBRG` letters, empty for
    /// colourless.
    identity: String,
    /// @description The picture of each commander, in the order of
    /// `commanders`.
    leaders: Vec<Leader>,
    /// @description The picture of a deck without commanders; `null` for a
    /// deck with commanders or of lands alone.
    signature: Option<Leader>,
    /// @description How many main-deck cards, counting copies, this build
    /// cannot play.
    unplayable: u32,
}

/// One deck's history: every state it no longer holds, newest first.
///
/// The current cards are **not** in the list — they are the deck, and
/// `version` says which number they carry. A caller drawing a timeline puts
/// the deck at the top and these underneath it.
///
/// Every row, and the head, also says what changed against the state before
/// it (`delta`, by [`baylee_core::deckdiff`]: copies added and removed, rows
/// whose count, printing, finish, language or note changed; `null` on the
/// oldest, which has nothing before it) and how many cards it holds
/// (`card_count`, sums of counts, not rows). Computed from the stored rows on
/// every read (WG-6): no column, no migration.
///
/// @route GET /decks/{id}/history
/// @scope public
/// @summary What a deck used to be
/// @description The deck's version and every state it no longer holds,
/// newest first; the current cards are not in `past`. The head and every
/// past state say what changed against the state before them and how many
/// cards they hold.
/// @tag Decks
/// @security bearer
/// @insert `SessionRefusals`
/// @path-param id: String "The deck's id"
/// @return 403: `$ErrorBody` "Another account's deck; or the account is banned (`ban`) or must accept the terms in force first (`terms_required`)"
/// @return 404: `$ErrorBody` "No such deck"
pub(crate) async fn deck_history(
    State(state): State<Shared>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> Result<Json<DeckHistory>, (StatusCode, Json<ErrorBody>)> {
    let account_id = authed(&state, &headers).await?;
    let deck = readable_deck(&state, &id, &account_id).await?;
    let versions = store::deck_history(&state.db, &id)
        .await
        .map_err(|e| db_down(&e))?;
    // Newest first, so a row's predecessor is the one after it.
    let delta = |after: [&[String]; 3], before: Option<&store::DeckVersion>| {
        before.map(|b| deckdiff::deck_delta([&b.cards, &b.sideboard, &b.commanders], after))
    };
    let card_count = |cards: &[String], sideboard: &[String]| DeckCardCount {
        main: deckdiff::card_count(cards),
        side: deckdiff::card_count(sideboard),
    };
    let past: Vec<_> = versions
        .iter()
        .enumerate()
        .map(|(at, v)| DeckPast {
            version: v.version,
            cards: v.cards.len(),
            sideboard: v.sideboard.len(),
            commanders: v.commanders.clone(),
            summary: v.summary.clone(),
            superseded_at: v.superseded_at,
            delta: delta(
                [&v.cards, &v.sideboard, &v.commanders],
                versions.get(at + 1),
            ),
            card_count: card_count(&v.cards, &v.sideboard),
        })
        .collect();
    Ok(Json(DeckHistory {
        version: deck.version,
        updated_at: deck.updated_at,
        past,
        delta: delta(
            [&deck.cards, &deck.sideboard, &deck.commanders],
            versions.first(),
        ),
        card_count: card_count(&deck.cards, &deck.sideboard),
    }))
}

/// `GET /decks/{id}/history`.
/// @openapi
#[derive(Serialize)]
pub(crate) struct DeckHistory {
    /// @description The version the deck's current cards are.
    version: i32,
    /// @description When it was last saved, in unix seconds.
    updated_at: u64,
    /// @description Every state it no longer holds, newest first.
    past: Vec<DeckPast>,
    /// @description What the current state changed against the newest past
    /// one, `null` when it has none.
    delta: Option<Delta>,
    /// @description How many cards the current state holds.
    card_count: DeckCardCount,
}

/// One state a deck no longer holds, in `GET /decks/{id}/history`.
/// @openapi
#[derive(Serialize)]
pub(crate) struct DeckPast {
    /// @description Which version it was.
    version: i32,
    /// @description How many main-deck lines it had.
    cards: usize,
    /// @description How many sideboard lines it had.
    sideboard: usize,
    /// @description The commanders it had.
    commanders: Vec<String>,
    /// @description What the change that replaced it was called, `null`
    /// when it was given no name.
    summary: Option<String>,
    /// @description When it stopped being current, in unix seconds.
    superseded_at: u64,
    /// @description What it changed against the state before it, `null` on
    /// the oldest.
    delta: Option<Delta>,
    /// @description How many cards it held.
    card_count: DeckCardCount,
}

/// How many cards a state of a deck holds.
/// @openapi
#[derive(Serialize)]
pub(crate) struct DeckCardCount {
    /// @description Main-deck cards, counting copies.
    main: u32,
    /// @description Sideboard cards, counting copies.
    side: u32,
}

/// One superseded state, in full.
///
/// @route GET /decks/{id}/versions/{version}
/// @scope public
/// @summary One state of a deck
/// @description The lists of one version of the deck, in full: a past
/// state with its summary and when it was replaced (`current: false`),
/// or, for the deck's current version, the deck as it is now
/// (`current: true`).
/// @tag Decks
/// @security bearer
/// @insert `SessionRefusals`
/// @path-param id: String "The deck's id"
/// @path-param version: i32 "Which version"
/// @return 400: String text/plain "A version that is not a number (axum's rejection)"
/// @return 403: `$ErrorBody` "Another account's deck; or the account is banned (`ban`) or must accept the terms in force first (`terms_required`)"
/// @return 404: `$ErrorBody` "No such deck, or no such version of it"
pub(crate) async fn deck_version(
    State(state): State<Shared>,
    headers: HeaderMap,
    Path((id, version)): Path<(String, i32)>,
) -> Result<Json<DeckState>, (StatusCode, Json<ErrorBody>)> {
    let account_id = authed(&state, &headers).await?;
    let deck = readable_deck(&state, &id, &account_id).await?;
    if version == deck.version {
        // The current state is not in the history table and never will be.
        // Answering it from the deck itself is what makes "show me version
        // N" a question the caller can ask about any N it was given.
        return Ok(Json(DeckState::Current(DeckStateNow {
            version: deck.version,
            cards: deck.cards,
            sideboard: deck.sideboard,
            commanders: deck.commanders,
            current: true,
        })));
    }
    let past = store::deck_at_version(&state.db, &id, version)
        .await
        .map_err(|e| db_down(&e))?
        .ok_or_else(|| err(StatusCode::NOT_FOUND, "no such version"))?;
    Ok(Json(DeckState::Past(DeckStateThen {
        version: past.version,
        cards: past.cards,
        sideboard: past.sideboard,
        commanders: past.commanders,
        summary: past.summary,
        superseded_at: past.superseded_at,
        current: false,
    })))
}

/// `GET /decks/{id}/versions/{version}`: the deck as it is now, or a state
/// it no longer holds, told apart by `current`.
/// @openapi
#[derive(Serialize)]
#[serde(untagged)]
pub(crate) enum DeckState {
    /// The version asked for is the deck's current one.
    Current(DeckStateNow),
    /// The version asked for is one the deck no longer holds.
    Past(DeckStateThen),
}

/// The state a deck holds now.
/// @openapi
#[derive(Serialize)]
pub(crate) struct DeckStateNow {
    /// @description Its version, the deck's current one.
    version: i32,
    /// @description Main-deck lines.
    cards: Vec<String>,
    /// @description Sideboard lines.
    sideboard: Vec<String>,
    /// @description Its commanders.
    commanders: Vec<String>,
    /// @description Always `true`: this is the deck as it is now.
    /// @openapi const: true
    current: bool,
}

/// A state a deck no longer holds.
/// @openapi
#[derive(Serialize)]
pub(crate) struct DeckStateThen {
    /// @description Which version it was.
    version: i32,
    /// @description Main-deck lines it held.
    cards: Vec<String>,
    /// @description Sideboard lines it held.
    sideboard: Vec<String>,
    /// @description The commanders it had.
    commanders: Vec<String>,
    /// @description What the change that replaced it was called, `null`
    /// when it was given no name.
    summary: Option<String>,
    /// @description When it stopped being current, in unix seconds.
    superseded_at: u64,
    /// @description Always `false`: a state the deck no longer holds.
    /// @openapi const: false
    current: bool,
}

/// Put an earlier state back, as a new change.
///
/// Not a rewind: the deck's present is archived exactly as any other save
/// archives it, and the old lists become the new head one version higher.
/// So a revert can itself be reverted, and nothing in the history is ever
/// removed or rewritten.
///
/// @route POST /decks/{id}/versions/{version}/revert
/// @scope public
/// @summary Put an earlier state of a deck back
/// @description Saves the lists of an earlier version as a new change, one
/// version higher, keeping the present in the history as any save does,
/// and answers the new version. A revert can itself be reverted.
/// @tag Decks
/// @security bearer
/// @insert `SessionRefusals`
/// @path-param id: String "The deck's id"
/// @path-param version: i32 "The version to put back"
/// @return 400: String text/plain "A version that is not a number (axum's rejection)"
/// @return 403: `$ErrorBody` "Another account's deck, or a shared one; or the account is banned (`ban`) or must accept the terms in force first (`terms_required`)"
/// @return 404: `$ErrorBody` "No such deck, or no such version of it"
/// @return 409: `$ErrorBody` "That version is the deck's current state"
pub(crate) async fn revert_deck(
    State(state): State<Shared>,
    headers: HeaderMap,
    Path((id, version)): Path<(String, i32)>,
) -> Result<Json<DeckReverted>, (StatusCode, Json<ErrorBody>)> {
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
    Ok(Json(DeckReverted { version: next }))
}

/// `POST /decks/{id}/versions/{version}/revert`.
/// @openapi
#[derive(Serialize)]
pub(crate) struct DeckReverted {
    /// @description The deck's new version, which holds the lists put back.
    version: i32,
}

/// Take a copy of a deck anybody may play, as an account's own.
///
/// The copy is an ordinary account deck from the moment it exists — its own
/// history starts at version 1 — and it remembers which deck and which of
/// that deck's versions it came from, so "this is the Kess precon as it was"
/// survives the original moving on.
///
/// @route POST /decks/{id}/copy
/// @scope public
/// @summary Take a copy of a shared deck
/// @description Copies a deck the account may read (a house or
/// preconstructed deck, or one of its own) as a new deck of the account's
/// own, which remembers the deck and version it came from and starts with
/// the generated card back.
/// @tag Decks
/// @security bearer
/// @insert `SessionRefusals`
/// @path-param id: String "The id of the deck to copy"
/// @return 403: `$ErrorBody` "Another account's deck; or the account is banned (`ban`) or must accept the terms in force first (`terms_required`)"
/// @return 404: `$ErrorBody` "No such deck"
/// @return 410: `$ErrorBody` "A preconstructed deck this build no longer offers"
/// @return 500: `$ErrorBody` "The account was deleted while the copy was made"
pub(crate) async fn copy_deck(
    State(state): State<Shared>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> Result<Json<DeckCreated>, (StatusCode, Json<ErrorBody>)> {
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
    Ok(Json(DeckCreated { deck_id: copy }))
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

/// @route DELETE /decks/{id}
/// @scope public
/// @summary Throw a deck away
/// @description Deletes one of the account's own decks.
/// @tag Decks
/// @security bearer
/// @insert `SessionRefusals`
/// @path-param id: String "The deck's id"
/// @return 204: "Deleted"
/// @return 403: `$ErrorBody` "Another account's deck, or a shared one; or the account is banned (`ban`) or must accept the terms in force first (`terms_required`)"
/// @return 404: `$ErrorBody` "No such deck"
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
