//! The lobby screen: sign in, pick a deck, take a seat.
//!
//! A plugin of its own, deliberately not part of [`crate::DuelPlugin`]. The
//! duel has to stay embeddable in an application that already has its own
//! front door; this is the front door the standalone client uses when nobody
//! handed it a [`SeatTicket`].
//!
//! Everything that decides lives in [`baylee_client_core::lobby`] and is
//! tested there without a window. What is left here is the part that cannot
//! be: HTTP, a keyboard, and a pile of UI nodes.
//!
//! ```text
//!   Lobby  --LobbyRequest-->  ehttp  -->  gateway
//!     ^                                      |
//!     +---------- LobbyEvent ----- Mailbox <-+
//! ```
//!
//! The one thing the lobby does that the duel cannot undo: on a granted seat
//! it builds a [`NetworkHost`], installs it, and pushes [`DuelCommand::Open`].
//! From that moment the renderer above it cannot tell this game from one a
//! ticket handed it on the command line.

use std::sync::{Arc, Mutex};

use crate::cardmat::{CardUiMaterial, UiCardMaterials, UiCards};
use baylee_client_core::deckbuilder::{BuildField, Zone};
use baylee_client_core::images::FinishTreatment;
use baylee_client_core::lobby::{
    Field, GameMode, GameSummary, Lobby, LobbyEvent, LobbyRequest, MAX_CHAIRS, MIN_CHAIRS, Screen,
    SeatKind,
};
use baylee_core::ids::PlayerId;
use baylee_core::preset::Finish;
use bevy::input::keyboard::{Key, KeyboardInput};
use bevy::input::mouse::MouseScrollUnit;
use bevy::prelude::*;

use crate::hud::{UiFonts, btn_radius, palette, soft_shadow, tf};
use crate::net::{NetworkHost, SeatTicket};
use crate::softkeys::{SoftKey, SoftKeyboard};
use crate::{DuelCommand, DuelPhase, InstalledHost};

/// The ground the lobby sits on — dark enough that the felt never flashes
/// through on the way into a duel.
const BACKDROP: Color = Color::srgb(0.04, 0.05, 0.06);

/// The starter deck's name, and the section of the acceptance deck file it is
/// copied from. There is no deck builder yet; without this button a fresh
/// account cannot sit down anywhere.
const STARTER: &str = "Allytifact";

/// The lobby, as a plugin.
///
/// Adds nothing to [`DuelPhase::Playing`]: every system here is gated on the
/// duel being closed, or on it having finished.
#[derive(Default)]
pub struct LobbyPlugin;

impl Plugin for LobbyPlugin {
    fn build(&self, app: &mut App) {
        // The keymap is the account's, and the account is signed into here —
        // shared with the duel, whichever of the two got there first.
        crate::prefs::install(app);
        app.init_resource::<Mailbox>()
            .init_resource::<SoftKeyboard>()
            .init_resource::<Scrolled>()
            .insert_resource(LobbyState::new())
            .add_systems(Startup, ask_about_registration)
            .add_systems(
                Update,
                (
                    poll, watch, softkeys, keyboard, clicks, scrolls, hovers, ui, preview,
                )
                    .chain()
                    .run_if(in_state(DuelPhase::Closed)),
            )
            .add_systems(Update, leave_clicks.run_if(in_state(DuelPhase::Finished)))
            .add_systems(OnEnter(DuelPhase::Closed), (came_back, spawn_camera))
            .init_resource::<Hovered>()
            .add_message::<Pointer<Over>>()
            .add_message::<Pointer<Out>>()
            .add_systems(OnExit(DuelPhase::Closed), (teardown, despawn_preview))
            .add_systems(OnEnter(DuelPhase::Finished), spawn_leave_button)
            .add_systems(OnExit(DuelPhase::Finished), despawn_leave_button);
    }
}

// ------------------------------------------------------------- resources

/// The lobby's state, plus the gateway it is talking to.
#[derive(Resource)]
pub struct LobbyState {
    /// The renderer-free state machine.
    pub lobby: Lobby,
    /// Gateway base URL, resolved once at startup.
    pub gateway: String,
    /// The language the card pool is asked for, from the same setting the
    /// duel reads card text in — a builder in English over a table in German
    /// would be the same card under two names.
    pub lang: String,
    /// Whether a host is already installed for the seat the lobby holds.
    ///
    /// A request still in flight when the seat was granted answers *after*
    /// the connection is made, and without this its reply would run the
    /// same code again — a second socket to the same table, or, when that
    /// second dial fails, a player knocked out of the game they just joined.
    connected: bool,
    /// Whether the back button has already been pressed once on a deck with
    /// unsaved changes. Leaving is one tap away from the busiest corner of
    /// the screen, and a deck is half an hour of work.
    pub(crate) confirm_leave: bool,
    /// Whether a phone is showing the filter chips. They are three wrapped
    /// rows, which on a phone is most of the screen — the list they filter
    /// would be four rows tall underneath them.
    pub(crate) filters_open: bool,
    /// Which half of the builder a phone is showing. Purely a matter of how
    /// much room there is, so it lives here and not in the state machine:
    /// every wider frame shows both halves and never reads it.
    pub(crate) pane: Pane,
    /// Whether the settings screen is up, and what it is waiting for.
    settings: SettingsPane,
}

/// The settings overlay's state.
///
/// Not a `Screen`: the lobby's state machine is about what the *gateway* has
/// told us, and settings are neither asked for nor answered by it. This draws
/// over whatever the lobby was showing and puts it back untouched.
///
/// One enum rather than a flag plus an `Option`, because "waiting for a key
/// while closed" is not a state — and a pair of fields would let it happen,
/// with the symptom that the next key pressed anywhere rebinds something.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
enum SettingsPane {
    /// Not showing.
    #[default]
    Closed,
    /// Showing.
    Open,
    /// Showing, with one action's row listening for the next keystroke.
    Rebinding(baylee_client_core::prefs::Action),
}

impl SettingsPane {
    /// Whether the screen is up at all.
    const fn is_open(self) -> bool {
        !matches!(self, Self::Closed)
    }

    /// The action waiting for a key, if any.
    const fn capturing(self) -> Option<baylee_client_core::prefs::Action> {
        match self {
            Self::Rebinding(action) => Some(action),
            _ => None,
        }
    }
}

/// The half of the deck builder a narrow screen is showing.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub(crate) enum Pane {
    /// The searchable pool.
    #[default]
    Cards,
    /// The deck being built.
    Deck,
}

impl LobbyState {
    /// A signed-out lobby pointed at the configured gateway.
    #[must_use]
    pub fn new() -> Self {
        Self {
            lobby: Lobby::new(),
            gateway: crate::settings::gateway_url(),
            lang: crate::settings::ClientSettings::load().lang,
            connected: false,
            confirm_leave: false,
            filters_open: false,
            pane: Pane::Cards,
            settings: SettingsPane::Closed,
        }
    }
}

impl Default for LobbyState {
    fn default() -> Self {
        Self::new()
    }
}

/// Where a finished HTTP call leaves its answer for the next frame.
///
/// Separate from [`LobbyState`] on purpose: touching it must not count as a
/// change to the lobby, or the UI would rebuild itself every frame.
#[derive(Resource, Clone, Default)]
struct Mailbox(Arc<Mutex<Vec<Reply>>>);

/// What a finished HTTP call hands back.
enum Reply {
    /// The outcome of a [`LobbyRequest`].
    Event(LobbyEvent),
    /// `GET /auth/config` said whether sign-ups are open.
    Registration(bool),
    /// The gateway no longer honours the account token we hold.
    Expired,
}

/// What the shell should make of a successful response body.
#[derive(Clone, Copy)]
enum Expect {
    /// `{"ok":true}` — nothing to read.
    Registered,
    /// `{"token":…}`.
    LoggedIn,
    /// A deck list.
    Decks,
    /// `{"deck_id":…}` from a new deck, or nothing at all from an edit.
    DeckSaved,
    /// The playable card pool.
    Pool,
    /// Every printing of one card.
    Printings,
    /// One deck, with its rows.
    DeckLoaded,
    /// A deck is gone; the gateway answers `204` with no body.
    DeckDeleted,
    /// A game list.
    Games,
    /// A seat handover.
    Seat,
    /// A chair given up; the gateway answers `204` with no body.
    Left,
}

// ------------------------------------------------------------------ HTTP

/// Performs a request the state machine asked for, if it asked for one.
fn dispatch(state: &LobbyState, mailbox: &Mailbox, request: Option<LobbyRequest>) {
    let Some(request) = request else {
        return;
    };
    let token = state.lobby.token();
    let (request, expect) = build(&state.gateway, token, &state.lang, request);
    fetch(request, expect, token.is_some(), mailbox);
}

/// The HTTP call one lobby request becomes, and what to make of its answer.
///
/// Separate from [`dispatch`] so the mapping onto the gateway's routes can be
/// tested without a socket: a wrong path or a misspelled field would otherwise
/// only show up as a 404 in somebody's hands.
#[allow(clippy::too_many_lines)] // one arm per route, read top to bottom
fn build(
    base: &str,
    token: Option<&str>,
    lang: &str,
    request: LobbyRequest,
) -> (ehttp::Request, Expect) {
    // A gateway URL out of a `.env` file very often ends in one.
    let base = base.trim_end_matches('/');
    let (request, expect) = match request {
        LobbyRequest::Register {
            email,
            display_name,
            password,
        } => (
            json_post(
                &format!("{base}/auth/register"),
                &serde_json::json!({
                    "email": email,
                    "display_name": display_name,
                    "password": password,
                }),
            ),
            Expect::Registered,
        ),
        LobbyRequest::LogIn { email, password } => (
            json_post(
                &format!("{base}/auth/login"),
                &serde_json::json!({ "email": email, "password": password }),
            ),
            Expect::LoggedIn,
        ),
        LobbyRequest::ListDecks => (ehttp::Request::get(format!("{base}/decks")), Expect::Decks),
        LobbyRequest::LoadPool => (
            // The pool is public reference data and needs no token; the lang
            // is what decides whether names and rules text come back
            // translated, and it is the same one the duel reads card text in.
            ehttp::Request::get(format!("{base}/pool?lang={lang}")),
            Expect::Pool,
        ),
        LobbyRequest::LoadPrintings { card } => (
            // Public for the same reason the pool is: which sets a card
            // appeared in is reference data, not something about an account.
            ehttp::Request::get(format!("{base}/printings?card={card}")),
            Expect::Printings,
        ),
        LobbyRequest::LoadDeck { deck_id } => (
            ehttp::Request::get(format!("{base}/decks/{deck_id}")),
            Expect::DeckLoaded,
        ),
        LobbyRequest::SaveDeck {
            deck_id,
            name,
            cards,
            sideboard,
            commander,
        } => {
            let body = serde_json::json!({
                "name": name,
                "cards": cards,
                "sideboard": sideboard,
                "commander": commander,
            });
            match deck_id {
                // Editing an existing deck overwrites it; without an id this
                // is a new one. Getting that backwards would either lose the
                // original or leave a duplicate behind on every save.
                Some(id) => (
                    json_body("PUT", &format!("{base}/decks/{id}"), &body),
                    Expect::DeckSaved,
                ),
                None => (
                    json_post(&format!("{base}/decks"), &body),
                    Expect::DeckSaved,
                ),
            }
        }
        LobbyRequest::DeleteDeck { deck_id } => (
            ehttp::Request {
                method: "DELETE".to_string(),
                ..ehttp::Request::get(format!("{base}/decks/{deck_id}"))
            },
            Expect::DeckDeleted,
        ),
        LobbyRequest::ListGames => (
            ehttp::Request::get(format!("{base}/lobby/games")),
            Expect::Games,
        ),
        LobbyRequest::CreateGame {
            deck_id,
            mode,
            chairs,
            name,
        } => (
            json_post(
                &format!("{base}/lobby/games"),
                &serde_json::json!({
                    "deck_id": deck_id,
                    "mode": mode.wire(),
                    "seats": chairs,
                    "name": name,
                }),
            ),
            Expect::Seat,
        ),
        LobbyRequest::JoinGame {
            game_id,
            deck_id,
            seat,
        } => (
            json_post(
                &format!("{base}/lobby/games/{game_id}/join"),
                &serde_json::json!({ "deck_id": deck_id, "seat": seat }),
            ),
            Expect::Seat,
        ),
        LobbyRequest::SetSeat {
            game_id,
            seat,
            kind,
            ai,
            deck_id,
        } => (
            json_post(
                &format!("{base}/lobby/games/{game_id}/seats/{seat}"),
                &serde_json::json!({
                    "kind": kind.map(|k| match k {
                        SeatKind::Human => "human",
                        SeatKind::Ai => "ai",
                    }),
                    "ai": ai,
                    "deck_id": deck_id,
                }),
            ),
            // Arranging a chair answers with the listing, so the room the
            // player is looking at redraws without a second round trip.
            Expect::Games,
        ),
        LobbyRequest::LeaveGame { game_id } => (
            json_post(
                &format!("{base}/lobby/games/{game_id}/leave"),
                &serde_json::json!({}),
            ),
            Expect::Left,
        ),
    };
    (bearer(request, token), expect)
}

/// A JSON `POST`. Built by hand rather than through `ehttp`'s `json` feature,
/// which would pull serde into a crate that already has it.
///
/// The headers are replaced, not added to: `ehttp`'s `insert` appends, and
/// `Request::post` has already set a `text/plain` content type that axum's
/// `Json` extractor refuses.
fn json_post(url: &str, body: &serde_json::Value) -> ehttp::Request {
    json_body("POST", url, body)
}

/// A JSON request with any method. `ehttp` only builds `GET` and `POST`, and
/// updating a deck is a `PUT`.
fn json_body(method: &str, url: &str, body: &serde_json::Value) -> ehttp::Request {
    let mut request = ehttp::Request::post(url, serde_json::to_vec(body).unwrap_or_default());
    request.method = method.to_string();
    request.headers = ehttp::Headers::new(&[
        ("Accept", "application/json"),
        ("Content-Type", "application/json"),
    ]);
    request
}

/// Signs a request with the account token, when there is one.
fn bearer(mut request: ehttp::Request, token: Option<&str>) -> ehttp::Request {
    if let Some(token) = token {
        request
            .headers
            .insert("Authorization", format!("Bearer {token}"));
    }
    request
}

/// Sends a request and posts its outcome to the mailbox.
fn fetch(request: ehttp::Request, expect: Expect, signed: bool, mailbox: &Mailbox) {
    let box_ = Arc::clone(&mailbox.0);
    ehttp::fetch(request, move |result| {
        let reply = match result {
            Ok(response) if response.ok => Reply::Event(decode(expect, &response)),
            // Only a *signed* 401 means the token is spent; on the sign-in
            // form it means the password was wrong.
            Ok(response) if signed && response.status == 401 => Reply::Expired,
            Ok(response) => Reply::Event(LobbyEvent::Failed(gateway_error(&response))),
            Err(err) => Reply::Event(LobbyEvent::Failed(format!(
                "the gateway did not answer: {err}"
            ))),
        };
        if let Ok(mut box_) = box_.lock() {
            box_.push(reply);
        }
    });
}

/// Turns a successful response into the event the lobby is waiting for.
fn decode(expect: Expect, response: &ehttp::Response) -> LobbyEvent {
    /// `POST /auth/login`.
    #[derive(serde::Deserialize)]
    struct TokenBody {
        token: String,
    }

    /// `POST /decks`. An edit answers `204` and parses to nothing.
    #[derive(serde::Deserialize)]
    struct SavedDeck {
        deck_id: String,
    }

    /// `GET /pool`.
    #[derive(serde::Deserialize)]
    struct PoolBody {
        cards: Vec<baylee_client_core::PoolCard>,
        #[serde(default)]
        has_text: bool,
    }

    /// `GET /printings`.
    #[derive(serde::Deserialize)]
    struct PrintingsBody {
        card: u32,
        printings: Vec<baylee_client_core::deckbuilder::Printing>,
        #[serde(default)]
        from_catalog: bool,
    }

    /// `GET /decks/{id}`.
    #[derive(serde::Deserialize)]
    struct StoredDeck {
        id: String,
        name: String,
        cards: Vec<String>,
        #[serde(default)]
        sideboard: Vec<String>,
        #[serde(default)]
        commander: Option<String>,
    }

    let body = response.text().unwrap_or_default();
    match expect {
        Expect::Registered => LobbyEvent::Registered,
        // An edit answers `204` with no body and needs no id: the builder
        // already holds the one it is editing.
        Expect::DeckSaved => LobbyEvent::DeckSaved {
            deck_id: serde_json::from_str::<SavedDeck>(body)
                .ok()
                .map(|d| d.deck_id),
        },
        Expect::DeckDeleted => LobbyEvent::DeckDeleted,
        Expect::Pool => serde_json::from_str::<PoolBody>(body).map_or_else(
            |_| unreadable("the card pool"),
            |b| LobbyEvent::Pool {
                cards: b.cards,
                has_text: b.has_text,
            },
        ),
        Expect::Printings => serde_json::from_str::<PrintingsBody>(body).map_or_else(
            |_| unreadable("the printings"),
            |b| LobbyEvent::Printings {
                card: b.card,
                printings: b.printings,
                from_catalog: b.from_catalog,
            },
        ),
        Expect::DeckLoaded => serde_json::from_str::<StoredDeck>(body).map_or_else(
            |_| unreadable("the deck"),
            |d| LobbyEvent::DeckLoaded {
                id: d.id,
                name: d.name,
                cards: d.cards,
                sideboard: d.sideboard,
                commander: d.commander,
            },
        ),
        Expect::LoggedIn => serde_json::from_str::<TokenBody>(body).map_or_else(
            |_| unreadable("the sign-in"),
            |b| LobbyEvent::LoggedIn { token: b.token },
        ),
        Expect::Decks => serde_json::from_str(body)
            .map_or_else(|_| unreadable("the deck list"), LobbyEvent::Decks),
        Expect::Games => serde_json::from_str(body)
            .map_or_else(|_| unreadable("the game list"), LobbyEvent::Games),
        Expect::Seat => {
            serde_json::from_str(body).map_or_else(|_| unreadable("the seat"), LobbyEvent::Seated)
        }
        // Nothing comes back, so the lobby re-reads the list to find out what
        // the table looks like without us.
        Expect::Left => LobbyEvent::Left,
    }
}

