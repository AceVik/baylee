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
use crate::textbuf::{Dir, Step as Reach, TextBuffer};
use baylee_core::ids::PlayerId;
use baylee_engine::win::GameResult;
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

/// How the line under the form should read.
///
/// Two, and not more. A status line is either the lobby getting on with
/// something or the lobby declining to, and only the second is one a player
/// has to act on — a refusal drawn in the same grey as "signing in…" is a
/// refusal that gets read past. Progress and success share a tone
/// deliberately: three shades on one line is a legend to learn, and "deck
/// saved" needs no colour to be good news.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Tone {
    /// Something is happening, or has happened.
    #[default]
    Note,
    /// Something was refused, and nothing more will happen until the player
    /// does something about it.
    Refusal,
}

/// Which way the Tab key moves the caret between fields.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Tab {
    /// Tab: on to the next field.
    #[default]
    Next,
    /// ⇧Tab: back to the previous one.
    Back,
}

/// What a shell should ask its platform for when a [`Field`] takes the caret.
///
/// A phone raises a different keyboard for an address than for a password,
/// and a password manager has to be told which is which — but "which" is a
/// finer question than "is this masked", which is why the three masked
/// answers below are three variants and not a flag. Asked to fill a password
/// in, a manager offers the one it has; asked for a *new* one, it offers to
/// make one; asked for neither, it stays out of the way.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FieldKind {
    /// An e-mail address: the address keyboard, and the username to autofill.
    Email,
    /// A plain name.
    Name,
    /// The account's password, as it already is: masked, and the saved
    /// password to fill in.
    Password,
    /// A password being *chosen*: masked, and a manager should offer to make
    /// one rather than put the old one back.
    NewPassword,
    /// A masked field that is nobody's credential — a room's password, which
    /// a table hands out and everyone at it types. Offering the account's
    /// password here is offering it to the wrong door.
    Secret,
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
    /// The commanders, for the deck formats that name one — or two, under
    /// the partner rule.
    #[serde(default)]
    pub commanders: Vec<String>,
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
    /// Which team the chair plays for. `None` is a chair on its own side,
    /// which is every chair at a table with no teams on it.
    #[serde(default)]
    pub team: Option<u8>,
}

impl GameSeat {
    /// Whether a person could sit down here.
    #[must_use]
    pub fn open(&self) -> bool {
        self.kind == SeatKind::Human && !self.taken
    }
}

/// A table, as `GET /lobby/games` lists it.
#[expect(
    clippy::struct_excessive_bools,
    reason = "a wire DTO, for the reason GameSeat above gives: each flag is \
              one of the row's own answers, and the gateway is what decides \
              how many there are"
)]
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
    /// Whether this room is the next table of a game that finished.
    ///
    /// The one thing about such a room that cannot be worked out from the
    /// rest of the row: it looks like any other waiting table with this
    /// player's chair already in it, and the button on it has to be *play
    /// again* rather than *ready*. A chair there is reserved until its player
    /// claims it, and only `POST …/rematch` mints the seat token that claim
    /// consists of — pressing ready is answered `200` and changes nothing
    /// anyone can see.
    #[serde(default)]
    pub rematch: bool,
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
    /// Whether the game this seat belongs to runs in this process.
    ///
    /// The lobby itself never reads it: a seat is a seat, and every screen
    /// above here is written once. It is here because the *shell* installs a
    /// host for it and the two hosts are not interchangeable — an offline
    /// table has no socket to dial, no ticket a gateway would honour and
    /// nothing to reconnect to. Defaulted on the wire, so a gateway that has
    /// never heard of the field grants an ordinary networked seat.
    #[serde(default)]
    pub local: bool,
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
        /// The cards that lead it, when it is a commander deck.
        commanders: Vec<String>,
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
    /// `POST /lobby/games/{id}/seat` — the ticket for a chair this player is
    /// already sitting in.
    ///
    /// Not a join: it brings no deck and takes no chair, because the chair is
    /// already theirs. A seat token is issued once and the gateway keeps only
    /// its hash, so a client that restarts has lost it — and `join` then
    /// refuses, which is how a player came to watch their own table run
    /// without them. Its reply is a [`LobbyEvent::Seated`] like any other, so
    /// everything downstream of a seat ticket is already written.
    TakeSeat {
        /// The table this player has a chair at.
        game_id: String,
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
        /// Which team the chair plays for. Teams are numbered from 1 and
        /// `0` puts the chair back on its own side, so that "leave it alone"
        /// stays the absent field it is for everything else here.
        team: Option<u8>,
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
    /// `POST /lobby/games/{id}/rematch` — take a chair at the next table.
    ///
    /// It answers a seat ticket like a join does, so its reply is
    /// [`LobbyEvent::Seated`] and everything downstream of that is already
    /// written. `game_id` is either end of the pair — the game that just
    /// finished, which is what the player pressing it from the veil has, or
    /// the room it opened, which is what a player back in the lobby sees.
    Rematch {
        /// The finished game, or the room opened from it.
        game_id: String,
    },
}

