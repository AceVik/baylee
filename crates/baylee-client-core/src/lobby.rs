//! The lobby: sign in, pick a deck, take a seat.
//!
//! This is the state machine only. It owns no socket and no HTTP client: it
//! answers a click or a keystroke with a [`LobbyRequest`] the shell is
//! expected to perform, and is fed the outcome back as a [`LobbyEvent`]. That
//! keeps it testable without a renderer or a running gateway, and it is the
//! same split [`crate::interaction`] already draws between the duel's rules
//! and its pixels.
//!
//! The gateway's HTTP surface is mirrored here as plain DTOs. Only one end of
//! the wire should know the field names, and the shell that encodes the
//! request is not it.

use crate::deckbuilder::DeckBuilder;
use crate::i18n::{Lang, Phrase};
use serde::{Deserialize, Serialize};

/// Which screen the lobby is showing.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Screen {
    /// Not signed in. `registering` swaps the form between log-in and sign-up.
    SignIn {
        /// Whether the form is creating an account rather than using one.
        registering: bool,
    },
    /// Signed in: the account's decks, and the tables that are open.
    Table,
    /// Building a deck. The builder itself lives on [`Lobby::builder`]: it is
    /// far larger than the other screens' state and outlives a visit, so
    /// leaving the pool in it means coming back costs no round trip.
    Build,
    /// A seat was granted. The shell connects a host and leaves the lobby.
    Seated(SeatHandover),
}

impl Default for Screen {
    fn default() -> Self {
        Self::SignIn { registering: false }
    }
}

/// A text field on the sign-in form.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Field {
    /// The account's e-mail address, which is also its login name.
    #[default]
    Email,
    /// The name other players see. Only asked for when registering.
    DisplayName,
    /// The password. A shell is expected to draw this masked.
    Password,
    /// A room's password, on the table screen. Not part of the sign-in form
    /// at all — it shares the caret machinery because a client has one caret,
    /// not because the two fields are related.
    RoomPassword,
    /// What the table list is being searched for. Also on the table screen,
    /// and also not a sign-in field.
    Search,
}

impl Field {
    /// What kind of text this field holds, for a platform that can help with
    /// it. A phone raises a different keyboard for an address than for a
    /// password, and a password manager has to be told which is which.
    #[must_use]
    pub fn kind(self) -> FieldKind {
        match self {
            Self::Email => FieldKind::Email,
            Self::DisplayName | Self::Search => FieldKind::Name,
            Self::Password | Self::RoomPassword => FieldKind::Password,
        }
    }
}

/// What a shell should ask its platform for when a [`Field`] takes the caret.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FieldKind {
    /// An e-mail address: the address keyboard, and the username to autofill.
    Email,
    /// A plain name.
    Name,
    /// A password: masked, and the password to autofill.
    Password,
}

/// One of the account's saved decks, as `GET /decks` lists it.
#[derive(Clone, Debug, Default, PartialEq, Eq, Deserialize, Serialize)]
pub struct DeckSummary {
    /// Opaque id, the handle every other deck call takes.
    pub id: String,
    /// The name the owner gave it.
    pub name: String,
    /// Number of stored *lines* ("4 Llanowar Elves" is one), not cards.
    #[serde(default)]
    pub cards: usize,
    /// Number of stored sideboard lines.
    #[serde(default)]
    pub sideboard: usize,
    /// The commander, for the deck formats that name one.
    #[serde(default)]
    pub commander: Option<String>,
}

/// The fewest chairs a table may have.
pub const MIN_CHAIRS: usize = 2;
/// The most chairs a table may have. The gateway enforces the same bound —
/// which is `GamePreset::validate`'s — and this is what stops a client
/// offering a number that would be refused.
pub const MAX_CHAIRS: usize = 8;

/// Who a chair is meant for.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum SeatKind {
    /// A person, once one takes it.
    #[default]
    Human,
    /// The house AI, at a named difficulty.
    Ai,
}

/// A seat in a listed game.
#[expect(
    clippy::struct_excessive_bools,
    reason = "a wire DTO: the four flags are four independent answers the \
              gateway sends, and packing them into an enum here would only \
              move the decoding somewhere it cannot be checked against JSON"
)]
#[derive(Clone, Debug, Default, PartialEq, Eq, Deserialize, Serialize)]
pub struct GameSeat {
    /// Seat number at the table.
    pub seat: u32,
    /// Whether this chair is for a person or for the AI.
    #[serde(default)]
    pub kind: SeatKind,
    /// The AI's difficulty, when it is one.
    #[serde(default)]
    pub ai: Option<String>,
    /// Whether somebody is already sitting there.
    pub taken: bool,
    /// Who is sitting there, by display name. Never an account id: knowing
    /// who is at a table does not require knowing their account.
    #[serde(default)]
    pub player: Option<String>,
    /// Whether that is the player reading the list.
    #[serde(default)]
    pub you: bool,
    /// Whether the person in this chair arranges the room.
    #[serde(default)]
    pub host: bool,
    /// The deck this chair plays, as far as it is decided.
    #[serde(default)]
    pub deck: String,
    /// Whether the chair is settled enough for the game to start.
    #[serde(default)]
    pub ready: bool,
}

impl GameSeat {
    /// Whether a person could sit down here.
    #[must_use]
    pub fn open(&self) -> bool {
        self.kind == SeatKind::Human && !self.taken
    }
}

/// A table, as `GET /lobby/games` lists it.
#[derive(Clone, Debug, Default, PartialEq, Eq, Deserialize, Serialize)]
pub struct GameSummary {
    /// Opaque game id.
    pub id: String,
    /// What the host called it. May be empty.
    #[serde(default)]
    pub name: String,
    /// Who arranges it, by display name.
    #[serde(default)]
    pub host: Option<String>,
    /// Whether the player reading the list is that host.
    #[serde(default)]
    pub yours: bool,
    /// `"waiting"`, `"playing"` or `"over"`.
    pub state: String,
    /// Whether the room asks for a password before letting anyone in. Never
    /// the password itself — the listing is public to every signed-in player.
    #[serde(default)]
    pub locked: bool,
    /// Whether every chair is ready, so the host's start button does
    /// something. Visible to everyone, because a player waiting to start
    /// should be able to see who they are waiting for.
    #[serde(default)]
    pub startable: bool,
    /// Every seat at the table, taken or not.
    #[serde(default)]
    pub seats: Vec<GameSeat>,
}

impl GameSummary {
    /// Whether another player can still sit down here.
    #[must_use]
    pub fn joinable(&self) -> bool {
        self.state == "waiting" && self.seats.iter().any(GameSeat::open)
    }

    /// Which seat is this player's, if any.
    #[must_use]
    pub fn my_seat(&self) -> Option<u32> {
        self.seats.iter().find(|s| s.you).map(|s| s.seat)
    }

    /// This player's own chair, if they are at the table.
    #[must_use]
    pub fn mine(&self) -> Option<&GameSeat> {
        self.seats.iter().find(|s| s.you)
    }

    /// Whether this player has said they are ready.
    #[must_use]
    pub fn i_am_ready(&self) -> bool {
        self.mine().is_some_and(|s| s.ready)
    }

    /// Whether this player is at the table at all.
    #[must_use]
    pub fn seated(&self) -> bool {
        self.my_seat().is_some()
    }

    /// How the table reads in a list: what it is called, and how full it is.
    ///
    /// *Occupied*, not ready. It used to count ready chairs, which meant the
    /// same thing back when a chair with a deck in it was ready — and stopped
    /// meaning it the moment a player had to say so, at which point a full
    /// table read "0/4 seated".
    #[must_use]
    pub fn headline(&self) -> String {
        let taken = self
            .seats
            .iter()
            .filter(|s| s.taken || s.kind == SeatKind::Ai)
            .count();
        let name = if self.name.trim().is_empty() {
            "table".to_string()
        } else {
            self.name.clone()
        };
        format!("{name}  \u{b7}  {taken}/{} seated", self.seats.len())
    }
}

/// One page of the table list, as `GET /lobby/games` answers it.
///
/// `total` is what the search matched, not what this page holds — the two
/// differ by exactly the rows the client did not ask for, and a pager with no
/// idea how many there are is a Next button that has to be pressed to find
/// out it does nothing.
#[derive(Clone, Debug, Default, PartialEq, Eq, Deserialize, Serialize)]
pub struct GameListing {
    /// This page's tables, in the order the gateway ordered them.
    #[serde(default)]
    pub games: Vec<GameSummary>,
    /// How many tables the search matched altogether.
    #[serde(default)]
    pub total: usize,
    /// Where in that list this page starts.
    #[serde(default)]
    pub offset: usize,
    /// How many rows a page holds.
    #[serde(default)]
    pub limit: usize,
}

impl GameListing {
    /// A single, whole page of these tables — what a test means when it is
    /// not about paging.
    #[must_use]
    pub fn of(games: Vec<GameSummary>) -> Self {
        let total = games.len();
        Self {
            games,
            total,
            offset: 0,
            limit: PAGE,
        }
    }
}

/// What the client asks a page of the table list for.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct GameQuery {
    /// Free text, matched against a table's name and its host's.
    pub q: String,
    /// Where the page starts.
    pub offset: usize,
    /// How many rows it holds.
    pub limit: usize,
}

/// How many tables one page of the list holds.
///
/// Smaller than the gateway's own default, because a row here is four lines
/// of chairs rather than one line of text.
pub const PAGE: usize = 8;

