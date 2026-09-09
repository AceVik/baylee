//! Playing with no gateway: the same lobby, answered in this process.
//!
//! Offline used to be one button that dealt a fixed duel — Allytifact against
//! the house, two seats, no choices — and everything the lobby can do was on
//! the other side of an account. That is a strange line to draw: a table of
//! AI chairs needs no server, and neither does a deck builder whose pool is
//! the compiled registry.
//!
//! So offline is **not a second lobby**. `LobbyRequest`/`LobbyEvent` is
//! already a complete, transport-free protocol between the state machine and
//! whatever performs it, and [`lobby::http`](super::http) is one performer.
//! This is the other one: it answers the same requests out of the registry
//! and a file, and every screen above it — the deck list, the builder, the
//! printing picker, the room with its chairs, the ready button, the start
//! button — is the code that was already there. The one thing the shell has
//! to know is which host to install, and [`SeatHandover::local`] says so.
//!
//! What is *not* offered is the half that needs other people: there is no
//! sign-in, no joining somebody else's table, and no handing the room on.
//! Those requests are refused in words rather than ignored, because a button
//! that does nothing is the bug this module is otherwise fixing.

#[allow(clippy::wildcard_imports)] // the lobby's own vocabulary
use super::*;
use client_core::lobby::{DeckSummary, GameListing, GameSeat, SeatHandover};

use baylee_core::preset::{AIProfile, GamePreset, SeatController};

/// The name under which the offline deck file is stored.
///
/// Read and written through [`crate::settings::store`], which is a config-dir
/// file natively and `localStorage` in a browser — the same pair the client's
/// own settings already use, so offline decks survive a restart everywhere
/// the client runs and nothing new had to be invented to hold them.
const DECKS: &str = "offline-decks.json";

/// The id of the one room offline play arranges.
///
/// A constant rather than a generated id because there is exactly one table
/// and one player: an id exists here to satisfy a protocol that was written
/// for a lobby with many.
const ROOM: &str = "offline";

/// A deck as the gateway's own store writes one.
///
/// Deliberately the gateway's field names and not a shape of this module's
/// own, so the file is liftable: a player who later makes an account has a
/// document whose deck rows go straight into `POST /decks`, rather than one
/// that has to be translated by something nobody has written yet.
/// `account_id` is carried for the same reason and is always `"offline"`.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
struct StoredDeck {
    id: String,
    #[serde(default)]
    account_id: String,
    name: String,
    cards: Vec<String>,
    #[serde(default)]
    sideboard: Vec<String>,
    #[serde(default)]
    commander: Option<String>,
    #[serde(default)]
    sleeve: Option<String>,
    #[serde(default)]
    playmat: Option<String>,
    #[serde(default)]
    updated_at: u64,
}

/// The offline deck file.
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
struct DeckFile {
    #[serde(default)]
    decks: Vec<StoredDeck>,
}

/// One chair at the offline table.
#[derive(Clone, Debug)]
struct Chair {
    kind: SeatKind,
    /// Which difficulty an AI chair plays at, by the names
    /// `AIProfile::NAMED` gives them.
    ai: String,
    /// The deck this chair brings, by the id [`Offline::decks`] lists.
    deck: Option<String>,
    ready: bool,
    /// The side this chair plays for, numbered from 1. `None` is a chair on
    /// its own side, which is what a free-for-all is.
    team: Option<u8>,
}

/// A table arranged offline, before it starts.
#[derive(Clone, Debug)]
struct Room {
    name: String,
    chairs: Vec<Chair>,
    /// Set by the start button. The lobby watches for it through the game
    /// list exactly as it watches an online room fill up, which is why
    /// starting needs no special event.
    playing: bool,
}

/// Everything offline play holds.
pub(crate) struct Offline {
    /// Decks the player built here, plus the two the acceptance file carries.
    decks: Vec<StoredDeck>,
    room: Option<Room>,
    /// The preset the start button built, waiting for the shell to install a
    /// host for it.
    ///
    /// Built at the moment of starting rather than when the host asks for it,
    /// because that is when the chairs' decks are known to resolve — a deck
    /// that no longer parses is a refusal the player should read on the room
    /// screen, not a black window after the lobby has already gone away.
    started: Option<GamePreset>,
    /// Whether a change is written back to the deck file.
    ///
    /// False only in this module's own tests. A test that wrote the store
    /// would edit the decks of whoever ran it, which is a thing a test suite
    /// may never do — and the alternative, mocking the file away, would
    /// leave the one line that actually persists untested in both directions.
    persist: bool,
}

