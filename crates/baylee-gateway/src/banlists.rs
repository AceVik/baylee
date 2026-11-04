//! Ban lists (`docs/banlists-and-formats.md`, `docs/protocol.md` §"Ban lists"):
//! the official ones and a player's own, and what a room makes of one.
//!
//! The official lists are not kept by hand. Each is a `builtin` row of the
//! store, brought in step with `card_legalities` whenever the catalog's stamp
//! moved ([`keep_official`]), so copying one, picking one and freezing one are
//! the same as for a list a player made. Without a catalog there are none.
//!
//! A list is read by its owner alone until a host picks it for a room; then
//! the room holds its own copy ([`crate::lobby::RoomList`]) and whoever sits
//! there may read that, and nothing else about the list. Nothing in this file
//! ever says whose a list is: `mine` answers "is it yours" and that is all.

use crate::{ErrorBody, HeaderMap, Json, Path, Shared, State, StatusCode, authed, db_down, err};
use axum::Router;
use axum::routing::get;
use baylee_cardtext::{ListBody, ListEntry, ListRow, ListStatus, MAX_LIST_NAME_CHARS};
use baylee_db::banlists::{self, Banlist, Changed, Created, Edit, Entry};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::sync::OnceLock;
use uuid::Uuid;

/// The stamp the official lists were last brought in step at.
#[derive(Default)]
pub(crate) struct OfficialLists {
    /// A burst of requests after an ingest waits here and writes once.
    synced: tokio::sync::Mutex<Option<i64>>,
}

/// Brings the official lists in step with the catalog when its stamp moved.
/// A catalog that cannot be read leaves them as they stand and says so in the
/// log: they are read, and a stale list is one ingest behind and not wrong.
pub(crate) async fn keep_official(state: &Shared) {
    let Some(catalog) = state.catalog.as_ref() else {
        return;
    };
    let mut synced = state.official_lists.synced.lock().await;
    let stamp = match catalog.data_version().await {
        Ok(stamp) => stamp,
        Err(e) => {
            tracing::warn!(%e, "the official ban lists stay as they are");
            return;
        }
    };
    if *synced == Some(stamp) {
        return;
    }
    match write_official(state, catalog).await {
        Ok(()) => *synced = Some(stamp),
        Err(e) => tracing::warn!("the official ban lists stay as they are: {e:#}"),
    }
}

/// The pool's cards only: a deck here holds no other, and a card outside the
/// pool stays out of what this gateway serves of the feed (`docs/legal.md` §3).
async fn write_official(state: &Shared, catalog: &baylee_catalog::Catalog) -> anyhow::Result<()> {
    for (format, cards) in catalog.official_lists().await? {
        let entries: Vec<Entry> = cards
            .into_iter()
            .filter(|(oracle_id, _)| baylee_cards::by_oracle_id(oracle_id).is_some())
            .filter_map(|(oracle_id, status)| {
                Some(Entry {
                    oracle_id: Uuid::parse_str(&oracle_id).ok()?,
                    status: status.into(),
                })
            })
            .collect();
        banlists::sync_official(&state.db, &format, &entries).await?;
    }
    Ok(())
}

/// The card an oracle id names: its registry index where this build compiles
/// it, and its English name from the ledger, which numbers every card there
/// is. A list may hold a card this pool does not.
fn card_of(oracle_id: &str) -> (Option<u32>, String) {
    static LEDGER: OnceLock<BTreeMap<&'static str, &'static str>> = OnceLock::new();
    if let Some(def) = baylee_cards::by_oracle_id(oracle_id) {
        return (Some(def.index.get()), def.name().to_string());
    }
    let ledger = LEDGER.get_or_init(|| {
        baylee_cards_index::ROWS
            .iter()
            .map(|row| (row.oracle_id, row.name))
            .collect()
    });
    (
        None,
        ledger
            .get(oracle_id)
            .map_or_else(String::new, |name| (*name).to_string()),
    )
}

/// A list as the wire writes it.
fn row_of(list: &Banlist, account: Option<Uuid>) -> ListRow {
    ListRow {
        id: list.id.to_string(),
        name: list.name.clone(),
        format: list.format.clone(),
        builtin: list.builtin,
        mine: account.is_some() && list.owner == account,
        version: list.version,
        updated_at: u64::try_from(list.updated_at.unix_timestamp()).unwrap_or(0),
        cards: list.cards,
    }
}