/// The message for a body that arrived but made no sense.
fn unreadable(what: &str) -> LobbyEvent {
    LobbyEvent::Failed(format!("could not read {what} the gateway sent"))
}

/// The gateway's own `{"error":…}`, or the bare status if it sent none.
fn gateway_error(response: &ehttp::Response) -> String {
    /// Every refusal the gateway sends has this shape.
    #[derive(serde::Deserialize)]
    struct Body {
        error: String,
    }

    response
        .text()
        .and_then(|body| serde_json::from_str::<Body>(body).ok())
        .map_or_else(
            || format!("the gateway answered {}", response.status),
            |b| b.error,
        )
}

/// Asks once, at startup, whether this gateway takes sign-ups.
fn ask_about_registration(state: Res<LobbyState>, mailbox: Res<Mailbox>) {
    /// `GET /auth/config`.
    #[derive(serde::Deserialize)]
    struct Body {
        registration_enabled: bool,
    }

    let box_ = Arc::clone(&mailbox.0);
    let url = format!("{}/auth/config", state.gateway);
    ehttp::fetch(ehttp::Request::get(&url), move |result| {
        let enabled = match result {
            Ok(response) if response.ok => response
                .text()
                .and_then(|body| serde_json::from_str::<Body>(body).ok())
                .map(|b| b.registration_enabled),
            // A gateway that is not up yet says nothing about registration.
            // Leaving the offer standing is the recoverable failure.
            _ => None,
        };
        if let Some(enabled) = enabled
            && let Ok(mut box_) = box_.lock()
        {
            box_.push(Reply::Registration(enabled));
        }
    });
}

// --------------------------------------------------------------- systems

/// Drains the mailbox, advances the lobby, and takes the seat it is granted.
fn poll(
    mut commands: Commands,
    mut state: ResMut<LobbyState>,
    mailbox: Res<Mailbox>,
    mut prefs: ResMut<crate::prefs::Prefs>,
    mut opens: MessageWriter<DuelCommand>,
) {
    let replies = {
        let Ok(mut box_) = mailbox.0.lock() else {
            return;
        };
        if box_.is_empty() {
            return;
        }
        std::mem::take(&mut *box_)
    };
    for reply in replies {
        match reply {
            Reply::Event(event) => {
                let next = state.lobby.apply(event);
                dispatch(&state, &mailbox, next);
            }
            Reply::Registration(enabled) => state.lobby.set_registration_enabled(enabled),
            Reply::Expired => state.lobby.sign_out(),
        }
    }
    // Keys and standing orders belong to the account, so signing in is what
    // fetches them and signing out is what stops writing them back. Both are
    // idempotent, which is why this can simply follow the token every frame
    // the mailbox delivers something.
    match state.lobby.token() {
        Some(token) => prefs.attach(&state.gateway, token),
        None => prefs.detach(),
    }
    let Screen::Seated(handover) = state.lobby.screen().clone() else {
        return;
    };
    if state.connected {
        return;
    }
    let ticket = SeatTicket {
        gateway: state.gateway.clone(),
        game_id: handover.game_id,
        // A hint only; the table's opening payload says which chair this is.
        seat: PlayerId::new(u8::try_from(handover.seat).unwrap_or(0)),
        seat_token: handover.seat_token,
    };
    match NetworkHost::connect(ticket) {
        Ok(host) => {
            state.connected = true;
            commands.insert_resource(InstalledHost(Box::new(host)));
            opens.write(DuelCommand::Open);
        }
        Err(reason) => state
            .lobby
            .unseat(format!("could not reach the table: {reason}")),
    }
}

/// How often a table of ours that is open is checked for an opponent.
const WATCH_SECS: f32 = 2.0;

/// Re-reads the table list while we are holding a seat nobody can use yet.
///
/// The gateway has nothing to push here — the seat exists but the game does
/// not, so there is no socket to be told on. Two seconds is well under the
/// time it takes a person to notice, and it stops the moment the wait ends.
fn watch(
    time: Res<Time>,
    mut since: Local<f32>,
    mut state: ResMut<LobbyState>,
    mailbox: Res<Mailbox>,
) {
    if state.lobby.awaiting().is_none() {
        *since = 0.0;
        return;
    }
    *since += time.delta_secs();
    if *since < WATCH_SECS {
        return;
    }
    *since = 0.0;
    let request = state.lobby.refresh();
    dispatch(&state, &mailbox, request);
}

/// Hands the sign-in form to the platform's own text input, where there is one.
///
/// Only the browser has one. Focusing a field there focuses a real `<input>`,
/// which is what raises a phone's keyboard and what makes autofill, paste and
/// an IME work at all; the value comes back whole rather than as keystrokes.
/// The keyboard is *not* raised on arrival — only when a field is tapped —
/// because a form that covers half the screen before anyone asked for it is
/// the thing every mobile web app gets wrong.
fn softkeys(
    mut keys: ResMut<SoftKeyboard>,
    mut state: ResMut<LobbyState>,
    mut scrolled: ResMut<Scrolled>,
    mailbox: Res<Mailbox>,
    mut epoch: Local<u64>,
    mut build_epoch: Local<u64>,
) {
    if !SoftKeyboard::owns_typing() {
        return;
    }
    // The builder counts its own placements, so it gets its own tally: one
    // shared counter would open the keyboard on the way between the screens.
    if matches!(state.lobby.screen(), Screen::Build) {
        let builder = state.lobby.builder();
        if *build_epoch != builder.focus_epoch() {
            *build_epoch = builder.focus_epoch();
            keys.open(builder.focus().kind(), builder.focused_text());
            return;
        }
        for key in keys.drain() {
            match key {
                SoftKey::Text(value) => {
                    let searching = state.lobby.builder().focus() == BuildField::Search;
                    state.lobby.builder_mut().set_focused(&value);
                    if searching {
                        scrolled.set(List::Pool, 0.0);
                    }
                }
                // Nothing to submit: a deck is saved from the bar, and
                // closing the keyboard is what "done" means here.
                SoftKey::Submit => keys.close(),
            }
        }
        return;
    }
    *build_epoch = state.lobby.builder().focus_epoch();
    if !matches!(state.lobby.screen(), Screen::SignIn { .. }) {
        keys.close();
        *epoch = state.lobby.focus_epoch();
        return;
    }
    // A tap on a field is what opens it — including a tap on the field the
    // caret is already in, which is why this counts placements rather than
    // watching which field is focused.
    if *epoch != state.lobby.focus_epoch() {
        *epoch = state.lobby.focus_epoch();
        let field = state.lobby.focus();
        keys.open(field.kind(), state.lobby.field(field));
        return;
    }
    for key in keys.drain() {
        match key {
            SoftKey::Text(value) => {
                let field = state.lobby.focus();
                state.lobby.set_field(field, &value);
            }
            SoftKey::Submit => {
                let request = state.lobby.submit();
                dispatch(&state, &mailbox, request);
            }
        }
    }
}

/// Types into the sign-in form from a keyboard the client itself reads.
///
/// Skipped entirely where [`SoftKeyboard`] owns the typing: the browser's
/// input has focus, so the canvas sees nothing anyway, and anything it did see
/// would be entered twice.
fn keyboard(
    mut keys: MessageReader<KeyboardInput>,
    codes: Res<ButtonInput<KeyCode>>,
    mut state: ResMut<LobbyState>,
    mut prefs: ResMut<crate::prefs::Prefs>,
    mut scrolled: ResMut<Scrolled>,
    mailbox: Res<Mailbox>,
) {
    // A rebinding in progress takes every key, including the ones that mean
    // something everywhere else — a player who wants `Esc` on some other
    // action has to be able to press it. Escape and backspace are the two
    // exceptions, and they are what makes the row escapable at all.
    if let Some(action) = state.settings.capturing() {
        keys.clear();
        if codes.just_pressed(KeyCode::Escape) {
            state.settings = SettingsPane::Open;
        } else if codes.any_just_pressed([KeyCode::Backspace, KeyCode::Delete]) {
            // Unbinding is a real answer: a pointer still reaches everything.
            prefs.edit().keymap.bind(action, vec![]);
            state.settings = SettingsPane::Open;
        } else if let Some(chord) = crate::keys::captured(&codes) {
            prefs.edit().keymap.bind(action, vec![chord]);
            state.settings = SettingsPane::Open;
        }
        return;
    }
    if state.settings.is_open() {
        // Nothing on the settings screen is typed into.
        keys.clear();
        return;
    }
    if SoftKeyboard::owns_typing() {
        keys.clear();
        return;
    }
    if matches!(state.lobby.screen(), Screen::Build) {
        for key in keys.read() {
            if !key.state.is_pressed() {
                continue;
            }
            let builder = state.lobby.builder_mut();
            let searching = builder.focus() == BuildField::Search;
            let mut narrowed = false;
            match &key.logical_key {
                Key::Backspace => {
                    builder.backspace_focused();
                    narrowed = searching;
                }
                Key::Tab => builder.cycle_focus(),
                // Enter in the search box adds the first result: the fastest
                // way to type a deck is name, return, name, return.
                Key::Enter => {
                    if builder.focus() == BuildField::Search {
                        let first = builder.results().first().copied();
                        let zone = builder.zone();
                        if let Some(slot) = first {
                            builder.add(slot, zone);
                        }
                    }
                }
                _ => {
                    if let Some(text) = key.text.as_ref() {
                        for ch in text.chars() {
                            builder.type_focused(ch);
                            narrowed = searching;
                        }
                    }
                }
            }
            // A different search is a different list; the row that was
            // halfway down it is not in this one.
            if narrowed {
                scrolled.set(List::Pool, 0.0);
            }
        }
        return;
    }
    if !matches!(state.lobby.screen(), Screen::SignIn { .. }) {
        keys.clear();
        return;
    }
    for key in keys.read() {
        if !key.state.is_pressed() {
            continue;
        }
        match &key.logical_key {
            Key::Backspace => state.lobby.backspace(),
            Key::Tab => state.lobby.cycle_focus(),
            Key::Enter => {
                let request = state.lobby.submit();
                dispatch(&state, &mailbox, request);
            }
            // Everything else is text or nothing. `type_char` drops the
            // control characters Tab and Enter also produce.
            _ => {
                if let Some(text) = key.text.as_ref() {
                    for ch in text.chars() {
                        state.lobby.type_char(ch);
                    }
                }
            }
        }
    }
}

/// Turns a click on a lobby control into an intent.
#[allow(clippy::too_many_arguments)] // two pointer streams, then the usual
#[allow(clippy::too_many_lines)] // one flat match, read top to bottom
fn clicks(
    mut pointer: MessageReader<Pointer<Click>>,
    mut ends: MessageReader<Pointer<DragEnd>>,
    mut scrolled: ResMut<Scrolled>,
    presses: Query<&Press>,
    parents: Query<&ChildOf>,
    mut state: ResMut<LobbyState>,
    mut prefs: ResMut<crate::prefs::Prefs>,
    mailbox: Res<Mailbox>,
    mut commands: Commands,
    mut opens: MessageWriter<DuelCommand>,
) {
    // A release always fires a click, drag or no drag, so a swipe down the
    // card list would add whichever card it started on. The scroll it already
    // performed is what the gesture meant.
    let swiped = ends.read().any(|end| end.distance.length() > DRAG_SLOP);
    if swiped {
        pointer.clear();
        return;
    }
    for click in pointer.read() {
        let Some(press) = in_lineage(click.entity, &presses, &parents) else {
            continue;
        };
        // Any other control answers the question the back button asked.
        if *press != Press::CloseBuilder {
            state.confirm_leave = false;
        }
        // A filter that changes what is in the list puts it back at the top:
        // finding yourself halfway down a fresh search is disorienting, and
        // the row you were reading is not in it any more anyway.
        if matches!(
            *press,
            Press::ToggleColor(_)
                | Press::SetKind(_)
                | Press::SetCmc(_)
                | Press::TogglePlayable
                | Press::CycleSort
                | Press::ClearFilters
        ) {
            scrolled.set(List::Pool, 0.0);
        }
        // Any click that is not the rebinding chip itself calls off a
        // rebinding in progress. Leaving it armed would mean the next key
        // pressed anywhere lands on whichever row was last tapped.
        if state.settings.is_open() && !matches!(*press, Press::Rebind(_)) {
            state.settings = SettingsPane::Open;
        }
        match *press {
            Press::OpenSettings => state.settings = SettingsPane::Open,
            Press::CloseSettings => state.settings = SettingsPane::Closed,
            Press::Rebind(action) => {
                // Tapping the armed row again disarms it, so the chip is its
                // own cancel and there is no way to get stuck waiting.
                state.settings = if state.settings.capturing() == Some(action) {
                    SettingsPane::Open
                } else {
                    SettingsPane::Rebinding(action)
                };
            }
            Press::ResetBinding(action) => prefs.edit().keymap.reset(action),
            Press::ResetAllBindings => {
                prefs.edit().keymap = baylee_client_core::prefs::Keymap::standard();
            }
            Press::ToggleAuto(rule) => {
                let mut edit = prefs.edit();
                rule.toggle(&mut edit.auto);
            }
            Press::ToggleMotion => {
                let mut edit = prefs.edit();
                edit.reduce_motion = !edit.reduce_motion;
            }
            Press::ToggleRail(side, row) => prefs.edit().orders.toggle(side, row),
            Press::Focus(field) => state.lobby.focus_on(field),
            Press::ToggleRegistering => state.lobby.toggle_registering(),
            Press::Submit => {
                let request = state.lobby.submit();
                dispatch(&state, &mailbox, request);
            }
            Press::SignOut => state.lobby.sign_out(),
            Press::Refresh => {
                let request = state.lobby.refresh();
                dispatch(&state, &mailbox, request);
            }
            Press::StarterDeck => {
                let rows = starter_rows();
                let request = state.lobby.create_deck(STARTER, rows);
                dispatch(&state, &mailbox, request);
            }
            Press::SelectDeck(index) => state.lobby.select_deck(index),
            Press::Host(mode) => {
                let request = state.lobby.host(mode);
                dispatch(&state, &mailbox, request);
            }
            Press::Join(index) => {
                let game = state.lobby.games().get(index).map(|g| g.id.clone());
                if let Some(game) = game {
                    let request = state.lobby.join(&game);
                    dispatch(&state, &mailbox, request);
                }
            }
            Press::OpenRoom(chairs) => {
                let request = state.lobby.open_room(GameMode::Open, chairs, String::new());
                dispatch(&state, &mailbox, request);
            }
            Press::JoinSeat(index, seat) => {
                let game = state.lobby.games().get(index).map(|g| g.id.clone());
                if let Some(game) = game {
                    let request = state.lobby.join_seat(&game, Some(seat));
                    dispatch(&state, &mailbox, request);
                }
            }
            Press::LeaveTable(index) => {
                let game = state.lobby.games().get(index).map(|g| g.id.clone());
                if let Some(game) = game {
                    let request = state.lobby.leave_table(&game);
                    dispatch(&state, &mailbox, request);
                }
            }
            Press::SeatKind(index, seat, kind) => {
                let game = state.lobby.games().get(index).map(|g| g.id.clone());
                if let Some(game) = game {
                    let request = state.lobby.set_seat(&game, seat, Some(kind), None);
                    dispatch(&state, &mailbox, request);
                }
            }
            Press::SeatAi(index, seat, profile) => {
                let game = state.lobby.games().get(index).map(|g| g.id.clone());
                if let Some(game) = game {
                    let request =
                        state
                            .lobby
                            .set_seat(&game, seat, None, Some(profile.to_string()));
                    dispatch(&state, &mailbox, request);
                }
            }
            Press::SeatDeck(index, seat) => {
                let game = state.lobby.games().get(index).map(|g| g.id.clone());
                if let Some(game) = game {
                    let request = state.lobby.seat_deck(&game, seat);
                    dispatch(&state, &mailbox, request);
                }
            }
            Press::PlayOffline => match crate::host::house_duel() {
                Some(host) => {
                    state.connected = true;
                    commands.insert_resource(InstalledHost(Box::new(host)));
                    opens.write(DuelCommand::Open);
                }
                None => state.lobby.say("could not start the offline duel"),
            },
            // `Leave` is only ever spawned on the finished screen, and
            // `PickerNothing` exists to stop a tap inside the picker
            // reaching the shade behind it. Neither does anything here.
            Press::Leave | Press::PickerNothing => {}
            Press::NewDeck => {
                let request = state.lobby.build_deck();
                dispatch(&state, &mailbox, request);
            }
            Press::EditDeck(index) => {
                state.pane = Pane::Deck;
                let request = state.lobby.edit_deck(index);
                dispatch(&state, &mailbox, request);
            }
            Press::DeleteDeck(index) => {
                let request = state.lobby.delete_deck(index);
                dispatch(&state, &mailbox, request);
            }
            Press::CloseBuilder => {
                if state.lobby.builder().dirty() && !state.confirm_leave {
                    state.confirm_leave = true;
                    state.lobby.say("unsaved changes — press again to leave");
                } else {
                    state.confirm_leave = false;
                    let request = state.lobby.close_builder();
                    dispatch(&state, &mailbox, request);
                }
            }
            Press::SaveDeck => {
                let request = state.lobby.save_deck();
                dispatch(&state, &mailbox, request);
            }
            Press::FocusBuild(field) => state.lobby.builder_mut().focus_on(field),
            Press::AddCard(slot) => {
                let zone = state.lobby.builder().zone();
                if !state.lobby.builder_mut().add(slot, zone) {
                    state.lobby.say("no room for another copy of that");
                }
            }
            Press::PickPrint(slot) => {
                let zone = state.lobby.builder().zone();
                let request = state.lobby.builder_mut().open_picker(slot, zone);
                dispatch(&state, &mailbox, request);
            }
            Press::PickerStep(by) => state.lobby.builder_mut().picker_step(by),
            Press::PickerGo(at) => state.lobby.builder_mut().picker_go(at),
            Press::PickerLang(which) => {
                // The list the index came from is the one being read here, so
                // a stale index simply selects nothing rather than panicking.
                let lang = which.and_then(|i| {
                    state
                        .lobby
                        .builder()
                        .picker()
                        .and_then(|p| p.langs().get(i).cloned())
                });
                state.lobby.builder_mut().picker_set_lang(lang.as_deref());
            }
            Press::PickerFinish(finish) => state.lobby.builder_mut().picker_set_finish(finish),
            Press::PickerConfirm => {
                if !state.lobby.builder_mut().picker_confirm() {
                    state.lobby.say("no room for another copy of that");
                }
            }
            Press::PickerClose => state.lobby.builder_mut().close_picker(),
            Press::RemoveRow(at) => {
                let zone = state.lobby.builder().zone();
                state.lobby.builder_mut().remove_at(at, zone);
            }
            Press::MoveRow(at) => {
                let from = state.lobby.builder().zone();
                let to = match from {
                    Zone::Main => Zone::Side,
                    Zone::Side => Zone::Main,
                };
                state.lobby.builder_mut().move_entry(at, from, to);
            }
            Press::AddCardTo(slot, zone) => {
                state.lobby.builder_mut().add(slot, zone);
            }
            Press::SetCommander(slot) => {
                state.lobby.builder_mut().set_commander(slot);
            }
            Press::ClearCommander => state.lobby.builder_mut().clear_commander(),
            Press::SetZone(zone) => state.lobby.builder_mut().set_zone(zone),
            Press::ToggleColor(color) => state.lobby.builder_mut().toggle_color(color),
            Press::SetKind(kind) => {
                let builder = state.lobby.builder_mut();
                // A second tap on the open chip is how it is closed again;
                // without it a filter can only be dropped from "Clear".
                let same = builder.kind() == kind;
                builder.set_kind(if same { None } else { kind });
            }
            Press::SetCmc(cmc) => state.lobby.builder_mut().set_cmc(Some(cmc)),
            Press::TogglePlayable => state.lobby.builder_mut().toggle_playable_only(),
            Press::CycleSort => state.lobby.builder_mut().cycle_sort(),
            Press::ClearFilters => state.lobby.builder_mut().clear_filters(),
            Press::ClearDeck => state.lobby.builder_mut().clear_deck(),
            Press::ShowPane(pane) => state.pane = pane,
            Press::Inspect(slot) => state.lobby.builder_mut().inspect(slot),
            Press::CloseCard => state.lobby.builder_mut().stop_inspecting(),
            Press::ToggleFilters => state.filters_open = !state.filters_open,
        }
    }
}

