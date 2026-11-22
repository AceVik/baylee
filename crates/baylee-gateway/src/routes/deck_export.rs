//! Read-only private migration for the local projection of a Central session.
use crate::{
    Deserialize, ErrorBody, HeaderMap, Json, Shared, State, StatusCode, auth, authed, db_down, err,
    store,
};
use axum::extract::Query;
use baylee_account::decks::{DeckDraft, gateway_export::GatewayDeckVersion};
use sha2::{Digest, Sha256};
use uuid::Uuid;

/// Keyset cursor for one saved version of one private deck.
/// @openapi
#[derive(Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct DeckExportCursor {
    /// @description Start the next deck after this ID. Absent starts at the first owned deck.
    after: Option<Uuid>,
    /// @description Continue this particular owned deck.
    deck: Option<Uuid>,
    /// @description Last received historical version of that deck.
    #[serde(default)]
    version: i32,
}

/// @route GET /decks/private-export
/// @scope public
/// @summary Read one owned gateway deck save for migration
/// @operation-id `readPrivateDeckSave`
/// @description Returns one save in ascending version order, including the head.
/// A null page ends the scan. Each response binds the Central identity, local
/// owner and full head generation. The source is read in a read-only transaction.
/// @tag Decks
/// @security bearer
/// @insert `SessionRefusals`
/// @query-params `$DeckExportCursor`
/// @return 200: `{ anyOf: [{ $ref: $GatewayDeckVersion }, { type: "null" }] }` "One owned deck revision. A null page ends the scan."
/// @return 400: `$ErrorBody` "Invalid cursor"
/// @return 403: `$ErrorBody` "A linked Central session is required"
/// @return 409: `$ErrorBody` "The source deck changed or was removed"
/// @return 429: `$ErrorBody` "The gateway is already reading its available private-library pages"
pub(crate) async fn read(
    State(state): State<Shared>,
    headers: HeaderMap,
    Query(cursor): Query<DeckExportCursor>,
) -> Result<Json<Option<GatewayDeckVersion>>, (StatusCode, Json<ErrorBody>)> {
    let local = authed(&state, &headers).await?;
    let forbidden = || {
        err(
            StatusCode::FORBIDDEN,
            "a linked Central session is required",
        )
    };
    let bearer = crate::bearer_token(&headers).ok_or_else(forbidden)?;
    let central = state
        .sessions
        .by_bearer(&auth::token_digest(bearer), crate::sessions::now())
        .filter(|live| live.account() == local)
        .and_then(|live| live.signed().map(|signed| Uuid::from_bytes(signed.central)))
        .ok_or_else(forbidden)?;
    let owner = Uuid::parse_str(&local).map_err(|_| forbidden())?;
    if cursor.after.is_some() && cursor.deck.is_some()
        || cursor.version < 0
        || (cursor.deck.is_none() && cursor.version != 0)
    {
        return Err(err(StatusCode::BAD_REQUEST, "invalid export cursor"));
    }
    let _permit = state
        .shelf_turns
        .try_acquire()
        .map_err(|_| err(StatusCode::TOO_MANY_REQUESTS, "private deck export is busy"))?;
    let Some((head, old)) =
        store::deck_export::read(&state.db, owner, cursor.after, cursor.deck, cursor.version)
            .await
            .map_err(|e| db_down(&e.into()))?
    else {
        if cursor.deck.is_some() {
            return Err(err(
                StatusCode::CONFLICT,
                "source deck changed or was removed",
            ));
        }
        return Ok(Json(None));
    };
    let invalid = || err(StatusCode::CONFLICT, "invalid source deck version");
    let deck = Uuid::parse_str(&head.id).map_err(|_| invalid())?;
    let head_revision = u64::try_from(head.version).map_err(|_| invalid())?;
    if cursor.version >= head.version {
        return Err(invalid());
    }
    let copied = head
        .origin
        .map(|(id, version)| {
            Ok((
                Uuid::parse_str(&id).map_err(|_| invalid())?,
                u64::try_from(version).map_err(|_| invalid())?,
            ))
        })
        .transpose()?;
    let mut draft = DeckDraft {
        name: head.name,
        format: head.format,
        description: head.description.unwrap_or_default(),
        cards: head.cards,
        sideboard: head.sideboard,
        commanders: head.commanders,
        sleeve: head.sleeve,
        playmat: head.playmat,
        ..DeckDraft::default()
    };
    let bytes = serde_json::to_vec(&(deck, owner, head_revision, head.updated_at, copied, &draft))
        .map_err(|_| invalid())?;
    let generation = Sha256::digest(bytes).into();
    let (revision, at) = if let Some(old) = old {
        if old.version <= cursor.version || old.version >= head.version {
            return Err(invalid());
        }
        draft.cards = old.cards;
        draft.sideboard = old.sideboard;
        draft.commanders = old.commanders;
        draft.summary = old.summary;
        (
            u64::try_from(old.version).map_err(|_| invalid())?,
            old.superseded_at,
        )
    } else {
        (head_revision, head.updated_at)
    };
    Ok(Json(Some(GatewayDeckVersion {
        central,
        owner,
        deck,
        generation,
        head_revision,
        revision,
        saved_at: i64::try_from(at).map_err(|_| invalid())?,
        copied,
        draft,
    })))
}