/// The outcome of a [`LobbyRequest`], handed back by the shell.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum LobbyEvent {
    /// The account now exists. It comes with no token, so a log-in follows
    /// — unless the gateway sends confirmation mail, in which case the
    /// log-in would be refused until the link is clicked and there is
    /// nothing to do but say so.
    Registered {
        /// Whether the gateway wants the address confirmed first.
        confirmation_required: bool,
    },
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
        /// The cards that lead it, when it is a commander deck.
        commanders: Vec<String>,
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

/// Who performs the requests this lobby produces.
///
/// Three states, not a token beside a flag, which is four — and the fourth
/// is a lobby holding an account it never sends anything to. It also splits
/// the two questions the token alone was answering: "is there an account"
/// (a keymap and standing orders to attach, which the shell asks) and "is
/// there anybody at all to ask", which every intent method here asks. Online
/// those are one question. Offline they are not, and reading the first as
/// the second left the deck list as the last screen offline — the builder,
/// the room, ready and start one dead button each.
#[derive(Clone, Debug, Default)]
enum Performer {
    /// Nobody yet: the sign-in screen.
    #[default]
    Nobody,
    /// A gateway, holding the account's bearer token.
    Gateway(String),
    /// This process, with no account behind it.
    Offline,
}

/// The lobby's whole state.
///
/// One request is in flight at a time ([`Lobby::busy`]): every intent method
/// returns `None` while one is, so a double click cannot open two tables.
#[derive(Clone, Debug, Default)]
pub struct Lobby {
    screen: Screen,
    focus: Field,
    email: TextBuffer,
    display_name: TextBuffer,
    password: TextBuffer,
    room_password: TextBuffer,
    search: TextBuffer,
    performer: Performer,
    decks: Vec<DeckSummary>,
    games: Vec<GameSummary>,
    /// How many tables the current search matched, of which `games` is a page.
    total: usize,
    /// Where that page starts.
    offset: usize,
    deck: Option<usize>,
    /// The one masked field the player has asked to see, if any.
    ///
    /// One at a time, and never for long: the account's password and a room's
    /// are two different secrets and showing one is no reason to show the
    /// other. It is dropped whenever the caret leaves the field it names, so
    /// a box that was revealed is never found revealed later.
    revealed: Option<Field>,
    status: String,
    /// How that line reads. Written with it and never apart from it, which
    /// is what [`Lobby::write`] is for.
    tone: Tone,
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
    /// The tables whose chairs we have already asked to be handed back.
    ///
    /// One request per table, not one per listing: the ask fires off a
    /// listing, so without this a refusal — a table that turned `over`
    /// between two pages, a gateway older than this client that has no such
    /// route — would be re-sent every time the lobby refreshed. A list rather
    /// than one id because a player may hold chairs at several rooms, and
    /// remembering only the last would let two of them ask about each other
    /// for ever.
    reclaimed: Vec<String>,
    /// What the seat now being granted was asked for.
    asked_for: Option<GameMode>,
    /// A game the player has asked to play again, not yet asked for.
    ///
    /// The press happens on the veil over a finished game and the request has
    /// to go out after the shell has left it, because that shell tears the
    /// seat down on the way — a request sent first would have its `busy` flag
    /// and its status line cleared out from under it, and the seat screen
    /// would still be pointing at the game that ended. So the button records
    /// the intent and [`Lobby::take_rematch`] is what turns it into a
    /// request, on the other side of the unseating. Deliberately *not*
    /// cleared by [`Lobby::unseat`], which is the whole reason it exists.
    rematch_wanted: Option<String>,
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