/// How far a pointer has to travel before the gesture is a scroll rather than
/// a tap. Below it a shaky finger would still add a card; above it, a swipe
/// down a list would.
const DRAG_SLOP: f32 = 8.0;

/// What one line of wheel travel moves a list, in logical pixels.
const WHEEL_LINE: f32 = 32.0;

/// A list that scrolls its own contents, and which one it is.
///
/// `Overflow::scroll_y` only *clips*: Bevy moves the content when
/// [`ScrollPosition`] changes and nothing changes it on its own. Without this
/// system a sixty-row result list would simply end at the bottom of the panel
/// with no way to reach the rest.
#[derive(Component, Clone, Copy, PartialEq, Eq, Debug)]
struct Scrollable(List);

/// The lists that remember where they were left.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum List {
    /// The searchable card pool.
    Pool,
    /// The deck being built.
    Deck,
    /// The tables and decks on the lobby screen.
    Table,
}

/// Where each list was left, across rebuilds of the node tree.
///
/// Deliberately not part of [`LobbyState`]: the tree is rebuilt whenever that
/// changes, so keeping the offsets there would rebuild sixty rows on every
/// notch of the wheel. Kept apart, adding a card rebuilds the list *and*
/// leaves it where the player was reading — which is the only reason they
/// scrolled there.
#[derive(Resource, Default)]
pub(crate) struct Scrolled {
    pool: f32,
    deck: f32,
    table: f32,
}

impl Scrolled {
    pub(crate) fn get(&self, list: List) -> f32 {
        match list {
            List::Pool => self.pool,
            List::Deck => self.deck,
            List::Table => self.table,
        }
    }

    fn set(&mut self, list: List, at: f32) {
        match list {
            List::Pool => self.pool = at,
            List::Deck => self.deck = at,
            List::Table => self.table = at,
        }
    }
}

/// Turns a wheel or a swipe into scrolling on the list under the pointer.
fn scrolls(
    mut wheels: MessageReader<Pointer<Scroll>>,
    mut drags: MessageReader<Pointer<Drag>>,
    parents: Query<&ChildOf>,
    mut lists: Query<(&mut ScrollPosition, &ComputedNode, &Scrollable)>,
    mut memory: ResMut<Scrolled>,
) {
    for wheel in wheels.read() {
        let travel = match wheel.unit {
            MouseScrollUnit::Line => wheel.y * WHEEL_LINE,
            MouseScrollUnit::Pixel => wheel.y,
        };
        // A wheel pushed away from the reader moves the content up, which is
        // an *increase* in the scroll offset.
        scroll_lineage(wheel.entity, -travel, &parents, &mut lists, &mut memory);
    }
    for drag in drags.read() {
        // A finger drags the content itself, so it goes the other way again.
        scroll_lineage(
            drag.entity,
            -drag.delta.y,
            &parents,
            &mut lists,
            &mut memory,
        );
    }
}

/// Scrolls the nearest list at or above an entity, so a gesture over a row
/// scrolls the list the row is in.
fn scroll_lineage(
    entity: Entity,
    by: f32,
    parents: &Query<&ChildOf>,
    lists: &mut Query<(&mut ScrollPosition, &ComputedNode, &Scrollable)>,
    memory: &mut Scrolled,
) {
    let mut current = Some(entity);
    for _ in 0..8 {
        let Some(e) = current else {
            return;
        };
        if let Ok((mut position, computed, which)) = lists.get_mut(e) {
            position.y = scrolled(
                position.y,
                by,
                computed.size().y,
                computed.content_size().y,
                computed.inverse_scale_factor(),
            );
            memory.set(which.0, position.y);
            return;
        }
        current = parents.get(e).ok().map(ChildOf::parent);
    }
}

/// Where a list ends up after a gesture.
///
/// Bevy clamps what it *draws* but leaves [`ScrollPosition`] alone, so an
/// offset past the end would have to be unwound before the list moved again —
/// a swipe that ran off the bottom would then need the same distance back
/// before anything happened. The two sizes are physical pixels and the offset
/// is logical, which is what `scale` (a `ComputedNode`'s inverse scale factor)
/// converts between.
fn scrolled(from: f32, by: f32, view: f32, content: f32, scale: f32) -> f32 {
    let room = (content - view).max(0.0) * scale;
    (from + by).clamp(0.0, room)
}

/// Leaves a finished game and comes back here.
fn leave_clicks(
    mut pointer: MessageReader<Pointer<Click>>,
    presses: Query<&Press>,
    parents: Query<&ChildOf>,
    mut closes: MessageWriter<DuelCommand>,
) {
    for click in pointer.read() {
        if let Some(Press::Leave) = in_lineage(click.entity, &presses, &parents) {
            closes.write(DuelCommand::Close);
        }
    }
}

/// The lobby is on screen again: forget the seat and re-read the tables.
fn came_back(mut commands: Commands, mut state: ResMut<LobbyState>, mailbox: Res<Mailbox>) {
    // Drops the socket (or the in-process engine) with it: a stale host would
    // keep a dead table's messages queued behind the next game's.
    commands.remove_resource::<InstalledHost>();
    state.connected = false;
    if !matches!(state.lobby.screen(), Screen::Seated(_)) {
        return;
    }
    state.lobby.unseat("the game ended");
    let request = state.lobby.refresh();
    dispatch(&state, &mailbox, request);
}

/// A component whose click means something.
#[derive(Component, Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum Press {
    /// Put the caret in this field.
    Focus(Field),
    /// Swap the form between log-in and sign-up.
    ToggleRegistering,
    /// Send the sign-in form.
    Submit,
    /// Play the house AI in this process, no account needed.
    PlayOffline,
    /// Forget the account.
    SignOut,
    /// Re-read decks and tables.
    Refresh,
    /// Save the starter deck.
    StarterDeck,
    /// Pick a deck by its index in the list.
    SelectDeck(usize),
    /// Open a new table.
    Host(GameMode),
    /// Open a table with a chosen number of chairs.
    OpenRoom(usize),
    /// Sit down at a listed table by its index.
    Join(usize),
    /// Sit down in a named chair of a listed table.
    JoinSeat(usize, u32),
    /// Give up a chair, or close the room when hosting it.
    LeaveTable(usize),
    /// Make a chair a person's or the AI's.
    SeatKind(usize, u32, SeatKind),
    /// Set an AI chair's difficulty.
    SeatAi(usize, u32, &'static str),
    /// Put the selected deck in a chair.
    SeatDeck(usize, u32),
    /// Leave a finished game.
    Leave,
    /// Open the settings screen.
    OpenSettings,
    /// Leave it.
    CloseSettings,
    /// Wait for a key and bind it to this action.
    Rebind(baylee_client_core::prefs::Action),
    /// Put one action back to its default key.
    ResetBinding(baylee_client_core::prefs::Action),
    /// Put every key back.
    ResetAllBindings,
    /// Flip one automation switch.
    ToggleAuto(baylee_client_core::prefs::AutoRule),
    /// Stop the table moving, or let it move again.
    ToggleMotion,
    /// Turn one step of the phase rail red or green.
    ToggleRail(
        baylee_client_core::automation::RailSide,
        baylee_client_core::automation::RailRow,
    ),
    /// Open the builder on a new deck.
    NewDeck,
    /// Open the builder on a saved deck, by its index in the list.
    EditDeck(usize),
    /// Throw a saved deck away, by its index in the list.
    DeleteDeck(usize),
    /// Leave the builder for the tables.
    CloseBuilder,
    /// Save whatever the builder holds.
    SaveDeck,
    /// Put the caret in one of the builder's boxes.
    FocusBuild(BuildField),
    /// Add one copy of a pool card, by its slot, to the open zone.
    AddCard(usize),
    /// Build into the main deck or the sideboard.
    SetZone(Zone),
    /// Turn one colour of the identity filter on or off.
    ToggleColor(char),
    /// Show only one card type, or all of them again.
    SetKind(Option<&'static str>),
    /// Show only one mana value, or all of them again. Doubles as the click
    /// target on a curve bar.
    SetCmc(u32),
    /// Hide the cards the engine does not play properly, or stop hiding them.
    TogglePlayable,
    /// Change what the results are sorted by.
    CycleSort,
    /// Drop every filter at once.
    ClearFilters,
    /// Empty both zones.
    ClearDeck,
    /// Show the pool or the deck, on a screen with room for one.
    ShowPane(Pane),
    /// Read a card in full, by its slot in the pool.
    Inspect(usize),
    /// Open the printing picker on a pool card, by its slot.
    PickPrint(usize),
    /// Move the picker's carousel.
    PickerStep(i32),
    /// Jump the carousel to one printing, by its place in the visible list.
    PickerGo(usize),
    /// Limit the carousel to one language, by its place in the picker's list,
    /// or `None` for all of them. An index rather than the code itself
    /// because a `Press` is `Copy` and a language code is a `String`.
    PickerLang(Option<usize>),
    /// Choose a finish for the printing the carousel is on.
    PickerFinish(Finish),
    /// Add the picked printing to the deck.
    PickerConfirm,
    /// Put the picker away, adding nothing.
    PickerClose,
    /// Nothing. Carried by the picker's own panel so a tap inside it is
    /// not also a tap on the shade behind it, which would close it.
    PickerNothing,
    /// Take one copy out of a named row of the deck list.
    RemoveRow(usize),
    /// Move one copy of a named row to the other list — deck to sideboard,
    /// or back. The row keeps the printing it was chosen with.
    MoveRow(usize),
    /// Add one copy of a pool card to a named list, whichever one is open.
    AddCardTo(usize, Zone),
    /// Make a pool card the deck's commander.
    SetCommander(usize),
    /// Take the commander mark off, leaving the card in the deck.
    ClearCommander,
    /// Put it away again.
    CloseCard,
    /// Show or hide the filter chips on a narrow screen.
    ToggleFilters,
}

/// The nearest [`Press`] at or above an entity, so a click on a button's
/// label counts as a click on the button.
fn in_lineage<'a>(
    entity: Entity,
    presses: &'a Query<&Press>,
    parents: &Query<&ChildOf>,
) -> Option<&'a Press> {
    let mut current = Some(entity);
    for _ in 0..6 {
        let e = current?;
        if let Ok(found) = presses.get(e) {
            return Some(found);
        }
        current = parents.get(e).ok().map(ChildOf::parent);
    }
    None
}

// ------------------------------------------------------------ hover preview

/// A row that has a card behind it, and what that card looks like.
///
/// The URL is worked out when the row is spawned rather than when it is
/// hovered: the row already knows which printing it is showing, and a hover
/// that had to go looking would be doing it on the pointer's schedule.
#[derive(Component, Clone)]
pub struct HoverCard {
    /// The card's art, if there is a printing to fetch.
    pub url: Option<String>,
    /// How the printing is finished, so a foil previews as one.
    pub finish: FinishTreatment,
}

/// The card the pointer is over, and where the pointer was.
#[derive(Resource, Default)]
struct Hovered {
    /// What to draw, or `None` when the pointer is over nothing.
    card: Option<HoverCard>,
    /// Where to draw it, in logical pixels.
    at: Vec2,
    /// Bumped whenever either changes, so the preview knows to redraw
    /// without comparing an image handle.
    epoch: u64,
}

/// The preview node itself.
#[derive(Component)]
struct CardPreview {
    /// The epoch this node was drawn for.
    epoch: u64,
}

/// Tracks which row the pointer is over.
fn hovers(
    mut overs: MessageReader<Pointer<Over>>,
    mut outs: MessageReader<Pointer<Out>>,
    cards: Query<&HoverCard>,
    parents: Query<&ChildOf>,
    mut hovered: ResMut<Hovered>,
) {
    for out in outs.read() {
        if lineage_card(out.entity, &cards, &parents).is_some() {
            hovered.card = None;
            hovered.epoch = hovered.epoch.wrapping_add(1);
        }
    }
    for over in overs.read() {
        if let Some(card) = lineage_card(over.entity, &cards, &parents) {
            hovered.card = Some(card.clone());
            hovered.at = over.pointer_location.position;
            hovered.epoch = hovered.epoch.wrapping_add(1);
        }
    }
}

/// The nearest [`HoverCard`] at or above an entity.
fn lineage_card<'a>(
    entity: Entity,
    cards: &'a Query<&HoverCard>,
    parents: &Query<&ChildOf>,
) -> Option<&'a HoverCard> {
    let mut current = Some(entity);
    for _ in 0..6 {
        let e = current?;
        if let Ok(found) = cards.get(e) {
            return Some(found);
        }
        current = parents.get(e).ok().map(ChildOf::parent);
    }
    None
}

/// Draws the hovered card beside the pointer.
///
/// Its own entity, spawned and despawned on its own: rebuilding the whole
/// builder on every hover would mean tearing down two hundred rows to show
/// one picture.
fn preview(
    mut commands: Commands,
    hovered: Res<Hovered>,
    existing: Query<(Entity, &CardPreview)>,
    windows: Query<&Window>,
    assets: Option<Res<AssetServer>>,
    ui_materials: Option<ResMut<UiCardMaterials>>,
    material_assets: Option<ResMut<Assets<CardUiMaterial>>>,
) {
    let current = existing.iter().next().map(|(_, p)| p.epoch);
    if current == Some(hovered.epoch) {
        return;
    }
    for (entity, _) in existing {
        commands.entity(entity).despawn();
    }
    let (Some(card), Some(assets)) = (hovered.card.as_ref(), assets) else {
        return;
    };
    let Some(url) = card.url.clone() else {
        return;
    };
    let (Some(mut cache), Some(mut store)) = (ui_materials, material_assets) else {
        return;
    };
    let mut cards = UiCards {
        cache: &mut cache,
        assets: &mut store,
    };

    // Big enough to read the art, small enough to leave the list visible.
    let height = 340.0_f32;
    let width = height * baylee_client_core::layout::CARD_ASPECT;
    let window = windows.iter().next();
    let (w, h) = window.map_or((1280.0, 800.0), |win| (win.width(), win.height()));
    // Beside the pointer, flipped to the other side when there is no room
    // and clamped so a row near the bottom does not push it off screen.
    let left = if hovered.at.x + width + 32.0 < w {
        hovered.at.x + 24.0
    } else {
        (hovered.at.x - width - 24.0).max(8.0)
    };
    let top = (hovered.at.y - height / 2.0).clamp(8.0, (h - height - 8.0).max(8.0));

    let material = cards.preview(&url, card.finish, assets.load(url.clone()));
    commands.spawn((
        CardPreview {
            epoch: hovered.epoch,
        },
        MaterialNode(material),
        Node {
            position_type: PositionType::Absolute,
            left: px(left),
            top: px(top),
            width: px(width),
            height: px(height),
            border_radius: BorderRadius::all(px(12)),
            ..default()
        },
        GlobalZIndex(600),
        // A preview must never eat the click that would add the card.
        Pickable::IGNORE,
    ));
}