impl Offline {
    /// Reads the deck file and stands the built-in decks beside it.
    pub(crate) fn load() -> Self {
        let mut decks = builtin_decks();
        let stored: DeckFile = crate::settings::store::read_named(DECKS)
            .and_then(|text| serde_json::from_str(&text).ok())
            .unwrap_or_default();
        decks.extend(stored.decks);
        Self {
            decks,
            room: None,
            started: None,
            persist: true,
        }
    }

    /// The preset the start button built, taken once.
    pub(crate) fn take_started(&mut self) -> Option<GamePreset> {
        self.started.take()
    }

    /// Writes back the decks that are this player's own.
    ///
    /// The built-ins are filtered out rather than written: they come from the
    /// acceptance file, they would be re-added on the next start, and a copy
    /// in the store would quietly stop tracking the file it was copied from.
    fn save(&self) {
        if !self.persist {
            return;
        }
        let file = DeckFile {
            decks: self
                .decks
                .iter()
                .filter(|d| !d.id.starts_with(BUILTIN))
                .cloned()
                .collect(),
        };
        if let Ok(text) = serde_json::to_string_pretty(&file) {
            crate::settings::store::write_named(DECKS, &text);
        }
    }

    /// Answers one request, the way the gateway would have.
    pub(crate) fn perform(&mut self, request: LobbyRequest) -> LobbyEvent {
        match request {
            LobbyRequest::ListDecks => LobbyEvent::Decks(
                self.decks
                    .iter()
                    .map(|d| DeckSummary {
                        id: d.id.clone(),
                        name: d.name.clone(),
                        cards: d.cards.len(),
                        sideboard: d.sideboard.len(),
                        commander: d.commander.clone(),
                    })
                    .collect(),
            ),
            LobbyRequest::LoadPool => LobbyEvent::Pool {
                cards: baylee_cards::pool::rows().iter().map(pool_row).collect(),
                // No catalog, so no rules text and no translated names. The
                // builder draws every card as textless and says so once,
                // which is the same thing it does against a gateway that has
                // no `DATABASE_URL`.
                has_text: false,
            },
            LobbyRequest::LoadPrintings { card } => LobbyEvent::Printings {
                card,
                printings: reference_printing(card).into_iter().collect(),
                from_catalog: false,
            },
            LobbyRequest::LoadDeck { deck_id } => self.deck(&deck_id).map_or_else(
                || LobbyEvent::Failed("no such deck".to_string()),
                |d| LobbyEvent::DeckLoaded {
                    id: d.id.clone(),
                    name: d.name.clone(),
                    cards: d.cards.clone(),
                    sideboard: d.sideboard.clone(),
                    commander: d.commander.clone(),
                },
            ),
            LobbyRequest::SaveDeck {
                deck_id,
                name,
                cards,
                sideboard,
                commander,
            } => self.save_deck(deck_id, name, cards, sideboard, commander),
            LobbyRequest::DeleteDeck { deck_id } => {
                self.decks.retain(|d| d.id != deck_id);
                self.forget_deck(&deck_id);
                self.save();
                LobbyEvent::DeckDeleted
            }
            LobbyRequest::ListGames(_) => LobbyEvent::Games(GameListing::of(
                self.room
                    .as_ref()
                    .map(|room| self.summary(room))
                    .into_iter()
                    .collect(),
            )),
            LobbyRequest::CreateGame {
                deck_id,
                mode,
                chairs,
                name,
                ..
            } => {
                let seated = self.open_room(&deck_id, chairs, name);
                // `mode: "ai"` is the one-tap duel, and online it means the
                // table is arranged *and* the engine ordered before the
                // answer comes back — the seat handed over is usable at once,
                // and the lobby takes it straight to the table. Answering it
                // with a seat and no game behind it is a button that fails
                // every time it is pressed: the shell takes the handover,
                // finds nothing to install and drops the player back.
                if mode == GameMode::Ai {
                    match self.start() {
                        LobbyEvent::Moved => seated,
                        refusal => refusal,
                    }
                } else {
                    seated
                }
            }
            LobbyRequest::SetSeat {
                seat,
                kind,
                ai,
                deck_id,
                team,
                ..
            } => self.set_seat(seat, kind, ai, deck_id, team),
            LobbyRequest::SetReady { ready, .. } => {
                if let Some(room) = self.room.as_mut()
                    && let Some(chair) = room.chairs.first_mut()
                {
                    chair.ready = ready;
                }
                LobbyEvent::Moved
            }
            LobbyRequest::StartGame { .. } => self.start(),
            LobbyRequest::LeaveGame { .. } => {
                self.room = None;
                LobbyEvent::Left
            }
            // The half that needs other people. Refused in words rather than
            // ignored: a button that answers nothing is indistinguishable
            // from a client that has stopped working.
            LobbyRequest::Register { .. }
            | LobbyRequest::LogIn { .. }
            | LobbyRequest::JoinGame { .. }
            | LobbyRequest::HandOver { .. }
            | LobbyRequest::Rematch { .. } => {
                LobbyEvent::Failed("offline play has no gateway to ask".to_string())
            }
        }
    }