    /// Replaces a field wholesale, caret to the end.
    ///
    /// For a shell whose platform owns the text — a browser's own input, where
    /// autofill, paste and an IME all change the value without a keystroke the
    /// client ever sees. Use [`Lobby::set_field_at`] where that platform can
    /// also say where its caret is.
    pub fn set_field(&mut self, field: Field, value: &str) {
        if self.field(field) == value {
            return;
        }
        self.buffer_mut(field).set(value, value.len(), None);
    }

    /// The same, with the caret and selection the platform reports.
    ///
    /// Unlike [`Lobby::set_field`] this writes even when the text has not
    /// changed, because moving the caret inside unchanged text is exactly what
    /// an arrow key in a browser's own `<input>` does.
    pub fn set_field_at(
        &mut self,
        field: Field,
        value: &str,
        cursor: usize,
        anchor: Option<usize>,
    ) {
        self.buffer_mut(field).set(value, cursor, anchor);
    }

    /// Moves the caret inside a field whose text a platform owns, leaving the
    /// text alone — an arrow key pressed inside a browser's own `<input>`.
    pub fn set_caret(&mut self, field: Field, cursor: usize, anchor: Option<usize>) {
        self.buffer_mut(field).place(cursor, anchor);
    }

    /// What kind of text a field holds, for a platform that can help with it.
    ///
    /// It hangs off the lobby rather than off [`Field`] because one field is
    /// two different things depending on the form it is on: the password box
    /// asks a password manager to *fill a password in* on the sign-in form
    /// and to *offer a new one* on the sign-up form, and those are opposite
    /// requests. Only the form knows which is being asked.
    #[must_use]
    pub fn field_kind(&self, field: Field) -> FieldKind {
        match field {
            Field::Email => FieldKind::Email,
            Field::DisplayName | Field::Search => FieldKind::Name,
            Field::Password if self.registering() => FieldKind::NewPassword,
            Field::Password => FieldKind::Password,
            Field::RoomPassword => FieldKind::Secret,
        }
    }

    /// The text in one field.
    #[must_use]
    pub fn field(&self, field: Field) -> &str {
        self.buffer(field).text()
    }

    /// One field's whole state — text, caret and selection — for a shell that
    /// draws the caret rather than letting a platform draw it.
    #[must_use]
    pub fn buffer(&self, field: Field) -> &TextBuffer {
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
        match &self.performer {
            Performer::Gateway(token) => Some(token),
            Performer::Nobody | Performer::Offline => None,
        }
    }