/// Takes the preview down when the builder does.
fn despawn_preview(mut commands: Commands, previews: Query<Entity, With<CardPreview>>) {
    for entity in previews {
        commands.entity(entity).despawn();
    }
}

/// The art a pool row previews: the printing the registry names.
pub(crate) fn hover_of_card(card: &baylee_client_core::deckbuilder::PoolCard) -> HoverCard {
    HoverCard {
        url: baylee_client_core::images::image_url(
            &baylee_view::PrintEntry {
                scryfall_id: card.scryfall_id.clone(),
                lang: "en".to_string(),
                finish: baylee_view::Finish::Normal,
            },
            baylee_client_core::images::Face::Front,
            baylee_client_core::images::ArtSize::Normal,
        ),
        finish: FinishTreatment::Plain,
    }
}

/// The art a deck row previews: the printing that row actually names.
pub(crate) fn hover_of_entry(
    card: &baylee_client_core::deckbuilder::PoolCard,
    print: &baylee_core::deckrow::PrintChoice,
) -> HoverCard {
    let finish = print.finish_or_default();
    HoverCard {
        url: baylee_client_core::images::image_url(
            &baylee_view::PrintEntry {
                // A row that named an exact printing previews that one; one
                // that only narrowed by set has no id to fetch with, so it
                // falls back to the art the pool row shows.
                scryfall_id: print
                    .scryfall_id
                    .clone()
                    .unwrap_or_else(|| card.scryfall_id.clone()),
                lang: print.lang_or_default().to_string(),
                finish: match finish {
                    Finish::Foil => baylee_view::Finish::Foil,
                    Finish::Etched => baylee_view::Finish::Etched,
                    Finish::Normal => baylee_view::Finish::Normal,
                },
            },
            baylee_client_core::images::Face::Front,
            baylee_client_core::images::ArtSize::Normal,
        ),
        finish: crate::buildui::treatment(finish),
    }
}

/// The starter deck's rows, in the `"N Card Name"` form `POST /decks` takes.
fn starter_rows() -> Vec<String> {
    use baylee_core::acceptance::Zone;

    baylee_core::acceptance::parse_decks(&crate::host::acceptance_text())
        .unwrap_or_default()
        .into_iter()
        .filter(|row| row.deck == STARTER && row.zone == Zone::Main)
        .map(|row| format!("{} {}", row.count, row.name))
        .collect()
}

// -------------------------------------------------------------------- UI

/// Everything the lobby owns on screen, camera included.
#[derive(Component)]
struct LobbyScreen;

/// The root of the rebuilt node tree.
#[derive(Component)]
struct LobbyRoot;

/// The "leave table" button shown over a finished game.
#[derive(Component)]
struct LeaveButton;

/// How much room there is, in three sizes.
///
/// Breakpoints rather than a continuous scale: what changes between a phone
/// and a desktop is the *shape* of the screen — one column or two, a card that
/// fills the width or one that floats — and shape does not interpolate.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum Frame {
    /// A phone held upright, or a very narrow window.
    Phone,
    /// A tablet, or a half-screen window.
    Tablet,
    /// A desktop window.
    Desktop,
}

impl Frame {
    /// The frame a window of this width is in.
    fn of(width: f32) -> Self {
        if width < 760.0 {
            Self::Phone
        } else if width < 1180.0 {
            Self::Tablet
        } else {
            Self::Desktop
        }
    }
}

/// Every size the layout takes from the frame, in one place.
#[derive(Clone, Copy)]
pub(crate) struct Metrics {
    pub(crate) frame: Frame,
    /// Body text.
    pub(crate) text: f32,
    /// Headings.
    pub(crate) head: f32,
    /// Captions and secondary lines.
    pub(crate) small: f32,
    /// The minimum height of anything meant to be tapped. 44 logical pixels
    /// is the smallest target a finger hits reliably.
    pub(crate) tap: f32,
    /// Padding around and inside panels.
    pub(crate) pad: f32,
    /// Gap between stacked controls.
    pub(crate) gap: f32,
}

impl Metrics {
    fn of(width: f32) -> Self {
        match Frame::of(width) {
            Frame::Phone => Self {
                frame: Frame::Phone,
                text: 15.0,
                head: 17.0,
                small: 12.0,
                tap: 48.0,
                pad: 14.0,
                gap: 12.0,
            },
            Frame::Tablet => Self {
                frame: Frame::Tablet,
                text: 14.0,
                head: 16.0,
                small: 11.5,
                tap: 44.0,
                pad: 16.0,
                gap: 10.0,
            },
            Frame::Desktop => Self {
                frame: Frame::Desktop,
                text: 13.0,
                head: 15.0,
                small: 11.0,
                tap: 38.0,
                pad: 18.0,
                gap: 9.0,
            },
        }
    }

    /// Whether the table screen stacks its two panels instead of pairing them.
    fn stacked(self) -> bool {
        self.frame == Frame::Phone
    }

    /// The width of the deck panel beside the table list.
    fn decks_width(self) -> Val {
        match self.frame {
            Frame::Phone => percent(100),
            Frame::Tablet => px(280),
            Frame::Desktop => px(330),
        }
    }
}

/// The lobby's own camera. The duel brings its own and the two never coexist:
/// this one is despawned on the way out of [`DuelPhase::Closed`], before the
/// stage is built.
fn spawn_camera(mut commands: Commands) {
    commands.spawn((
        LobbyScreen,
        Camera2d,
        Camera {
            clear_color: ClearColorConfig::Custom(BACKDROP),
            ..default()
        },
    ));
}

/// Drops the whole lobby when a duel takes the screen.
fn teardown(mut commands: Commands, screen: Query<Entity, With<LobbyScreen>>) {
    for entity in &screen {
        commands.entity(entity).despawn();
    }
}

/// Rebuilds the node tree when the lobby changed, or when the window crossed
/// into a different frame.
///
/// The same retained-UI trick the HUD uses, with change detection standing in
/// for a revision struct. Resizing *within* a frame is left to flexbox — the
/// layout is written in percentages and gaps for exactly that reason.
#[allow(clippy::too_many_arguments)] // a Bevy system: every one is an injection
fn ui(
    mut commands: Commands,
    state: Res<LobbyState>,
    scrolled_to: Res<Scrolled>,
    fonts: Option<Res<UiFonts>>,
    windows: Query<&Window>,
    root: Query<Entity, With<LobbyRoot>>,
    // Only the printing picker draws a remote image, and a headless test has
    // no asset server {2014} nor should it reach the CDN to build a tree.
    assets: Option<Res<AssetServer>>,
    ui_materials: Option<ResMut<UiCardMaterials>>,
    material_assets: Option<ResMut<Assets<CardUiMaterial>>>,
    prefs: Res<crate::prefs::Prefs>,
    mut drawn: Local<Option<Frame>>,
) {
    let mut cards = match (ui_materials, material_assets) {
        (Some(cache), Some(assets)) => Some((cache, assets)),
        _ => None,
    };
    let mut cards = cards.as_mut().map(|(cache, assets)| UiCards {
        cache: cache.as_mut(),
        assets: assets.as_mut(),
    });
    let width = windows
        .iter()
        .next()
        .map_or(1280.0, |w| w.resolution.width());
    let metrics = Metrics::of(width);
    if !state.is_changed()
        && !prefs.is_changed()
        && !root.is_empty()
        && *drawn == Some(metrics.frame)
    {
        return;
    }
    // The fonts are inserted by the duel plugin's startup system, so the first
    // frame or two has none. Leaving the tree empty until then is correct; the
    // `root.is_empty()` arm above brings us back.
    let Some(fonts) = fonts else {
        return;
    };
    for entity in &root {
        commands.entity(entity).despawn();
    }
    *drawn = Some(metrics.frame);

    let full_bleed =
        state.settings.is_open() || matches!(state.lobby.screen(), Screen::Table | Screen::Build);
    // A phone puts the sign-in form near the top instead of centring it: the
    // soft keyboard takes the bottom half of the screen, and a centred form
    // ends up underneath it.
    let top = full_bleed || metrics.frame == Frame::Phone;
    let root = commands
        .spawn((
            LobbyScreen,
            LobbyRoot,
            Node {
                position_type: PositionType::Absolute,
                left: px(0),
                top: px(0),
                width: percent(100),
                height: percent(100),
                flex_direction: FlexDirection::Column,
                align_items: if full_bleed {
                    AlignItems::Stretch
                } else {
                    AlignItems::Center
                },
                justify_content: if top {
                    JustifyContent::FlexStart
                } else {
                    JustifyContent::Center
                },
                padding: if full_bleed {
                    UiRect::ZERO
                } else {
                    UiRect::all(px(metrics.pad))
                },
                ..default()
            },
            BackgroundColor(BACKDROP),
        ))
        .id();

    // Settings sit over the lobby rather than beside it: they are the
    // account's, not the gateway's, and coming back has to land exactly where
    // the player left — including halfway through a deck.
    if state.settings.is_open() {
        crate::settingsui::screen(
            &mut commands,
            root,
            prefs.all(),
            state.settings.capturing(),
            state.lobby.token().is_some(),
            &fonts,
            metrics,
        );
        return;
    }

    match state.lobby.screen() {
        Screen::SignIn { registering } => {
            let panel = sign_in(&mut commands, &state, &fonts, metrics, *registering);
            commands.entity(root).add_child(panel);
        }
        Screen::Table => table(&mut commands, root, &state, &fonts, metrics, &scrolled_to),
        Screen::Build => crate::buildui::builder(
            &mut commands,
            root,
            &state,
            &fonts,
            metrics,
            &scrolled_to,
            assets.as_deref(),
            cards.as_mut(),
        ),
        Screen::Seated(_) => {
            let note = commands
                .spawn((
                    Text::new("taking your seat…"),
                    tf(&fonts, metrics.head),
                    TextColor(palette::MUTED),
                ))
                .id();
            commands.entity(root).add_child(note);
        }
    }
}

/// The sign-in card.
#[allow(clippy::too_many_lines)] // one flat form, read top to bottom
fn sign_in(
    commands: &mut Commands,
    state: &LobbyState,
    fonts: &UiFonts,
    metrics: Metrics,
    registering: bool,
) -> Entity {
    let lobby = &state.lobby;
    let panel = commands
        .spawn((
            Node {
                // Fills a phone, floats on anything wider.
                width: percent(100),
                max_width: px(420),
                margin: if metrics.frame == Frame::Phone {
                    UiRect::top(px(metrics.pad * 2.0))
                } else {
                    UiRect::ZERO
                },
                flex_direction: FlexDirection::Column,
                row_gap: px(metrics.gap),
                padding: UiRect::all(px(metrics.pad * 1.4)),
                border_radius: BorderRadius::all(px(12)),
                ..default()
            },
            BackgroundColor(palette::PANEL_LIT),
            soft_shadow(),
        ))
        .id();

    let title = commands
        .spawn((
            Text::new("baylee"),
            tf(fonts, metrics.head * 1.8),
            TextColor(palette::INK),
            Pickable::IGNORE,
        ))
        .id();
    let where_ = commands
        .spawn((
            Text::new(state.gateway.clone()),
            tf(fonts, metrics.small * 0.9),
            TextColor(palette::MUTED),
            Pickable::IGNORE,
        ))
        .id();
    commands.entity(panel).add_child(title);
    commands.entity(panel).add_child(where_);

    let email = text_field(
        commands,
        fonts,
        metrics,
        "E-MAIL",
        lobby.field(Field::Email),
        lobby.focus() == Field::Email,
        Field::Email,
    );
    commands.entity(panel).add_child(email);
    if registering {
        let name = text_field(
            commands,
            fonts,
            metrics,
            "DISPLAY NAME",
            lobby.field(Field::DisplayName),
            lobby.focus() == Field::DisplayName,
            Field::DisplayName,
        );
        commands.entity(panel).add_child(name);
    }
    let secret = "•".repeat(lobby.field(Field::Password).chars().count());
    let password = text_field(
        commands,
        fonts,
        metrics,
        "PASSWORD",
        &secret,
        lobby.focus() == Field::Password,
        Field::Password,
    );
    commands.entity(panel).add_child(password);

    let submit = button(
        commands,
        fonts,
        metrics,
        if registering {
            "Create account"
        } else {
            "Sign in"
        },
        Press::Submit,
        palette::ACCENT,
        !lobby.busy(),
    );
    commands.entity(panel).add_child(submit);

    if lobby.registration_enabled() || registering {
        let swap = button(
            commands,
            fonts,
            metrics,
            if registering {
                "I already have an account"
            } else {
                "Create an account"
            },
            Press::ToggleRegistering,
            palette::PANEL,
            true,
        );
        commands.entity(panel).add_child(swap);
    }

    let status = commands
        .spawn((
            Text::new(lobby.status()),
            tf(fonts, metrics.small),
            TextColor(palette::MUTED),
            Pickable::IGNORE,
        ))
        .id();
    commands.entity(panel).add_child(status);

    let rule = commands
        .spawn((
            Node {
                width: percent(100),
                height: px(1),
                margin: UiRect::vertical(px(4)),
                ..default()
            },
            BackgroundColor(Color::srgba(1.0, 1.0, 1.0, 0.08)),
            Pickable::IGNORE,
        ))
        .id();
    let offline = button(
        commands,
        fonts,
        metrics,
        "Play the house AI offline",
        Press::PlayOffline,
        palette::PANEL,
        true,
    );
    // Reachable before signing in as well: an offline duel against the house
    // AI is played with the same keys, and a player with a keyboard they
    // cannot use is not going to make an account first.
    let settings = button(
        commands,
        fonts,
        metrics,
        "Settings",
        Press::OpenSettings,
        palette::PANEL,
        true,
    );
    commands.entity(panel).add_child(rule);
    commands.entity(panel).add_child(offline);
    commands.entity(panel).add_child(settings);
    panel
}

