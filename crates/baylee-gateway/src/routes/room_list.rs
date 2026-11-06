//! What a room holds a deck to: the host's choice, who may read it, and the
//! check every deck meets on its way to the table.
//!
//! The check is `baylee_cards::formats::validate`, and it is the one that
//! counts; a client's is a courtesy. It reads two things the lobby's lock may
//! not wait for, the list in the store and the pool's table, so [`guard`]
//! fetches them first and a route then holds the deck up to the [`Guard`]
//! under the lock, as `own_deck` reads a deck before it.
//!
//! The gateway runs no rules: a game that has started is never asked anything.
//! A deck that became illegal after the start plays on, and the list a game
//! started under is the version its decks passed.

use crate::lobby::{self, LobbyGame, LobbyState, RoomList};
use crate::routes::{LobbyListing, loaded_deck};
use crate::{
    Deck, Deserialize, ErrorBody, HeaderMap, Json, Path, Shared, State, StatusCode, authed,
    banlists, db_down, err, err_saying, listing,
};
use baylee_cards::formats::{self, FREEFORM, ListRules, PoolFacts, Problem, Report, Rules};
use baylee_cardtext::{DeckRefusal, ListBody};
use baylee_db::banlists as store;
use std::sync::Arc;
use uuid::Uuid;

/// A room's list and the table it is read with, fetched outside the lock.
pub(crate) struct Guard {
    /// The list as it stands now.
    pub(crate) list: RoomList,
    /// The format a deck is held to.
    format: String,
    /// The pool's table; `None` without a catalog, and the check then asks
    /// the list and the shape and leaves the format's pool alone.
    facts: Option<Arc<PoolFacts>>,
}

impl Guard {
    fn rules(&self) -> Rules<'_> {
        let list: &ListRules = &self.list.rules;
        Rules {
            format: &self.format,
            facts: self.facts.as_deref(),
            list: Some(list),
        }
    }

    fn report(&self, deck: &Deck) -> Result<Report, (StatusCode, Json<ErrorBody>)> {
        Ok(formats::validate_loaded(&loaded_deck(deck)?, self.rules()))
    }

    /// Lets `deck` sit down, or refuses it with every reason it has. Only
    /// what is wrong refuses it: what could not be asked (no catalog, a
    /// format whose shape this build does not know) lets it through.
    pub(crate) fn admit(&self, deck: &Deck) -> Result<(), (StatusCode, Json<ErrorBody>)> {
        let report = self.report(deck)?;
        if report.problems.is_empty() {
            return Ok(());
        }
        Err(refused(
            deck,
            &self.list.body,
            &self.format,
            report.problems,
        ))
    }

    /// Says which seated decks the list would turn away. A deck that cannot
    /// be read is left unmarked: the next door refuses it anyway.
    fn mark_the_table(&self, game: &mut LobbyGame) {
        for seat in &mut game.seats {
            seat.off_list = seat.deck.as_ref().is_some_and(|deck| {
                self.report(deck)
                    .is_ok_and(|report| !report.problems.is_empty())
            });
        }
    }

    /// Holds every deck at a table up to the list once more, as the game is
    /// about to start. A chair that no longer passes loses its ready, and the
    /// refusal names the chair and not its cards: the host may not read
    /// another player's deck, and that player reads why at their next ready.
    fn check_table(&self, game: &mut LobbyGame) -> Result<(), (StatusCode, Json<ErrorBody>)> {
        let mut failed = None;
        for seat in &mut game.seats {
            let Some(deck) = &seat.deck else {
                continue;
            };
            if !self.report(deck)?.problems.is_empty() {
                seat.said_ready = false;
                failed = failed.or(Some(seat.seat));
            }
        }
        match failed {
            Some(seat) => Err(err_saying(
                StatusCode::CONFLICT,
                format!("the deck in seat {seat} no longer keeps to this table's list"),
            )),
            None => Ok(()),
        }
    }
}

/// A refusal of `deck` under `list`, with the reasons as data for a client to
/// word and a sentence for one that cannot.
fn refused(
    deck: &Deck,
    list: &ListBody,
    format: &str,
    problems: Vec<Problem>,
) -> (StatusCode, Json<ErrorBody>) {
    let cards: Vec<&str> = problems
        .iter()
        .filter_map(|problem| problem.card())
        .filter_map(crate::blocks::pool_name)
        .take(3)
        .collect();
    let about = if cards.is_empty() {
        String::new()
    } else {
        format!(": {}", cards.join(", "))
    };
    let (status, Json(mut body)) = err_saying(
        StatusCode::CONFLICT,
        format!(
            "the deck \u{201c}{}\u{201d} does not keep to the list \u{201c}{}\u{201d}{about}",
            deck.name, list.row.name
        ),
    );
    body.refused = Some(Box::new(DeckRefusal {
        list: list.row.name.clone(),
        format: format.to_string(),
        problems,
    }));
    (status, Json(body))
}