    /// One stored deck by id.
    fn deck(&self, id: &str) -> Option<&StoredDeck> {
        self.decks.iter().find(|d| d.id == id)
    }

    /// Takes a deleted deck out of every chair that was bringing it.
    fn forget_deck(&mut self, id: &str) {
        if let Some(room) = self.room.as_mut() {
            for chair in &mut room.chairs {
                if chair.deck.as_deref() == Some(id) {
                    chair.deck = None;
                    chair.ready = false;
                }
            }
        }
    }

    /// Creates or overwrites one deck.
    fn save_deck(
        &mut self,
        deck_id: Option<String>,
        name: String,
        cards: Vec<String>,
        sideboard: Vec<String>,
        commander: Option<String>,
    ) -> LobbyEvent {
        // An edit of a built-in becomes a deck of the player's own. The file
        // it came from is read every launch, so writing over it would be a
        // change that lasts until the next start and then silently is not
        // there — which is worse than plainly making a copy.
        let existing = deck_id.filter(|id| !id.starts_with(BUILTIN));
        let fresh = existing.is_none();
        let id = existing.unwrap_or_else(|| format!("local-{:016x}", crate::host::fresh_seed()));
        let deck = StoredDeck {
            id: id.clone(),
            account_id: "offline".to_string(),
            name,
            cards,
            sideboard,
            commander,
            sleeve: None,
            playmat: None,
            updated_at: 0,
        };
        match self.decks.iter_mut().find(|d| d.id == id) {
            Some(slot) => *slot = deck,
            None => self.decks.push(deck),
        }
        self.save();
        LobbyEvent::DeckSaved {
            // The same shape the gateway answers with: an id for a new deck,
            // nothing for an edit, because the editor already has one.
            deck_id: fresh.then_some(id),
        }
    }

    /// Opens the offline room and sits the player in chair zero.
    fn open_room(&mut self, deck_id: &str, chairs: usize, name: String) -> LobbyEvent {
        let chairs = chairs.clamp(MIN_CHAIRS, MAX_CHAIRS);
        // Every other chair starts as an AI at the middle difficulty with a
        // deck already in it, because a room that opens with nothing in any
        // chair is a room whose start button is dead for reasons a player has
        // to go and discover. They are all changeable.
        let fallback = self.decks.first().map(|d| d.id.clone());
        let mut seats = Vec::with_capacity(chairs);
        seats.push(Chair {
            kind: SeatKind::Human,
            ai: String::new(),
            deck: Some(deck_id.to_string()),
            ready: false,
            team: None,
        });
        for at in 1..chairs {
            seats.push(Chair {
                kind: SeatKind::Ai,
                ai: "steady".to_string(),
                // Round-robin over what there is, so a four-seat table with
                // two decks is two mirrors rather than four of one.
                deck: self
                    .decks
                    .get(at % self.decks.len().max(1))
                    .map(|d| d.id.clone())
                    .or_else(|| fallback.clone()),
                ready: true,
                team: None,
            });
        }
        self.room = Some(Room {
            name,
            chairs: seats,
            playing: false,
        });
        LobbyEvent::Seated(SeatHandover {
            game_id: ROOM.to_string(),
            seat: 0,
            seat_token: String::new(),
            local: true,
        })
    }