/// The signed-in screen: decks and tables, side by side or stacked.
#[allow(clippy::too_many_lines)] // two panels and a bar, built in order
fn table(
    commands: &mut Commands,
    root: Entity,
    state: &LobbyState,
    fonts: &UiFonts,
    metrics: Metrics,
    scrolled_to: &Scrolled,
) {
    let lobby = &state.lobby;
    let phone = metrics.frame == Frame::Phone;

    // ---- top bar
    let bar = commands
        .spawn((
            Node {
                width: percent(100),
                min_height: px(metrics.tap + metrics.pad),
                align_items: AlignItems::Center,
                column_gap: px(metrics.gap),
                row_gap: px(6),
                flex_wrap: FlexWrap::Wrap,
                padding: UiRect::axes(px(metrics.pad), px(metrics.pad * 0.5)),
                ..default()
            },
            BackgroundColor(palette::PANEL),
        ))
        .id();
    let brand = commands
        .spawn((
            Text::new("baylee"),
            tf(fonts, metrics.head * 1.2),
            TextColor(palette::INK),
            Pickable::IGNORE,
        ))
        .id();
    commands.entity(bar).add_child(brand);
    // The gateway address is reassurance, not information, and the first thing
    // a narrow screen can do without.
    if !phone {
        let host = commands
            .spawn((
                Text::new(state.gateway.clone()),
                tf(fonts, metrics.small * 0.9),
                TextColor(palette::MUTED),
                Pickable::IGNORE,
            ))
            .id();
        commands.entity(bar).add_child(host);
    }
    let gap = commands.spawn((spacer(), Pickable::IGNORE)).id();
    let status = commands
        .spawn((
            Text::new(lobby.status()),
            tf(fonts, metrics.small),
            TextColor(palette::MUTED),
            Pickable::IGNORE,
        ))
        .id();
    let settings = button(
        commands,
        fonts,
        metrics,
        "Settings",
        Press::OpenSettings,
        palette::PANEL_LIT,
        true,
    );
    let out = button(
        commands,
        fonts,
        metrics,
        "Sign out",
        Press::SignOut,
        palette::PANEL_LIT,
        true,
    );
    commands.entity(bar).add_child(gap);
    commands.entity(bar).add_child(status);
    commands.entity(bar).add_child(settings);
    commands.entity(bar).add_child(out);
    commands.entity(root).add_child(bar);

    if let Some(handover) = lobby.awaiting() {
        let banner = commands
            .spawn((
                Node {
                    width: percent(100),
                    align_items: AlignItems::Center,
                    padding: UiRect::axes(px(metrics.pad), px(metrics.pad * 0.5)),
                    ..default()
                },
                BackgroundColor(palette::PANEL_LIT),
                Pickable::IGNORE,
            ))
            .id();
        let line = commands
            .spawn((
                Text::new(format!(
                    "your table {} is open — waiting for an opponent",
                    short_id(&handover.game_id)
                )),
                tf(fonts, metrics.small),
                TextColor(palette::ACTIVE),
                Pickable::IGNORE,
            ))
            .id();
        commands.entity(banner).add_child(line);
        commands.entity(root).add_child(banner);
    }

    // ---- body
    let body = commands
        .spawn((
            Node {
                width: percent(100),
                flex_grow: 1.0,
                flex_direction: if metrics.stacked() {
                    FlexDirection::Column
                } else {
                    FlexDirection::Row
                },
                column_gap: px(metrics.pad),
                row_gap: px(metrics.pad),
                padding: UiRect::all(px(metrics.pad)),
                // A phone runs out of height long before it runs out of
                // games; without this the list is simply cut off.
                overflow: Overflow::scroll_y(),
                ..default()
            },
            Scrollable(List::Table),
            ScrollPosition(Vec2::new(0.0, scrolled_to.get(List::Table))),
        ))
        .id();
    commands.entity(root).add_child(body);

    // ---- decks
    let decks = panel(commands, metrics, metrics.decks_width(), 0.0);
    let decks_head = heading(commands, fonts, metrics, "Your decks");
    commands.entity(decks).add_child(decks_head);
    let deck_tools = row(commands, metrics, true);
    let new_deck = button(
        commands,
        fonts,
        metrics,
        "New deck",
        Press::NewDeck,
        palette::ACCENT,
        true,
    );
    let starter = button(
        commands,
        fonts,
        metrics,
        "Add the starter deck",
        Press::StarterDeck,
        palette::PANEL_LIT,
        !lobby.busy(),
    );
    commands.entity(deck_tools).add_child(new_deck);
    commands.entity(deck_tools).add_child(starter);
    commands.entity(decks).add_child(deck_tools);
    if lobby.decks().is_empty() {
        let empty = note(
            commands,
            fonts,
            metrics,
            "no decks yet — add the starter deck",
        );
        commands.entity(decks).add_child(empty);
    }
    for (index, deck) in lobby.decks().iter().enumerate() {
        let row = commands
            .spawn((
                Node {
                    width: percent(100),
                    min_height: px(metrics.tap),
                    align_items: AlignItems::Center,
                    column_gap: px(metrics.gap),
                    padding: UiRect::axes(px(metrics.pad * 0.7), px(metrics.pad * 0.4)),
                    border: UiRect::all(px(1)),
                    border_radius: btn_radius(),
                    ..default()
                },
                BackgroundColor(palette::PANEL_LIT),
                BorderColor::all(if lobby.selected() == Some(index) {
                    palette::ACCENT
                } else {
                    Color::NONE
                }),
                Press::SelectDeck(index),
            ))
            .id();
        let name = commands
            .spawn((
                Text::new(deck.name.clone()),
                tf(fonts, metrics.text),
                TextColor(palette::INK),
                Pickable::IGNORE,
            ))
            .id();
        let gap = commands.spawn((spacer(), Pickable::IGNORE)).id();
        let size = commands
            .spawn((
                Text::new(if deck.sideboard == 0 {
                    format!("{} rows", deck.cards)
                } else {
                    format!("{} + {}", deck.cards, deck.sideboard)
                }),
                tf(fonts, metrics.small),
                TextColor(palette::MUTED),
                Pickable::IGNORE,
            ))
            .id();
        for child in [name, gap, size] {
            commands.entity(row).add_child(child);
        }
        // Nested inside a row that is itself a `Press`: `in_lineage` takes the
        // nearest one, so these win over selecting the deck.
        for (label, press) in [
            ("Edit", Press::EditDeck(index)),
            ("Delete", Press::DeleteDeck(index)),
        ] {
            let tool = chip(commands, fonts, metrics, label, press, false);
            commands.entity(row).add_child(tool);
        }
        commands.entity(decks).add_child(row);
    }
    commands.entity(body).add_child(decks);

    // ---- tables
    let games = panel(commands, metrics, percent(100), 1.0);
    let head_row = commands
        .spawn((
            Node {
                width: percent(100),
                align_items: AlignItems::Center,
                column_gap: px(metrics.gap),
                row_gap: px(metrics.gap),
                flex_wrap: FlexWrap::Wrap,
                ..default()
            },
            Pickable::IGNORE,
        ))
        .id();
    let head = heading(commands, fonts, metrics, "Tables");
    commands.entity(head_row).add_child(head);
    if !phone {
        let gap = commands.spawn((spacer(), Pickable::IGNORE)).id();
        commands.entity(head_row).add_child(gap);
    }
    for (label, press, tone) in [
        ("Refresh", Press::Refresh, palette::PANEL_LIT),
        ("Play the house", Press::Host(GameMode::Ai), palette::ACCENT),
    ] {
        let b = button(commands, fonts, metrics, label, press, tone, !lobby.busy());
        commands.entity(head_row).add_child(b);
    }
    // How many chairs is the one thing that cannot be changed after the
    // table exists, so it is asked before it does.
    for chairs in MIN_CHAIRS..=MAX_CHAIRS {
        let b = button(
            commands,
            fonts,
            metrics,
            &format!("Open a table for {chairs}"),
            Press::OpenRoom(chairs),
            palette::PANEL_LIT,
            !lobby.busy(),
        );
        commands.entity(head_row).add_child(b);
    }
    commands.entity(games).add_child(head_row);

    if lobby.games().is_empty() {
        let empty = note(commands, fonts, metrics, "no tables are open — start one");
        commands.entity(games).add_child(empty);
    }
    for (index, game) in lobby.games().iter().enumerate() {
        let row = commands
            .spawn((
                Node {
                    width: percent(100),
                    min_height: px(metrics.tap),
                    align_items: AlignItems::Center,
                    column_gap: px(metrics.gap),
                    row_gap: px(6),
                    flex_wrap: FlexWrap::Wrap,
                    padding: UiRect::axes(px(metrics.pad * 0.7), px(metrics.pad * 0.4)),
                    border_radius: btn_radius(),
                    ..default()
                },
                BackgroundColor(palette::PANEL_LIT),
                Pickable::IGNORE,
            ))
            .id();
        // The headline: what the table is called, who opened it, how it is
        // going, and the one button that applies to the whole thing.
        let label = commands
            .spawn((
                Text::new(if game.name.trim().is_empty() {
                    short_id(&game.id)
                } else {
                    game.name.clone()
                }),
                tf(fonts, metrics.text),
                TextColor(palette::INK),
                Pickable::IGNORE,
            ))
            .id();
        let by = match &game.host {
            Some(host) => format!("{}  ·  {host}  ·  {}", game.state, host_note(game)),
            None => format!("{}  ·  {}", game.state, host_note(game)),
        };
        let seats = commands
            .spawn((
                Text::new(by),
                tf(fonts, metrics.small),
                TextColor(palette::MUTED),
                Pickable::IGNORE,
            ))
            .id();
        let gap = commands.spawn((spacer(), Pickable::IGNORE)).id();
        commands.entity(row).add_child(label);
        commands.entity(row).add_child(seats);
        commands.entity(row).add_child(gap);
        if game.joinable() && !game.seated() {
            let join = button(
                commands,
                fonts,
                metrics,
                "Join",
                Press::Join(index),
                palette::ACCENT,
                !lobby.busy(),
            );
            commands.entity(row).add_child(join);
        }
        if game.seated() && game.state == "waiting" {
            let leave = button(
                commands,
                fonts,
                metrics,
                if game.yours { "Close" } else { "Leave" },
                Press::LeaveTable(index),
                palette::PANEL,
                !lobby.busy(),
            );
            commands.entity(row).add_child(leave);
        }
        commands.entity(games).add_child(row);

        // Its chairs, one row each. A room is arranged in the open, so this
        // is drawn for every table and not only for the one you are at.
        if game.state == "waiting" {
            let chairs = seat_rows(commands, fonts, metrics, game, index, lobby.busy());
            commands.entity(games).add_child(chairs);
        }
    }
    commands.entity(body).add_child(games);
}

/// How a table reads under its name: how full it is, and what it waits for.
fn host_note(game: &GameSummary) -> String {
    let ready = game.seats.iter().filter(|s| s.ready).count();
    let total = game.seats.len();
    if game.state != "waiting" {
        return format!("{total} seats");
    }
    let waiting = total - ready;
    if waiting == 0 {
        format!("{ready}/{total} seated")
    } else {
        format!("{ready}/{total} seated · waiting for {waiting}")
    }
}

/// One row per chair: who is in it, what they brought, and — for the host —
/// the controls that arrange it.
#[allow(clippy::too_many_lines)] // one chair, and everything offered on it
fn seat_rows(
    commands: &mut Commands,
    fonts: &UiFonts,
    metrics: Metrics,
    game: &GameSummary,
    index: usize,
    busy: bool,
) -> Entity {
    let holder = commands
        .spawn((
            Node {
                width: percent(100),
                flex_direction: FlexDirection::Column,
                row_gap: px(4),
                padding: UiRect::new(
                    px(metrics.pad * 1.4),
                    px(metrics.pad * 0.7),
                    px(2),
                    px(metrics.pad * 0.4),
                ),
                ..default()
            },
            Pickable::IGNORE,
        ))
        .id();
    for seat in &game.seats {
        let line = row(commands, metrics, true);
        let who = match (seat.kind, seat.player.as_deref()) {
            (SeatKind::Ai, _) => format!(
                "seat {} · AI ({})",
                seat.seat,
                seat.ai.as_deref().unwrap_or("steady")
            ),
            (SeatKind::Human, Some(name)) if seat.you => {
                format!("seat {} · {name} (you)", seat.seat)
            }
            (SeatKind::Human, Some(name)) => format!("seat {} · {name}", seat.seat),
            (SeatKind::Human, None) => format!("seat {} · open", seat.seat),
        };
        let label = commands
            .spawn((
                Text::new(who),
                tf(fonts, metrics.small),
                TextColor(if seat.ready {
                    palette::INK
                } else {
                    palette::MUTED
                }),
                Pickable::IGNORE,
            ))
            .id();
        commands.entity(line).add_child(label);
        if !seat.deck.is_empty() {
            let deck = note(commands, fonts, metrics, &seat.deck);
            commands.entity(line).add_child(deck);
        }
        let gap = commands.spawn((spacer(), Pickable::IGNORE)).id();
        commands.entity(line).add_child(gap);

        // A player brings their own deck; the host brings an AI's. The
        // gateway checks both again — this only decides what to offer.
        let mine = seat.you;
        let ai_chair = seat.kind == SeatKind::Ai;
        if mine || (game.yours && ai_chair) {
            let set = chip(
                commands,
                fonts,
                metrics,
                "use my deck",
                Press::SeatDeck(index, seat.seat),
                false,
            );
            commands.entity(line).add_child(set);
        }
        // Only the host arranges chairs, and never one somebody is sitting in.
        if game.yours && (mine || !seat.taken) {
            let (label, press) = if ai_chair {
                (
                    "\u{2192} open",
                    Press::SeatKind(index, seat.seat, SeatKind::Human),
                )
            } else {
                (
                    "\u{2192} AI",
                    Press::SeatKind(index, seat.seat, SeatKind::Ai),
                )
            };
            // The host's own chair is theirs as a player, not as the host:
            // handing it to the AI would seat them out of their own table.
            if !mine {
                let swap = chip(commands, fonts, metrics, label, press, false);
                commands.entity(line).add_child(swap);
            }
            if ai_chair {
                for name in ["novice", "steady", "sharp"] {
                    let lit = seat.ai.as_deref() == Some(name);
                    let pick = chip(
                        commands,
                        fonts,
                        metrics,
                        name,
                        Press::SeatAi(index, seat.seat, name),
                        lit,
                    );
                    commands.entity(line).add_child(pick);
                }
            }
        }
        // A free chair is one anyone else can take, by name rather than by
        // whichever one the gateway would have picked.
        if seat.open() && !game.seated() && !busy {
            let sit = chip(
                commands,
                fonts,
                metrics,
                "sit here",
                Press::JoinSeat(index, seat.seat),
                false,
            );
            commands.entity(line).add_child(sit);
        }
        commands.entity(holder).add_child(line);
    }
    holder
}

/// A wrapping row of controls.
pub(crate) fn row(commands: &mut Commands, metrics: Metrics, wrap: bool) -> Entity {
    commands
        .spawn((
            Node {
                width: percent(100),
                align_items: AlignItems::Center,
                column_gap: px(metrics.gap * 0.5),
                row_gap: px(metrics.gap * 0.5),
                flex_wrap: if wrap {
                    FlexWrap::Wrap
                } else {
                    FlexWrap::NoWrap
                },
                ..default()
            },
            Pickable::IGNORE,
        ))
        .id()
}

/// A list that scrolls inside its panel rather than pushing it off screen.
///
/// Deliberately *not* `Pickable::IGNORE`: a wheel over the gap between two
/// rows has to land on something, and [`scrolls`] walks up from whatever the
/// pointer hit to find this.
pub(crate) fn scroller(commands: &mut Commands, metrics: Metrics, which: List, at: f32) -> Entity {
    commands
        .spawn((
            Scrollable(which),
            // Not implied by the overflow: Bevy reads this component when it
            // has one and never adds it, so a list without it clips its rows
            // away and nothing can bring them back. It is seeded from where
            // the player left this list, because adding a card rebuilds the
            // tree and a list that jumped to the top on every tap would be
            // unusable.
            ScrollPosition(Vec2::new(0.0, at)),
            Node {
                width: percent(100),
                flex_grow: 1.0,
                min_height: px(0),
                flex_direction: FlexDirection::Column,
                row_gap: px(metrics.gap * 0.35),
                overflow: Overflow::scroll_y(),
                ..default()
            },
        ))
        .id()
}

/// A small toggle. Same shape as [`button`], sized for a row of them.
pub(crate) fn chip(
    commands: &mut Commands,
    fonts: &UiFonts,
    metrics: Metrics,
    label: &str,
    press: Press,
    on: bool,
) -> Entity {
    let text = commands
        .spawn((
            Text::new(label),
            tf(fonts, metrics.small),
            TextColor(if on { palette::INK } else { palette::MUTED }),
            Pickable::IGNORE,
        ))
        .id();
    let id = commands
        .spawn((
            Node {
                // Still a finger target on a phone: the chips are the busiest
                // controls on the screen, and a 30px one is a mis-tap.
                min_height: px(metrics.tap * 0.8),
                min_width: px(metrics.tap * 0.8),
                padding: UiRect::axes(px(metrics.pad * 0.6), px(2)),
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                border_radius: btn_radius(),
                ..default()
            },
            BackgroundColor(if on {
                palette::ACCENT
            } else {
                palette::PANEL_LIT
            }),
            press,
        ))
        .id();
    commands.entity(id).add_child(text);
    id
}

/// A labelled text box that takes the caret when tapped, addressed by a
/// [`Press`] of the caller's choosing.
///
/// [`text_field`] is the same control bound to the sign-in form's [`Field`];
/// this one serves the builder's two boxes.
pub(crate) fn text_box(
    commands: &mut Commands,
    fonts: &UiFonts,
    metrics: Metrics,
    label: &str,
    value: &str,
    focused: bool,
    press: Press,
) -> Entity {
    let column = commands
        .spawn((
            Node {
                width: percent(100),
                flex_direction: FlexDirection::Column,
                row_gap: px(4),
                ..default()
            },
            Pickable::IGNORE,
        ))
        .id();
    let caption = commands
        .spawn((
            Text::new(label),
            tf(fonts, metrics.small * 0.8),
            TextColor(palette::MUTED),
            Pickable::IGNORE,
        ))
        .id();
    let text = commands
        .spawn((
            Text::new(if focused {
                format!("{value}▏")
            } else {
                value.to_string()
            }),
            tf(fonts, metrics.text),
            TextColor(palette::INK),
            Pickable::IGNORE,
        ))
        .id();
    let boxed = commands
        .spawn((
            Node {
                width: percent(100),
                min_height: px(metrics.tap),
                align_items: AlignItems::Center,
                padding: UiRect::axes(px(metrics.pad * 0.7), px(6)),
                border: UiRect::all(px(1)),
                border_radius: btn_radius(),
                ..default()
            },
            BackgroundColor(palette::PANEL),
            BorderColor::all(if focused {
                palette::ACCENT
            } else {
                Color::srgba(1.0, 1.0, 1.0, 0.08)
            }),
            press,
        ))
        .id();
    commands.entity(boxed).add_child(text);
    commands.entity(column).add_child(caption);
    commands.entity(column).add_child(boxed);
    column
}

/// The head of an opaque game id — enough to tell two tables apart, and short
/// enough to fit on a phone.
fn short_id(id: &str) -> String {
    id.chars().take(8).collect()
}

/// The "leave table" button, over a game that has ended.
fn spawn_leave_button(
    mut commands: Commands,
    fonts: Option<Res<UiFonts>>,
    windows: Query<&Window>,
) {
    let Some(fonts) = fonts else {
        return;
    };
    let width = windows
        .iter()
        .next()
        .map_or(1280.0, |w| w.resolution.width());
    let metrics = Metrics::of(width);
    let holder = commands
        .spawn((
            LeaveButton,
            Node {
                position_type: PositionType::Absolute,
                top: px(64),
                width: percent(100),
                justify_content: JustifyContent::Center,
                ..default()
            },
            Pickable::IGNORE,
        ))
        .id();
    let leave = button(
        &mut commands,
        &fonts,
        metrics,
        "Back to the lobby",
        Press::Leave,
        palette::ACCENT,
        true,
    );
    commands.entity(holder).add_child(leave);
}

/// Removes it again on the way out.
fn despawn_leave_button(mut commands: Commands, buttons: Query<Entity, With<LeaveButton>>) {
    for entity in &buttons {
        commands.entity(entity).despawn();
    }
}

// ----------------------------------------------------------- node makers