/// What the gateway hands back when a seat is granted: everything a client
/// needs to open the duel socket, and nothing else.
#[derive(Clone, Debug, Default, PartialEq, Eq, Deserialize, Serialize)]
pub struct SeatHandover {
    /// The game to connect to.
    pub game_id: String,
    /// Which seat of it is ours.
    pub seat: u32,
    /// The bearer of that seat. Not the account token — losing it costs one
    /// game, not the account.
    pub seat_token: String,
}

/// What a new table is opened against.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GameMode {
    /// The house AI takes the other seat and the game starts at once.
    Ai,
    /// The table waits for a second human.
    Open,
}

impl GameMode {
    /// The string the gateway expects in `mode`.
    #[must_use]
    pub fn wire(self) -> &'static str {
        match self {
            Self::Ai => "ai",
            Self::Open => "open",
        }
    }
}

/// A call the shell should make on the lobby's behalf.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum LobbyRequest {
    /// `POST /auth/register`.
    Register {
        /// The address to register.
        email: String,
        /// The name other players will see.
        display_name: String,
        /// The password to set.
        password: String,
    },
    /// `POST /auth/login`.
    LogIn {
        /// The registered address.
        email: String,
        /// Its password.
        password: String,
    },
    /// `GET /decks`.
    ListDecks,
    /// `GET /pool` — every card a deck may be built from.
    LoadPool,
    /// `GET /printings` — every printing of one card, for the picker.
    LoadPrintings {
        /// Registry index of the card being picked for.
        card: u32,
    },
    /// `GET /decks/{id}` — one deck, with its rows, for editing.
    LoadDeck {
        /// Which deck.
        deck_id: String,
    },
    /// `POST /decks`, or `PUT /decks/{id}` when editing an existing one.
    SaveDeck {
        /// The deck to overwrite, or `None` to create one.
        deck_id: Option<String>,
        /// The deck's name.
        name: String,
        /// Its rows, each `"N Card Name"`.
        cards: Vec<String>,
        /// Its sideboard rows, in the same form.
        sideboard: Vec<String>,
        /// The card that leads it, when it is a commander deck.
        commander: Option<String>,
    },
    /// `DELETE /decks/{id}`.
    DeleteDeck {
        /// Which deck.
        deck_id: String,
    },
    /// `GET /lobby/games` — one page of it.
    ListGames(GameQuery),
    /// `POST /lobby/games`.
    CreateGame {
        /// The deck to sit down with.
        deck_id: String,
        /// Against the house, or against whoever shows up.
        mode: GameMode,
        /// How many chairs the table has. Ignored for [`GameMode::Ai`],
        /// which is a whole table decided in one request.
        chairs: usize,
        /// What to call it in the list.
        name: String,
        /// A password for the room. Empty leaves it open.
        password: String,
    },
    /// `POST /lobby/games/{id}/join`.
    JoinGame {
        /// The table to sit down at.
        game_id: String,
        /// The deck to bring.
        deck_id: String,
        /// Which chair, or the first free one.
        seat: Option<u32>,
        /// The room's password, for a locked room.
        password: String,
    },
    /// `POST /lobby/games/{id}/seats/{seat}` — arrange one chair.
    SetSeat {
        /// The table.
        game_id: String,
        /// The chair.
        seat: u32,
        /// Make it a person's or the AI's. `None` leaves it alone.
        kind: Option<SeatKind>,
        /// Which difficulty an AI chair plays at.
        ai: Option<String>,
        /// The deck the chair plays.
        deck_id: Option<String>,
    },
    /// `POST /lobby/games/{id}/ready` — say whether this player is ready.
    SetReady {
        /// The table.
        game_id: String,
        /// Ready, or taking it back.
        ready: bool,
    },
    /// `POST /lobby/games/{id}/start` — the host's go.
    StartGame {
        /// The table to start.
        game_id: String,
    },
    /// `POST /lobby/games/{id}/host` — hand the room to another chair.
    HandOver {
        /// The table.
        game_id: String,
        /// The chair that takes it over.
        seat: u32,
    },
    /// `POST /lobby/games/{id}/leave` — give up a chair, or close the room.
    LeaveGame {
        /// The table to get up from.
        game_id: String,
    },
}

/// The outcome of a [`LobbyRequest`], handed back by the shell.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum LobbyEvent {
    /// The account now exists. It comes with no token; a log-in follows.
    Registered,
    /// Signed in.
    LoggedIn {
        /// The account bearer token, for every later call.
        token: String,
    },
    /// The account's decks.
    Decks(Vec<DeckSummary>),
    /// The playable card pool.
    Pool {
        /// Every card the engine can play.
        cards: Vec<crate::deckbuilder::PoolCard>,
        /// Whether the gateway could serve rules text with them.
        has_text: bool,
    },
    /// Every printing of one card.
    Printings {
        /// Which card was asked about.
        card: u32,
        /// Its printings, newest set first.
        printings: Vec<crate::deckbuilder::Printing>,
        /// Whether a catalog answered. `false` means the single printing
        /// below is this build's own reference, not the whole history.
        from_catalog: bool,
    },
    /// One deck, with its rows, ready to edit.
    DeckLoaded {
        /// The deck's id.
        id: String,
        /// Its name.
        name: String,
        /// Its rows.
        cards: Vec<String>,
        /// Its sideboard rows.
        sideboard: Vec<String>,
        /// The card that leads it, when it is a commander deck.
        commander: Option<String>,
    },
    /// A deck was saved. `deck_id` is the id `POST /decks` hands back for a
    /// *new* deck; an edit answers `204` and carries none, having had one.
    DeckSaved {
        /// The id the gateway filed it under, when this was a new deck.
        deck_id: Option<String>,
    },
    /// A deck was deleted.
    DeckDeleted,
    /// A chair was given up, or a room closed.
    Left,
    /// The tables that are open — one page of them.
    Games(GameListing),
    /// A chair moved: somebody sat down, said they were ready, handed the
    /// room on. What changed is not carried, because the page being read is
    /// what has to be redrawn and only this client knows which page that is.
    Moved,
    /// A seat, and the ticket that proves it.
    Seated(SeatHandover),
    /// The request failed, with something worth showing a player.
    Failed(String),
}

/// The lobby's whole state.
///
/// One request is in flight at a time ([`Lobby::busy`]): every intent method
/// returns `None` while one is, so a double click cannot open two tables.
#[derive(Clone, Debug, Default)]
pub struct Lobby {
    screen: Screen,
    focus: Field,
    email: String,
    display_name: String,
    password: String,
    room_password: String,
    search: String,
    token: Option<String>,
    decks: Vec<DeckSummary>,
    games: Vec<GameSummary>,
    /// How many tables the current search matched, of which `games` is a page.
    total: usize,
    /// Where that page starts.
    offset: usize,
    deck: Option<usize>,
    status: String,
    busy: bool,
    registration_enabled: bool,
    /// The deck builder. Kept across visits so its pool is fetched once.
    builder: DeckBuilder,
    /// Whether the pool has been asked for. See [`Lobby::needs_pool`].
    pool_requested: bool,
    /// Bumped every time the caret is placed, including onto the field it is
    /// already in. A shell that has to *do* something when a field is picked —
    /// raise a keyboard, say — cannot tell that from the field alone.
    focus_epoch: u64,
    /// The language everything this lobby says is said in.
    ///
    /// Held here rather than passed to each method because the status line is
    /// written *at* the moment something happens, and the shell that renders
    /// it a frame later has no idea what was meant. A line already on screen
    /// is not re-translated when the language changes: it is one transient
    /// sentence about something that has already finished, and the next one
    /// arrives in the new language.
    lang: crate::i18n::Lang,
    /// A table of ours that is open and has nobody in the other chair yet.
    awaiting: Option<SeatHandover>,
    /// What the seat now being granted was asked for.
    asked_for: Option<GameMode>,
}

impl Lobby {
    /// An empty lobby at the sign-in screen.
    ///
    /// Registration is assumed to be open until `GET /auth/config` says
    /// otherwise: a gateway that refuses sign-ups will refuse the request too,
    /// and hiding the button on a guess is the worse failure of the two.
    #[must_use]
    pub fn new() -> Self {
        Self {
            registration_enabled: true,
            ..Self::default()
        }
    }

    /// The screen to draw.
    #[must_use]
    pub fn screen(&self) -> &Screen {
        &self.screen
    }

    /// Which sign-in field has the caret.
    #[must_use]
    pub fn focus(&self) -> Field {
        self.focus
    }

    /// How many times the caret has been placed.
    #[must_use]
    pub fn focus_epoch(&self) -> u64 {
        self.focus_epoch
    }

    /// Replaces a field wholesale.
    ///
    /// For a shell whose platform owns the text — a browser's own input, where
    /// autofill, paste and an IME all change the value without a keystroke the
    /// client ever sees.
    pub fn set_field(&mut self, field: Field, value: &str) {
        if self.field(field) == value {
            return;
        }
        *self.field_mut(field) = value.to_string();
    }

    /// The text in one field.
    #[must_use]
    pub fn field(&self, field: Field) -> &str {
        match field {
            Field::Email => &self.email,
            Field::DisplayName => &self.display_name,
            Field::Password => &self.password,
            Field::RoomPassword => &self.room_password,
            Field::Search => &self.search,
        }
    }

    /// The account bearer token, once there is one.
    #[must_use]
    pub fn token(&self) -> Option<&str> {
        self.token.as_deref()
    }

    /// Whether a request is in flight.
    #[must_use]
    pub fn busy(&self) -> bool {
        self.busy
    }

