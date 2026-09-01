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

use baylee_client_core::deckbuilder::{
    BuildField, CURVE_BUCKETS, Coverage, DeckBuilder, Group, Zone,
};
use baylee_client_core::lobby::{Field, GameMode, Lobby, LobbyEvent, LobbyRequest, Screen};
use baylee_core::ids::PlayerId;
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
        app.init_resource::<Mailbox>()
            .init_resource::<SoftKeyboard>()
            .init_resource::<Scrolled>()
            .insert_resource(LobbyState::new())
            .add_systems(Startup, ask_about_registration)
            .add_systems(
                Update,
                (poll, watch, softkeys, keyboard, clicks, scrolls, ui)
                    .chain()
                    .run_if(in_state(DuelPhase::Closed)),
            )
            .add_systems(Update, leave_clicks.run_if(in_state(DuelPhase::Finished)))
            .add_systems(OnEnter(DuelPhase::Closed), (came_back, spawn_camera))
            .add_systems(OnExit(DuelPhase::Closed), teardown)
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
    confirm_leave: bool,
    /// Whether a phone is showing the filter chips. They are three wrapped
    /// rows, which on a phone is most of the screen — the list they filter
    /// would be four rows tall underneath them.
    filters_open: bool,
    /// Which half of the builder a phone is showing. Purely a matter of how
    /// much room there is, so it lives here and not in the state machine:
    /// every wider frame shows both halves and never reads it.
    pane: Pane,
}