/// A labelled text box that takes the caret when tapped.
pub(crate) fn text_field(
    commands: &mut Commands,
    fonts: &UiFonts,
    metrics: Metrics,
    label: &str,
    value: &str,
    focused: bool,
    field: Field,
) -> Entity {
    let column = commands
        .spawn((
            Node {
                width: percent(100),
                flex_direction: FlexDirection::Column,
                row_gap: px(4),
                ..default()
            },
            Pickable::IGNORE,
        ))
        .id();
    let caption = commands
        .spawn((
            Text::new(label),
            tf(fonts, metrics.small * 0.8),
            TextColor(palette::MUTED),
            Pickable::IGNORE,
        ))
        .id();
    // The caret is drawn into the string: one glyph is cheaper than a second
    // node, and the lobby has no text selection to speak of.
    let shown = if focused {
        format!("{value}▏")
    } else {
        value.to_string()
    };
    let text = commands
        .spawn((
            Text::new(shown),
            tf(fonts, metrics.text),
            TextColor(palette::INK),
            Pickable::IGNORE,
        ))
        .id();
    let boxed = commands
        .spawn((
            Node {
                width: percent(100),
                min_height: px(metrics.tap),
                align_items: AlignItems::Center,
                padding: UiRect::axes(px(metrics.pad * 0.7), px(6)),
                border: UiRect::all(px(1)),
                border_radius: btn_radius(),
                ..default()
            },
            BackgroundColor(palette::PANEL),
            BorderColor::all(if focused {
                palette::ACCENT
            } else {
                Color::srgba(1.0, 1.0, 1.0, 0.08)
            }),
            Press::Focus(field),
        ))
        .id();
    commands.entity(boxed).add_child(text);
    commands.entity(column).add_child(caption);
    commands.entity(column).add_child(boxed);
    column
}

/// A button. A disabled one carries no [`Press`], so a click cannot find it.
pub(crate) fn button(
    commands: &mut Commands,
    fonts: &UiFonts,
    metrics: Metrics,
    label: &str,
    press: Press,
    tone: Color,
    enabled: bool,
) -> Entity {
    let text = commands
        .spawn((
            Text::new(label),
            tf(fonts, metrics.text),
            TextColor(if enabled { palette::INK } else { palette::DEAD }),
            Pickable::IGNORE,
        ))
        .id();
    let id = {
        let mut entity = commands.spawn((
            Node {
                min_height: px(metrics.tap),
                padding: UiRect::axes(px(metrics.pad), px(metrics.pad * 0.45)),
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                border_radius: btn_radius(),
                ..default()
            },
            BackgroundColor(if enabled { tone } else { palette::PANEL }),
            soft_shadow(),
        ));
        if enabled {
            entity.insert(press);
        }
        entity.id()
    };
    commands.entity(id).add_child(text);
    id
}

/// A column panel: a fixed width beside its neighbour, or the full width
/// above it.
pub(crate) fn panel(commands: &mut Commands, metrics: Metrics, width: Val, grow: f32) -> Entity {
    commands
        .spawn((
            Node {
                width,
                flex_grow: grow,
                flex_shrink: 0.0,
                flex_direction: FlexDirection::Column,
                row_gap: px(metrics.gap * 0.8),
                padding: UiRect::all(px(metrics.pad * 0.8)),
                border_radius: BorderRadius::all(px(12)),
                ..default()
            },
            BackgroundColor(palette::PANEL),
            Pickable::IGNORE,
        ))
        .id()
}

/// A panel heading.
pub(crate) fn heading(
    commands: &mut Commands,
    fonts: &UiFonts,
    metrics: Metrics,
    label: &str,
) -> Entity {
    commands
        .spawn((
            Text::new(label),
            tf(fonts, metrics.head),
            TextColor(palette::INK),
            Pickable::IGNORE,
        ))
        .id()
}

/// A muted line where a list would be.
pub(crate) fn note(
    commands: &mut Commands,
    fonts: &UiFonts,
    metrics: Metrics,
    label: &str,
) -> Entity {
    commands
        .spawn((
            Text::new(label),
            tf(fonts, metrics.small),
            TextColor(palette::MUTED),
            Pickable::IGNORE,
        ))
        .id()
}

/// A deck row's printing, short enough to sit at the end of a list line.
///
/// Not the row's own text form: that repeats the count and the name, both of
/// which are already on the line, and it would be the widest thing on it.
pub(crate) fn print_mark(print: &baylee_core::deckrow::PrintChoice) -> String {
    let mut parts: Vec<String> = Vec::new();
    if let Some(set) = &print.set {
        parts.push(match &print.collector_number {
            Some(number) => format!("{set} {number}"),
            None => set.clone(),
        });
    } else if print.scryfall_id.is_some() {
        // A row pinned to one printing by id has nothing readable to show; it
        // still must not look like the plain row next to it.
        parts.push("pinned".to_string());
    }
    if let Some(lang) = &print.lang {
        parts.push(lang.to_uppercase());
    }
    match print.finish {
        Some(Finish::Foil) => parts.push("foil".to_string()),
        Some(Finish::Etched) => parts.push("etched".to_string()),
        Some(Finish::Normal) | None => {}
    }
    parts.join(" \u{b7} ")
}