    /// Whether the requests this lobby produces are performed in this
    /// process.
    ///
    /// The screen asks because several of its controls are questions only a
    /// gateway can answer — a search over other people's tables, a room
    /// password, the address being dialled. Offline they are not merely
    /// inert, they are untrue, so the shell leaves them out.
    #[must_use]
    pub fn offline(&self) -> bool {
        matches!(self.performer, Performer::Offline)
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

    /// How that line should read — see [`Tone`].
    #[must_use]
    pub fn tone(&self) -> Tone {
        self.tone
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
    /// code beside the prose, which is a protocol change and not this. A
    /// refusal, because that is what a gateway sends prose for.
    pub fn say(&mut self, message: impl Into<String>) {
        self.write(message.into(), Tone::Refusal);
    }

    /// Says one of the client's own sentences, in the language it is set to.
    pub(crate) fn note(&mut self, phrase: Phrase) {
        let said = phrase.text(self.lang).to_string();
        self.write(said, Tone::Note);
    }

    /// The same, for a sentence that is a *no* — see [`Tone`].
    pub(crate) fn refuse(&mut self, phrase: Phrase) {
        let said = phrase.text(self.lang).to_string();
        self.write(said, Tone::Refusal);
    }

    /// The same, for the shell — which has its own sentences to say and no
    /// business knowing which language this lobby is in.
    pub fn tell(&mut self, phrase: Phrase, args: &[&str]) {
        let said = phrase.fill(self.lang, args);
        self.write(said, Tone::Note);
    }

    /// The shell's own refusals: a change it will not throw away unasked, a
    /// deck with no room left in it.
    pub fn tell_refusal(&mut self, phrase: Phrase, args: &[&str]) {
        let said = phrase.fill(self.lang, args);
        self.write(said, Tone::Refusal);
    }

    /// The one door the status line is written through, so that a line and
    /// its tone can never be set apart from each other.
    fn write(&mut self, said: String, tone: Tone) {
        self.status = said;
        self.tone = tone;
    }

    /// Takes the line away. A blank line has no tone to read.
    fn clear_status(&mut self) {
        self.write(String::new(), Tone::Note);
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
        if self.revealed != Some(field) {
            self.revealed = None;
        }
        self.focus = field;
        self.focus_epoch += 1;
    }

    /// Whether a masked field is being shown in the clear.
    #[must_use]
    pub fn showing(&self, field: Field) -> bool {
        self.revealed == Some(field)
    }

    /// Shows a masked field, or covers it again.
    ///
    /// The caret goes into it: pressing the eye beside a box is a way of
    /// saying *this box*, and a player who reveals a password does it to read
    /// what they are typing there. It moves only when it was somewhere else,
    /// though. Naming a field the caret is already in is a real event — it is
    /// what a tap on that field means, and a browser answers it by pointing
    /// its own `<input>` at the value again, which leaves the caret after the
    /// text. The eye pressed halfway through a word would then send the caret
    /// to the end there and leave it where it was on the desktop.
    pub fn toggle_reveal(&mut self, field: Field) {
        self.revealed = (self.revealed != Some(field)).then_some(field);
        if self.focus != field {
            self.focus_on(field);
        }
    }

    /// Moves the caret to the next or previous field — Tab and ⇧Tab. The
    /// display name is not in the ring when the form is logging in, because it
    /// is not shown.
    ///
    /// The table screen is its own ring of two — the search box and the room
    /// password — because those two are the fields on it, and Tab between
    /// screens would move the caret somewhere nobody can see it. A ring of two
    /// reverses to itself, which is why the direction only reads on the
    /// sign-in form.
    ///
    /// The field arrived at has all of its text selected, as tabbing into a
    /// field in a browser does: the next character typed replaces what is
    /// there, which is what a player correcting an address expects and what
    /// append-only fields could never do.
    pub fn cycle_focus(&mut self, dir: Tab) {
        if self.screen == Screen::Table {
            self.focus = match self.focus {
                Field::Search => Field::RoomPassword,
                _ => Field::Search,
            };
        } else {
            self.focus = match (self.focus, self.registering(), dir) {
                // Logging in: two fields, and a ring of two reverses to
                // itself. The display name is not drawn, so it is not in it.
                (Field::Email | Field::DisplayName, false, _) => Field::Password,
                (Field::Password, false, _) => Field::Email,
                // Signing up: three, and the direction finally reads.
                (Field::Email, true, Tab::Next) | (Field::Password, true, Tab::Back) => {
                    Field::DisplayName
                }
                (Field::DisplayName, true, Tab::Next) | (Field::Email, true, Tab::Back) => {
                    Field::Password
                }
                (Field::Password, true, Tab::Next) | (Field::DisplayName, true, Tab::Back) => {
                    Field::Email
                }
                // Neither of the table screen's two fields is on this one, but
                // the caret survives a change of screen, so Tab has to answer.
                (Field::RoomPassword, _, _) => Field::Search,
                (Field::Search, _, _) => Field::RoomPassword,
            };
        }
        // Tab always leaves the field it was in, and a reveal belongs to the
        // field it was asked for.
        self.revealed = None;
        let focus = self.focus;
        self.buffer_mut(focus).select_all();
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
        self.room_password.text()
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

    /// Types one character into the focused field, at the caret.
    pub fn type_char(&mut self, ch: char) {
        let mut buf = [0u8; 4];
        self.insert(ch.encode_utf8(&mut buf));
    }

    /// Types a run of text into the focused field — a paste, or an IME
    /// committing several characters at once. Control characters are dropped.
    pub fn insert(&mut self, text: &str) {
        let focus = self.focus;
        self.buffer_mut(focus).insert(text);
    }

    /// Selects everything in the focused field — ⌘A.
    pub fn select_all(&mut self) {
        let focus = self.focus;
        self.buffer_mut(focus).select_all();
    }

    /// Moves the caret in the focused field, extending the selection when
    /// `select` is set — the arrow keys, Home and End, with or without shift.
    pub fn move_caret(&mut self, reach: Reach, dir: Dir, select: bool) {
        let focus = self.focus;
        self.buffer_mut(focus).move_caret(reach, dir, select);
    }

    /// Deletes the character after the caret, or the selection — Delete.
    pub fn delete_forward(&mut self) {
        let focus = self.focus;
        self.buffer_mut(focus).delete_forward();
    }

    /// Deletes the character before the caret, or the selection — Backspace.
    pub fn backspace(&mut self) {
        let focus = self.focus;
        self.buffer_mut(focus).delete_back();
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
            self.clear_status();
            if registering && self.focus == Field::DisplayName {
                self.focus = Field::Password;
                self.focus_epoch += 1;
            } else if self.focus == Field::Password {
                // The caret has not moved, but the field under it has become
                // a different *kind* of field — see [`Lobby::field_kind`] —
                // and a platform holding it was told the old one. Counting
                // this as a placement is what makes it ask again.
                self.focus_epoch += 1;
            }
        } else {
            self.refuse(Phrase::NoSignUps);
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
        if self.email.text().trim().is_empty() || self.password.is_empty() {
            self.refuse(Phrase::NeedEmailAndPassword);
            return None;
        }
        if registering && self.display_name.text().trim().is_empty() {
            self.refuse(Phrase::NeedDisplayName);
            return None;
        }
        self.busy = true;
        if registering {
            self.note(Phrase::CreatingAccount);
            Some(LobbyRequest::Register {
                email: self.email.text().trim().to_string(),
                display_name: self.display_name.text().trim().to_string(),
                password: self.password.text().to_string(),
            })
        } else {
            self.note(Phrase::SigningIn);
            Some(LobbyRequest::LogIn {
                email: self.email.text().trim().to_string(),
                password: self.password.text().to_string(),
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
        if self.busy || !self.has_a_performer() || cards.is_empty() {
            return None;
        }
        self.busy = true;
        self.note(Phrase::SavingDeck);
        Some(LobbyRequest::SaveDeck {
            deck_id: None,
            name: name.to_string(),
            cards,
            sideboard: Vec::new(),
            commanders: Vec::new(),
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
        if !self.has_a_performer() {
            return None;
        }
        self.builder.start_new();
        self.screen = Screen::Build;
        self.needs_pool()
    }

    /// Opens the builder on a saved deck.
    pub fn edit_deck(&mut self, index: usize) -> Option<LobbyRequest> {
        if self.busy || !self.has_a_performer() {
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
        if self.busy || !self.has_a_performer() {
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
        if self.busy || !self.has_a_performer() {
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
            q: self.search.text().trim().to_string(),
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

    /// Asks for the ticket to a chair this player is in and holds nothing for.
    ///
    /// Three things have to be true at once and each is a different reason.
    /// The chair is **theirs** — `you`, which the gateway answers per account,
    /// so it survives a restart in a way nothing on this side does. The table
    /// is not `over`, because a finished game has no seat to hand back. And
    /// nothing is in flight for it already: `awaiting` is a ticket we hold,
    /// `reclaimed` one we have asked for.
    ///
    /// A **rematch** room is left alone. Its chair is reserved rather than
    /// taken and only `POST …/rematch` claims one, so asking here would mint
    /// a ticket for a table the player has not yet said they want to play.
    ///
    /// Only from [`Screen::Table`]: the lobby is where a player is looking
    /// for their table. Somebody in the deck builder did not ask to be moved,
    /// and the sign-in screen has no account to ask with.
    ///
    /// And **never offline**, where the whole question is unaskable: a ticket
    /// that outlives the client is what this recovers, and offline the table
    /// itself dies with the process, so there is nothing on the other side to
    /// hand a chair back. The offline performer says exactly that and refuses
    /// in words — which is a red line in the corner of a lobby that has done
    /// nothing wrong, and is how this was found.
    fn reclaim_a_seat(&mut self) -> Option<LobbyRequest> {
        if !matches!(self.screen, Screen::Table) || self.awaiting.is_some() || self.offline() {
            return None;
        }
        let game_id = self
            .games
            .iter()
            .find(|g| {
                g.seated()
                    && !g.rematch
                    && g.state != "over"
                    && !self.reclaimed.iter().any(|id| id == &g.id)
            })
            .map(|g| g.id.clone())?;
        self.reclaimed.push(game_id.clone());
        self.busy = true;
        Some(LobbyRequest::TakeSeat { game_id })
    }

    /// Reads the list again for what the search box now says.
    ///
    /// From the first page: a search is a different list, and the row that
    /// was fourth in the old one is not the fourth in this one.
    pub fn search_again(&mut self) -> Option<LobbyRequest> {
        if self.busy || !self.has_a_performer() {
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
        if self.busy || !self.has_a_performer() {
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
        if self.busy || !self.has_a_performer() {
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
        let password = self.room_password.text().to_string();
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
        let password = self.room_password.text().to_string();
        self.room_password.clear();
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

    /// Asks for a chair at the next table.
    ///
    /// `game_id` is either end of the pair — the game that just ended, or the
    /// room opened from it — because the two people pressing this are looking
    /// at different screens and the gateway answers both with the same ticket.
    ///
    /// [`Lobby::asked_for`] is cleared for the reason [`Lobby::join_seat`]
    /// clears it: what comes back is a seat at a table that may not have
    /// started, and the open-table veil is a different screen from the seat
    /// one.
    pub fn rematch(&mut self, game_id: &str) -> Option<LobbyRequest> {
        if self.busy || !self.has_a_performer() {
            return None;
        }
        self.busy = true;
        self.asked_for = None;
        self.note(Phrase::PlayingAgain);
        Some(LobbyRequest::Rematch {
            game_id: game_id.to_string(),
        })
    }

    /// Remembers that the player pressed *play again* on a finished game.
    ///
    /// Nothing goes out yet; see the field of the same name.
    pub fn want_rematch(&mut self, game_id: impl Into<String>) {
        self.rematch_wanted = Some(game_id.into());
    }

    /// The game a press is still waiting to be spent on, if there is one.
    #[must_use]
    pub fn rematch_wanted(&self) -> Option<&str> {
        self.rematch_wanted.as_deref()
    }

    /// The request that press turned into, once the shell has left the table.
    pub fn take_rematch(&mut self) -> Option<LobbyRequest> {
        let game_id = self.rematch_wanted.take()?;
        self.rematch(&game_id)
    }

    /// Says whether this player is ready to play.
    pub fn set_ready(&mut self, game_id: &str, ready: bool) -> Option<LobbyRequest> {
        if self.busy || !self.has_a_performer() {
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
        if self.busy || !self.has_a_performer() {
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
        if self.busy || !self.has_a_performer() {
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
        if self.busy || !self.has_a_performer() {
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
            team: None,
        })
    }

    /// Moves a chair onto a side. `0` puts it back on its own.
    ///
    /// The host's, not the player's: a side is the format, and one the people
    /// at the table can change is not a format. The gateway says so again.
    pub fn seat_team(&mut self, game_id: &str, seat: u32, team: u8) -> Option<LobbyRequest> {
        if self.busy || !self.has_a_performer() {
            return None;
        }
        self.busy = true;
        self.note(Phrase::ArrangingTable);
        Some(LobbyRequest::SetSeat {
            game_id: game_id.to_string(),
            seat,
            kind: None,
            ai: None,
            deck_id: None,
            team: Some(team),
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
            team: None,
        })
    }

    /// Gets up from a table, or closes it when this account is the host.
    pub fn leave_table(&mut self, game_id: &str) -> Option<LobbyRequest> {
        if self.busy || !self.has_a_performer() {
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
    /// connect to the table it was handed.
    pub fn unseat_because(&mut self, phrase: Phrase, args: &[&str]) {
        let why = phrase.fill(self.lang, args);
        self.unseat(why);
    }

    /// The same, in words somebody else chose — the gateway's, usually.
    pub fn unseat(&mut self, why: impl Into<String>) {
        self.leave_the_seat(why.into(), Tone::Refusal);
    }

    /// The same movement, because the game is simply **over**.
    ///
    /// [`Tone`] is the only channel that tells a refusal from a note, so the
    /// two ways out of a seat cannot share one: every finished duel used to
    /// put its own ending in the corner of the lobby in the red reserved for
    /// "nothing more will happen until you do something about it". Nothing
    /// had gone wrong; the player had won.
    pub fn stand_up(&mut self, phrase: Phrase, args: &[&str]) {
        let why = phrase.fill(self.lang, args);
        self.leave_the_seat(why, Tone::Note);
    }

    /// The same, saying how the game ended rather than that it ended.
    ///
    /// [`Self::stand_up`] writes `Phrase::GameEnded` — "das Spiel ist
    /// vorbei" — which is true of every game ever played here and says
    /// nothing about the one just finished (#155). The verdict is not
    /// missing information: the end screen has been drawing it all along
    /// from this same `GameResult`.
    ///
    /// So it is worded here through the same [`crate::interaction::verdict`]
    /// the sheet uses, rather than by a second reader of `GameResult` that
    /// could come to disagree with it about who won. The reason is appended
    /// where there is one; [`crate::interaction::ending_reason`] answers
    /// `None` for a draw, because the only thing it could say there is the
    /// verdict again.
    ///
    /// The two are joined with a full stop and not with the em dash this
    /// repository reaches for by habit, because one of the verdicts already
    /// contains one: `Phrase::YourTeamWon` is "Team {0} gewinnt \u{2014} deins",
    /// and a dash joiner would render "Team 2 gewinnt \u{2014} deins \u{2014} Nur noch
    /// ein Team im Spiel". A full stop cannot collide with any of the five.
    pub fn stand_up_after(&mut self, result: &GameResult, seat: PlayerId, team: Option<u8>) {
        let verdict = crate::interaction::verdict(self.lang, result, seat, team);
        let why = match crate::interaction::ending_reason(self.lang, result) {
            Some(reason) => format!("{verdict}. {reason}"),
            None => verdict,
        };
        self.leave_the_seat(why, Tone::Note);
    }

    /// What both of the above do, differing only in how it should read.
    fn leave_the_seat(&mut self, why: String, tone: Tone) {
        if matches!(self.screen, Screen::Seated(_)) {
            self.screen = Screen::Table;
        }
        self.busy = false;
        self.awaiting = None;
        self.asked_for = None;
        self.write(why, tone);
    }

    /// Goes to the table screen with no account behind it.
    ///
    /// Every screen from here on asks the same questions and reads the same
    /// answers; what differs is only who performs the requests, and that is
    /// the shell's to know. The one thing the lobby learns is that somebody
    /// will — see [`Performer`].
    ///
    /// [`Lobby::token`] stays `None` on purpose, and is a different question
    /// from this one: it is what the shell reads to decide whether to attach
    /// the account's keymap and standing orders, and there is no account here
    /// to attach.
    pub fn play_offline(&mut self) -> Option<LobbyRequest> {
        self.performer = Performer::Offline;
        self.screen = Screen::Table;
        self.note(Phrase::PlayingOffline);
        self.busy = true;
        Some(LobbyRequest::ListDecks)
    }

    /// Forgets the account. Called on a log-out button, and by the shell when
    /// the gateway rejects the token it holds.
    pub fn sign_out(&mut self) {
        self.performer = Performer::Nobody;
        self.decks.clear();
        self.games.clear();
        self.deck = None;
        self.busy = false;
        self.awaiting = None;
        self.asked_for = None;
        self.rematch_wanted = None;
        self.password.clear();
        self.revealed = None;
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
            LobbyEvent::Registered {
                confirmation_required,
            } => {
                if confirmation_required {
                    self.note(Phrase::ConfirmYourEmail);
                    self.password.clear();
                    // Back to the log-in form: the account exists, and what
                    // is left to do is click a link in a mailbox and come
                    // back.
                    self.screen = Screen::SignIn { registering: false };
                    return None;
                }
                self.note(Phrase::AccountCreated);
                self.busy = true;
                Some(LobbyRequest::LogIn {
                    email: self.email.text().trim().to_string(),
                    password: self.password.text().to_string(),
                })
            }
            LobbyEvent::LoggedIn { token } => {
                self.performer = Performer::Gateway(token);
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
                self.clear_status();
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
                commanders,
            } => {
                self.builder
                    .load(&id, &name, &cards, &sideboard, &commanders);
                self.clear_status();
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
                self.clear_status();
                self.list()
            }
            LobbyEvent::Moved => self.list(),
            LobbyEvent::Games(listing) => {
                self.games = listing.games;
                self.total = listing.total;
                self.offset = listing.offset;
                // A seat becomes usable when its table starts playing, which
                // is a different moment from being given the seat — see
                // [`LobbyEvent::Seated`] below. This is the one place that
                // knows the difference, because the listing is what carries
                // the table's state.
                let started = self.awaiting.as_ref().is_some_and(|h| {
                    self.games
                        .iter()
                        .any(|g| g.id == h.game_id && g.state == "playing")
                });
                if started && let Some(handover) = self.awaiting.take() {
                    self.note(Phrase::TakingTheSeat);
                    self.screen = Screen::Seated(handover);
                }
                // Tables close while somebody is reading page three of them.
                // A page past the end of the list is an empty screen with a
                // Back button, which is a worse answer than the first page.
                if self.offset > 0 && self.offset >= self.total {
                    self.offset = 0;
                    return self.list();
                }
                // A chair this player is *in* with no ticket for it is a
                // client that restarted: the seat token was issued once and
                // died with the process, and `join` refuses a table you are
                // already at. The listing is what notices, because it is the
                // only thing holding both halves of the question — that chair
                // is mine, and I have nothing for it.
                self.reclaim_a_seat()
            }
            LobbyEvent::Seated(handover) => {
                // **A seat is not a game**, and this used to ask only whether
                // the seat was one we had *opened*: `!= Some(Open)` put a
                // table against the house and a room somebody else is
                // hosting in the same branch and sent both straight to the
                // duel. For the house that is right — `mode: "ai"` orders an
                // engine before it answers. For a room it is not: a room
                // starts on two statements by two people (this seat's own
                // `ready`, then the host's `start`), so a player who sat down
                // at one was shown a duel with no game behind it — the sky,
                // two seat mats and nothing else — and no way back to the
                // lobby to give the `ready` it was waiting for. The owner
                // found it by sitting down at a table I was hosting.
                //
                // `join_seat` has said the right thing in a comment the whole
                // time ("sitting down does not begin the game — the seat
                // screen waits either way") and then cleared `asked_for`,
                // which is what put it in the branch that does not wait.
                match self.asked_for.take() {
                    // The house is already at the table.
                    Some(GameMode::Ai) => {
                        self.note(Phrase::TakingTheSeat);
                        self.screen = Screen::Seated(handover);
                        None
                    }
                    // Ours, and empty. Offline nobody is coming — every other
                    // chair is the house already — so what the table waits
                    // for is the player arranging it, and saying "waiting for
                    // an opponent" there would be a sentence about a person
                    // who does not exist.
                    Some(GameMode::Open) => {
                        self.note(if self.offline() {
                            Phrase::TableOpenHouse
                        } else {
                            Phrase::TableOpen
                        });
                        self.awaiting = Some(handover);
                        self.list()
                    }
                    // Somebody else's room, a rematch, or a chair handed back
                    // to a client that restarted. The first two are seats at
                    // a table that has not started, and the player's next
                    // move is in the lobby rather than at the table.
                    //
                    // The third is not: a ticket for a game that is **already
                    // playing** has a game behind it right now, so there is
                    // nothing to wait for and telling that player to go and
                    // press Bereit would be advice about a button that is no
                    // longer there. The listing we are holding is what knows
                    // the difference, and it is fresh — asking for the ticket
                    // is what it answered.
                    None => {
                        let live = self
                            .games
                            .iter()
                            .any(|g| g.id == handover.game_id && g.state == "playing");
                        if live {
                            self.note(Phrase::TakingTheSeat);
                            self.screen = Screen::Seated(handover);
                            None
                        } else {
                            self.note(Phrase::YouAreSeated);
                            self.awaiting = Some(handover);
                            self.list()
                        }
                    }
                }
            }
            LobbyEvent::Failed(why) => {
                self.write(why, Tone::Refusal);
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

    /// Whether there is anybody to perform a request — see [`Performer`].
    fn has_a_performer(&self) -> bool {
        !matches!(self.performer, Performer::Nobody)
    }

    fn buffer_mut(&mut self, field: Field) -> &mut TextBuffer {
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
        if self.busy || !self.has_a_performer() {
            return None;
        }
        let Some(deck) = self.deck.and_then(|i| self.decks.get(i)) else {
            self.refuse(Phrase::PickADeckFirst);
            return None;
        };
        Some(deck.id.clone())
    }
}

#[cfg(test)]
mod tests;