/// The format a room with this list holds a deck to: what the host said,
/// else the list's.
fn format_of(game: &LobbyGame, list: &RoomList) -> String {
    game.format
        .clone()
        .unwrap_or_else(|| list.body.row.format.clone())
}

/// A list read from the store, as a room holds it.
async fn read_list(state: &Shared, id: Uuid) -> anyhow::Result<Option<RoomList>> {
    let Some(list) = store::held(&state.db, id).await? else {
        return Ok(None);
    };
    let entries = store::entries(&state.db, id).await?;
    Ok(Some(RoomList::of(
        id,
        banlists::body_of(&list, &entries, None),
    )))
}

/// The list a waiting room holds, read again; `None` for a room with none, or
/// one that is not waiting, which is never asked anything.
///
/// A list that went from the store (its owner deleted it, or the account)
/// stays what the room last read, so a host is not left with rules nobody can
/// name. Written back so that everyone at the table reads the one that was
/// checked.
pub(crate) async fn guard(
    state: &Shared,
    id: &str,
) -> Result<Option<Guard>, (StatusCode, Json<ErrorBody>)> {
    let held = {
        let lobby = state.lobby.lock();
        lobby
            .games
            .get(id)
            .filter(|game| game.state == LobbyState::Waiting)
            .and_then(|game| game.list.clone())
    };
    let Some(held) = held else {
        return Ok(None);
    };
    if held.body.row.builtin {
        banlists::keep_official(state).await;
    }
    let fresh = read_list(state, held.id).await.map_err(|e| db_down(&e))?;
    let list = fresh.unwrap_or(held);
    let facts = state.pool_table().await;
    let mut lobby = state.lobby.lock();
    let Some(game) = lobby.games.get_mut(id) else {
        return Ok(None);
    };
    if game.state != LobbyState::Waiting {
        return Ok(None);
    }
    let format = format_of(game, &list);
    game.list = Some(list.clone());
    Ok(Some(Guard {
        list,
        format,
        facts,
    }))
}

/// Holds the table up to its list as it is about to start.
///
/// # Errors
/// When a deck no longer keeps to it; the chair is then not ready, and the
/// listing moved.
pub(crate) fn hold_the_table(
    state: &Shared,
    id: &str,
    guard: &Guard,
) -> Result<(), (StatusCode, Json<ErrorBody>)> {
    let checked = {
        let mut lobby = state.lobby.lock();
        match lobby.games.get_mut(id) {
            Some(game) if game.state == LobbyState::Waiting => guard.check_table(game),
            _ => Ok(()),
        }
    };
    if checked.is_err() {
        state.lobby_moved();
    }
    checked
}

/// What the host picks for a room.
///
/// @description The list a room holds decks to, and the format it plays.
/// Both absent take any list away and leave a table that takes every deck.
/// @openapi
#[derive(Deserialize)]
pub(crate) struct RoomListBody {
    /// @description An official list or one of the host's own
    /// (`GET /banlists`); `null` for none, and then every deck may sit down.
    #[serde(default)]
    list_id: Option<String>,
    /// @description The format the table plays, by Scryfall's key. With a
    /// list it must be the list's, unless the list is `freeform`; absent, it
    /// is the list's. Without a list it is said on the table and checks
    /// nothing.
    #[serde(default)]
    format: Option<String>,
}