/// The stretch between the left and right halves of a row.
pub(crate) fn spacer() -> Node {
    Node {
        flex_grow: 1.0,
        ..default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use baylee_client_core::lobby::{DeckSummary, GameSeat, GameSummary, SeatHandover};

    fn body(request: &ehttp::Request) -> serde_json::Value {
        serde_json::from_slice(&request.body).expect("a JSON body")
    }

    fn answer(status: u16, body: &str) -> ehttp::Response {
        ehttp::Response {
            url: "http://gw/".to_string(),
            ok: (200..300).contains(&status),
            status,
            status_text: String::new(),
            headers: ehttp::Headers::new(&[]),
            bytes: body.as_bytes().to_vec(),
        }
    }

    #[test]
    fn every_request_hits_the_route_the_gateway_serves() {
        let cases = [
            (
                LobbyRequest::LogIn {
                    email: "a@b.c".to_string(),
                    password: "pw".to_string(),
                },
                "POST",
                "http://gw/auth/login",
            ),
            (
                LobbyRequest::Register {
                    email: "a@b.c".to_string(),
                    display_name: "V".to_string(),
                    password: "pw".to_string(),
                },
                "POST",
                "http://gw/auth/register",
            ),
            (LobbyRequest::ListDecks, "GET", "http://gw/decks"),
            (
                LobbyRequest::SaveDeck {
                    deck_id: None,
                    name: "d".to_string(),
                    cards: vec!["1 Forest".to_string()],
                    sideboard: Vec::new(),
                    commander: None,
                },
                "POST",
                "http://gw/decks",
            ),
            (
                LobbyRequest::SaveDeck {
                    deck_id: Some("d1".to_string()),
                    name: "d".to_string(),
                    cards: vec!["1 Forest".to_string()],
                    sideboard: Vec::new(),
                    commander: None,
                },
                "PUT",
                "http://gw/decks/d1",
            ),
            (LobbyRequest::LoadPool, "GET", "http://gw/pool?lang=en"),
            (
                LobbyRequest::LoadDeck {
                    deck_id: "d1".to_string(),
                },
                "GET",
                "http://gw/decks/d1",
            ),
            (
                LobbyRequest::DeleteDeck {
                    deck_id: "d1".to_string(),
                },
                "DELETE",
                "http://gw/decks/d1",
            ),
            (LobbyRequest::ListGames, "GET", "http://gw/lobby/games"),
            (
                LobbyRequest::CreateGame {
                    deck_id: "d1".to_string(),
                    mode: GameMode::Ai,
                    chairs: 2,
                    name: String::new(),
                },
                "POST",
                "http://gw/lobby/games",
            ),
            (
                LobbyRequest::JoinGame {
                    game_id: "g1".to_string(),
                    deck_id: "d1".to_string(),
                    seat: None,
                },
                "POST",
                "http://gw/lobby/games/g1/join",
            ),
        ];
        for (request, method, url) in cases {
            let (built, _) = build("http://gw", None, "en", request.clone());
            assert_eq!(built.method, method, "{request:?}");
            assert_eq!(built.url, url, "{request:?}");
        }
    }

    #[test]
    fn the_bodies_carry_the_field_names_the_gateway_deserialises() {
        let (login, _) = build(
            "http://gw",
            None,
            "en",
            LobbyRequest::LogIn {
                email: "a@b.c".to_string(),
                password: "pw".to_string(),
            },
        );
        assert_eq!(
            body(&login),
            serde_json::json!({ "email": "a@b.c", "password": "pw" })
        );
        let (register, _) = build(
            "http://gw",
            None,
            "en",
            LobbyRequest::Register {
                email: "a@b.c".to_string(),
                display_name: "V".to_string(),
                password: "pw".to_string(),
            },
        );
        assert_eq!(
            body(&register),
            serde_json::json!({ "email": "a@b.c", "display_name": "V", "password": "pw" })
        );
        let (deck, _) = build(
            "http://gw",
            None,
            "en",
            LobbyRequest::SaveDeck {
                deck_id: None,
                name: "Starter".to_string(),
                cards: vec!["1 Forest".to_string()],
                sideboard: vec!["2 Naturalize".to_string()],
                commander: None,
            },
        );
        assert_eq!(
            body(&deck),
            serde_json::json!({
                "name": "Starter",
                "cards": ["1 Forest"],
                "sideboard": ["2 Naturalize"],
                "commander": null
            })
        );
        let (game, _) = build(
            "http://gw",
            None,
            "en",
            LobbyRequest::CreateGame {
                deck_id: "d1".to_string(),
                mode: GameMode::Open,
                chairs: 2,
                name: String::new(),
            },
        );
        assert_eq!(
            body(&game),
            serde_json::json!({ "deck_id": "d1", "mode": "open", "seats": 2, "name": "" })
        );
        let (join, _) = build(
            "http://gw",
            None,
            "en",
            LobbyRequest::JoinGame {
                game_id: "g1".to_string(),
                deck_id: "d1".to_string(),
                seat: None,
            },
        );
        assert_eq!(
            body(&join),
            serde_json::json!({ "deck_id": "d1", "seat": null })
        );

        // Arranging a chair, which is the room's own verb.
        let (chair, expect) = build(
            "http://gw",
            None,
            "en",
            LobbyRequest::SetSeat {
                game_id: "g1".to_string(),
                seat: 2,
                kind: Some(SeatKind::Ai),
                ai: Some("sharp".to_string()),
                deck_id: None,
            },
        );
        assert_eq!(chair.url, "http://gw/lobby/games/g1/seats/2");
        assert_eq!(
            body(&chair),
            serde_json::json!({ "kind": "ai", "ai": "sharp", "deck_id": null })
        );
        assert!(
            matches!(expect, Expect::Games),
            "the answer redraws the room"
        );
    }

    #[test]
    fn a_trailing_slash_on_the_gateway_does_not_double_up() {
        // `gateway_url()` trims one, but a hand-set `.env` is not the only way
        // in and a `//decks` is a 404 with no explanation.
        let (built, _) = build("http://gw/", None, "en", LobbyRequest::ListDecks);
        assert!(!built.url.contains("//decks"), "{}", built.url);
    }

    #[test]
    fn only_a_signed_in_lobby_sends_a_token() {
        let (anonymous, _) = build("http://gw", None, "en", LobbyRequest::ListDecks);
        assert_eq!(anonymous.headers.get("Authorization"), None);
        let (signed, _) = build("http://gw", Some("tok"), "en", LobbyRequest::ListDecks);
        assert_eq!(signed.headers.get("Authorization"), Some("Bearer tok"));
    }

    #[test]
    fn a_json_body_says_so() {
        let (built, _) = build("http://gw", None, "en", LobbyRequest::ListGames);
        assert!(built.body.is_empty(), "a GET carries none");
        let (built, _) = build(
            "http://gw",
            None,
            "en",
            LobbyRequest::SaveDeck {
                deck_id: None,
                name: "d".to_string(),
                cards: vec!["1 Forest".to_string()],
                sideboard: Vec::new(),
                commander: None,
            },
        );
        assert_eq!(built.headers.get("Content-Type"), Some("application/json"));
    }

    #[test]
    fn the_gateways_own_answers_decode() {
        assert_eq!(
            decode(
                Expect::LoggedIn,
                &answer(200, r#"{"token":"tok","expires_at":123}"#)
            ),
            LobbyEvent::LoggedIn {
                token: "tok".to_string()
            }
        );
        assert_eq!(
            decode(
                Expect::Decks,
                &answer(
                    200,
                    r#"[{"id":"d1","name":"Allytifact","cards":96,"commander":null}]"#
                )
            ),
            LobbyEvent::Decks(vec![DeckSummary {
                id: "d1".to_string(),
                name: "Allytifact".to_string(),
                cards: 96,
                sideboard: 0,
                commander: None,
            }])
        );
        assert_eq!(
            decode(
                Expect::Seat,
                &answer(200, r#"{"game_id":"g1","seat":1,"seat_token":"st"}"#)
            ),
            LobbyEvent::Seated(SeatHandover {
                game_id: "g1".to_string(),
                seat: 1,
                seat_token: "st".to_string(),
            })
        );
        assert_eq!(
            decode(Expect::Registered, &answer(200, r#"{"ok":true}"#)),
            LobbyEvent::Registered
        );
        assert_eq!(
            decode(Expect::DeckSaved, &answer(200, r#"{"deck_id":"d1"}"#)),
            LobbyEvent::DeckSaved {
                deck_id: Some("d1".to_string())
            }
        );
    }

    #[test]
    fn a_body_that_makes_no_sense_is_a_failure_not_a_panic() {
        assert!(matches!(
            decode(Expect::LoggedIn, &answer(200, "<html>proxy</html>")),
            LobbyEvent::Failed(_)
        ));
    }

    #[test]
    fn a_refusal_is_shown_in_the_gateways_own_words() {
        assert_eq!(
            gateway_error(&answer(401, r#"{"error":"invalid credentials"}"#)),
            "invalid credentials"
        );
        assert_eq!(
            gateway_error(&answer(502, "<html>bad gateway</html>")),
            "the gateway answered 502"
        );
    }

    /// A headless app wired exactly as the plugin wires a real one. No
    /// renderer, so this exercises the systems and the node tree, not pixels.
    fn headless() -> App {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .add_plugins(bevy::state::app::StatesPlugin)
            .init_state::<DuelPhase>()
            .add_message::<DuelCommand>()
            .add_message::<KeyboardInput>()
            .add_message::<Pointer<Click>>()
            .add_message::<Pointer<Scroll>>()
            .add_message::<Pointer<Drag>>()
            .add_message::<Pointer<DragEnd>>()
            // The duel plugin's startup system would load these; a test has
            // no asset server and does not need one to build a tree.
            .insert_resource(UiFonts {
                text: Handle::default(),
                icons: Handle::default(),
                mana: Handle::default(),
            })
            .add_plugins(LobbyPlugin);
        // The startup probe asks a gateway whether sign-ups are open. Left
        // pointing at the default address it reaches a gateway that happens to
        // be running on this machine, and that answer lands a frame or two
        // later — inside whatever the test is measuring. An address no request
        // can be built from keeps a headless test off the network entirely.
        app.world_mut().resource_mut::<LobbyState>().gateway = String::new();
        app.update();
        app
    }

    fn presses(app: &mut App) -> Vec<Press> {
        let mut query = app.world_mut().query::<&Press>();
        let mut found: Vec<Press> = query.iter(app.world()).copied().collect();
        found.sort_by_key(|p| format!("{p:?}"));
        found
    }

    fn roots(app: &mut App) -> Vec<Entity> {
        let mut query = app.world_mut().query_filtered::<Entity, With<LobbyRoot>>();
        query.iter(app.world()).collect()
    }

    fn typed(ch: char) -> KeyboardInput {
        KeyboardInput {
            key_code: KeyCode::KeyA,
            logical_key: Key::Character(ch.to_string().into()),
            state: bevy::input::ButtonState::Pressed,
            text: Some(ch.to_string().into()),
            repeat: false,
            window: Entity::PLACEHOLDER,
        }
    }

    #[test]
    fn the_sign_in_screen_builds_with_its_controls() {
        let mut app = headless();
        assert_eq!(roots(&mut app).len(), 1, "exactly one tree");
        let found = presses(&mut app);
        for wanted in [
            Press::Focus(Field::Email),
            Press::Focus(Field::Password),
            Press::Submit,
            Press::ToggleRegistering,
            Press::PlayOffline,
        ] {
            assert!(found.contains(&wanted), "{wanted:?} missing from {found:?}");
        }
        assert!(
            !found.contains(&Press::Focus(Field::DisplayName)),
            "the display name is only asked for when registering"
        );
    }

    #[test]
    fn the_lobby_brings_its_own_camera() {
        let mut app = headless();
        let mut query = app
            .world_mut()
            .query_filtered::<Entity, (With<Camera>, With<LobbyScreen>)>();
        assert_eq!(query.iter(app.world()).count(), 1);
    }

    #[test]
    fn typing_reaches_the_form() {
        let mut app = headless();
        for ch in ['h', 'i'] {
            app.world_mut()
                .resource_mut::<Messages<KeyboardInput>>()
                .write(typed(ch));
        }
        app.update();
        assert_eq!(
            app.world()
                .resource::<LobbyState>()
                .lobby
                .field(Field::Email),
            "hi"
        );
    }

    #[test]
    fn a_quiet_frame_does_not_rebuild_the_tree() {
        let mut app = headless();
        let before = roots(&mut app);
        app.update();
        app.update();
        assert_eq!(roots(&mut app), before, "the retained tree survived");
    }

    #[test]
    fn the_table_screen_builds_once_there_is_a_deck() {
        let mut app = headless();
        {
            let mut state = app.world_mut().resource_mut::<LobbyState>();
            state.lobby.apply(LobbyEvent::LoggedIn {
                token: "tok".to_string(),
            });
            state.lobby.apply(LobbyEvent::Decks(vec![DeckSummary {
                id: "d1".to_string(),
                name: "Allytifact".to_string(),
                cards: 96,
                sideboard: 0,
                commander: None,
            }]));
            state.lobby.apply(LobbyEvent::Games(vec![GameSummary {
                id: "0123456789abcdef".to_string(),
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
            }]));
        }
        app.update();
        let found = presses(&mut app);
        for wanted in [
            Press::SignOut,
            Press::Refresh,
            Press::StarterDeck,
            Press::SelectDeck(0),
            Press::Host(GameMode::Ai),
            Press::OpenRoom(2),
            Press::OpenRoom(4),
            Press::Join(0),
            // The chairs of a waiting table are drawn for everyone, so a
            // player can take the one they want rather than whichever the
            // gateway would have handed them.
            Press::JoinSeat(0, 1),
        ] {
            assert!(found.contains(&wanted), "{wanted:?} missing from {found:?}");
        }
    }

    fn labels(app: &mut App) -> Vec<String> {
        let mut query = app.world_mut().query::<&Text>();
        query.iter(app.world()).map(|t| t.0.clone()).collect()
    }

    #[test]
    fn a_table_we_are_waiting_at_is_announced_and_not_sat_at() {
        let mut app = headless();
        {
            let mut state = app.world_mut().resource_mut::<LobbyState>();
            state.lobby.apply(LobbyEvent::LoggedIn {
                token: "tok".to_string(),
            });
            state.lobby.apply(LobbyEvent::Decks(vec![DeckSummary {
                id: "d1".to_string(),
                name: "Allytifact".to_string(),
                cards: 96,
                sideboard: 0,
                commander: None,
            }]));
            state.lobby.apply(LobbyEvent::Games(vec![]));
            state.lobby.host(GameMode::Open);
            state.lobby.apply(LobbyEvent::Seated(SeatHandover {
                game_id: "0123456789".to_string(),
                seat: 0,
                seat_token: "st".to_string(),
            }));
        }
        app.update();
        assert!(
            labels(&mut app)
                .iter()
                .any(|l| l.contains("waiting for an opponent")),
            "the open table is on screen"
        );
        // And no duel was opened: a socket here would be closed straight back.
        assert!(app.world().get_resource::<InstalledHost>().is_none());
    }

    #[test]
    fn a_reply_that_lands_after_the_seat_was_taken_does_not_dial_again() {
        let mut app = headless();
        {
            let mut state = app.world_mut().resource_mut::<LobbyState>();
            state.lobby.apply(LobbyEvent::LoggedIn {
                token: "tok".to_string(),
            });
            state.lobby.apply(LobbyEvent::Seated(SeatHandover {
                game_id: "g1".to_string(),
                seat: 0,
                seat_token: "st".to_string(),
            }));
            // Stand in for a dial that already succeeded.
            state.connected = true;
        }
        // A `ListGames` that was already in flight when the seat was granted.
        app.world()
            .resource::<Mailbox>()
            .0
            .lock()
            .expect("mailbox")
            .push(Reply::Event(LobbyEvent::Games(vec![])));
        app.update();
        assert!(
            matches!(
                app.world().resource::<LobbyState>().lobby.screen(),
                Screen::Seated(_)
            ),
            "a second dial would have failed and unseated us"
        );
    }

    /// A window of a given width, so the breakpoints can be exercised without
    /// a windowing system.
    fn sized(app: &mut App, width: f32) {
        let mut existing = app.world_mut().query::<&mut Window>();
        if let Some(mut window) = existing.iter_mut(app.world_mut()).next() {
            window.resolution.set(width, 900.0);
            return;
        }
        let mut window = Window::default();
        window.resolution.set(width, 900.0);
        app.world_mut().spawn(window);
    }

    #[test]
    fn the_frame_follows_the_width() {
        assert_eq!(Frame::of(390.0), Frame::Phone, "a phone held upright");
        assert_eq!(Frame::of(759.0), Frame::Phone);
        assert_eq!(Frame::of(760.0), Frame::Tablet);
        assert_eq!(
            Frame::of(1024.0),
            Frame::Tablet,
            "a tablet, or a half window"
        );
        assert_eq!(Frame::of(1180.0), Frame::Desktop);
        assert_eq!(Frame::of(2560.0), Frame::Desktop);
    }

    #[test]
    fn a_finger_gets_a_target_it_can_hit() {
        for width in [360.0_f32, 400.0, 700.0, 900.0, 1400.0] {
            let metrics = Metrics::of(width);
            assert!(
                metrics.tap >= 38.0,
                "{width} gave a {}px target",
                metrics.tap
            );
        }
        assert!(
            Metrics::of(390.0).tap >= 44.0,
            "a touch screen needs the full 44"
        );
        assert!(Metrics::of(390.0).stacked(), "a phone has one column");
        assert!(!Metrics::of(1400.0).stacked(), "a desktop has two");
    }

    #[test]
    fn a_phone_drops_what_it_has_no_room_for() {
        let mut app = headless();
        app.world_mut()
            .resource_mut::<LobbyState>()
            .lobby
            .apply(LobbyEvent::LoggedIn {
                token: "tok".to_string(),
            });
        sized(&mut app, 1400.0);
        app.update();
        let wide = labels(&mut app);
        sized(&mut app, 390.0);
        app.update();
        let narrow = labels(&mut app);

        let gateway = app.world().resource::<LobbyState>().gateway.clone();
        assert!(wide.contains(&gateway), "a desktop has room to say where");
        assert!(
            !narrow.contains(&gateway),
            "a phone does not, and the address is reassurance rather than \
             information"
        );
        assert!(
            narrow.iter().any(|l| l == "Your decks"),
            "everything that matters is still there: {narrow:?}"
        );
    }

    #[test]
    fn crossing_a_breakpoint_rebuilds_the_tree() {
        let mut app = headless();
        sized(&mut app, 1400.0);
        app.update();
        let wide = roots(&mut app);
        app.update();
        assert_eq!(roots(&mut app), wide, "the same frame keeps its tree");
        sized(&mut app, 390.0);
        app.update();
        assert_ne!(
            roots(&mut app),
            wide,
            "a different frame is a different layout, not a resize"
        );
    }

    #[test]
    fn a_table_that_is_full_offers_no_join() {
        let mut app = headless();
        {
            let mut state = app.world_mut().resource_mut::<LobbyState>();
            state.lobby.apply(LobbyEvent::LoggedIn {
                token: "tok".to_string(),
            });
            state.lobby.apply(LobbyEvent::Games(vec![GameSummary {
                id: "g".to_string(),
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
            }]));
        }
        app.update();
        assert!(!presses(&mut app).contains(&Press::Join(0)));
    }

    /// Two cards, in the shape `GET /pool` sends them.
    fn pool_cards() -> Vec<baylee_client_core::PoolCard> {
        serde_json::from_value(serde_json::json!([
            {
                "index": 1,
                "name": "Llanowar Elves",
                "english_name": "Llanowar Elves",
                "mana_cost": "{G}",
                "cmc": 1,
                "colors": "G",
                "identity": "G",
                "type_line": "Creature — Elf Druid",
                "kinds": ["Creature"],
                "stats": "1/1",
                "oracle_text": "{T}: Add {G}.",
                "coverage": "implemented",
                "note": null,
                "commander": false,
                "basic_land": false
            },
            {
                "index": 2,
                "name": "Forest",
                "english_name": "Forest",
                "mana_cost": "",
                "cmc": 0,
                "colors": "",
                "identity": "G",
                "type_line": "Basic Land — Forest",
                "kinds": ["Land"],
                "stats": null,
                "oracle_text": "",
                "coverage": "implemented",
                "note": null,
                "commander": false,
                "basic_land": true
            }
        ]))
        .expect("the pool shape")
    }

    /// A lobby signed in, with a deck listed and the pool loaded.
    fn stocked(app: &mut App) {
        let mut state = app.world_mut().resource_mut::<LobbyState>();
        state.lobby.apply(LobbyEvent::LoggedIn {
            token: "tok".to_string(),
        });
        state.lobby.apply(LobbyEvent::Decks(vec![DeckSummary {
            id: "d1".to_string(),
            name: "Allytifact".to_string(),
            cards: 96,
            sideboard: 0,
            commander: None,
        }]));
        state.lobby.apply(LobbyEvent::Pool {
            cards: pool_cards(),
            has_text: true,
        });
    }

    #[test]
    fn a_deck_can_be_opened_edited_and_thrown_away_from_the_list() {
        let mut app = headless();
        stocked(&mut app);
        app.update();
        let found = presses(&mut app);
        for wanted in [
            Press::NewDeck,
            Press::EditDeck(0),
            Press::DeleteDeck(0),
            Press::StarterDeck,
        ] {
            assert!(found.contains(&wanted), "{wanted:?} missing from {found:?}");
        }
    }

    #[test]
    fn the_builder_screen_builds_with_its_controls() {
        let mut app = headless();
        stocked(&mut app);
        sized(&mut app, 1400.0);
        app.world_mut()
            .resource_mut::<LobbyState>()
            .lobby
            .build_deck();
        app.update();
        let found = presses(&mut app);
        for wanted in [
            Press::CloseBuilder,
            Press::FocusBuild(BuildField::Search),
            Press::FocusBuild(BuildField::Name),
            Press::SetZone(Zone::Main),
            Press::SetZone(Zone::Side),
            Press::ToggleColor('G'),
            Press::SetKind(Some("Creature")),
            Press::SetCmc(0),
            Press::TogglePlayable,
            Press::CycleSort,
            // Both pool rows are offered, so the search does not have to be
            // used to reach a two-card pool.
            Press::AddCard(0),
            Press::AddCard(1),
            // Every row can be read as well as taken.
            Press::Inspect(0),
        ] {
            assert!(found.contains(&wanted), "{wanted:?} missing from {found:?}");
        }
        // Nothing is saveable yet: no name, no cards.
        assert!(
            !found.contains(&Press::SaveDeck),
            "a deck the gateway would refuse offers no save"
        );
    }

    #[test]
    fn a_deck_worth_saving_offers_the_save() {
        let mut app = headless();
        stocked(&mut app);
        sized(&mut app, 1400.0);
        {
            let mut state = app.world_mut().resource_mut::<LobbyState>();
            state.lobby.build_deck();
            let builder = state.lobby.builder_mut();
            builder.set_name("Elves");
            assert!(builder.add(0, Zone::Main), "the pool has that card");
        }
        app.update();
        let found = presses(&mut app);
        assert!(found.contains(&Press::SaveDeck), "{found:?}");
        assert!(
            found.contains(&Press::RemoveRow(0)),
            "a card in the deck can come back out: {found:?}"
        );
    }

    #[test]
    fn a_phone_shows_one_half_of_the_builder_at_a_time() {
        let mut app = headless();
        stocked(&mut app);
        sized(&mut app, 390.0);
        app.world_mut()
            .resource_mut::<LobbyState>()
            .lobby
            .build_deck();
        app.update();
        let cards = presses(&mut app);
        assert!(cards.contains(&Press::AddCard(0)), "the pool is showing");
        assert!(
            !cards.contains(&Press::SetZone(Zone::Side)),
            "and the deck is not: {cards:?}"
        );
        assert!(
            cards.contains(&Press::ShowPane(Pane::Deck)),
            "with a way over"
        );
        // The chips are folded away, or the list under them would be four
        // rows tall.
        assert!(
            !cards.contains(&Press::SetKind(Some("Creature"))),
            "{cards:?}"
        );
        assert!(cards.contains(&Press::ToggleFilters), "but reachable");
        app.world_mut().resource_mut::<LobbyState>().filters_open = true;
        app.update();
        assert!(
            presses(&mut app).contains(&Press::SetKind(Some("Creature"))),
            "unfolded, every filter is there"
        );
        app.world_mut().resource_mut::<LobbyState>().filters_open = false;

        app.world_mut().resource_mut::<LobbyState>().pane = Pane::Deck;
        app.update();
        let list = presses(&mut app);
        assert!(list.contains(&Press::SetZone(Zone::Side)), "{list:?}");
        assert!(!list.contains(&Press::AddCard(0)), "{list:?}");

        // Both halves are reachable on a desktop at once.
        sized(&mut app, 1400.0);
        app.update();
        let both = presses(&mut app);
        assert!(both.contains(&Press::AddCard(0)) && both.contains(&Press::SetZone(Zone::Side)));
    }

    #[test]
    fn typing_reaches_the_builder_and_return_adds_the_first_hit() {
        let mut app = headless();
        stocked(&mut app);
        app.world_mut()
            .resource_mut::<LobbyState>()
            .lobby
            .build_deck();
        // A new deck starts on its name, which is what stops it being saved.
        for ch in ['E', 'l', 'f'] {
            app.world_mut()
                .resource_mut::<Messages<KeyboardInput>>()
                .write(typed(ch));
        }
        app.update();
        assert_eq!(
            app.world().resource::<LobbyState>().lobby.builder().name(),
            "Elf"
        );

        app.world_mut()
            .resource_mut::<LobbyState>()
            .lobby
            .builder_mut()
            .focus_on(BuildField::Search);
        for ch in ['E', 'l', 'v'] {
            app.world_mut()
                .resource_mut::<Messages<KeyboardInput>>()
                .write(typed(ch));
        }
        app.update();
        {
            let state = app.world().resource::<LobbyState>();
            assert_eq!(state.lobby.builder().text(), "Elv");
            assert_eq!(state.lobby.builder().results().len(), 1, "one match");
        }
        app.world_mut()
            .resource_mut::<Messages<KeyboardInput>>()
            .write(KeyboardInput {
                key_code: KeyCode::Enter,
                logical_key: Key::Enter,
                state: bevy::input::ButtonState::Pressed,
                text: None,
                repeat: false,
                window: Entity::PLACEHOLDER,
            });
        app.update();
        let state = app.world().resource::<LobbyState>();
        assert_eq!(
            state.lobby.builder().count_of(0, Zone::Main),
            1,
            "return took the one card the search left"
        );
    }

    #[test]
    #[allow(clippy::float_cmp)] // every value here is exact by construction
    fn a_long_list_can_be_scrolled_and_stops_at_both_ends() {
        // Three hundred pixels of window over nine hundred of cards.
        assert_eq!(scrolled(0.0, 120.0, 300.0, 900.0, 1.0), 120.0);
        assert_eq!(
            scrolled(500.0, 400.0, 300.0, 900.0, 1.0),
            600.0,
            "the bottom of the list is the end of it"
        );
        assert_eq!(
            scrolled(40.0, -400.0, 300.0, 900.0, 1.0),
            0.0,
            "and so is the top"
        );
        assert_eq!(
            scrolled(0.0, 50.0, 300.0, 300.0, 1.0),
            0.0,
            "a list that fits does not move at all"
        );
        // Physical sizes, logical offset: a 2× screen has half the room.
        assert_eq!(scrolled(0.0, 999.0, 300.0, 900.0, 0.5), 300.0);
    }

    #[test]
    fn every_scrolling_list_carries_what_bevy_needs_to_scroll_it() {
        let mut app = headless();
        stocked(&mut app);
        sized(&mut app, 1400.0);
        app.world_mut()
            .resource_mut::<LobbyState>()
            .lobby
            .build_deck();
        app.update();
        let mut query = app
            .world_mut()
            .query_filtered::<(&Node, Option<&ScrollPosition>), With<Scrollable>>();
        let lists: Vec<_> = query.iter(app.world()).collect();
        assert_eq!(lists.len(), 2, "the pool and the deck each scroll");
        for (node, position) in lists {
            assert_eq!(node.overflow.y, OverflowAxis::Scroll);
            assert!(
                position.is_some(),
                "an overflow with no ScrollPosition only clips"
            );
        }
    }

    /// Presses one control by name, in one line.
    fn press(app: &mut App, wanted: Press) {
        let target = press_target(app, wanted);
        tap(app, target);
        app.update();
    }

    /// The whole rebinding flow, without a window: open the screen, arm a
    /// row, press a key, and find it bound. Every step of it is a place the
    /// keymap could quietly not be written.
    #[test]
    fn a_key_can_be_rebound_from_the_settings_screen() {
        use baylee_client_core::prefs::{Action, Chord};

        let mut app = headless();
        stocked(&mut app);
        sized(&mut app, 1400.0);
        app.update();

        press(&mut app, Press::OpenSettings);
        assert!(app.world().resource::<LobbyState>().settings.is_open());

        press(&mut app, Press::Rebind(Action::Confirm));
        assert_eq!(
            app.world().resource::<LobbyState>().settings.capturing(),
            Some(Action::Confirm),
            "the row is not waiting for a key"
        );

        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .press(KeyCode::KeyP);
        app.update();
        assert_eq!(
            app.world()
                .resource::<crate::prefs::Prefs>()
                .keymap()
                .chords(Action::Confirm),
            &[Chord::key("KeyP")],
            "the key was not bound"
        );
        assert_eq!(
            app.world().resource::<LobbyState>().settings.capturing(),
            None,
            "the row is still armed after taking a key"
        );

        // And it can be put back, one row at a time.
        press(&mut app, Press::ResetBinding(Action::Confirm));
        assert_eq!(
            app.world()
                .resource::<crate::prefs::Prefs>()
                .keymap()
                .chords(Action::Confirm),
            &[Chord::key("Enter")]
        );
    }

    /// Escape is a key a player may legitimately want to bind, so while a row
    /// is armed it means "never mind" rather than "cancel". Backspace is the
    /// other way out: unbinding is a real answer, since a pointer still
    /// reaches everything.
    #[test]
    fn arming_a_row_can_be_backed_out_of_or_used_to_unbind() {
        use baylee_client_core::prefs::Action;

        let mut app = headless();
        stocked(&mut app);
        sized(&mut app, 1400.0);
        app.update();
        press(&mut app, Press::OpenSettings);

        press(&mut app, Press::Rebind(Action::Cancel));
        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .press(KeyCode::Escape);
        app.update();
        assert_eq!(
            app.world().resource::<LobbyState>().settings.capturing(),
            None
        );
        assert!(
            !app.world()
                .resource::<crate::prefs::Prefs>()
                .keymap()
                .chords(Action::Cancel)
                .is_empty(),
            "escape rebound the row instead of backing out of it"
        );

        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .clear();
        press(&mut app, Press::Rebind(Action::Cancel));
        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .press(KeyCode::Backspace);
        app.update();
        assert!(
            app.world()
                .resource::<crate::prefs::Prefs>()
                .keymap()
                .chords(Action::Cancel)
                .is_empty(),
            "backspace did not unbind the row"
        );
    }

    #[test]
    fn the_settings_screen_offers_every_switch_and_both_rails() {
        use baylee_client_core::automation::{RAIL_ROWS, RailSide};
        use baylee_client_core::prefs::{Action, AutoRule};

        let mut app = headless();
        stocked(&mut app);
        sized(&mut app, 1400.0);
        app.update();
        press(&mut app, Press::OpenSettings);

        let found = presses(&mut app);
        for action in Action::ALL {
            assert!(
                found.contains(&Press::Rebind(action)),
                "{action:?} cannot be rebound from the screen"
            );
        }
        for rule in AutoRule::ALL {
            assert!(
                found.contains(&Press::ToggleAuto(rule)),
                "{rule:?} is missing"
            );
        }
        for side in RailSide::BOTH {
            for row in RAIL_ROWS {
                assert!(
                    found.contains(&Press::ToggleRail(side, row)),
                    "{side:?}/{row:?} is missing from the rail"
                );
            }
        }
        assert!(found.contains(&Press::CloseSettings), "no way back");

        // A switch actually flips, and the screen redraws to say so.
        press(&mut app, Press::ToggleAuto(AutoRule::SkipEmptyBlocks));
        assert!(
            app.world()
                .resource::<crate::prefs::Prefs>()
                .auto()
                .skip_empty_blocks,
            "the switch did not take"
        );
    }

    /// Settings sit *over* the lobby: coming back has to land exactly where
    /// the player left, including halfway through a deck.
    #[test]
    fn closing_the_settings_puts_the_lobby_back_as_it_was() {
        let mut app = headless();
        stocked(&mut app);
        sized(&mut app, 1400.0);
        app.world_mut()
            .resource_mut::<LobbyState>()
            .lobby
            .build_deck();
        app.update();
        assert!(matches!(
            app.world().resource::<LobbyState>().lobby.screen(),
            Screen::Build
        ));

        // The builder has no settings button of its own — the screen is
        // reached from the tables or from sign-in — so it is opened here the
        // way a press would.
        app.world_mut().resource_mut::<LobbyState>().settings = SettingsPane::Open;
        app.update();
        press(&mut app, Press::CloseSettings);
        assert!(
            matches!(
                app.world().resource::<LobbyState>().lobby.screen(),
                Screen::Build
            ),
            "the builder was lost"
        );
        assert!(
            presses(&mut app).contains(&Press::CloseBuilder),
            "not redrawn"
        );
    }

    /// The entity carrying a control, so a test can press it.
    fn press_target(app: &mut App, wanted: Press) -> Entity {
        let mut query = app.world_mut().query::<(Entity, &Press)>();
        let found = query.iter(app.world()).find(|(_, press)| **press == wanted);
        match found {
            Some((entity, _)) => entity,
            None => panic!("{wanted:?} is on screen"),
        }
    }

    /// A plain press on one control.
    fn tap(app: &mut App, entity: Entity) {
        app.world_mut()
            .resource_mut::<Messages<Pointer<Click>>>()
            .write(aimed(
                entity,
                Click {
                    button: PointerButton::Primary,
                    hit: bevy::picking::backend::HitData::new(Entity::PLACEHOLDER, 0.0, None, None),
                    duration: std::time::Duration::ZERO,
                    count: 1,
                },
            ));
    }

    /// A pointer event aimed at one entity. The location is required and
    /// never read by anything the lobby runs.
    fn aimed<E: std::fmt::Debug + Clone + Reflect>(entity: Entity, event: E) -> Pointer<E> {
        use bevy::camera::NormalizedRenderTarget;
        use bevy::picking::pointer::{Location, PointerId};
        use bevy::window::WindowRef;
        Pointer::new(
            PointerId::Mouse,
            Location {
                target: NormalizedRenderTarget::Window(
                    WindowRef::Primary
                        .normalize(Some(Entity::PLACEHOLDER))
                        .expect("a window reference"),
                ),
                position: Vec2::ZERO,
            },
            event,
            entity,
        )
    }

    #[test]
    fn a_swipe_scrolls_the_list_rather_than_adding_the_card_under_it() {
        let mut app = headless();
        stocked(&mut app);
        sized(&mut app, 1400.0);
        app.world_mut()
            .resource_mut::<LobbyState>()
            .lobby
            .build_deck();
        app.update();

        // The row a finger would land on, and the list it sits in. Layout
        // never runs here, so the list is told how big it is.
        let mut rows = app.world_mut().query::<(Entity, &Press)>();
        let card = rows
            .iter(app.world())
            .find(|(_, press)| **press == Press::AddCard(0))
            .map(|(entity, _)| entity)
            .expect("a card row");
        let mut lists = app.world_mut().query_filtered::<Entity, With<Scrollable>>();
        let list = lists.iter(app.world()).next().expect("a scrolling list");
        app.world_mut().entity_mut(list).insert(ComputedNode {
            size: Vec2::new(300.0, 300.0),
            content_size: Vec2::new(300.0, 900.0),
            ..default()
        });

        app.world_mut()
            .resource_mut::<Messages<Pointer<Drag>>>()
            .write(aimed(
                card,
                Drag {
                    button: PointerButton::Primary,
                    distance: Vec2::new(0.0, -40.0),
                    delta: Vec2::new(0.0, -40.0),
                },
            ));
        app.world_mut()
            .resource_mut::<Messages<Pointer<DragEnd>>>()
            .write(aimed(
                card,
                DragEnd {
                    button: PointerButton::Primary,
                    distance: Vec2::new(0.0, -40.0),
                },
            ));
        app.world_mut()
            .resource_mut::<Messages<Pointer<Click>>>()
            .write(aimed(
                card,
                Click {
                    button: PointerButton::Primary,
                    hit: bevy::picking::backend::HitData::new(Entity::PLACEHOLDER, 0.0, None, None),
                    duration: std::time::Duration::ZERO,
                    count: 1,
                },
            ));
        app.update();

        assert_eq!(
            app.world()
                .resource::<LobbyState>()
                .lobby
                .builder()
                .count_of(0, Zone::Main),
            0,
            "a swipe is not a tap"
        );
        assert_eq!(
            app.world()
                .entity(list)
                .get::<ScrollPosition>()
                .map(|p| p.y),
            Some(40.0),
            "and it moved the list under the finger"
        );
    }

    #[test]
    fn leaving_a_deck_with_unsaved_work_takes_two_presses() {
        let mut app = headless();
        stocked(&mut app);
        sized(&mut app, 1400.0);
        {
            let mut state = app.world_mut().resource_mut::<LobbyState>();
            state.lobby.build_deck();
            state.lobby.builder_mut().set_name("Half a deck");
        }
        app.update();
        let back = press_target(&mut app, Press::CloseBuilder);

        tap(&mut app, back);
        app.update();
        assert!(
            matches!(
                app.world().resource::<LobbyState>().lobby.screen(),
                Screen::Build
            ),
            "the first press asks rather than leaves"
        );
        assert!(
            labels(&mut app).iter().any(|l| l == "Leave without saving"),
            "and says so"
        );

        let back = press_target(&mut app, Press::CloseBuilder);
        tap(&mut app, back);
        app.update();
        assert!(matches!(
            app.world().resource::<LobbyState>().lobby.screen(),
            Screen::Table
        ));
    }

    #[test]
    fn a_card_can_be_read_in_the_builder() {
        let mut app = headless();
        stocked(&mut app);
        sized(&mut app, 1400.0);
        app.world_mut()
            .resource_mut::<LobbyState>()
            .lobby
            .build_deck();
        app.world_mut()
            .resource_mut::<LobbyState>()
            .lobby
            .builder_mut()
            .inspect(0);
        app.update();
        let shown = labels(&mut app);
        assert!(
            shown.iter().any(|l| l == "{T}: Add {G}."),
            "the rules text is on screen: {shown:?}"
        );
        assert!(presses(&mut app).contains(&Press::CloseCard), "and closes");
    }

    #[test]
    fn an_edit_answers_with_no_body_and_that_is_not_a_failure() {
        // `PUT /decks/{id}` is a 204. Reading an id out of nothing is not an
        // error here — the builder already holds the one it is editing.
        assert_eq!(
            decode(Expect::DeckSaved, &answer(204, "")),
            LobbyEvent::DeckSaved { deck_id: None }
        );
    }

    #[test]
    fn a_list_keeps_its_place_when_adding_a_card_rebuilds_it() {
        let mut app = headless();
        stocked(&mut app);
        sized(&mut app, 1400.0);
        app.world_mut()
            .resource_mut::<LobbyState>()
            .lobby
            .build_deck();
        app.update();
        app.world_mut()
            .resource_mut::<Scrolled>()
            .set(List::Pool, 90.0);

        // Adding a card changes the lobby, which rebuilds the whole tree.
        let card = press_target(&mut app, Press::AddCard(0));
        tap(&mut app, card);
        app.update();

        let mut lists = app.world_mut().query::<(&ScrollPosition, &Scrollable)>();
        let pool = lists
            .iter(app.world())
            .find(|(_, which)| which.0 == List::Pool)
            .map(|(position, _)| position.y)
            .expect("the pool list");
        assert!(
            (pool - 90.0).abs() < f32::EPSILON,
            "the new list opens where the old one was, not at the top: {pool}"
        );

        // A different search *is* a different list, and starts at the top.
        app.world_mut()
            .resource_mut::<LobbyState>()
            .lobby
            .builder_mut()
            .focus_on(BuildField::Search);
        app.world_mut()
            .resource_mut::<Messages<KeyboardInput>>()
            .write(typed('F'));
        app.update();
        assert!(app.world().resource::<Scrolled>().get(List::Pool).abs() < f32::EPSILON);
    }

    #[test]
    fn the_pool_and_a_saved_deck_decode() {
        let cards = serde_json::to_string(&serde_json::json!({
            "total": 2,
            "pool_hash": "abc",
            "lang": "en",
            "has_text": true,
            "cards": []
        }))
        .expect("a body");
        assert_eq!(
            decode(Expect::Pool, &answer(200, &cards)),
            LobbyEvent::Pool {
                cards: Vec::new(),
                has_text: true
            }
        );
        assert_eq!(
            decode(
                Expect::DeckLoaded,
                &answer(
                    200,
                    r#"{"id":"d1","name":"Elves","cards":["4 Llanowar Elves"],
                       "sideboard":["1 Forest"],"commander":null}"#
                )
            ),
            LobbyEvent::DeckLoaded {
                id: "d1".to_string(),
                name: "Elves".to_string(),
                cards: vec!["4 Llanowar Elves".to_string()],
                sideboard: vec!["1 Forest".to_string()],
                commander: None,
            }
        );
        assert_eq!(
            decode(Expect::DeckDeleted, &answer(204, "")),
            LobbyEvent::DeckDeleted
        );
    }

    #[test]
    fn the_starter_deck_is_one_the_gateway_will_accept() {
        let rows = starter_rows();
        assert!(
            !rows.is_empty(),
            "the acceptance file has an {STARTER} deck"
        );
        assert!(rows.len() <= 250, "the gateway caps the list at 250 rows");
        for row in &rows {
            let (count, name) = row.split_once(' ').expect("\"N Card Name\"");
            let count: u32 = count.parse().expect("a leading count");
            assert!((1..=4).contains(&count), "{row}");
            // The gateway resolves every name against the same registry, and
            // answers a miss with a 400 that says only "unknown card".
            assert!(
                baylee_cards::decks::by_name(name).is_some(),
                "{name} is not in the registry"
            );
        }
    }
    /// The picker is a dialog over the whole builder, and every control it
    /// offers has to be reachable — a carousel with no way to move it, or a
    /// finish with no way to choose it, is a dialog a player is stuck in.
    #[test]
    fn the_printing_picker_offers_every_control_it_needs() {
        let mut app = headless();
        stocked(&mut app);
        sized(&mut app, 1400.0);
        {
            let mut state = app.world_mut().resource_mut::<LobbyState>();
            state.lobby.build_deck();
            let asked = state.lobby.builder_mut().open_picker(0, Zone::Main);
            assert_eq!(asked, Some(LobbyRequest::LoadPrintings { card: 1 }));
            state.lobby.apply(LobbyEvent::Printings {
                card: 1,
                printings: serde_json::from_value(serde_json::json!([
                    {
                        "scryfall_id": "11111111-2222-3333-4444-555555555555",
                        "oracle_id": "o", "lang": "en", "set": "m19",
                        "set_name": "Core Set 2019", "collector_number": "314",
                        "finishes": ["nonfoil", "foil"], "name": "Llanowar Elves"
                    },
                    {
                        "scryfall_id": "66666666-7777-8888-9999-aaaaaaaaaaaa",
                        "oracle_id": "o", "lang": "de", "set": "dom",
                        "set_name": "Dominaria", "collector_number": "168",
                        "finishes": ["nonfoil"], "name": "Elfen von Llanowar"
                    }
                ]))
                .expect("printings decode"),
                from_catalog: true,
            });
        }
        app.update();
        let found = presses(&mut app);
        for wanted in [
            Press::PickerStep(-1),
            Press::PickerStep(1),
            Press::PickerGo(1),
            Press::PickerLang(None),
            Press::PickerLang(Some(0)),
            Press::PickerLang(Some(1)),
            Press::PickerFinish(Finish::Foil),
            Press::PickerConfirm,
            Press::PickerClose,
        ] {
            assert!(found.contains(&wanted), "{wanted:?} missing from {found:?}");
        }
    }

    /// The row the pool draws is the one that opens the picker; without it
    /// the whole feature is unreachable from the builder.
    #[test]
    fn a_pool_row_offers_a_way_to_choose_its_printing() {
        let mut app = headless();
        stocked(&mut app);
        sized(&mut app, 1400.0);
        {
            let mut state = app.world_mut().resource_mut::<LobbyState>();
            state.lobby.build_deck();
        }
        app.update();
        let found = presses(&mut app);
        assert!(found.contains(&Press::PickPrint(0)), "{found:?}");
    }
    /// A deck row previews the printing it names, not the one the registry
    /// happens to point at — that is the whole point of having chosen one.
    #[test]
    fn a_deck_row_previews_the_printing_it_names() {
        use baylee_client_core::deckbuilder::PoolCard;
        let card = PoolCard {
            scryfall_id: "aaaaaaaa-bbbb-cccc-dddd-eeeeeeeeeeee".to_string(),
            ..PoolCard::default()
        };
        let chosen = baylee_core::deckrow::PrintChoice {
            scryfall_id: Some("11111111-2222-3333-4444-555555555555".to_string()),
            lang: Some("de".to_string()),
            finish: Some(Finish::Foil),
            ..baylee_core::deckrow::PrintChoice::default()
        };
        let hover = hover_of_entry(&card, &chosen);
        let url = hover.url.expect("a real id has art");
        assert!(
            url.contains("11111111-2222-3333-4444-555555555555"),
            "{url}"
        );
        assert_eq!(hover.finish, FinishTreatment::Foil);

        // A row that only narrowed by set has no id of its own and falls
        // back to the card's, rather than previewing nothing at all.
        let vague = baylee_core::deckrow::PrintChoice {
            set: Some("M11".to_string()),
            ..baylee_core::deckrow::PrintChoice::default()
        };
        let fallback = hover_of_entry(&card, &vague).url.expect("falls back");
        assert!(
            fallback.contains("aaaaaaaa-bbbb-cccc-dddd-eeeeeeeeeeee"),
            "{fallback}"
        );
    }

    /// The pool's own rows preview plainly: a player has not chosen a finish
    /// there, and showing one would be inventing a choice.
    #[test]
    fn a_pool_row_previews_plainly() {
        use baylee_client_core::deckbuilder::PoolCard;
        let card = PoolCard {
            scryfall_id: "aaaaaaaa-bbbb-cccc-dddd-eeeeeeeeeeee".to_string(),
            ..PoolCard::default()
        };
        assert_eq!(hover_of_card(&card).finish, FinishTreatment::Plain);
        assert!(hover_of_card(&card).url.is_some());
    }

    /// A card with no usable printing must preview nothing rather than fetch
    /// a guaranteed 404 — the nil id is what a preset carries.
    #[test]
    fn a_card_with_no_printing_previews_nothing() {
        use baylee_client_core::deckbuilder::PoolCard;
        let card = PoolCard {
            scryfall_id: "00000000-0000-0000-0000-000000000000".to_string(),
            ..PoolCard::default()
        };
        assert!(hover_of_card(&card).url.is_none());
    }
}
