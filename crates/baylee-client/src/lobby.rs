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

use baylee_client_core::lobby::{Field, GameMode, Lobby, LobbyEvent, LobbyRequest, Screen};
use baylee_core::ids::PlayerId;
use bevy::input::keyboard::{Key, KeyboardInput};
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
            .insert_resource(LobbyState::new())
            .add_systems(Startup, ask_about_registration)
            .add_systems(
                Update,
                (poll, watch, softkeys, keyboard, clicks, ui)
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
    /// Whether a host is already installed for the seat the lobby holds.
    ///
    /// A request still in flight when the seat was granted answers *after*
    /// the connection is made, and without this its reply would run the
    /// same code again — a second socket to the same table, or, when that
    /// second dial fails, a player knocked out of the game they just joined.
    connected: bool,
}

impl LobbyState {
    /// A signed-out lobby pointed at the configured gateway.
    #[must_use]
    pub fn new() -> Self {
        Self {
            lobby: Lobby::new(),
            gateway: crate::settings::gateway_url(),
            connected: false,
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
    /// `{"deck_id":…}` — nothing worth reading.
    DeckCreated,
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
    let (request, expect) = build(&state.gateway, token, request);
    fetch(request, expect, token.is_some(), mailbox);
}

/// The HTTP call one lobby request becomes, and what to make of its answer.
///
/// Separate from [`dispatch`] so the mapping onto the gateway's routes can be
/// tested without a socket: a wrong path or a misspelled field would otherwise
/// only show up as a 404 in somebody's hands.
fn build(base: &str, token: Option<&str>, request: LobbyRequest) -> (ehttp::Request, Expect) {
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
        LobbyRequest::CreateDeck { name, cards } => (
            json_post(
                &format!("{base}/decks"),
                &serde_json::json!({ "name": name, "cards": cards, "commander": null }),
            ),
            Expect::DeckCreated,
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
    let mut request = ehttp::Request::post(url, serde_json::to_vec(body).unwrap_or_default());
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

    let body = response.text().unwrap_or_default();
    match expect {
        Expect::Registered => LobbyEvent::Registered,
        Expect::DeckCreated => LobbyEvent::DeckCreated,
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
    mailbox: Res<Mailbox>,
    mut epoch: Local<u64>,
) {
    if !SoftKeyboard::owns_typing() {
        return;
    }
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
    mailbox: Res<Mailbox>,
) {
    if SoftKeyboard::owns_typing() || !matches!(state.lobby.screen(), Screen::SignIn { .. }) {
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
fn clicks(
    mut pointer: MessageReader<Pointer<Click>>,
    presses: Query<&Press>,
    parents: Query<&ChildOf>,
    mut state: ResMut<LobbyState>,
    mailbox: Res<Mailbox>,
    mut commands: Commands,
    mut opens: MessageWriter<DuelCommand>,
) {
    for click in pointer.read() {
        let Some(press) = in_lineage(click.entity, &presses, &parents) else {
            continue;
        };
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
        }
    }
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

    let table_screen = matches!(state.lobby.screen(), Screen::Table);
    // A phone puts the sign-in form near the top instead of centring it: the
    // soft keyboard takes the bottom half of the screen, and a centred form
    // ends up underneath it.
    let top = table_screen || metrics.frame == Frame::Phone;
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
                align_items: if table_screen {
                    AlignItems::Stretch
                } else {
                    AlignItems::Center
                },
                justify_content: if top {
                    JustifyContent::FlexStart
                } else {
                    JustifyContent::Center
                },
                padding: if table_screen {
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
        Screen::Table => table(&mut commands, root, &state, &fonts, metrics),
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
            Pickable::IGNORE,
        ))
        .id();
    commands.entity(root).add_child(body);

    // ---- decks
    let decks = panel(commands, metrics, metrics.decks_width(), 0.0);
    let decks_head = heading(commands, fonts, metrics, "Your decks");
    commands.entity(decks).add_child(decks_head);
    let starter = button(
        commands,
        fonts,
        metrics,
        "Add the starter deck",
        Press::StarterDeck,
        palette::PANEL_LIT,
        !lobby.busy(),
    );
    commands.entity(decks).add_child(starter);
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
                Text::new(format!("{} rows", deck.cards)),
                tf(fonts, metrics.small),
                TextColor(palette::MUTED),
                Pickable::IGNORE,
            ))
            .id();
        for child in [name, gap, size] {
            commands.entity(row).add_child(child);
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
                LobbyRequest::CreateDeck {
                    name: "d".to_string(),
                    cards: vec!["1 Forest".to_string()],
                },
                "POST",
                "http://gw/decks",
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
            let (built, _) = build("http://gw", None, request.clone());
            assert_eq!(built.method, method, "{request:?}");
            assert_eq!(built.url, url, "{request:?}");
        }
    }

    #[test]
    fn the_bodies_carry_the_field_names_the_gateway_deserialises() {
        let (login, _) = build(
            "http://gw",
            None,
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
            LobbyRequest::CreateDeck {
                name: "Starter".to_string(),
                cards: vec!["1 Forest".to_string()],
            },
        );
        assert_eq!(
            body(&deck),
            serde_json::json!({ "name": "Starter", "cards": ["1 Forest"], "commander": null })
        );
        let (game, _) = build(
            "http://gw",
            None,
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
        let (built, _) = build("http://gw/", None, LobbyRequest::ListDecks);
        assert!(!built.url.contains("//decks"), "{}", built.url);
    }

    #[test]
    fn only_a_signed_in_lobby_sends_a_token() {
        let (anonymous, _) = build("http://gw", None, LobbyRequest::ListDecks);
        assert_eq!(anonymous.headers.get("Authorization"), None);
        let (signed, _) = build("http://gw", Some("tok"), LobbyRequest::ListDecks);
        assert_eq!(signed.headers.get("Authorization"), Some("Bearer tok"));
    }

    #[test]
    fn a_json_body_says_so() {
        let (built, _) = build("http://gw", None, LobbyRequest::ListGames);
        assert!(built.body.is_empty(), "a GET carries none");
        let (built, _) = build(
            "http://gw",
            None,
            LobbyRequest::CreateDeck {
                name: "d".to_string(),
                cards: vec!["1 Forest".to_string()],
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
            decode(Expect::DeckCreated, &answer(200, r#"{"deck_id":"d1"}"#)),
            LobbyEvent::DeckCreated
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