/// A list and its cards as the wire writes them.
pub(crate) fn body_of(list: &Banlist, entries: &[Entry], account: Option<Uuid>) -> ListBody {
    let mut entries: Vec<ListEntry> = entries
        .iter()
        .map(|entry| {
            let oracle_id = entry.oracle_id.to_string();
            let (index, name) = card_of(&oracle_id);
            ListEntry {
                oracle_id,
                status: entry.status.into(),
                index,
                name,
            }
        })
        .collect();
    entries.sort_by(|a, b| {
        a.name
            .cmp(&b.name)
            .then_with(|| a.oracle_id.cmp(&b.oracle_id))
    });
    ListBody {
        row: row_of(list, account),
        entries,
    }
}

/// The account behind a session, as the store's key.
fn account_key(account: &str) -> Result<Uuid, (StatusCode, Json<ErrorBody>)> {
    Uuid::parse_str(account).map_err(|_| {
        err(
            StatusCode::INTERNAL_SERVER_ERROR,
            "an unreadable account id",
        )
    })
}

/// A list's id, where a malformed one is a list that does not exist.
fn list_key(id: &str) -> Result<Uuid, (StatusCode, Json<ErrorBody>)> {
    Uuid::parse_str(id).map_err(|_| no_such_list())
}

/// What a list that is not there, and one that is not the caller's, both
/// answer: which of the two it is would say a list exists.
pub(crate) fn no_such_list() -> (StatusCode, Json<ErrorBody>) {
    err(StatusCode::NOT_FOUND, "no such list")
}

/// A database failure as the routes answer it.
fn down(e: &sea_orm::DbErr) -> (StatusCode, Json<ErrorBody>) {
    db_down(&anyhow::anyhow!("{e}"))
}

/// Whether a name may stand on a list.
fn check_name(name: &str) -> Result<(), (StatusCode, Json<ErrorBody>)> {
    let length = name.chars().count();
    if name.trim().is_empty() || length > MAX_LIST_NAME_CHARS || name.chars().any(char::is_control)
    {
        return Err(err(StatusCode::BAD_REQUEST, "invalid list name"));
    }
    Ok(())
}

/// Whether a format's key may stand on a list. The formats follow the feed,
/// so this asks only that it reads as a key.
pub(crate) fn check_format(format: &str) -> Result<(), (StatusCode, Json<ErrorBody>)> {
    let plain = !format.is_empty()
        && format.len() <= 32
        && format
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit());
    if plain {
        Ok(())
    } else {
        Err(err(StatusCode::BAD_REQUEST, "invalid format"))
    }
}

/// `GET /banlists`.
/// @openapi
#[derive(Serialize)]
pub(crate) struct BanlistsBody {
    /// @description Every official list, then the caller's own, by name.
    lists: Vec<ListRow>,
}

/// @route GET /banlists
/// @scope public
/// @summary The ban lists the caller may use
/// @description Every official list (one a format, following the card feed;
/// none without a catalog) and the caller's own. No list of anyone else's.
/// For a signed-in session, a guest's included.
/// @tag Decks
/// @security bearer
/// @insert `SessionRefusals`
pub(crate) async fn list_banlists(
    State(state): State<Shared>,
    headers: HeaderMap,
) -> Result<Json<BanlistsBody>, (StatusCode, Json<ErrorBody>)> {
    let account = account_key(&authed(&state, &headers).await?)?;
    keep_official(&state).await;
    let lists = banlists::listed(&state.db, account)
        .await
        .map_err(|e| down(&e))?;
    Ok(Json(BanlistsBody {
        lists: lists
            .iter()
            .map(|list| row_of(list, Some(account)))
            .collect(),
    }))
}