    /// Arranges one chair.
    fn set_seat(
        &mut self,
        seat: u32,
        kind: Option<SeatKind>,
        ai: Option<String>,
        deck_id: Option<String>,
        team: Option<u8>,
    ) -> LobbyEvent {
        let Some(room) = self.room.as_mut() else {
            return LobbyEvent::Failed("no table is open".to_string());
        };
        let Some(chair) = room.chairs.get_mut(seat as usize) else {
            return LobbyEvent::Failed("no such chair".to_string());
        };
        // Chair zero is the person at the keyboard and every other chair is
        // the house. Those are one rule seen from its two sides, and both
        // sides have to be *said*: the room screen offline is the room screen
        // online, drawing a control that can ask for either. Swallowing the
        // press leaves the harder half silent — `SetReady` speaks for chair
        // zero alone, so a chair turned human is a chair that can never be
        // ready, and the start button stays grey with nothing saying why.
        if let Some(kind) = kind {
            let house = seat != 0;
            if (kind == SeatKind::Ai) != house {
                return LobbyEvent::Failed(
                    "offline, your own chair is yours and the rest are the house".to_string(),
                );
            }
        }
        if let Some(ai) = ai {
            chair.ai = ai;
        }
        if let Some(deck) = deck_id {
            chair.deck = Some(deck);
        }
        // Zero means "off a side", which is how the gateway spells it too, so
        // that "leave it alone" stays the absent field it is everywhere else.
        if let Some(team) = team {
            chair.team = (team != 0).then_some(team);
        }
        LobbyEvent::Moved
    }

    /// Builds the preset and marks the room playing.
    fn start(&mut self) -> LobbyEvent {
        let Some(room) = self.room.as_ref() else {
            return LobbyEvent::Failed("no table is open".to_string());
        };
        let mut loaded = Vec::with_capacity(room.chairs.len());
        for chair in &room.chairs {
            let Some(deck) = chair.deck.as_ref().and_then(|id| self.deck(id)) else {
                return LobbyEvent::Failed("a chair has no deck".to_string());
            };
            match baylee_cards::decks::from_lines(
                &deck.name,
                &deck.cards,
                &deck.sideboard,
                deck.commander.as_deref(),
            ) {
                Ok(loaded_deck) => loaded.push(loaded_deck),
                Err(why) => return LobbyEvent::Failed(format!("{}: {why}", deck.name)),
            }
        }
        let refs: Vec<&baylee_cards::decks::LoadedDeck> = loaded.iter().collect();
        let mut preset = baylee_cards::decks::preset_for_all(crate::host::fresh_seed(), &refs);
        for (spec, chair) in preset.seats.iter_mut().zip(&room.chairs) {
            spec.controller = match chair.kind {
                SeatKind::Human => SeatController::Open,
                SeatKind::Ai => SeatController::Ai(profile(&chair.ai)),
            };
            spec.team = chair.team;
        }
        if let Err(why) = preset.validate() {
            return LobbyEvent::Failed(why.to_string());
        }
        self.started = Some(preset);
        if let Some(room) = self.room.as_mut() {
            room.playing = true;
        }
        LobbyEvent::Moved
    }

    /// The room as the game list describes it.
    fn summary(&self, room: &Room) -> GameSummary {
        GameSummary {
            id: ROOM.to_string(),
            name: room.name.clone(),
            host: Some(YOU.to_string()),
            yours: true,
            state: if room.playing { "playing" } else { "waiting" }.to_string(),
            locked: false,
            startable: room.chairs.iter().all(|c| c.ready && c.deck.is_some()),
            rematch: false,
            seats: room
                .chairs
                .iter()
                .enumerate()
                .map(|(at, chair)| GameSeat {
                    seat: at as u32,
                    kind: chair.kind,
                    ai: (chair.kind == SeatKind::Ai).then(|| chair.ai.clone()),
                    taken: true,
                    player: Some(if at == 0 {
                        YOU.to_string()
                    } else {
                        format!("{} {at}", chair.ai)
                    }),
                    you: at == 0,
                    host: at == 0,
                    deck: chair
                        .deck
                        .as_ref()
                        .and_then(|id| self.deck(id))
                        .map(|d| d.name.clone())
                        .unwrap_or_default(),
                    ready: chair.ready,
                    team: chair.team,
                })
                .collect(),
        }
    }
}