/// The host picks what a room holds a deck to.
///
/// @route POST /lobby/games/{id}/list
/// @scope public
/// @summary Pick a room's list and format
/// @operation-id `choose_room_list`
/// @description The host's. With a list, a deck it rejects cannot sit down,
/// and the player is told exactly why; with none, everything may. A change
/// takes back every chair's ready, because it is not the table they said yes
/// to. A deck already seated that the new list rejects stays in its chair and
/// is refused at its next ready. Answers the whole listing as the caller sees
/// it.
/// @tag Lobby
/// @security bearer
/// @insert `SessionRefusals`
/// @path-param id: String "The room's game id"
/// @return 400: `$ErrorBody` "A format that does not read as a key, or that is not the list's"
/// @return 403: `$ErrorBody` "The account is banned (`ban`) or must accept the terms first (`terms_required`), or the caller is not the host (`only the host arranges seats`)"
/// @return 404: `$ErrorBody` "No such room (`no such game`), or no list the host may read (`no such list`)"
/// @return 409: `$ErrorBody` "The game has started"
pub(crate) async fn choose(
    State(state): State<Shared>,
    headers: HeaderMap,
    Path(id): Path<String>,
    Json(body): Json<RoomListBody>,
) -> Result<Json<LobbyListing>, (StatusCode, Json<ErrorBody>)> {
    let account = authed(&state, &headers).await?;
    if let Some(format) = &body.format {
        banlists::check_format(format)?;
    }
    let chosen = match &body.list_id {
        Some(list_id) => Some(readable_by(&state, list_id, &account).await?),
        None => None,
    };
    let format = match (&chosen, body.format) {
        (Some(list), Some(format)) => {
            let own = &list.body.row.format;
            if own != FREEFORM && *own != format {
                return Err(err(
                    StatusCode::BAD_REQUEST,
                    "that list is for another format",
                ));
            }
            Some(format)
        }
        (Some(list), None) => {
            let own = &list.body.row.format;
            (own != FREEFORM).then(|| own.clone())
        }
        (None, format) => format,
    };
    {
        let mut lobby = state.lobby.lock();
        let game = lobby
            .games
            .get_mut(&id)
            .ok_or_else(|| err(StatusCode::NOT_FOUND, "no such game"))?;
        if !game.hosted_by(&account) {
            return Err(err(StatusCode::FORBIDDEN, "only the host arranges seats"));
        }
        if game.state != LobbyState::Waiting {
            return Err(err(StatusCode::CONFLICT, "game already started"));
        }
        let changed = game.list.as_ref().map(|list| list.id) != chosen.as_ref().map(|list| list.id)
            || game.format != format;
        game.list = chosen;
        game.format = format;
        if changed {
            for seat in &mut game.seats {
                seat.said_ready = false;
            }
        }
    }
    let held = guard(&state, &id).await?;
    if let Some(game) = state.lobby.lock().games.get_mut(&id) {
        match &held {
            Some(guard) => guard.mark_the_table(game),
            None => game.seats.iter_mut().for_each(|seat| seat.off_list = false),
        }
    }
    state.lobby_moved();
    Ok(Json(listing(&state, &account).await))
}

/// The list `id` as a room would hold it, if `account` may read it.
async fn readable_by(
    state: &Shared,
    id: &str,
    account: &str,
) -> Result<RoomList, (StatusCode, Json<ErrorBody>)> {
    banlists::keep_official(state).await;
    let id = Uuid::parse_str(id).map_err(|_| banlists::no_such_list())?;
    let account = Uuid::parse_str(account).map_err(|_| banlists::no_such_list())?;
    let list = store::readable(&state.db, id, account)
        .await
        .map_err(|e| db_down(&anyhow::anyhow!("{e}")))?
        .ok_or_else(banlists::no_such_list)?;
    let entries = store::entries(&state.db, id)
        .await
        .map_err(|e| db_down(&anyhow::anyhow!("{e}")))?;
    Ok(RoomList::of(id, banlists::body_of(&list, &entries, None)))
}

/// Whether `account` is at this table: in a chair, or its host.
fn at_the_table(game: &LobbyGame, account: &str) -> bool {
    game.hosted_by(account)
        || game
            .seats
            .iter()
            .any(|seat: &lobby::LobbySeat| seat.account_id.as_deref() == Some(account))
}

/// @route GET /lobby/games/{id}/list
/// @scope public
/// @summary The list a table holds decks to
/// @operation-id `read_room_list`
/// @description For those at the table: its list and every card on it, as
/// the room holds it. A running game answers the version it froze. Nothing
/// says whose the list is.
/// @tag Lobby
/// @security bearer
/// @insert `SessionRefusals`
/// @path-param id: String "The game's id"
/// @return 403: `$ErrorBody` "The caller has no chair there (`you are not at this table`); or the account is banned (`ban`) or must accept the terms first (`terms_required`)"
/// @return 404: `$ErrorBody` "No such game, or the table has no list"
pub(crate) async fn read(
    State(state): State<Shared>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> Result<Json<ListBody>, (StatusCode, Json<ErrorBody>)> {
    let account = authed(&state, &headers).await?;
    // A waiting room reads what a check would read now.
    guard(&state, &id).await?;
    let lobby = state.lobby.lock();
    let game = lobby
        .games
        .get(&id)
        .ok_or_else(|| err(StatusCode::NOT_FOUND, "no such game"))?;
    if !at_the_table(game, &account) {
        return Err(err(StatusCode::FORBIDDEN, "you are not at this table"));
    }
    let list = game
        .list
        .as_ref()
        .ok_or_else(|| err(StatusCode::NOT_FOUND, "this table has no list"))?;
    Ok(Json((*list.body).clone()))
}