/// @route GET /banlists/{id}
/// @scope public
/// @summary One ban list and its cards
/// @description An official list, or one of the caller's own. Anyone else's
/// reads as no list at all.
/// @tag Decks
/// @security bearer
/// @insert `SessionRefusals`
/// @path-param id: String "The list's id"
/// @return 404: `$ErrorBody` "No such list, or one that is not the caller's (`no such list`)"
pub(crate) async fn get_banlist(
    State(state): State<Shared>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> Result<Json<ListBody>, (StatusCode, Json<ErrorBody>)> {
    let account = account_key(&authed(&state, &headers).await?)?;
    let id = list_key(&id)?;
    keep_official(&state).await;
    let list = banlists::readable(&state.db, id, account)
        .await
        .map_err(|e| down(&e))?
        .ok_or_else(no_such_list)?;
    let entries = banlists::entries(&state.db, id)
        .await
        .map_err(|e| down(&e))?;
    Ok(Json(body_of(&list, &entries, Some(account))))
}

/// What `POST /banlists` makes.
///
/// @description A new list of the caller's own: empty, or a copy of one the
/// caller may read.
/// @openapi
#[derive(Deserialize)]
pub(crate) struct NewBanlist {
    /// @description What it is called, 1 to 60 characters.
    name: String,
    /// @description The format it is for: Scryfall's key, or `freeform` for
    /// a list that keeps to no format.
    format: String,
    /// @description The list to start from: an official one or the caller's
    /// own; absent for an empty list.
    #[serde(default)]
    copy_of: Option<String>,
    /// @description Every card it holds, at most 2000, each once, in place
    /// of what `copy_of` holds.
    #[serde(default)]
    entries: Option<Vec<NewEntry>>,
}

/// @route POST /banlists
/// @scope public
/// @summary Make a ban list of your own
/// @description An empty list, or a copy of an official list or of one of
/// the caller's own. A copy stands on its own after that: the feed moving
/// does not move it. Answers the new list. An account holds at most 50.
/// @tag Decks
/// @security bearer
/// @insert `SessionRefusals`
/// @return 400: `$ErrorBody` "A name that is empty, over 60 characters or holds a control character, a format that does not read as a key, a card id that is not one, or a card named twice"
/// @return 404: `$ErrorBody` "`copy_of` names no list the caller may read (`no such list`)"
/// @return 409: `$ErrorBody` "The account holds 50 lists already, or the list to copy is over 2000 cards"
pub(crate) async fn create_banlist(
    State(state): State<Shared>,
    headers: HeaderMap,
    Json(body): Json<NewBanlist>,
) -> Result<Json<ListBody>, (StatusCode, Json<ErrorBody>)> {
    let account = account_key(&authed(&state, &headers).await?)?;
    check_name(&body.name)?;
    check_format(&body.format)?;
    let copy_of = body.copy_of.as_deref().map(list_key).transpose()?;
    let cards = body.entries.as_deref().map(entries_of).transpose()?;
    if copy_of.is_some() {
        keep_official(&state).await;
    }
    let made = banlists::create(&state.db, account, &body.name, &body.format, copy_of)
        .await
        .map_err(|e| down(&e))?;
    let mut list = match made {
        Created::Made(list) => list,
        Created::TooMany => return Err(err(StatusCode::CONFLICT, "too many lists")),
        Created::NoSuchList => return Err(no_such_list()),
    };
    if let Some(cards) = cards {
        let change = Edit {
            entries: Some(cards),
            ..Edit::default()
        };
        match banlists::edit(&state.db, list.id, account, &change)
            .await
            .map_err(|e| down(&e))?
        {
            Changed::Made(written) => list = written,
            _ => return Err(err(StatusCode::CONFLICT, "too many cards")),
        }
    }
    let entries = banlists::entries(&state.db, list.id)
        .await
        .map_err(|e| down(&e))?;
    Ok(Json(body_of(&list, &entries, Some(account))))
}

/// One card of a list being written.
///
/// @description One card of a list being written.
/// @openapi
#[derive(Deserialize)]
pub(crate) struct NewEntry {
    /// @description The card's oracle id.
    oracle_id: String,
    /// @description What the list says about it.
    status: ListStatus,
}

/// What `PUT /banlists/{id}` changes; an absent field leaves that part alone.
///
/// @description What to change about a list; an absent field leaves that
/// part alone. `entries` is every card the list then names.
/// @openapi
#[derive(Deserialize)]
pub(crate) struct BanlistChange {
    /// @description Its new name.
    #[serde(default)]
    name: Option<String>,
    /// @description Its new format.
    #[serde(default)]
    format: Option<String>,
    /// @description Every card it then names, at most 2000, each once.
    #[serde(default)]
    entries: Option<Vec<NewEntry>>,
}

/// Reads the entries of a change, refusing a card named twice: which of the
/// two a player meant is not this route's to guess.
fn entries_of(entries: &[NewEntry]) -> Result<Vec<Entry>, (StatusCode, Json<ErrorBody>)> {
    let mut seen = BTreeMap::new();
    for entry in entries {
        let oracle_id = Uuid::parse_str(&entry.oracle_id)
            .map_err(|_| err(StatusCode::BAD_REQUEST, "invalid card id"))?;
        if seen.insert(oracle_id, entry.status).is_some() {
            return Err(err(StatusCode::BAD_REQUEST, "a card named twice"));
        }
    }
    Ok(seen
        .into_iter()
        .map(|(oracle_id, status)| Entry {
            oracle_id,
            status: status.into(),
        })
        .collect())
}

/// @route PUT /banlists/{id}
/// @scope public
/// @summary Change one of your ban lists
/// @description All of it or none of it. A change to the cards or the
/// format counts the version up, and a waiting room holding the list reads it
/// at its next check; a game already started keeps the version it started
/// with. Answers the list as it now stands.
/// @tag Decks
/// @security bearer
/// @insert `SessionRefusals`
/// @path-param id: String "The list's id"
/// @return 400: `$ErrorBody` "A name or format that may not stand, a card id that is not one, or a card named twice"
/// @return 403: `$ErrorBody` "An official list (`official lists are read-only`); or the account is banned (`ban`) or must accept the terms first (`terms_required`)"
/// @return 404: `$ErrorBody` "No such list, or one that is not the caller's (`no such list`)"
/// @return 409: `$ErrorBody` "More than 2000 cards"
pub(crate) async fn update_banlist(
    State(state): State<Shared>,
    headers: HeaderMap,
    Path(id): Path<String>,
    Json(body): Json<BanlistChange>,
) -> Result<Json<ListBody>, (StatusCode, Json<ErrorBody>)> {
    let account = account_key(&authed(&state, &headers).await?)?;
    let id = list_key(&id)?;
    if let Some(name) = &body.name {
        check_name(name)?;
    }
    if let Some(format) = &body.format {
        check_format(format)?;
    }
    let change = Edit {
        entries: body.entries.as_deref().map(entries_of).transpose()?,
        name: body.name,
        format: body.format,
    };
    let done = banlists::edit(&state.db, id, account, &change)
        .await
        .map_err(|e| down(&e))?;
    let list = match done {
        Changed::Made(list) => list,
        Changed::NoSuchList => return Err(no_such_list()),
        Changed::Official => {
            return Err(err(StatusCode::FORBIDDEN, "official lists are read-only"));
        }
        Changed::TooManyCards => return Err(err(StatusCode::CONFLICT, "too many cards")),
    };
    let entries = banlists::entries(&state.db, id)
        .await
        .map_err(|e| down(&e))?;
    Ok(Json(body_of(&list, &entries, Some(account))))
}

/// @route DELETE /banlists/{id}
/// @scope public
/// @summary Delete one of your ban lists
/// @description A room holding the list keeps the copy it read, until it
/// closes. An official list is nobody's to delete.
/// @tag Decks
/// @security bearer
/// @insert `SessionRefusals`
/// @path-param id: String "The list's id"
/// @return 204: "The list is gone"
/// @return 404: `$ErrorBody` "No such list, or one that is not the caller's (`no such list`)"
pub(crate) async fn delete_banlist(
    State(state): State<Shared>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> Result<StatusCode, (StatusCode, Json<ErrorBody>)> {
    let account = account_key(&authed(&state, &headers).await?)?;
    let id = list_key(&id)?;
    if banlists::delete(&state.db, id, account)
        .await
        .map_err(|e| down(&e))?
    {
        Ok(StatusCode::NO_CONTENT)
    } else {
        Err(no_such_list())
    }
}

/// The ban list routes, as `main` merges them.
pub(crate) fn routes() -> Router<Shared> {
    Router::new()
        .route("/banlists", get(list_banlists).post(create_banlist))
        .route(
            "/banlists/{id}",
            get(get_banlist).put(update_banlist).delete(delete_banlist),
        )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_key_reads_as_a_key() {
        assert!(check_format("commander").is_ok());
        assert!(check_format("freeform").is_ok());
        assert!(check_format("").is_err());
        assert!(check_format("Modern").is_err());
        assert!(check_format("a b").is_err());
    }

    #[test]
    fn a_name_has_bounds() {
        assert!(check_name("Kitchen table").is_ok());
        assert!(check_name("   ").is_err());
        assert!(check_name(&"x".repeat(61)).is_err());
        assert!(check_name("line\nbreak").is_err());
    }

    /// A card named twice is refused, whichever status each says.
    #[test]
    fn a_card_is_named_once() {
        let id = Uuid::now_v7().to_string();
        let twice = [
            NewEntry {
                oracle_id: id.clone(),
                status: ListStatus::Banned,
            },
            NewEntry {
                oracle_id: id,
                status: ListStatus::Restricted,
            },
        ];
        assert!(entries_of(&twice).is_err());
    }

    /// A card the build compiles has its index; one only the ledger knows
    /// keeps its name; one nobody knows keeps its place on the list.
    #[test]
    fn a_card_is_named_by_whoever_knows_it() {
        let ring = baylee_cards::decks::by_name("Sol Ring")
            .and_then(baylee_cards::by_index)
            .expect("in the registry");
        let (index, name) = card_of(ring.oracle_id);
        assert_eq!(index, Some(ring.index.get()));
        assert_eq!(name, "Sol Ring");
        assert_eq!(
            card_of("00000000-0000-0000-0000-000000000000"),
            (None, String::new())
        );
    }
}