/// What an offline chair calls the player.
const YOU: &str = "You";

/// The id prefix the acceptance file's decks are listed under.
const BUILTIN: &str = "builtin:";

/// The decks that are there before the player has built one.
///
/// The acceptance file is the only deck data every build carries — it is
/// embedded for the browser — so offline play opens with two real decks
/// rather than an empty list and a builder the player has to visit first.
fn builtin_decks() -> Vec<StoredDeck> {
    let text = crate::host::acceptance_text();
    baylee_cards::decks::acceptance_names(&text)
        .into_iter()
        .filter_map(|name| {
            // Round-tripped through rows rather than kept as a `LoadedDeck`,
            // so a built-in is the same kind of thing as a deck the player
            // built: it lists, it loads into the builder, and saving it makes
            // a copy of the player's own.
            let deck = baylee_cards::decks::load_acceptance(&text, &name).ok()?;
            Some(StoredDeck {
                id: format!("{BUILTIN}{name}"),
                account_id: "offline".to_string(),
                name,
                cards: rows_of(&deck.main),
                sideboard: rows_of(&deck.sideboard),
                commander: deck
                    .commanders
                    .first()
                    .and_then(|c| baylee_cards::by_index(c.index))
                    .map(|def| def.name().to_string()),
                sleeve: None,
                playmat: None,
                updated_at: 0,
            })
        })
        .collect()
}

/// A card list as the `"N Card Name"` rows a deck is stored as.
///
/// The loaded lists hold one entry per copy, which is what the engine wants
/// and not what a deck file says, so the copies are counted back up. Order is
/// first appearance, so a deck reads the way it was written.
fn rows_of(cards: &[baylee_cards::decks::DeckCard]) -> Vec<String> {
    let mut rows: Vec<(baylee_core::ids::CardIndex, u32)> = Vec::new();
    for card in cards {
        match rows.iter_mut().find(|(index, _)| *index == card.index) {
            Some((_, count)) => *count += 1,
            None => rows.push((card.index, 1)),
        }
    }
    rows.into_iter()
        .filter_map(|(index, count)| {
            let def = baylee_cards::by_index(index)?;
            Some(format!("{count} {}", def.name()))
        })
        .collect()
}

/// One registry row in the shape the builder reads.
///
/// Two `PoolCard`s exist — the registry's, which is what `GET /pool`
/// serializes, and the builder's, which is what it deserializes — and offline
/// there is no JSON in between to make them the same type. This is that
/// crossing, written once and checked by the compiler, which is the reason
/// not to reach for a serialize-then-parse round trip.
fn pool_row(card: &baylee_cards::pool::PoolCard) -> client_core::deckbuilder::PoolCard {
    use client_core::deckbuilder::Coverage;
    client_core::deckbuilder::PoolCard {
        index: card.index,
        name: card.name.clone(),
        english_name: card.english_name.clone(),
        mana_cost: card.mana_cost.clone(),
        cmc: card.cmc,
        colors: card.colors.clone(),
        identity: card.identity.clone(),
        type_line: card.type_line.clone(),
        kinds: card.kinds.iter().map(|k| (*k).to_string()).collect(),
        stats: card.stats.clone(),
        oracle_text: String::new(),
        coverage: match card.coverage {
            "implemented" => Coverage::Implemented,
            "partial" => Coverage::Partial,
            _ => Coverage::Unimplemented,
        },
        note: card.note.map(str::to_string),
        commander: card.commander,
        basic_land: card.basic_land,
        two_faced: card.two_faced,
        scryfall_id: card.scryfall_id.to_string(),
        oracle_id: card.oracle_id.to_string(),
        alt_names: Vec::new(),
    }
}

/// The one printing offline can name: the one codegen referenced.
///
/// A picker with a single entry is the honest answer here — the printing
/// history lives in the catalog, and offline there is no catalog. It is
/// offered rather than refused so the picker opens and shows the player what
/// their card will be drawn as.
fn reference_printing(card: u32) -> Option<client_core::deckbuilder::Printing> {
    let def = baylee_cards::all().find(|d| d.index.get() == card)?;
    Some(client_core::deckbuilder::Printing {
        scryfall_id: def.scryfall_id.to_string(),
        oracle_id: def.oracle_id.to_string(),
        lang: "en".to_string(),
        ..Default::default()
    })
}