/// The half of the deck builder a narrow screen is showing.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
enum Pane {
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
    /// One deck, with its rows.
    DeckLoaded,
    /// A deck is gone; the gateway answers `204` with no body.
    DeckDeleted,
    /// A game list.
    Games,
    /// A seat handover.
    Seat,
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
        LobbyRequest::LoadDeck { deck_id } => (
            ehttp::Request::get(format!("{base}/decks/{deck_id}")),
            Expect::DeckLoaded,
        ),
        LobbyRequest::SaveDeck {
            deck_id,
            name,
            cards,
            sideboard,
        } => {
            let body = serde_json::json!({
                "name": name,
                "cards": cards,
                "sideboard": sideboard,
                "commander": null,
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
        LobbyRequest::CreateGame { deck_id, mode } => (
            json_post(
                &format!("{base}/lobby/games"),
                &serde_json::json!({ "deck_id": deck_id, "mode": mode.wire() }),
            ),
            Expect::Seat,
        ),
        LobbyRequest::JoinGame { game_id, deck_id } => (
            json_post(
                &format!("{base}/lobby/games/{game_id}/join"),
                &serde_json::json!({ "deck_id": deck_id }),
            ),
            Expect::Seat,
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

    /// `GET /decks/{id}`.
    #[derive(serde::Deserialize)]
    struct StoredDeck {
        id: String,
        name: String,
        cards: Vec<String>,
        #[serde(default)]
        sideboard: Vec<String>,
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
        Expect::DeckLoaded => serde_json::from_str::<StoredDeck>(body).map_or_else(
            |_| unreadable("the deck"),
            |d| LobbyEvent::DeckLoaded {
                id: d.id,
                name: d.name,
                cards: d.cards,
                sideboard: d.sideboard,
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
    mut state: ResMut<LobbyState>,
    mut scrolled: ResMut<Scrolled>,
    mailbox: Res<Mailbox>,
) {
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
        match *press {
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
            Press::PlayOffline => match crate::host::house_duel() {
                Some(host) => {
                    state.connected = true;
                    commands.insert_resource(InstalledHost(Box::new(host)));
                    opens.write(DuelCommand::Open);
                }
                None => state.lobby.say("could not start the offline duel"),
            },
            // Only ever spawned on the finished screen.
            Press::Leave => {}
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
            Press::RemoveCard(slot) => {
                let zone = state.lobby.builder().zone();
                state.lobby.builder_mut().remove(slot, zone);
            }
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
enum List {
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
struct Scrolled {
    pool: f32,
    deck: f32,
    table: f32,
}

impl Scrolled {
    fn get(&self, list: List) -> f32 {
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
enum Press {
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
    /// Sit down at a listed table by its index.
    Join(usize),
    /// Leave a finished game.
    Leave,
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
    /// Take one copy of a pool card, by its slot, out of the open zone.
    RemoveCard(usize),
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
enum Frame {
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
struct Metrics {
    frame: Frame,
    /// Body text.
    text: f32,
    /// Headings.
    head: f32,
    /// Captions and secondary lines.
    small: f32,
    /// The minimum height of anything meant to be tapped. 44 logical pixels
    /// is the smallest target a finger hits reliably.
    tap: f32,
    /// Padding around and inside panels.
    pad: f32,
    /// Gap between stacked controls.
    gap: f32,
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
fn ui(
    mut commands: Commands,
    state: Res<LobbyState>,
    scrolled_to: Res<Scrolled>,
    fonts: Option<Res<UiFonts>>,
    windows: Query<&Window>,
    root: Query<Entity, With<LobbyRoot>>,
    mut drawn: Local<Option<Frame>>,
) {
    let width = windows
        .iter()
        .next()
        .map_or(1280.0, |w| w.resolution.width());
    let metrics = Metrics::of(width);
    if !state.is_changed() && !root.is_empty() && *drawn == Some(metrics.frame) {
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

    let full_bleed = matches!(state.lobby.screen(), Screen::Table | Screen::Build);
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

    match state.lobby.screen() {
        Screen::SignIn { registering } => {
            let panel = sign_in(&mut commands, &state, &fonts, metrics, *registering);
            commands.entity(root).add_child(panel);
        }
        Screen::Table => table(&mut commands, root, &state, &fonts, metrics, &scrolled_to),
        Screen::Build => builder(&mut commands, root, &state, &fonts, metrics, &scrolled_to),
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
    commands.entity(panel).add_child(rule);
    commands.entity(panel).add_child(offline);
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
        (
            "Open a table",
            Press::Host(GameMode::Open),
            palette::PANEL_LIT,
        ),
    ] {
        let b = button(commands, fonts, metrics, label, press, tone, !lobby.busy());
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
        let taken = game.seats.iter().filter(|s| s.taken).count();
        let label = commands
            .spawn((
                Text::new(short_id(&game.id)),
                tf(fonts, metrics.text),
                TextColor(palette::INK),
                Pickable::IGNORE,
            ))
            .id();
        let seats = commands
            .spawn((
                Text::new(format!(
                    "{}  ·  {taken}/{} seated",
                    game.state,
                    game.seats.len()
                )),
                tf(fonts, metrics.small),
                TextColor(palette::MUTED),
                Pickable::IGNORE,
            ))
            .id();
        let gap = commands.spawn((spacer(), Pickable::IGNORE)).id();
        commands.entity(row).add_child(label);
        commands.entity(row).add_child(seats);
        commands.entity(row).add_child(gap);
        if game.joinable() {
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
        commands.entity(games).add_child(row);
    }
    commands.entity(body).add_child(games);
}

// --------------------------------------------------------- the deck builder

/// How many result rows are drawn before the list stops and says how many it
/// left out.
///
/// The pool is small enough to send whole and to *filter* on every keystroke,
/// but not small enough to spawn whole: a few hundred rows is a few thousand
/// UI nodes, rebuilt on every letter typed. This is a drawing budget, not a
/// filter — the line above the list always names the real total.
const SHOWN_RESULTS: usize = 60;

/// The tallest a mana-curve bar gets, in logical pixels.
const CURVE_HEIGHT: f32 = 54.0;

/// The colours the identity filter offers, and the pips it counts.
const COLORS: [(char, &str); 6] = [
    ('W', "White"),
    ('U', "Blue"),
    ('B', "Black"),
    ('R', "Red"),
    ('G', "Green"),
    ('C', "Colourless"),
];

/// The card types worth a chip of their own. Anything else is reached
/// through the search box.
const KINDS: [&str; 7] = [
    "Creature",
    "Instant",
    "Sorcery",
    "Artifact",
    "Enchantment",
    "Planeswalker",
    "Land",
];

/// The deck builder: the pool on one side, the deck on the other.
fn builder(
    commands: &mut Commands,
    root: Entity,
    state: &LobbyState,
    fonts: &UiFonts,
    metrics: Metrics,
    scrolled_to: &Scrolled,
) {
    let deck = state.lobby.builder();
    let phone = metrics.frame == Frame::Phone;
    let counts = deck.counts();

    let bar = build_bar(commands, state, fonts, metrics);
    commands.entity(root).add_child(bar);

    // A phone has room for one half at a time, and the switch has to say what
    // is in the other one — a deck count is the whole reason to look.
    if phone {
        let switch = row(commands, metrics, true);
        for (pane, label) in [
            (Pane::Cards, format!("Cards ({})", deck.results().len())),
            (
                Pane::Deck,
                format!("Deck ({} / {})", counts.main, counts.side),
            ),
        ] {
            let chosen = state.pane == pane;
            let tab = chip(
                commands,
                fonts,
                metrics,
                &label,
                Press::ShowPane(pane),
                chosen,
            );
            commands.entity(tab).insert(Node {
                flex_grow: 1.0,
                min_height: px(metrics.tap),
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                border_radius: btn_radius(),
                ..default()
            });
            commands.entity(switch).add_child(tab);
        }
        commands.entity(switch).insert(Node {
            width: percent(100),
            column_gap: px(metrics.gap),
            padding: UiRect::axes(px(metrics.pad), px(metrics.pad * 0.4)),
            ..default()
        });
        commands.entity(root).add_child(switch);
    }

    let body = commands
        .spawn((
            Node {
                width: percent(100),
                flex_grow: 1.0,
                min_height: px(0),
                flex_direction: FlexDirection::Row,
                column_gap: px(metrics.pad),
                padding: UiRect::all(px(metrics.pad)),
                ..default()
            },
            Pickable::IGNORE,
        ))
        .id();
    commands.entity(root).add_child(body);

    if !phone || state.pane == Pane::Cards {
        let pool = pool_panel(commands, state, fonts, metrics, scrolled_to);
        commands.entity(body).add_child(pool);
    }
    if !phone || state.pane == Pane::Deck {
        let list = deck_panel(commands, state, fonts, metrics, scrolled_to);
        commands.entity(body).add_child(list);
    }
}

/// The builder's top bar: out, what is being built, and save.
fn build_bar(
    commands: &mut Commands,
    state: &LobbyState,
    fonts: &UiFonts,
    metrics: Metrics,
) -> Entity {
    let deck = state.lobby.builder();
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
    let back = button(
        commands,
        fonts,
        metrics,
        if state.confirm_leave {
            "Leave without saving"
        } else {
            "‹ Decks"
        },
        Press::CloseBuilder,
        if state.confirm_leave {
            palette::DANGER
        } else {
            palette::PANEL_LIT
        },
        true,
    );
    commands.entity(bar).add_child(back);
    if metrics.frame != Frame::Phone {
        let title = commands
            .spawn((
                Text::new(if deck.editing().is_some() {
                    "Editing a deck"
                } else {
                    "A new deck"
                }),
                tf(fonts, metrics.head),
                TextColor(palette::INK),
                Pickable::IGNORE,
            ))
            .id();
        commands.entity(bar).add_child(title);
    }
    let gap = commands.spawn((spacer(), Pickable::IGNORE)).id();
    commands.entity(bar).add_child(gap);
    if !state.lobby.status().is_empty() {
        let status = commands
            .spawn((
                Text::new(state.lobby.status().to_string()),
                tf(fonts, metrics.small),
                TextColor(palette::MUTED),
                Pickable::IGNORE,
            ))
            .id();
        commands.entity(bar).add_child(status);
    }
    // A saved deck with nothing changed says so rather than offering a save
    // that would do nothing; a deck the gateway would refuse offers none
    // either, and the reason is standing in the problems list.
    let (label, live) = match (deck.saveable(), deck.dirty()) {
        (false, _) => ("Save deck", false),
        (true, false) => ("Saved", false),
        (true, true) => ("Save deck", !state.lobby.busy()),
    };
    let save = button(
        commands,
        fonts,
        metrics,
        label,
        Press::SaveDeck,
        palette::ACCENT,
        live,
    );
    commands.entity(bar).add_child(save);
    bar
}

/// The searchable pool: the filters, then what they leave.
#[allow(clippy::too_many_lines)] // a filter bar and a list, in order
fn pool_panel(
    commands: &mut Commands,
    state: &LobbyState,
    fonts: &UiFonts,
    metrics: Metrics,
    scrolled_to: &Scrolled,
) -> Entity {
    let deck = state.lobby.builder();
    let panel = build_panel(commands, metrics, percent(100), 1.0);

    let search = text_box(
        commands,
        fonts,
        metrics,
        "SEARCH",
        deck.text(),
        deck.focus() == BuildField::Search,
        Press::FocusBuild(BuildField::Search),
    );
    commands.entity(panel).add_child(search);

    // A phone folds the chips away: three wrapped rows of them is most of a
    // phone screen, and what is under them is the point. Anything wider shows
    // them, because there the trade does not exist.
    let phone = metrics.frame == Frame::Phone;
    if phone {
        let bar = row(commands, metrics, true);
        let open = chip(
            commands,
            fonts,
            metrics,
            if state.filters_open {
                "Hide filters"
            } else {
                "Filters"
            },
            Press::ToggleFilters,
            state.filters_open,
        );
        commands.entity(bar).add_child(open);
        // While they are folded away, the two that are worth reaching without
        // unfolding stand out here — and "clear" only when there is something
        // to clear, because folded away is not the same as off.
        if !state.filters_open {
            if deck.filtered() {
                let clear = chip(commands, fonts, metrics, "Clear", Press::ClearFilters, true);
                commands.entity(bar).add_child(clear);
            }
            let sort = chip(
                commands,
                fonts,
                metrics,
                &format!("Sort: {}", deck.sort().label()),
                Press::CycleSort,
                false,
            );
            commands.entity(bar).add_child(sort);
        }
        commands.entity(panel).add_child(bar);
    }
    let chips_shown = !phone || state.filters_open;

    if chips_shown {
        // ---- colours
        let colors = row(commands, metrics, true);
        for (letter, name) in COLORS {
            let on = deck.colors().contains(&letter);
            let label = if metrics.frame == Frame::Desktop {
                name.to_string()
            } else {
                letter.to_string()
            };
            let c = chip(
                commands,
                fonts,
                metrics,
                &label,
                Press::ToggleColor(letter),
                on,
            );
            if on {
                commands
                    .entity(c)
                    .insert(BackgroundColor(mana_tone(letter)));
            }
            commands.entity(colors).add_child(c);
        }
        commands.entity(panel).add_child(colors);

        // ---- types
        let kinds = row(commands, metrics, true);
        for kind in KINDS {
            let on = deck.kind() == Some(kind);
            let c = chip(
                commands,
                fonts,
                metrics,
                kind,
                Press::SetKind(Some(kind)),
                on,
            );
            commands.entity(kinds).add_child(c);
        }
        commands.entity(panel).add_child(kinds);

        // ---- mana value, and the two switches
        let tail = row(commands, metrics, true);
        for cmc in 0..u32::try_from(CURVE_BUCKETS).unwrap_or(8) {
            let last = cmc as usize == CURVE_BUCKETS - 1;
            let label = if last {
                format!("{cmc}+")
            } else {
                cmc.to_string()
            };
            let c = chip(
                commands,
                fonts,
                metrics,
                &label,
                Press::SetCmc(cmc),
                deck.cmc() == Some(cmc),
            );
            commands.entity(tail).add_child(c);
        }
        commands.entity(panel).add_child(tail);

        let switches = row(commands, metrics, true);
        let sort = chip(
            commands,
            fonts,
            metrics,
            &format!("Sort: {}", deck.sort().label()),
            Press::CycleSort,
            false,
        );
        // The default is on, and it is the honest one: everything hidden by it is
        // a card the engine cannot play as printed.
        let playable = chip(
            commands,
            fonts,
            metrics,
            "Playable only",
            Press::TogglePlayable,
            deck.playable_only(),
        );
        commands.entity(switches).add_child(sort);
        commands.entity(switches).add_child(playable);
        if deck.filtered() {
            let clear = chip(
                commands,
                fonts,
                metrics,
                "Clear",
                Press::ClearFilters,
                false,
            );
            commands.entity(switches).add_child(clear);
        }
        commands.entity(panel).add_child(switches);
    }

    // ---- the results
    let shown = deck.results().len().min(SHOWN_RESULTS);
    let tally = note(
        commands,
        fonts,
        metrics,
        &if deck.loaded() {
            format!(
                "{} of {} cards{}",
                deck.results().len(),
                deck.pool().len(),
                if shown < deck.results().len() {
                    format!(" — showing {shown}, keep typing to narrow it")
                } else {
                    String::new()
                }
            )
        } else {
            "loading the card pool…".to_string()
        },
    );
    commands.entity(panel).add_child(tally);

    let list = scroller(commands, metrics, List::Pool, scrolled_to.get(List::Pool));
    commands.entity(panel).add_child(list);
    for &slot in deck.results().iter().take(shown) {
        let Some(card) = deck.card(slot) else {
            continue;
        };
        let held = deck.count_of(slot, deck.zone());
        let entry = commands
            .spawn((
                Node {
                    width: percent(100),
                    min_height: px(metrics.tap),
                    align_items: AlignItems::Center,
                    column_gap: px(metrics.gap * 0.8),
                    padding: UiRect::axes(px(metrics.pad * 0.6), px(metrics.pad * 0.3)),
                    border_radius: btn_radius(),
                    ..default()
                },
                BackgroundColor(if held > 0 {
                    palette::PANEL_LIT
                } else {
                    Color::NONE
                }),
                Press::AddCard(slot),
            ))
            .id();
        if held > 0 {
            let badge = commands
                .spawn((
                    Text::new(format!("{held}×")),
                    tf(fonts, metrics.small),
                    TextColor(palette::ACCENT),
                    Pickable::IGNORE,
                ))
                .id();
            commands.entity(entry).add_child(badge);
        }
        let name = commands
            .spawn((
                Text::new(card.name.clone()),
                tf(fonts, metrics.text),
                TextColor(if card.coverage.trustworthy() {
                    palette::INK
                } else {
                    palette::MUTED
                }),
                Pickable::IGNORE,
            ))
            .id();
        commands.entity(entry).add_child(name);
        if metrics.frame != Frame::Phone {
            let kind = commands
                .spawn((
                    Text::new(card.type_line.clone()),
                    tf(fonts, metrics.small * 0.9),
                    TextColor(palette::MUTED),
                    Pickable::IGNORE,
                ))
                .id();
            commands.entity(entry).add_child(kind);
        }
        let gap = commands.spawn((spacer(), Pickable::IGNORE)).id();
        commands.entity(entry).add_child(gap);
        if let Some(mark) = coverage_mark(card.coverage) {
            let flag = commands
                .spawn((
                    Text::new(mark.0),
                    tf(fonts, metrics.small * 0.85),
                    TextColor(mark.1),
                    Pickable::IGNORE,
                ))
                .id();
            commands.entity(entry).add_child(flag);
        }
        let cost = commands
            .spawn((
                Text::new(if card.mana_cost.is_empty() {
                    card.stats.clone().unwrap_or_default()
                } else {
                    card.mana_cost.clone()
                }),
                tf(fonts, metrics.small),
                TextColor(palette::MUTED),
                Pickable::IGNORE,
            ))
            .id();
        commands.entity(entry).add_child(cost);
        // Reading a card is its own target. There is no hover on a touch
        // screen to read one with, and the row itself has to stay the fast
        // way to add — a builder is mostly typing a name and tapping once.
        let read = chip(commands, fonts, metrics, "?", Press::Inspect(slot), false);
        commands.entity(entry).add_child(read);
        commands.entity(list).add_child(entry);
    }
    if deck.loaded() && deck.results().is_empty() {
        let empty = note(
            commands,
            fonts,
            metrics,
            "nothing matches — try fewer filters",
        );
        commands.entity(list).add_child(empty);
    }
    if let Some(slot) = deck.inspecting() {
        let card = card_detail(commands, fonts, metrics, deck, slot);
        commands.entity(panel).add_child(card);
    }
    panel
}

/// One card, read in full: what is printed on it, and what this build does
/// with it.
fn card_detail(
    commands: &mut Commands,
    fonts: &UiFonts,
    metrics: Metrics,
    deck: &DeckBuilder,
    slot: usize,
) -> Entity {
    let holder = commands
        .spawn((
            Node {
                width: percent(100),
                flex_shrink: 0.0,
                flex_direction: FlexDirection::Column,
                row_gap: px(4),
                padding: UiRect::all(px(metrics.pad * 0.7)),
                border_radius: BorderRadius::all(px(10)),
                ..default()
            },
            BackgroundColor(palette::PANEL_LIT),
            Pickable::IGNORE,
        ))
        .id();
    let Some(card) = deck.card(slot) else {
        return holder;
    };

    let head = row(commands, metrics, false);
    let title = commands
        .spawn((
            Text::new(card.name.clone()),
            tf(fonts, metrics.text),
            TextColor(palette::INK),
            Pickable::IGNORE,
        ))
        .id();
    let gap = commands.spawn((spacer(), Pickable::IGNORE)).id();
    let cost = commands
        .spawn((
            Text::new(card.mana_cost.clone()),
            tf(fonts, metrics.small),
            TextColor(palette::MUTED),
            Pickable::IGNORE,
        ))
        .id();
    let close = chip(commands, fonts, metrics, "\u{d7}", Press::CloseCard, false);
    for child in [title, gap, cost, close] {
        commands.entity(head).add_child(child);
    }
    commands.entity(holder).add_child(head);

    let kind = note(
        commands,
        fonts,
        metrics,
        &match &card.stats {
            Some(stats) => format!("{}  \u{b7}  {stats}", card.type_line),
            None => card.type_line.clone(),
        },
    );
    commands.entity(holder).add_child(kind);

    // The gateway serves rules text only when it has a catalog behind it, and
    // saying so beats an empty box that reads as a card with no abilities.
    let body = if card.oracle_text.is_empty() {
        if deck.has_text() {
            String::new()
        } else {
            "no rules text \u{2014} this gateway has no card catalog".to_string()
        }
    } else {
        card.oracle_text.clone()
    };
    if !body.is_empty() {
        let text = commands
            .spawn((
                Text::new(body),
                tf(fonts, metrics.small),
                TextColor(palette::INK),
                Pickable::IGNORE,
            ))
            .id();
        commands.entity(holder).add_child(text);
    }
    if let Some(mark) = coverage_mark(card.coverage) {
        let why = match &card.note {
            Some(note) => format!("{}: {note}", mark.0),
            None => format!("{} \u{2014} this card will not play as printed", mark.0),
        };
        let line = commands
            .spawn((
                Text::new(why),
                tf(fonts, metrics.small),
                TextColor(mark.1),
                Pickable::IGNORE,
            ))
            .id();
        commands.entity(holder).add_child(line);
    }
    holder
}

/// The deck itself: what it is called, what it adds up to, and every card.
#[allow(clippy::too_many_lines)] // the name, four summaries and the list
fn deck_panel(
    commands: &mut Commands,
    state: &LobbyState,
    fonts: &UiFonts,
    metrics: Metrics,
    scrolled_to: &Scrolled,
) -> Entity {
    let deck = state.lobby.builder();
    let counts = deck.counts();
    let width = match metrics.frame {
        Frame::Phone => percent(100),
        Frame::Tablet => px(320),
        Frame::Desktop => px(380),
    };
    let grow = f32::from(u8::from(metrics.frame == Frame::Phone));
    let panel = build_panel(commands, metrics, width, grow);

    let name = text_box(
        commands,
        fonts,
        metrics,
        "DECK NAME",
        deck.name(),
        deck.focus() == BuildField::Name,
        Press::FocusBuild(BuildField::Name),
    );
    commands.entity(panel).add_child(name);

    // ---- which list is being filled
    let zones = row(commands, metrics, false);
    for (zone, label) in [
        (Zone::Main, format!("Main {}", counts.main)),
        (Zone::Side, format!("Sideboard {}", counts.side)),
    ] {
        let tab = chip(
            commands,
            fonts,
            metrics,
            &label,
            Press::SetZone(zone),
            deck.zone() == zone,
        );
        commands.entity(tab).insert(Node {
            flex_grow: 1.0,
            min_height: px(metrics.tap * 0.9),
            align_items: AlignItems::Center,
            justify_content: JustifyContent::Center,
            border_radius: btn_radius(),
            ..default()
        });
        commands.entity(zones).add_child(tab);
    }
    commands.entity(zones).insert(Node {
        width: percent(100),
        column_gap: px(metrics.gap * 0.6),
        ..default()
    });
    commands.entity(panel).add_child(zones);

    let summary = note(
        commands,
        fonts,
        metrics,
        &format!(
            "{} lands · {} creatures · {} other spells",
            counts.lands, counts.creatures, counts.spells
        ),
    );
    commands.entity(panel).add_child(summary);

    let curve = curve_bars(commands, fonts, metrics, deck);
    commands.entity(panel).add_child(curve);

    let pips = pip_row(commands, fonts, metrics, deck);
    commands.entity(panel).add_child(pips);

    for problem in deck.problems() {
        let line = commands
            .spawn((
                Text::new(problem.message.clone()),
                tf(fonts, metrics.small),
                TextColor(if problem.blocking {
                    palette::DANGER
                } else {
                    palette::MUTED
                }),
                Pickable::IGNORE,
            ))
            .id();
        commands.entity(panel).add_child(line);
    }

    // ---- the list itself
    let list = scroller(commands, metrics, List::Deck, scrolled_to.get(List::Deck));
    commands.entity(panel).add_child(list);
    let entries = deck.entries(deck.zone());
    if entries.is_empty() {
        let empty = note(
            commands,
            fonts,
            metrics,
            "empty — tap a card on the left to add it",
        );
        commands.entity(list).add_child(empty);
    }
    let mut group: Option<Group> = None;
    for entry in entries {
        let Some(card) = deck.card(entry.slot) else {
            continue;
        };
        if group != Some(card.group()) {
            group = Some(card.group());
            let heading = commands
                .spawn((
                    Text::new(card.group().label()),
                    tf(fonts, metrics.small * 0.85),
                    TextColor(palette::MUTED),
                    Node {
                        margin: UiRect::top(px(metrics.gap * 0.6)),
                        ..default()
                    },
                    Pickable::IGNORE,
                ))
                .id();
            commands.entity(list).add_child(heading);
        }
        let row_id = commands
            .spawn((
                Node {
                    width: percent(100),
                    min_height: px(metrics.tap * 0.9),
                    align_items: AlignItems::Center,
                    column_gap: px(metrics.gap * 0.6),
                    padding: UiRect::axes(px(metrics.pad * 0.5), px(metrics.pad * 0.25)),
                    border_radius: btn_radius(),
                    ..default()
                },
                BackgroundColor(palette::PANEL_LIT),
                Pickable::IGNORE,
            ))
            .id();
        let count = commands
            .spawn((
                Text::new(format!("{}×", entry.count)),
                tf(fonts, metrics.small),
                TextColor(palette::ACCENT),
                Pickable::IGNORE,
            ))
            .id();
        let title = commands
            .spawn((
                Text::new(card.name.clone()),
                tf(fonts, metrics.text),
                TextColor(if card.coverage.trustworthy() {
                    palette::INK
                } else {
                    palette::MUTED
                }),
                Pickable::IGNORE,
            ))
            .id();
        let gap = commands.spawn((spacer(), Pickable::IGNORE)).id();
        let cost = commands
            .spawn((
                Text::new(card.mana_cost.clone()),
                tf(fonts, metrics.small),
                TextColor(palette::MUTED),
                Pickable::IGNORE,
            ))
            .id();
        for child in [count, title, gap, cost] {
            commands.entity(row_id).add_child(child);
        }
        // Two targets rather than "click removes": a deck list is read far
        // more often than it is edited, and a stray tap that silently took a
        // card out would be found much later, if at all.
        for (label, press) in [
            ("−", Press::RemoveCard(entry.slot)),
            ("+", Press::AddCard(entry.slot)),
        ] {
            let step = chip(commands, fonts, metrics, label, press, false);
            commands.entity(row_id).add_child(step);
        }
        commands.entity(list).add_child(row_id);
    }

    if !entries.is_empty() {
        let clear = chip(
            commands,
            fonts,
            metrics,
            "Empty the deck",
            Press::ClearDeck,
            false,
        );
        commands.entity(panel).add_child(clear);
    }
    for missing in deck.missing() {
        let line = note(commands, fonts, metrics, &format!("dropped: {missing}"));
        commands.entity(panel).add_child(line);
    }
    panel
}

/// The mana curve, as eight bars that are also the mana-value filter.
fn curve_bars(
    commands: &mut Commands,
    fonts: &UiFonts,
    metrics: Metrics,
    deck: &DeckBuilder,
) -> Entity {
    let curve = deck.curve();
    let tallest = curve.iter().copied().max().unwrap_or(0).max(1);
    let holder = commands
        .spawn((
            Node {
                width: percent(100),
                height: px(CURVE_HEIGHT + metrics.small * 2.4),
                align_items: AlignItems::FlexEnd,
                column_gap: px(3),
                ..default()
            },
            Pickable::IGNORE,
        ))
        .id();
    for (bucket, count) in curve.iter().copied().enumerate() {
        let cmc = u32::try_from(bucket).unwrap_or(0);
        let chosen = deck.cmc() == Some(cmc);
        let column = commands
            .spawn((
                Node {
                    flex_grow: 1.0,
                    flex_basis: px(0),
                    height: percent(100),
                    flex_direction: FlexDirection::Column,
                    justify_content: JustifyContent::FlexEnd,
                    align_items: AlignItems::Center,
                    row_gap: px(2),
                    ..default()
                },
                Press::SetCmc(cmc),
            ))
            .id();
        let tally = commands
            .spawn((
                Text::new(if count == 0 {
                    String::new()
                } else {
                    count.to_string()
                }),
                tf(fonts, metrics.small * 0.8),
                TextColor(palette::MUTED),
                Pickable::IGNORE,
            ))
            .id();
        // A bar for an empty bucket still needs a body, or there is nothing
        // under the label to aim at.
        let height = 3.0 + (CURVE_HEIGHT - 3.0) * f32::from(count) / f32::from(tallest);
        let bar = commands
            .spawn((
                Node {
                    width: percent(100),
                    height: px(height),
                    border_radius: BorderRadius::all(px(3)),
                    ..default()
                },
                BackgroundColor(if chosen {
                    palette::ACCENT
                } else if count == 0 {
                    palette::PANEL_LIT
                } else {
                    palette::ACTIVE
                }),
                Pickable::IGNORE,
            ))
            .id();
        let label = commands
            .spawn((
                Text::new(if bucket + 1 == curve.len() {
                    format!("{cmc}+")
                } else {
                    cmc.to_string()
                }),
                tf(fonts, metrics.small * 0.8),
                TextColor(if chosen { palette::INK } else { palette::MUTED }),
                Pickable::IGNORE,
            ))
            .id();
        for child in [tally, bar, label] {
            commands.entity(column).add_child(child);
        }
        commands.entity(holder).add_child(column);
    }
    holder
}

/// The coloured pips the main deck asks for, which is what a mana base is
/// built from.
fn pip_row(
    commands: &mut Commands,
    fonts: &UiFonts,
    metrics: Metrics,
    deck: &DeckBuilder,
) -> Entity {
    let pips = deck.pips();
    let holder = commands
        .spawn((
            Node {
                width: percent(100),
                column_gap: px(metrics.gap * 0.8),
                flex_wrap: FlexWrap::Wrap,
                ..default()
            },
            Pickable::IGNORE,
        ))
        .id();
    for (at, count) in pips.iter().copied().enumerate() {
        if count == 0 {
            continue;
        }
        let letter = "WUBRG".as_bytes()[at] as char;
        let text = commands
            .spawn((
                Text::new(format!("{letter} {count}")),
                tf(fonts, metrics.small),
                TextColor(mana_tone(letter)),
                Pickable::IGNORE,
            ))
            .id();
        commands.entity(holder).add_child(text);
    }
    holder
}

/// The colour a mana symbol is drawn in. Muted rather than saturated: these
/// sit next to body text, and a full-strength red would shout over it.
fn mana_tone(letter: char) -> Color {
    match letter {
        'W' => Color::srgb(0.93, 0.90, 0.78),
        'U' => Color::srgb(0.42, 0.65, 0.88),
        'B' => Color::srgb(0.62, 0.56, 0.68),
        'R' => Color::srgb(0.88, 0.48, 0.42),
        'G' => Color::srgb(0.46, 0.74, 0.52),
        _ => palette::MUTED,
    }
}

/// What a list says about a card the engine does not play as printed.
fn coverage_mark(coverage: Coverage) -> Option<(&'static str, Color)> {
    match coverage {
        Coverage::Implemented => None,
        Coverage::Partial => Some(("partial", palette::ACTIVE)),
        Coverage::Unimplemented => Some(("stub", palette::DANGER)),
    }
}

/// A builder panel: a column that scrolls its own contents instead of
/// growing past the bottom of the window. [`panel`] cannot: its children set
/// the height, which is right for a short list of decks and wrong for two
/// hundred cards.
fn build_panel(commands: &mut Commands, metrics: Metrics, width: Val, grow: f32) -> Entity {
    commands
        .spawn((
            Node {
                width,
                flex_grow: grow,
                flex_shrink: if grow > 0.0 { 1.0 } else { 0.0 },
                min_width: px(0),
                min_height: px(0),
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

/// A wrapping row of controls.
fn row(commands: &mut Commands, metrics: Metrics, wrap: bool) -> Entity {
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
fn scroller(commands: &mut Commands, metrics: Metrics, which: List, at: f32) -> Entity {
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
fn chip(
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
fn text_box(
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
fn text_field(
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
fn button(
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
fn panel(commands: &mut Commands, metrics: Metrics, width: Val, grow: f32) -> Entity {
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
fn heading(commands: &mut Commands, fonts: &UiFonts, metrics: Metrics, label: &str) -> Entity {
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
fn note(commands: &mut Commands, fonts: &UiFonts, metrics: Metrics, label: &str) -> Entity {
    commands
        .spawn((
            Text::new(label),
            tf(fonts, metrics.small),
            TextColor(palette::MUTED),
            Pickable::IGNORE,
        ))
        .id()
}

/// The stretch between the left and right halves of a row.
fn spacer() -> Node {
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
                },
                "POST",
                "http://gw/lobby/games",
            ),
            (
                LobbyRequest::JoinGame {
                    game_id: "g1".to_string(),
                    deck_id: "d1".to_string(),
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
            },
        );
        assert_eq!(
            body(&game),
            serde_json::json!({ "deck_id": "d1", "mode": "open" })
        );
        let (join, _) = build(
            "http://gw",
            None,
            "en",
            LobbyRequest::JoinGame {
                game_id: "g1".to_string(),
                deck_id: "d1".to_string(),
            },
        );
        assert_eq!(body(&join), serde_json::json!({ "deck_id": "d1" }));
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
            })
            .add_plugins(LobbyPlugin);
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
                    },
                    GameSeat {
                        seat: 1,
                        taken: false,
                    },
                ],
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
            Press::Host(GameMode::Open),
            Press::Join(0),
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
                    },
                    GameSeat {
                        seat: 1,
                        taken: true,
                    },
                ],
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
            found.contains(&Press::RemoveCard(0)),
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
}