    /// The line of text under the form.
    #[must_use]
    pub fn status(&self) -> &str {
        &self.status
    }

    /// The account's decks.
    #[must_use]
    pub fn decks(&self) -> &[DeckSummary] {
        &self.decks
    }

    /// The tables that are open.
    #[must_use]
    pub fn games(&self) -> &[GameSummary] {
        &self.games
    }

    /// Which deck is picked, if any.
    #[must_use]
    pub fn selected(&self) -> Option<usize> {
        self.deck
    }

    /// Our own table that is open and still waiting for an opponent.
    ///
    /// A seat we hold but cannot use yet: the gateway builds the game's
    /// session when the second player joins, so a socket opened before that
    /// would be closed again with nothing on it.
    #[must_use]
    pub fn awaiting(&self) -> Option<&SeatHandover> {
        self.awaiting.as_ref()
    }

    /// Whether this gateway takes sign-ups.
    #[must_use]
    pub fn registration_enabled(&self) -> bool {
        self.registration_enabled
    }

    /// Records what `GET /auth/config` said, and leaves the sign-up form if
    /// it is no longer on offer.
    pub fn set_registration_enabled(&mut self, enabled: bool) {
        self.registration_enabled = enabled;
        if !enabled && self.screen == (Screen::SignIn { registering: true }) {
            self.screen = Screen::SignIn { registering: false };
        }
    }

    /// Says something to the player without touching anything else.
    ///
    /// Words the *gateway* chose come through here: it is the gateway that
    /// knows why it said no, and translating its refusals means sending a
    /// code beside the prose, which is a protocol change and not this.
    pub fn say(&mut self, message: impl Into<String>) {
        self.status = message.into();
    }

    /// Says one of the client's own sentences, in the language it is set to.
    pub(crate) fn note(&mut self, phrase: Phrase) {
        self.status = phrase.text(self.lang).to_string();
    }

    /// The same, for the shell — which has its own sentences to say and no
    /// business knowing which language this lobby is in.
    pub fn tell(&mut self, phrase: Phrase, args: &[&str]) {
        self.status = phrase.fill(self.lang, args);
    }

    /// The language this lobby speaks.
    #[must_use]
    pub fn lang(&self) -> Lang {
        self.lang
    }

    /// Changes it. The shell does this when the setting is read at startup
    /// and whenever the player picks another language.
    pub fn set_lang(&mut self, lang: Lang) {
        self.lang = lang;
    }

    /// Puts the caret in a field.
    pub fn focus_on(&mut self, field: Field) {
        if field == Field::DisplayName && !self.registering() {
            return;
        }
        self.focus = field;
        self.focus_epoch += 1;
    }

    /// Moves the caret to the next field — the Tab key. The display name is
    /// not in the ring when the form is logging in, because it is not shown.
    ///
    /// The table screen is its own ring of two — the search box and the room
    /// password — because those two are the fields on it, and Tab between
    /// screens would move the caret somewhere nobody can see it.
    pub fn cycle_focus(&mut self) {
        if self.screen == Screen::Table {
            self.focus = match self.focus {
                Field::Search => Field::RoomPassword,
                _ => Field::Search,
            };
            self.focus_epoch += 1;
            return;
        }
        self.focus = match (self.focus, self.registering()) {
            (Field::Email, true) => Field::DisplayName,
            (Field::Email, false) | (Field::DisplayName, _) => Field::Password,
            (Field::Password, _) => Field::Email,
            (Field::RoomPassword, _) => Field::Search,
            (Field::Search, _) => Field::RoomPassword,
        };
        self.focus_epoch += 1;
    }

    /// Whether the caret is in a field the screen on show actually draws.
    ///
    /// A shell asks this before typing: the caret survives a change of
    /// screen, and characters going into a field nobody can see is how a
    /// password ends up half-typed into a search box.
    #[must_use]
    pub fn typing_here(&self) -> bool {
        match self.screen {
            Screen::SignIn { registering } => match self.focus {
                Field::Email | Field::Password => true,
                Field::DisplayName => registering,
                Field::RoomPassword | Field::Search => false,
            },
            Screen::Table => matches!(self.focus, Field::RoomPassword | Field::Search),
            Screen::Build | Screen::Seated(_) => false,
        }
    }

    /// What has been typed into the room password box.
    #[must_use]
    pub fn room_password(&self) -> &str {
        &self.room_password
    }

    /// Empties the room password box.
    ///
    /// Called once a room has been opened or joined: a password left lying in
    /// a text box is the next room's password by accident.
    pub fn clear_room_password(&mut self) {
        self.room_password.clear();
        if self.focus == Field::RoomPassword {
            self.focus = Field::Email;
        }
    }

    /// Appends one typed character to the focused field.
    pub fn type_char(&mut self, ch: char) {
        if ch.is_control() {
            return;
        }
        let focus = self.focus;
        self.field_mut(focus).push(ch);
    }

    /// Deletes the last character of the focused field.
    pub fn backspace(&mut self) {
        let focus = self.focus;
        self.field_mut(focus).pop();
    }

    /// Swaps the form between log-in and sign-up.
    pub fn toggle_registering(&mut self) {
        let Screen::SignIn { registering } = self.screen else {
            return;
        };
        if registering || self.registration_enabled {
            self.screen = Screen::SignIn {
                registering: !registering,
            };
            self.status.clear();
            if registering && self.focus == Field::DisplayName {
                self.focus = Field::Password;
                self.focus_epoch += 1;
            }
        } else {
            self.note(Phrase::NoSignUps);
        }
    }

    /// Submits the sign-in form — the Enter key, or the button.
    pub fn submit(&mut self) -> Option<LobbyRequest> {
        let Screen::SignIn { registering } = self.screen else {
            return None;
        };
        if self.busy {
            return None;
        }
        if self.email.trim().is_empty() || self.password.is_empty() {
            self.note(Phrase::NeedEmailAndPassword);
            return None;
        }
        if registering && self.display_name.trim().is_empty() {
            self.note(Phrase::NeedDisplayName);
            return None;
        }
        self.busy = true;
        if registering {
            self.note(Phrase::CreatingAccount);
            Some(LobbyRequest::Register {
                email: self.email.trim().to_string(),
                display_name: self.display_name.trim().to_string(),
                password: self.password.clone(),
            })
        } else {
            self.note(Phrase::SigningIn);
            Some(LobbyRequest::LogIn {
                email: self.email.trim().to_string(),
                password: self.password.clone(),
            })
        }
    }

    /// Picks a deck to sit down with.
    pub fn select_deck(&mut self, index: usize) {
        if index < self.decks.len() {
            self.deck = Some(index);
        }
    }

    /// Saves a deck outright. `cards` are gateway rows, each `"N Card Name"`.
    ///
    /// This is the starter-deck button; the builder saves through
    /// [`Lobby::save_deck`].
    pub fn create_deck(&mut self, name: &str, cards: Vec<String>) -> Option<LobbyRequest> {
        if self.busy || self.token.is_none() || cards.is_empty() {
            return None;
        }
        self.busy = true;
        self.note(Phrase::SavingDeck);
        Some(LobbyRequest::SaveDeck {
            deck_id: None,
            name: name.to_string(),
            cards,
            sideboard: Vec::new(),
            commander: None,
        })
    }

    /// The deck builder, whatever screen is showing.
    #[must_use]
    pub fn builder(&self) -> &DeckBuilder {
        &self.builder
    }

    /// The deck builder, to type into.
    pub fn builder_mut(&mut self) -> &mut DeckBuilder {
        &mut self.builder
    }

    /// Opens the builder on a new deck, fetching the pool the first time.
    ///
    /// The pool outlives a visit deliberately: it is the same few hundred
    /// cards every time, and a player who steps out to look at the tables
    /// should not pay for it again on the way back.
    pub fn build_deck(&mut self) -> Option<LobbyRequest> {
        self.token.as_ref()?;
        self.builder.start_new();
        self.screen = Screen::Build;
        self.needs_pool()
    }

    /// Opens the builder on a saved deck.
    pub fn edit_deck(&mut self, index: usize) -> Option<LobbyRequest> {
        if self.busy || self.token.is_none() {
            return None;
        }
        let deck_id = self.decks.get(index)?.id.clone();
        self.screen = Screen::Build;
        self.busy = true;
        self.note(Phrase::OpeningDeck);
        Some(LobbyRequest::LoadDeck { deck_id })
    }

    /// Deletes a saved deck.
    pub fn delete_deck(&mut self, index: usize) -> Option<LobbyRequest> {
        if self.busy || self.token.is_none() {
            return None;
        }
        let deck_id = self.decks.get(index)?.id.clone();
        self.busy = true;
        self.note(Phrase::DeletingDeck);
        Some(LobbyRequest::DeleteDeck { deck_id })
    }

    /// Leaves the builder for the tables.
    pub fn close_builder(&mut self) -> Option<LobbyRequest> {
        self.screen = Screen::Table;
        self.refresh()
    }

    /// Saves whatever the builder holds.
    pub fn save_deck(&mut self) -> Option<LobbyRequest> {
        if self.busy || self.token.is_none() {
            return None;
        }
        let request = self.builder.save()?;
        self.busy = true;
        self.note(Phrase::SavingDeck);
        Some(request)
    }