/// One of `AIProfile::NAMED`, or the default when the name is not one.
fn profile(name: &str) -> AIProfile {
    AIProfile::named(name).unwrap_or_default()
}

/// What each chair is called at the table, in seat order.
///
/// The names are what a seat's own bar and the roster show, so "You" has to
/// be the chair the player is in and the rest have to be told apart. An AI
/// chair is named for its difficulty and its seat number, which is the only
/// thing about it a player chose.
pub(crate) fn seat_names(preset: &GamePreset) -> Vec<String> {
    preset
        .seats
        .iter()
        .enumerate()
        .map(|(at, seat)| match &seat.controller {
            SeatController::Ai(profile) => {
                let level = AIProfile::NAMED
                    .iter()
                    .find(|(_, known)| known == profile)
                    .map_or("AI", |(name, _)| *name);
                format!("{level} {at}")
            }
            _ => YOU.to_string(),
        })
        .collect()
}

#[cfg(test)]
impl Offline {
    /// An offline lobby with the built-in decks and no file behind it.
    ///
    /// Both halves matter. A test that *read* the store would answer
    /// differently on every machine, and one that wrote it would edit the
    /// decks of whoever ran it.
    pub(crate) fn without_a_file() -> Self {
        Self {
            decks: builtin_decks(),
            room: None,
            started: None,
            persist: false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// An offline lobby with the built-in decks and nothing written back.
    fn offline() -> Offline {
        Offline::without_a_file()
    }

    /// The one press that opens a table, so the room tests all start here.
    fn with_a_room(chairs: usize) -> Offline {
        let mut offline = offline();
        let deck = offline.decks[0].id.clone();
        offline.perform(LobbyRequest::CreateGame {
            deck_id: deck,
            mode: GameMode::Open,
            chairs,
            name: "Offline".to_string(),
            password: String::new(),
        });
        offline
    }

    /// The room as the lobby would read it back.
    fn listed(offline: &Offline) -> GameSummary {
        let LobbyEvent::Games(listing) =
            offline_ref(offline).perform(LobbyRequest::ListGames(GameQuery::default()))
        else {
            panic!("listing a game answers a listing")
        };
        listing.games.into_iter().next().expect("the offline room")
    }

    /// `ListGames` needs `&mut` and nothing about it changes anything, which
    /// is a shape of the protocol rather than of this module.
    fn offline_ref(offline: &Offline) -> Offline {
        Offline {
            decks: offline.decks.clone(),
            room: offline.room.clone(),
            started: None,
            persist: false,
        }
    }

    /// The builder's pool offline is the registry, whole.
    ///
    /// The number is what matters: a mapping that dropped a card, or one that
    /// only offered the finished ones, would still produce a working screen —
    /// and a deck saved against a shorter pool is a deck with rows the player
    /// cannot get back.
    #[test]
    fn the_offline_pool_is_every_card_the_engine_knows() {
        let LobbyEvent::Pool { cards, has_text } = offline().perform(LobbyRequest::LoadPool) else {
            panic!("asking for the pool answers a pool")
        };
        assert_eq!(cards.len(), baylee_cards::count());
        assert!(!has_text, "there is no catalog offline");
    }

    /// A deck saved offline comes back with the rows it was saved with.
    #[test]
    fn a_deck_saved_offline_loads_again() {
        let mut offline = offline();
        let LobbyEvent::DeckSaved { deck_id } = offline.perform(LobbyRequest::SaveDeck {
            deck_id: None,
            name: "Mine".to_string(),
            cards: vec!["4 Island".to_string()],
            sideboard: vec![],
            commander: None,
        }) else {
            panic!("saving answers a save")
        };
        let id = deck_id.expect("a new deck is given an id");
        let LobbyEvent::DeckLoaded { name, cards, .. } =
            offline.perform(LobbyRequest::LoadDeck { deck_id: id })
        else {
            panic!("loading answers a deck")
        };
        assert_eq!(name, "Mine");
        assert_eq!(cards, vec!["4 Island".to_string()]);
    }

    /// Editing a built-in makes a copy rather than overwriting it.
    ///
    /// The acceptance file is read on every launch, so a write over a
    /// built-in would last until the next start and then silently not be
    /// there — which is worse than plainly making a deck of your own.
    #[test]
    fn editing_a_builtin_deck_makes_one_of_your_own() {
        let mut offline = offline();
        let builtin = offline.decks[0].id.clone();
        let before = offline.decks.len();
        let LobbyEvent::DeckSaved { deck_id } = offline.perform(LobbyRequest::SaveDeck {
            deck_id: Some(builtin.clone()),
            name: "Allytifact, edited".to_string(),
            cards: vec!["4 Island".to_string()],
            sideboard: vec![],
            commander: None,
        }) else {
            panic!("saving answers a save")
        };
        let id = deck_id.expect("an edit of a built-in is a new deck");
        assert_ne!(id, builtin);
        assert_eq!(offline.decks.len(), before + 1);
        assert!(
            offline
                .deck(&builtin)
                .is_some_and(|d| d.name != "Allytifact, edited"),
            "the built-in is still the deck the file describes"
        );
    }

    /// Opening a room seats the player in chair zero and the house in the
    /// rest, each with a deck already in it.
    ///
    /// The decks matter as much as the chairs: a room that opens with empty
    /// seats has a start button that is dead for a reason the player has to
    /// go and find.
    #[test]
    fn a_room_opens_with_you_in_chair_zero_and_the_house_in_the_rest() {
        let offline = with_a_room(4);
        let room = listed(&offline);
        assert_eq!(room.seats.len(), 4);
        assert!(room.seats[0].you && room.seats[0].host);
        assert_eq!(room.seats[0].kind, SeatKind::Human);
        for seat in &room.seats[1..] {
            assert_eq!(seat.kind, SeatKind::Ai, "every other chair is the house");
            assert!(!seat.deck.is_empty(), "and it brought a deck");
            assert!(seat.ready, "an AI chair is ready as soon as it is arranged");
        }
    }

    /// Chair zero stays the player's, whatever the seat control says.
    ///
    /// Offline is the mode where every *other* chair is the house, so a
    /// control that could hand your own seat over would leave a table with
    /// nobody at it — and the screen that draws that control is the room
    /// screen the online lobby uses, where the same press is legal.
    #[test]
    fn your_own_chair_cannot_be_handed_to_the_house() {
        let mut offline = with_a_room(2);
        offline.perform(LobbyRequest::SetSeat {
            game_id: ROOM.to_string(),
            seat: 0,
            kind: Some(SeatKind::Ai),
            ai: None,
            deck_id: None,
            team: None,
        });
        assert_eq!(listed(&offline).seats[0].kind, SeatKind::Human);
    }

    /// The chairs' teams reach the preset the game is played from.
    ///
    /// A 2v2 arranged on the room screen and played as a free-for-all is the
    /// failure this is against, and it is invisible until somebody attacks a
    /// partner: `GamePreset::validate` accepts both, and every seat is still
    /// dealt a hand either way.
    #[test]
    fn the_teams_the_chairs_were_given_reach_the_preset() {
        let mut offline = with_a_room(4);
        for (seat, team) in [(0, 1), (1, 2), (2, 1), (3, 2)] {
            offline.perform(LobbyRequest::SetSeat {
                game_id: ROOM.to_string(),
                seat,
                kind: None,
                ai: None,
                deck_id: None,
                team: Some(team),
            });
        }
        offline.perform(LobbyRequest::SetReady {
            game_id: ROOM.to_string(),
            ready: true,
        });
        assert_eq!(
            offline.perform(LobbyRequest::StartGame {
                game_id: ROOM.to_string()
            }),
            LobbyEvent::Moved
        );
        let preset = offline.take_started().expect("the start button built one");
        assert_eq!(
            preset.seats.iter().map(|s| s.team).collect::<Vec<_>>(),
            vec![Some(1), Some(2), Some(1), Some(2)]
        );
        assert!(
            matches!(preset.seats[0].controller, SeatController::Open),
            "chair zero is the person at the keyboard"
        );
        assert!(
            preset.seats[1..]
                .iter()
                .all(|s| matches!(s.controller, SeatController::Ai(_))),
            "and the rest are the house"
        );
    }

    /// The named difficulty a chair was given is the profile it plays at.
    #[test]
    fn a_chairs_difficulty_reaches_the_preset() {
        let mut offline = with_a_room(2);
        offline.perform(LobbyRequest::SetSeat {
            game_id: ROOM.to_string(),
            seat: 1,
            kind: None,
            ai: Some("sharp".to_string()),
            deck_id: None,
            team: None,
        });
        offline.perform(LobbyRequest::SetReady {
            game_id: ROOM.to_string(),
            ready: true,
        });
        offline.perform(LobbyRequest::StartGame {
            game_id: ROOM.to_string(),
        });
        let preset = offline.take_started().expect("started");
        assert_eq!(
            preset.seats[1].controller,
            SeatController::Ai(AIProfile::SHARP)
        );
    }

    /// A chair with no deck is a refusal, not a game that starts anyway.
    ///
    /// The counter-test to the two above: they would pass just as well
    /// against a start button that ignored the chairs entirely and dealt the
    /// acceptance duel, which is what the button used to do.
    #[test]
    fn a_chair_with_no_deck_refuses_to_start() {
        let mut offline = with_a_room(2);
        let deck = offline.decks[1].id.clone();
        offline.perform(LobbyRequest::DeleteDeck { deck_id: deck });
        let answer = offline.perform(LobbyRequest::StartGame {
            game_id: ROOM.to_string(),
        });
        assert!(
            matches!(answer, LobbyEvent::Failed(_)),
            "got {answer:?} instead of a refusal"
        );
        assert!(offline.take_started().is_none(), "and nothing to install");
    }

    /// The one-tap duel hands over a seat with a game behind it.
    ///
    /// `mode: "ai"` means the table is arranged and playing by the time the
    /// answer comes back — the lobby takes that handover straight to the
    /// table rather than waiting for anybody. A room with nothing started in
    /// it answers the same way and is a button that fails every press.
    #[test]
    fn playing_the_house_starts_the_duel_it_hands_over() {
        let mut offline = offline();
        let deck = offline.decks[0].id.clone();
        let answer = offline.perform(LobbyRequest::CreateGame {
            deck_id: deck,
            mode: GameMode::Ai,
            chairs: 2,
            name: String::new(),
            password: String::new(),
        });
        assert!(matches!(answer, LobbyEvent::Seated(_)), "got {answer:?}");
        let preset = offline.take_started().expect("the duel was built");
        assert_eq!(preset.seats.len(), 2);
        assert!(matches!(preset.seats[0].controller, SeatController::Open));
        assert!(matches!(preset.seats[1].controller, SeatController::Ai(_)));
    }

    /// Opening a room does not start it, which is the counter-test.
    #[test]
    fn opening_a_room_leaves_the_start_button_to_the_player() {
        let offline = with_a_room(2);
        assert!(offline.started.is_none());
        assert_eq!(listed(&offline).state, "waiting");
    }

    /// A second person cannot be sat at an offline table.
    ///
    /// The mirror of [`your_own_chair_cannot_be_handed_to_the_house`], and
    /// the half with teeth: `SetReady` speaks for chair zero alone, so a
    /// chair turned human is one that can never be ready.
    #[test]
    fn a_second_person_cannot_be_sat_down_offline() {
        let mut offline = with_a_room(2);
        offline.perform(LobbyRequest::SetReady {
            game_id: ROOM.to_string(),
            ready: true,
        });
        let answer = offline.perform(LobbyRequest::SetSeat {
            game_id: ROOM.to_string(),
            seat: 1,
            kind: Some(SeatKind::Human),
            ai: None,
            deck_id: None,
            team: None,
        });
        assert!(matches!(answer, LobbyEvent::Failed(_)), "got {answer:?}");
        let room = listed(&offline);
        assert_eq!(room.seats[1].kind, SeatKind::Ai);
        assert!(room.startable, "and the start button is still live");
    }

    /// The requests that need somebody else are refused in words.
    #[test]
    fn the_requests_that_need_a_gateway_say_so() {
        let mut offline = offline();
        assert!(matches!(
            offline.perform(LobbyRequest::LogIn {
                email: "a@b.c".to_string(),
                password: "x".to_string(),
            }),
            LobbyEvent::Failed(_)
        ));
    }
}