    /// The pool request, when the builder has not got one yet.
    ///
    /// Guarded by its own flag rather than by [`Lobby::busy`]: the lobby polls
    /// the table list every couple of seconds, so `busy` is true far too often
    /// for it to mean "do not open a screen" — a player clicking *new deck*
    /// while a poll was in flight would have found nothing happening. The flag
    /// says what is actually meant, which is that the pool is fetched once.
    fn needs_pool(&mut self) -> Option<LobbyRequest> {
        if self.builder.loaded() || self.pool_requested {
            return None;
        }
        self.pool_requested = true;
        self.note(Phrase::LoadingPool);
        Some(LobbyRequest::LoadPool)
    }

    /// What the next read of the table list asks for.
    #[must_use]
    pub fn query(&self) -> GameQuery {
        GameQuery {
            q: self.search.trim().to_string(),
            offset: self.offset,
            limit: PAGE,
        }
    }

    /// Asks for the current page of the table list.
    #[expect(
        clippy::unnecessary_wraps,
        reason = "every caller is an intent or an event handler answering \
                  Option<LobbyRequest>, and unwrapping this one would put a \
                  Some( at eight call sites to save it here"
    )]
    fn list(&mut self) -> Option<LobbyRequest> {
        self.busy = true;
        Some(LobbyRequest::ListGames(self.query()))
    }

    /// Reads the list again for what the search box now says.
    ///
    /// From the first page: a search is a different list, and the row that
    /// was fourth in the old one is not the fourth in this one.
    pub fn search_again(&mut self) -> Option<LobbyRequest> {
        if self.busy || self.token.is_none() {
            return None;
        }
        self.offset = 0;
        self.list()
    }

    /// How many tables the search matched, of which [`Lobby::games`] is a page.
    #[must_use]
    pub fn total(&self) -> usize {
        self.total
    }

    /// Where the shown page starts in that list.
    #[must_use]
    pub fn offset(&self) -> usize {
        self.offset
    }

    /// Whether there is a page after this one.
    #[must_use]
    pub fn more(&self) -> bool {
        self.offset + self.games.len() < self.total
    }

    /// Steps one page forwards or back, if there is one to step onto.
    pub fn page(&mut self, forwards: bool) -> Option<LobbyRequest> {
        if self.busy || self.token.is_none() {
            return None;
        }
        let next = if forwards {
            if !self.more() {
                return None;
            }
            self.offset + PAGE
        } else {
            self.offset.checked_sub(PAGE)?
        };
        self.offset = next;
        self.list()
    }

    /// Re-reads decks and tables. Decks first: the answer chains into games.
    pub fn refresh(&mut self) -> Option<LobbyRequest> {
        if self.busy || self.token.is_none() {
            return None;
        }
        self.busy = true;
        Some(LobbyRequest::ListDecks)
    }

    /// Opens a new table with the selected deck.
    pub fn host(&mut self, mode: GameMode) -> Option<LobbyRequest> {
        self.open_room(mode, 2, String::new())
    }

    /// Opens a table of a chosen size, under a chosen name, locked with
    /// whatever is in the room password box.
    pub fn open_room(
        &mut self,
        mode: GameMode,
        chairs: usize,
        name: String,
    ) -> Option<LobbyRequest> {
        let password = self.room_password.clone();
        self.open_locked_room(mode, chairs, name, password)
    }

    /// Opens a table with an explicit password, for a caller that has one
    /// that did not come from the box.
    pub fn open_locked_room(
        &mut self,
        mode: GameMode,
        chairs: usize,
        name: String,
        password: String,
    ) -> Option<LobbyRequest> {
        let deck_id = self.picked_deck()?;
        self.busy = true;
        self.room_password.clear();
        self.asked_for = Some(mode);
        self.note(Phrase::OpeningTable);
        Some(LobbyRequest::CreateGame {
            deck_id,
            mode,
            chairs: chairs.clamp(MIN_CHAIRS, MAX_CHAIRS),
            name,
            password,
        })
    }

    /// Sits down at somebody else's table with the selected deck.
    pub fn join(&mut self, game_id: &str) -> Option<LobbyRequest> {
        self.join_seat(game_id, None)
    }

    /// Sits down in a named chair, sending whatever is in the room password
    /// box — which a room that is not locked simply ignores.
    pub fn join_seat(&mut self, game_id: &str, seat: Option<u32>) -> Option<LobbyRequest> {
        let deck_id = self.picked_deck()?;
        let password = std::mem::take(&mut self.room_password);
        self.busy = true;
        // A table only starts once every chair is ready and the host says go,
        // so sitting down does not begin the game — the seat screen waits
        // either way.
        self.asked_for = None;
        self.note(Phrase::SittingDown);
        Some(LobbyRequest::JoinGame {
            game_id: game_id.to_string(),
            deck_id,
            seat,
            password,
        })
    }

    /// Says whether this player is ready to play.
    pub fn set_ready(&mut self, game_id: &str, ready: bool) -> Option<LobbyRequest> {
        if self.busy || self.token.is_none() {
            return None;
        }
        self.busy = true;
        self.note(if ready {
            Phrase::SayingReady
        } else {
            Phrase::SayingNotReady
        });
        Some(LobbyRequest::SetReady {
            game_id: game_id.to_string(),
            ready,
        })
    }

    /// Starts the room. The host's call, and only when every chair is ready.
    ///
    /// The button is hidden for anyone else and greyed until the listing says
    /// `startable`, but nothing here is a check — the gateway refuses both
    /// cases, and this is only about not offering a player a button that does
    /// nothing.
    pub fn start_room(&mut self, game_id: &str) -> Option<LobbyRequest> {
        if self.busy || self.token.is_none() {
            return None;
        }
        self.busy = true;
        self.note(Phrase::Starting);
        Some(LobbyRequest::StartGame {
            game_id: game_id.to_string(),
        })
    }

    /// Hands the room to another chair.
    pub fn hand_over(&mut self, game_id: &str, seat: u32) -> Option<LobbyRequest> {
        if self.busy || self.token.is_none() {
            return None;
        }
        self.busy = true;
        self.note(Phrase::HandingOver);
        Some(LobbyRequest::HandOver {
            game_id: game_id.to_string(),
            seat,
        })
    }

    /// Arranges one chair of a table this account hosts.
    ///
    /// Nothing is checked here that the gateway does not check again: the
    /// client hides what a player may not do, and the gateway is what makes
    /// it true.
    pub fn set_seat(
        &mut self,
        game_id: &str,
        seat: u32,
        kind: Option<SeatKind>,
        ai: Option<String>,
    ) -> Option<LobbyRequest> {
        if self.busy || self.token.is_none() {
            return None;
        }
        self.busy = true;
        self.note(Phrase::ArrangingTable);
        Some(LobbyRequest::SetSeat {
            game_id: game_id.to_string(),
            seat,
            kind,
            ai,
            deck_id: None,
        })
    }

    /// Puts the selected deck in a chair — one's own, or an AI's.
    pub fn seat_deck(&mut self, game_id: &str, seat: u32) -> Option<LobbyRequest> {
        let deck_id = self.picked_deck()?;
        self.busy = true;
        self.note(Phrase::ArrangingTable);
        Some(LobbyRequest::SetSeat {
            game_id: game_id.to_string(),
            seat,
            kind: None,
            ai: None,
            deck_id: Some(deck_id),
        })
    }

    /// Gets up from a table, or closes it when this account is the host.
    pub fn leave_table(&mut self, game_id: &str) -> Option<LobbyRequest> {
        if self.busy || self.token.is_none() {
            return None;
        }
        self.busy = true;
        self.asked_for = None;
        self.note(Phrase::LeavingTable);
        Some(LobbyRequest::LeaveGame {
            game_id: game_id.to_string(),
        })
    }

    /// Leaves the seat screen without a seat, because the shell could not
    /// connect to the table it was handed — or because the game it opened has
    /// ended and the player is back.
    pub fn unseat_because(&mut self, phrase: Phrase, args: &[&str]) {
        let why = phrase.fill(self.lang, args);
        self.unseat(why);
    }

    /// The same, in words somebody else chose — the gateway's, usually.
    pub fn unseat(&mut self, why: impl Into<String>) {
        if matches!(self.screen, Screen::Seated(_)) {
            self.screen = Screen::Table;
        }
        self.busy = false;
        self.awaiting = None;
        self.asked_for = None;
        self.status = why.into();
    }

    /// Forgets the account. Called on a log-out button, and by the shell when
    /// the gateway rejects the token it holds.
    pub fn sign_out(&mut self) {
        self.token = None;
        self.decks.clear();
        self.games.clear();
        self.deck = None;
        self.busy = false;
        self.awaiting = None;
        self.asked_for = None;
        self.password.clear();
        self.focus = Field::Email;
        self.screen = Screen::SignIn { registering: false };
        self.note(Phrase::SignedOut);
    }

    /// Feeds back the outcome of a request, and returns the next one the
    /// lobby wants made. Ending a request always clears [`Lobby::busy`] —
    /// chaining sets it again in the same breath.
    #[expect(
        clippy::too_many_lines,
        reason = "one arm per event, read top to bottom: splitting it would \
                  hide which events chain into another request"
    )]
    pub fn apply(&mut self, event: LobbyEvent) -> Option<LobbyRequest> {
        self.busy = false;
        match event {
            // Sign-up hands back no token, so the credentials that are still
            // in the form go straight into a log-in.
            LobbyEvent::Registered => {
                self.note(Phrase::AccountCreated);
                self.busy = true;
                Some(LobbyRequest::LogIn {
                    email: self.email.trim().to_string(),
                    password: self.password.clone(),
                })
            }
            LobbyEvent::LoggedIn { token } => {
                self.token = Some(token);
                self.password.clear();
                self.screen = Screen::Table;
                self.note(Phrase::SignedIn);
                self.busy = true;
                Some(LobbyRequest::ListDecks)
            }
            LobbyEvent::Decks(decks) => {
                self.decks = decks;
                // Keep a selection that still points at a deck.
                self.deck = match self.deck {
                    Some(i) if i < self.decks.len() => Some(i),
                    _ => (!self.decks.is_empty()).then_some(0),
                };
                self.list()
            }
            LobbyEvent::Pool { cards, has_text } => {
                self.builder.set_pool(cards, has_text);
                self.status = String::new();
                None
            }
            LobbyEvent::Printings {
                card,
                printings,
                from_catalog,
            } => {
                self.builder.set_printings(card, printings, from_catalog);
                None
            }
            LobbyEvent::DeckLoaded {
                id,
                name,
                cards,
                sideboard,
                commander,
            } => {
                self.builder
                    .load(&id, &name, &cards, &sideboard, commander.as_deref());
                self.status = String::new();
                // The pool may not have arrived yet — the rows are held by
                // name until it does, which is why loading is safe either way.
                self.needs_pool()
            }
            LobbyEvent::DeckSaved { deck_id } => {
                self.note(Phrase::DeckSaved);
                self.builder.saved(deck_id.as_deref());
                self.busy = true;
                Some(LobbyRequest::ListDecks)
            }
            LobbyEvent::DeckDeleted => {
                self.note(Phrase::DeckDeleted);
                self.busy = true;
                Some(LobbyRequest::ListDecks)
            }
            LobbyEvent::Left => {
                // The seat we were holding at that table is gone with it, so
                // nothing is being waited for any more.
                self.awaiting = None;
                self.status = String::new();
                self.list()
            }
            LobbyEvent::Moved => self.list(),
            LobbyEvent::Games(listing) => {
                self.games = listing.games;
                self.total = listing.total;
                self.offset = listing.offset;
                // The table we are waiting at starts playing the moment
                // somebody joins it; that is when the seat becomes usable.
                let started = self.awaiting.as_ref().is_some_and(|h| {
                    self.games
                        .iter()
                        .any(|g| g.id == h.game_id && g.state == "playing")
                });
                if started && let Some(handover) = self.awaiting.take() {
                    self.note(Phrase::OpponentSatDown);
                    self.screen = Screen::Seated(handover);
                }
                // Tables close while somebody is reading page three of them.
                // A page past the end of the list is an empty screen with a
                // Back button, which is a worse answer than the first page.
                if self.offset > 0 && self.offset >= self.total {
                    self.offset = 0;
                    return self.list();
                }
                None
            }
            LobbyEvent::Seated(handover) => {
                if self.asked_for.take() == Some(GameMode::Open) {
                    // Ours, but not playable yet: the gateway builds the
                    // session when the second seat is filled.
                    self.note(Phrase::TableOpen);
                    self.awaiting = Some(handover);
                    return self.list();
                }
                self.note(Phrase::TakingTheSeat);
                self.screen = Screen::Seated(handover);
                None
            }
            LobbyEvent::Failed(why) => {
                self.status = why;
                // A failed fetch may have been the pool's; letting it be asked
                // for again costs one request and un-wedges the builder.
                self.pool_requested = self.builder.loaded();
                None
            }
        }
    }

    /// Whether the sign-in form is creating an account.
    fn registering(&self) -> bool {
        self.screen == (Screen::SignIn { registering: true })
    }

    fn field_mut(&mut self, field: Field) -> &mut String {
        match field {
            Field::Email => &mut self.email,
            Field::DisplayName => &mut self.display_name,
            Field::Password => &mut self.password,
            Field::RoomPassword => &mut self.room_password,
            Field::Search => &mut self.search,
        }
    }

    /// The id of the selected deck, or `None` with a nudge on the status line.
    fn picked_deck(&mut self) -> Option<String> {
        if self.busy || self.token.is_none() {
            return None;
        }
        let Some(deck) = self.deck.and_then(|i| self.decks.get(i)) else {
            self.note(Phrase::PickADeckFirst);
            return None;
        };
        Some(deck.id.clone())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::deckbuilder::{Coverage, PoolCard, Zone};

    /// A signed-in lobby with one deck, without walking the whole flow.
    fn seated_lobby() -> Lobby {
        let mut lobby = Lobby::new();
        lobby.email.push_str("a@b.c");
        lobby.password.push_str("hunter22");
        assert!(lobby.submit().is_some());
        assert_eq!(
            lobby.apply(LobbyEvent::LoggedIn {
                token: "tok".to_string()
            }),
            Some(LobbyRequest::ListDecks)
        );
        assert_eq!(
            lobby.apply(LobbyEvent::Decks(vec![DeckSummary {
                sideboard: 0,
                id: "d1".to_string(),
                name: "Allytifact".to_string(),
                cards: 60,
                commander: None,
            }])),
            Some(LobbyRequest::ListGames(lobby.query()))
        );
        lobby.apply(LobbyEvent::Games(GameListing::default()));
        lobby
    }

    #[test]
    fn a_sign_in_needs_both_fields() {
        let mut lobby = Lobby::new();
        assert_eq!(lobby.submit(), None);
        assert!(!lobby.busy());
        lobby.type_char('a');
        assert_eq!(lobby.submit(), None, "a password is still missing");
        lobby.focus_on(Field::Password);
        lobby.type_char('x');
        assert!(matches!(lobby.submit(), Some(LobbyRequest::LogIn { .. })));
    }

    #[test]
    fn registering_also_needs_a_display_name() {
        let mut lobby = Lobby::new();
        lobby.toggle_registering();
        lobby.focus_on(Field::Email);
        lobby.type_char('a');
        lobby.focus_on(Field::Password);
        lobby.type_char('x');
        assert_eq!(lobby.submit(), None);
        lobby.focus_on(Field::DisplayName);
        lobby.type_char('V');
        assert!(matches!(
            lobby.submit(),
            Some(LobbyRequest::Register { .. })
        ));
    }

    #[test]
    fn a_sign_up_chains_into_a_log_in() {
        let mut lobby = Lobby::new();
        lobby.toggle_registering();
        lobby.type_char('a');
        lobby.focus_on(Field::DisplayName);
        lobby.type_char('V');
        lobby.focus_on(Field::Password);
        lobby.type_char('x');
        lobby.submit();
        assert_eq!(
            lobby.apply(LobbyEvent::Registered),
            Some(LobbyRequest::LogIn {
                email: "a".to_string(),
                password: "x".to_string(),
            }),
            "the gateway hands out no token on sign-up"
        );
        assert!(lobby.busy(), "the chained log-in is in flight");
    }

    #[test]
    fn signing_in_asks_for_decks_and_then_for_games() {
        let lobby = seated_lobby();
        assert_eq!(*lobby.screen(), Screen::Table);
        assert_eq!(lobby.token(), Some("tok"));
        assert_eq!(lobby.selected(), Some(0), "the only deck is picked for us");
        assert!(!lobby.busy(), "the chain ended");
    }

    #[test]
    fn the_password_is_dropped_once_it_has_been_spent() {
        let lobby = seated_lobby();
        assert_eq!(lobby.field(Field::Password), "");
    }

    /// The lobby's own sentences are drawn from the phrase table, so setting
    /// the language changes what the *next* one says. Nothing re-translates a
    /// line already on screen, which is the point: a status line is a record
    /// of what just happened, not a label that keeps re-rendering.
    #[test]
    fn a_status_line_is_said_in_the_lobbys_language() {
        let mut lobby = seated_lobby();
        lobby.apply(LobbyEvent::Decks(vec![]));
        lobby.apply(LobbyEvent::Games(GameListing::default()));
        assert_eq!(lobby.host(GameMode::Ai), None);
        assert_eq!(lobby.status(), "pick a deck first");

        lobby.set_lang(Lang::De);
        assert_eq!(lobby.lang(), Lang::De);
        assert_eq!(lobby.host(GameMode::Ai), None);
        assert_eq!(lobby.status(), "wähle zuerst ein Deck");
    }

    /// The shell says things too, and has no business knowing the language.
    #[test]
    fn the_shell_says_its_own_sentences_in_that_language_too() {
        let mut lobby = seated_lobby();
        lobby.set_lang(Lang::De);
        lobby.tell(Phrase::CouldNotReachTable, &["Zeitüberschreitung"]);
        assert_eq!(lobby.status(), "Tisch nicht erreichbar: Zeitüberschreitung");
    }

    #[test]
    fn a_table_cannot_be_opened_without_a_deck() {
        let mut lobby = seated_lobby();
        lobby.apply(LobbyEvent::Decks(vec![]));
        lobby.apply(LobbyEvent::Games(GameListing::default()));
        assert_eq!(lobby.selected(), None);
        assert_eq!(lobby.host(GameMode::Ai), None);
        assert_eq!(lobby.status(), "pick a deck first");
        assert!(!lobby.busy(), "a refused intent leaves nothing in flight");
    }

    #[test]
    fn hosting_names_the_selected_deck() {
        let mut lobby = seated_lobby();
        assert_eq!(
            lobby.host(GameMode::Ai),
            Some(LobbyRequest::CreateGame {
                deck_id: "d1".to_string(),
                mode: GameMode::Ai,
                chairs: 2,
                name: String::new(),
                password: String::new(),
            })
        );
    }

    #[test]
    fn only_one_request_is_in_flight_at_a_time() {
        let mut lobby = seated_lobby();
        assert!(lobby.host(GameMode::Open).is_some());
        assert_eq!(lobby.host(GameMode::Open), None, "a second click is idle");
        assert_eq!(lobby.join("g1"), None);
        assert_eq!(lobby.refresh(), None);
    }

    #[test]
    fn a_granted_seat_ends_on_the_seated_screen() {
        let mut lobby = seated_lobby();
        lobby.host(GameMode::Ai);
        let handover = SeatHandover {
            game_id: "g1".to_string(),
            seat: 0,
            seat_token: "st".to_string(),
        };
        assert_eq!(lobby.apply(LobbyEvent::Seated(handover.clone())), None);
        assert_eq!(*lobby.screen(), Screen::Seated(handover));
    }

    #[test]
    fn a_table_we_cannot_reach_hands_the_lobby_back() {
        let mut lobby = seated_lobby();
        lobby.host(GameMode::Ai);
        lobby.apply(LobbyEvent::Seated(SeatHandover {
            game_id: "g1".to_string(),
            seat: 0,
            seat_token: "st".to_string(),
        }));
        lobby.unseat("the table did not answer");
        assert_eq!(*lobby.screen(), Screen::Table);
        assert_eq!(lobby.status(), "the table did not answer");
        assert!(
            lobby.refresh().is_some(),
            "and the lobby takes requests again"
        );
    }

    #[test]
    fn joining_brings_the_selected_deck_to_someone_elses_table() {
        let mut lobby = seated_lobby();
        assert_eq!(
            lobby.join("g7"),
            Some(LobbyRequest::JoinGame {
                game_id: "g7".to_string(),
                deck_id: "d1".to_string(),
                seat: None,
                password: String::new(),
            })
        );
    }

    /// The box is typed into once and spent once — on opening a room or on
    /// joining one, whichever comes first. A password left lying in a text
    /// box is the next room's password by accident.
    #[test]
    fn the_room_password_goes_with_the_next_table_and_is_then_forgotten() {
        let mut lobby = seated_lobby();
        lobby.focus_on(Field::RoomPassword);
        for ch in "kitchen".chars() {
            lobby.type_char(ch);
        }
        assert_eq!(lobby.room_password(), "kitchen");
        assert_eq!(
            lobby.join_seat("g7", Some(2)),
            Some(LobbyRequest::JoinGame {
                game_id: "g7".to_string(),
                deck_id: "d1".to_string(),
                seat: Some(2),
                password: "kitchen".to_string(),
            })
        );
        assert!(lobby.room_password().is_empty(), "and it is gone");

        lobby.apply(LobbyEvent::Failed("wrong password".to_string()));
        lobby.focus_on(Field::RoomPassword);
        for ch in "supper".chars() {
            lobby.type_char(ch);
        }
        assert_eq!(
            lobby.open_room(GameMode::Open, 3, "Kitchen".to_string()),
            Some(LobbyRequest::CreateGame {
                deck_id: "d1".to_string(),
                mode: GameMode::Open,
                chairs: 3,
                name: "Kitchen".to_string(),
                password: "supper".to_string(),
            })
        );
        assert!(lobby.room_password().is_empty());
    }

    /// Ready, start and handover are three different statements, and the two
    /// that are the host's are not the one that is the player's.
    #[test]
    fn a_room_is_readied_started_and_handed_on_by_name() {
        let mut lobby = seated_lobby();
        assert_eq!(
            lobby.set_ready("g7", true),
            Some(LobbyRequest::SetReady {
                game_id: "g7".to_string(),
                ready: true,
            })
        );
        lobby.apply(LobbyEvent::Games(GameListing::default()));
        assert_eq!(
            lobby.set_ready("g7", false),
            Some(LobbyRequest::SetReady {
                game_id: "g7".to_string(),
                ready: false,
            })
        );
        lobby.apply(LobbyEvent::Games(GameListing::default()));
        assert_eq!(
            lobby.start_room("g7"),
            Some(LobbyRequest::StartGame {
                game_id: "g7".to_string(),
            })
        );
        lobby.apply(LobbyEvent::Games(GameListing::default()));
        assert_eq!(
            lobby.hand_over("g7", 2),
            Some(LobbyRequest::HandOver {
                game_id: "g7".to_string(),
                seat: 2,
            })
        );
        // And all three obey the one-request-in-flight rule, so a double tap
        // on "start" cannot order two engines.
        assert_eq!(lobby.start_room("g7"), None);
    }

    /// A full table is not a ready one, and the line under a room's name has
    /// to say which it is. It counted ready chairs back when having a deck
    /// *was* being ready, and would have read "0/4 seated" at a full table
    /// the moment that stopped being true.
    #[test]
    fn a_rooms_headline_counts_who_is_sitting_down_not_who_is_ready() {
        let room = GameSummary {
            id: "g".to_string(),
            name: "Kitchen".to_string(),
            state: "waiting".to_string(),
            seats: vec![
                GameSeat {
                    seat: 0,
                    taken: true,
                    ready: false,
                    ..GameSeat::default()
                },
                GameSeat {
                    seat: 1,
                    kind: SeatKind::Ai,
                    ready: true,
                    ..GameSeat::default()
                },
                GameSeat {
                    seat: 2,
                    ..GameSeat::default()
                },
            ],
            ..GameSummary::default()
        };
        assert!(room.headline().contains("2/3"), "{}", room.headline());
        assert!(!room.i_am_ready(), "nobody here is this player");
    }

    #[test]
    fn an_open_table_is_not_sat_at_until_somebody_joins() {
        let mut lobby = seated_lobby();
        lobby.host(GameMode::Open);
        let handover = SeatHandover {
            game_id: "g1".to_string(),
            seat: 0,
            seat_token: "st".to_string(),
        };
        assert_eq!(
            lobby.apply(LobbyEvent::Seated(handover.clone())),
            Some(LobbyRequest::ListGames(lobby.query())),
            "the gateway builds the session on the second seat, not the first"
        );
        assert_eq!(*lobby.screen(), Screen::Table);
        assert_eq!(lobby.awaiting(), Some(&handover));

        // Still only us at the table.
        lobby.apply(LobbyEvent::Games(GameListing::of(vec![GameSummary {
            id: "g1".to_string(),
            state: "waiting".to_string(),
            seats: vec![
                GameSeat {
                    seat: 0,
                    taken: true,
                    ..GameSeat::default()
                },
                GameSeat {
                    seat: 1,
                    taken: false,
                    ..GameSeat::default()
                },
            ],
            ..GameSummary::default()
        }])));
        assert_eq!(*lobby.screen(), Screen::Table);

        lobby.apply(LobbyEvent::Games(GameListing::of(vec![GameSummary {
            id: "g1".to_string(),
            state: "playing".to_string(),
            seats: vec![
                GameSeat {
                    seat: 0,
                    taken: true,
                    ..GameSeat::default()
                },
                GameSeat {
                    seat: 1,
                    taken: true,
                    ..GameSeat::default()
                },
            ],
            ..GameSummary::default()
        }])));
        assert_eq!(*lobby.screen(), Screen::Seated(handover));
        assert_eq!(lobby.awaiting(), None);
    }

    #[test]
    fn a_table_against_the_house_is_playable_at_once() {
        let mut lobby = seated_lobby();
        lobby.host(GameMode::Ai);
        lobby.apply(LobbyEvent::Seated(SeatHandover {
            game_id: "g1".to_string(),
            seat: 0,
            seat_token: "st".to_string(),
        }));
        assert!(matches!(lobby.screen(), Screen::Seated(_)));
        assert_eq!(lobby.awaiting(), None);
    }

    #[test]
    fn joining_someone_elses_table_is_playable_at_once() {
        let mut lobby = seated_lobby();
        // An earlier open table of ours must not turn this into a wait.
        lobby.host(GameMode::Open);
        lobby.apply(LobbyEvent::Failed("busy".to_string()));
        lobby.join("g7");
        lobby.apply(LobbyEvent::Seated(SeatHandover {
            game_id: "g7".to_string(),
            seat: 1,
            seat_token: "st".to_string(),
        }));
        assert!(matches!(lobby.screen(), Screen::Seated(_)));
    }

    #[test]
    fn signing_out_forgets_a_table_we_were_waiting_at() {
        let mut lobby = seated_lobby();
        lobby.host(GameMode::Open);
        lobby.apply(LobbyEvent::Seated(SeatHandover {
            game_id: "g1".to_string(),
            seat: 0,
            seat_token: "st".to_string(),
        }));
        lobby.sign_out();
        assert_eq!(lobby.awaiting(), None);
    }

    #[test]
    fn placing_the_caret_is_visible_even_when_it_does_not_move() {
        let mut lobby = Lobby::new();
        let start = lobby.focus_epoch();
        lobby.focus_on(Field::Email);
        assert!(
            lobby.focus_epoch() > start,
            "tapping the field you are already in still has to raise a keyboard"
        );
        let again = lobby.focus_epoch();
        lobby.cycle_focus();
        assert!(lobby.focus_epoch() > again);
        let refused = lobby.focus_epoch();
        lobby.focus_on(Field::DisplayName);
        assert_eq!(
            lobby.focus_epoch(),
            refused,
            "a field that is not on screen is not focused, so nothing happens"
        );
    }

    #[test]
    fn a_field_can_be_replaced_wholesale() {
        let mut lobby = Lobby::new();
        lobby.set_field(Field::Email, "pasted@example.com");
        assert_eq!(lobby.field(Field::Email), "pasted@example.com");
        lobby.set_field(Field::Email, "");
        assert_eq!(lobby.field(Field::Email), "", "clearing works too");
    }

    #[test]
    fn a_field_says_what_kind_of_keyboard_it_wants() {
        assert_eq!(Field::Email.kind(), FieldKind::Email);
        assert_eq!(Field::DisplayName.kind(), FieldKind::Name);
        assert_eq!(Field::Password.kind(), FieldKind::Password);
    }

    #[test]
    fn tab_skips_the_display_name_when_logging_in() {
        let mut lobby = Lobby::new();
        assert_eq!(lobby.focus(), Field::Email);
        lobby.cycle_focus();
        assert_eq!(lobby.focus(), Field::Password);
        lobby.cycle_focus();
        assert_eq!(lobby.focus(), Field::Email);
        lobby.toggle_registering();
        lobby.cycle_focus();
        assert_eq!(lobby.focus(), Field::DisplayName);
    }

    #[test]
    fn leaving_the_sign_up_form_moves_the_caret_off_a_hidden_field() {
        let mut lobby = Lobby::new();
        lobby.toggle_registering();
        lobby.focus_on(Field::DisplayName);
        lobby.toggle_registering();
        assert_eq!(lobby.focus(), Field::Password);
    }

    #[test]
    fn a_gateway_that_takes_no_sign_ups_offers_none() {
        let mut lobby = Lobby::new();
        lobby.set_registration_enabled(false);
        lobby.toggle_registering();
        assert_eq!(lobby.screen(), &Screen::SignIn { registering: false });
        assert_eq!(lobby.status(), "this gateway is not taking new accounts");
    }

    #[test]
    fn a_form_already_registering_survives_the_config_arriving_late() {
        let mut lobby = Lobby::new();
        lobby.toggle_registering();
        lobby.set_registration_enabled(false);
        assert_eq!(
            lobby.screen(),
            &Screen::SignIn { registering: false },
            "the offer is withdrawn, not left dangling"
        );
    }

    #[test]
    fn typing_lands_in_the_focused_field_and_control_keys_do_not() {
        let mut lobby = Lobby::new();
        lobby.type_char('h');
        lobby.type_char('\n');
        lobby.type_char('i');
        assert_eq!(lobby.field(Field::Email), "hi");
        lobby.backspace();
        assert_eq!(lobby.field(Field::Email), "h");
        lobby.backspace();
        lobby.backspace();
        assert_eq!(lobby.field(Field::Email), "", "an empty field survives");
    }

    #[test]
    fn a_selection_never_points_past_the_end_of_a_refreshed_list() {
        let mut lobby = seated_lobby();
        lobby.select_deck(0);
        lobby.refresh();
        lobby.apply(LobbyEvent::Decks(vec![]));
        assert_eq!(lobby.selected(), None);
    }

    #[test]
    fn a_deck_that_does_not_exist_cannot_be_selected() {
        let mut lobby = seated_lobby();
        lobby.select_deck(9);
        assert_eq!(lobby.selected(), Some(0));
    }

    #[test]
    fn saving_a_deck_re_reads_the_list() {
        let mut lobby = seated_lobby();
        assert_eq!(lobby.create_deck("Starter", vec![]), None, "no empty decks");
        let rows = vec!["40 Island".to_string(), "20 Forest".to_string()];
        assert_eq!(
            lobby.create_deck("Starter", rows.clone()),
            Some(LobbyRequest::SaveDeck {
                deck_id: None,
                name: "Starter".to_string(),
                cards: rows,
                sideboard: vec![],
                commander: None,
            })
        );
        assert_eq!(
            lobby.apply(LobbyEvent::DeckSaved {
                deck_id: Some("d9".to_string())
            }),
            Some(LobbyRequest::ListDecks)
        );
    }

    #[test]
    fn signing_out_forgets_the_token_and_everything_it_bought() {
        let mut lobby = seated_lobby();
        lobby.sign_out();
        assert_eq!(lobby.token(), None);
        assert!(lobby.decks().is_empty());
        assert!(lobby.games().is_empty());
        assert_eq!(lobby.selected(), None);
        assert_eq!(lobby.screen(), &Screen::SignIn { registering: false });
        assert_eq!(lobby.refresh(), None, "no token, no requests");
    }

    #[test]
    fn a_failure_is_shown_and_nothing_else() {
        let mut lobby = seated_lobby();
        lobby.host(GameMode::Ai);
        assert_eq!(
            lobby.apply(LobbyEvent::Failed("no such deck".to_string())),
            None
        );
        assert_eq!(lobby.status(), "no such deck");
        assert_eq!(*lobby.screen(), Screen::Table, "we stay where we were");
        assert!(!lobby.busy(), "and the lobby is usable again");
    }

    #[test]
    fn only_a_waiting_table_with_a_free_seat_is_joinable() {
        let waiting = GameSummary {
            id: "g".to_string(),
            state: "waiting".to_string(),
            seats: vec![
                GameSeat {
                    seat: 0,
                    taken: true,
                    ..GameSeat::default()
                },
                GameSeat {
                    seat: 1,
                    taken: false,
                    ..GameSeat::default()
                },
            ],
            ..GameSummary::default()
        };
        assert!(waiting.joinable());
        let playing = GameSummary {
            state: "playing".to_string(),
            ..waiting.clone()
        };
        assert!(!playing.joinable());
        let full = GameSummary {
            seats: vec![
                GameSeat {
                    seat: 0,
                    taken: true,
                    ..GameSeat::default()
                },
                GameSeat {
                    seat: 1,
                    taken: true,
                    ..GameSeat::default()
                },
            ],
            ..waiting
        };
        assert!(!full.joinable());
    }

    #[test]
    fn the_gateways_own_json_decodes() {
        let decks: Vec<DeckSummary> = serde_json::from_str(
            r#"[{"id":"d1","name":"Allytifact","cards":60,"commander":null}]"#,
        )
        .expect("deck list");
        assert_eq!(decks[0].name, "Allytifact");
        let listing: GameListing = serde_json::from_str(
            r#"{"games":[{"id":"g1","state":"waiting","seats":[{"seat":0,"taken":true},{"seat":1,"taken":false}]}],"total":9,"offset":8,"limit":8}"#,
        )
        .expect("game list");
        assert!(listing.games[0].joinable());
        assert_eq!((listing.total, listing.offset), (9, 8));
        let seat: SeatHandover =
            serde_json::from_str(r#"{"game_id":"g1","seat":0,"seat_token":"tok"}"#)
                .expect("handover");
        assert_eq!(seat.seat_token, "tok");
    }
    /// A page of `n` nameless tables, enough to page through.
    fn a_page(n: usize) -> Vec<GameSummary> {
        (0..n)
            .map(|i| GameSummary {
                id: format!("g{i}"),
                state: "waiting".to_string(),
                ..GameSummary::default()
            })
            .collect()
    }

    /// The pager asks for what it does not have, and walks off neither end.
    #[test]
    fn the_table_list_is_paged_in_both_directions() {
        let mut lobby = seated_lobby();
        assert_eq!(lobby.page(true), None, "no second page of an empty lobby");
        assert_eq!(lobby.page(false), None, "and nothing before the first");

        lobby.apply(LobbyEvent::Games(GameListing {
            games: a_page(PAGE),
            total: PAGE + 3,
            offset: 0,
            limit: PAGE,
        }));
        assert!(lobby.more(), "three tables did not fit");
        assert_eq!(lobby.page(false), None, "the first page is the first page");
        assert_eq!(
            lobby.page(true),
            Some(LobbyRequest::ListGames(GameQuery {
                q: String::new(),
                offset: PAGE,
                limit: PAGE,
            }))
        );

        lobby.apply(LobbyEvent::Games(GameListing {
            games: a_page(3),
            total: PAGE + 3,
            offset: PAGE,
            limit: PAGE,
        }));
        assert!(!lobby.more(), "that was the end of the list");
        assert_eq!(lobby.page(true), None);
        assert_eq!(
            lobby.page(false),
            Some(LobbyRequest::ListGames(GameQuery {
                q: String::new(),
                offset: 0,
                limit: PAGE,
            }))
        );
    }

    /// A search is a different list, so it is read from the top — the row
    /// that was ninth in the old one is not the ninth in this one.
    #[test]
    fn searching_starts_the_list_again() {
        let mut lobby = seated_lobby();
        lobby.apply(LobbyEvent::Games(GameListing {
            games: a_page(PAGE),
            total: PAGE + 1,
            offset: 0,
            limit: PAGE,
        }));
        lobby.page(true);
        lobby.apply(LobbyEvent::Games(GameListing {
            games: a_page(1),
            total: PAGE + 1,
            offset: PAGE,
            limit: PAGE,
        }));
        assert_eq!(lobby.offset(), PAGE);

        lobby.set_field(Field::Search, "kitchen");
        assert_eq!(
            lobby.search_again(),
            Some(LobbyRequest::ListGames(GameQuery {
                q: "kitchen".to_string(),
                offset: 0,
                limit: PAGE,
            }))
        );
        assert_eq!(lobby.offset(), 0);
    }

    /// The last table on page two closing must not leave a player looking at
    /// an empty page two.
    #[test]
    fn a_page_that_no_longer_exists_falls_back_to_the_first() {
        let mut lobby = seated_lobby();
        lobby.apply(LobbyEvent::Games(GameListing {
            games: a_page(PAGE),
            total: PAGE + 1,
            offset: 0,
            limit: PAGE,
        }));
        lobby.page(true);
        let next = lobby.apply(LobbyEvent::Games(GameListing {
            games: Vec::new(),
            total: 4,
            offset: PAGE,
            limit: PAGE,
        }));
        assert_eq!(
            next,
            Some(LobbyRequest::ListGames(GameQuery {
                q: String::new(),
                offset: 0,
                limit: PAGE,
            }))
        );
        assert_eq!(lobby.offset(), 0);
    }

    /// Opening the builder asks for the pool once. Coming back must not ask
    /// again: it is the same few hundred cards, and the round trip would be
    /// paid on every visit.
    #[test]
    fn the_card_pool_is_fetched_once() {
        let mut lobby = seated_lobby();
        assert_eq!(lobby.build_deck(), Some(LobbyRequest::LoadPool));
        assert_eq!(lobby.screen(), &Screen::Build);
        lobby.apply(LobbyEvent::Pool {
            cards: vec![PoolCard {
                index: 1,
                english_name: "Forest".to_string(),
                name: "Forest".to_string(),
                kinds: vec!["Land".to_string()],
                type_line: "Basic Land — Forest".to_string(),
                basic_land: true,
                coverage: Coverage::Implemented,
                ..PoolCard::default()
            }],
            has_text: false,
        });
        assert!(lobby.builder().loaded());
        lobby.close_builder();
        assert_eq!(lobby.build_deck(), None, "the pool is already here");
        assert_eq!(lobby.screen(), &Screen::Build);
    }

    /// Editing a saved deck asks for its rows — `GET /decks` lists counts, not
    /// contents, so the builder cannot fill itself from the list.
    #[test]
    fn editing_a_deck_asks_for_its_rows() {
        let mut lobby = seated_lobby();
        lobby.apply(LobbyEvent::Decks(vec![DeckSummary {
            id: "deck-1".to_string(),
            name: "Burn".to_string(),
            cards: 2,
            sideboard: 0,
            commander: None,
        }]));
        lobby.apply(LobbyEvent::Games(GameListing::default()));
        assert_eq!(
            lobby.edit_deck(0),
            Some(LobbyRequest::LoadDeck {
                deck_id: "deck-1".to_string()
            })
        );
        assert_eq!(lobby.edit_deck(9), None, "no such deck");
    }

    /// A deck that arrives before the pool is held by name and resolves when
    /// the pool lands — the two answers race, and neither order may lose rows.
    #[test]
    fn a_deck_loaded_before_the_pool_still_resolves() {
        let mut lobby = seated_lobby();
        assert_eq!(
            lobby.apply(LobbyEvent::DeckLoaded {
                id: "deck-1".to_string(),
                name: "Trees".to_string(),
                cards: vec!["3 Forest".to_string()],
                sideboard: vec![],
                commander: None,
            }),
            Some(LobbyRequest::LoadPool),
            "the rows arrived first; the pool is still needed"
        );
        assert!(
            lobby.builder().missing().is_empty(),
            "nothing is missing yet — the pool has not had its say"
        );
        lobby.apply(LobbyEvent::Pool {
            cards: vec![PoolCard {
                index: 1,
                english_name: "Forest".to_string(),
                name: "Forest".to_string(),
                kinds: vec!["Land".to_string()],
                type_line: "Basic Land — Forest".to_string(),
                basic_land: true,
                ..PoolCard::default()
            }],
            has_text: false,
        });
        assert_eq!(lobby.builder().name(), "Trees");
        assert_eq!(
            lobby.builder().counts().main,
            3,
            "the held row became a real entry once the pool arrived"
        );
        assert!(lobby.builder().missing().is_empty());
    }

    /// Saving from the builder goes through the builder's own rules, so a deck
    /// the gateway would refuse never leaves the client.
    #[test]
    fn the_builder_refuses_to_save_what_the_gateway_would_reject() {
        let mut lobby = seated_lobby();
        lobby.build_deck();
        lobby.apply(LobbyEvent::Pool {
            cards: vec![PoolCard {
                index: 1,
                english_name: "Forest".to_string(),
                name: "Forest".to_string(),
                kinds: vec!["Land".to_string()],
                type_line: "Basic Land — Forest".to_string(),
                basic_land: true,
                ..PoolCard::default()
            }],
            has_text: false,
        });
        assert_eq!(lobby.save_deck(), None, "nameless and empty");
        lobby.builder_mut().set_name("Trees");
        lobby.builder_mut().add(0, Zone::Main);
        assert_eq!(
            lobby.save_deck(),
            Some(LobbyRequest::SaveDeck {
                deck_id: None,
                name: "Trees".to_string(),
                cards: vec!["1 Forest".to_string()],
                sideboard: vec![],
                commander: None,
            })
        );
        assert_eq!(
            lobby.apply(LobbyEvent::DeckSaved {
                deck_id: Some("d9".to_string())
            }),
            Some(LobbyRequest::ListDecks)
        );
        assert!(!lobby.builder().dirty(), "saving settles the deck");
        assert_eq!(
            lobby.builder().editing(),
            Some("d9"),
            "and it is now the deck being edited"
        );
        // So a second save edits that deck rather than filing a copy of it.
        // (Through the list refresh the save kicked off, which is what frees
        // the lobby to send anything at all.)
        lobby.apply(LobbyEvent::Decks(vec![]));
        lobby.apply(LobbyEvent::Games(GameListing::default()));
        lobby.builder_mut().set_name("Trees II");
        assert_eq!(
            lobby.save_deck(),
            Some(LobbyRequest::SaveDeck {
                deck_id: Some("d9".to_string()),
                name: "Trees II".to_string(),
                cards: vec!["1 Forest".to_string()],
                sideboard: vec![],
                commander: None,
            })
        );
    }

    /// Deleting a deck re-reads the list, or the one that is gone stays on
    /// screen until something else happens to refresh it.
    #[test]
    fn deleting_a_deck_re_reads_the_list() {
        let mut lobby = seated_lobby();
        lobby.apply(LobbyEvent::Decks(vec![DeckSummary {
            id: "deck-1".to_string(),
            name: "Burn".to_string(),
            cards: 2,
            sideboard: 0,
            commander: None,
        }]));
        lobby.apply(LobbyEvent::Games(GameListing::default()));
        assert_eq!(
            lobby.delete_deck(0),
            Some(LobbyRequest::DeleteDeck {
                deck_id: "deck-1".to_string()
            })
        );
        assert_eq!(
            lobby.apply(LobbyEvent::DeckDeleted),
            Some(LobbyRequest::ListDecks)
        );
    }
    /// A table with one seat left is joinable; the same table with that seat
    /// handed to the AI is not, because there is no chair for a person.
    #[test]
    fn a_table_is_joinable_only_while_a_chair_is_free_for_a_person() {
        let mut room = GameSummary {
            id: "g".to_string(),
            name: "Kitchen table".to_string(),
            host: Some("ada".to_string()),
            yours: false,
            state: "waiting".to_string(),
            seats: vec![
                GameSeat {
                    seat: 0,
                    taken: true,
                    player: Some("ada".to_string()),
                    ready: true,
                    ..GameSeat::default()
                },
                GameSeat {
                    seat: 1,
                    ..GameSeat::default()
                },
            ],
            ..GameSummary::default()
        };
        assert!(room.joinable());
        room.seats[1].kind = SeatKind::Ai;
        room.seats[1].ai = Some("sharp".to_string());
        room.seats[1].ready = true;
        assert!(!room.joinable(), "the AI has that chair");
        assert_eq!(room.headline(), "Kitchen table  \u{b7}  2/2 seated");

        // A table already playing is never joinable, free chair or not.
        room.seats[1].kind = SeatKind::Human;
        room.seats[1].taken = false;
        room.state = "playing".to_string();
        assert!(!room.joinable());
    }

    /// Which chair is mine comes from the gateway saying so, not from the
    /// client comparing account ids it should not have.
    #[test]
    fn the_gateway_says_which_chair_is_mine() {
        let room = GameSummary {
            id: "g".to_string(),
            state: "waiting".to_string(),
            seats: vec![
                GameSeat {
                    seat: 0,
                    taken: true,
                    player: Some("ada".to_string()),
                    ..GameSeat::default()
                },
                GameSeat {
                    seat: 1,
                    taken: true,
                    you: true,
                    player: Some("grace".to_string()),
                    ..GameSeat::default()
                },
            ],
            ..GameSummary::default()
        };
        assert_eq!(room.my_seat(), Some(1));
        assert!(room.seated());
        assert!(!GameSummary::default().seated());
    }

    /// A table with no name still reads as something in the list.
    #[test]
    fn a_nameless_table_still_has_a_headline() {
        let room = GameSummary {
            state: "waiting".to_string(),
            seats: vec![GameSeat::default(), GameSeat::default()],
            ..GameSummary::default()
        };
        assert_eq!(room.headline(), "table  \u{b7}  0/2 seated");
    }

    /// The size a room is opened at is clamped to what the gateway accepts,
    /// so a client can never ask for a table that would be refused.
    #[test]
    fn a_room_is_opened_at_a_size_the_gateway_allows() {
        let mut lobby = seated_lobby();
        for (asked, expected) in [(1, MIN_CHAIRS), (3, 3), (9, MAX_CHAIRS)] {
            // Each open_room marks the lobby busy; the answer clears it.
            lobby.apply(LobbyEvent::Games(GameListing::default()));
            let Some(LobbyRequest::CreateGame { chairs, .. }) =
                lobby.open_room(GameMode::Open, asked, "Kitchen".to_string())
            else {
                panic!("a picked deck opens a room");
            };
            assert_eq!(chairs, expected, "asked for {asked}");
        }
    }
}
