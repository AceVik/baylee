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
use baylee_client_core as client_core;
use baylee_client_core::deckbuilder::{BuildField, Zone};
use baylee_client_core::filterdialog::FilterPanel;
use baylee_client_core::i18n::{Lang, Phrase};
use baylee_client_core::images::FinishTreatment;
use baylee_client_core::lobby::{
    Field, GameMode, GameQuery, GameSummary, Lobby, LobbyEvent, LobbyRequest, MAX_CHAIRS,
    MIN_CHAIRS, Screen, SeatKind, Tab, Tone,
};
use baylee_client_core::textbuf::{Dir, Step as Reach, TextBuffer};
use baylee_core::ids::PlayerId;
use baylee_core::preset::Finish;
use baylee_engine::win::GameResult;
use bevy::input::keyboard::{Key, KeyboardInput};
use bevy::input::mouse::MouseScrollUnit;
use bevy::prelude::*;

use crate::hud::{UiFonts, btn_radius, palette, soft_shadow, tf};
use crate::net::{NetworkHost, SeatTicket};
use crate::softkeys::{SoftKey, SoftKeyboard};
use crate::{DuelCommand, DuelPhase, InstalledHost};

/// The ground the lobby sits on — dark enough that the felt never flashes
/// through on the way into a duel.
const BACKDROP: Color = Color::srgb(0.018, 0.035, 0.09);

/// The starter deck's name, and the section of the acceptance deck file it is
/// copied from. There is no deck builder yet; without this button a fresh
/// account cannot sit down anywhere.
#[cfg(test)]
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
        crate::ambience::install(app);
        crate::loading::install(app);
        crate::flip::install(app);
        app.init_resource::<dock::Surfaces>()
            .init_resource::<Mailbox>()
            .init_resource::<feed::Feed>()
            .init_resource::<SoftKeyboard>()
            .init_resource::<Scrolled>()
            .insert_resource(LobbyState::new())
            .add_systems(Startup, ask_about_registration)
            .add_systems(
                Update,
                (
                    feed::feed,
                    poll,
                    watch,
                    softkeys,
                    keyboard,
                    clicks,
                    scrolls,
                    hovers,
                    ui,
                    ui::blink,
                    dock::materialize,
                    preview,
                    crate::buildui::virtual_rows::update,
                    thumbnails::load,
                    thumbnails::quantities,
                    waiting,
                )
                    .chain()
                    .run_if(in_state(DuelPhase::Closed)),
            )
            .add_systems(
                Update,
                (leave_clicks, leave_keys).run_if(in_state(DuelPhase::Finished)),
            )
            .add_systems(OnEnter(DuelPhase::Closed), (came_back, spawn_camera))
            .init_resource::<Hovered>()
            .add_message::<Pointer<Over>>()
            .add_message::<Pointer<Out>>()
            .add_systems(OnExit(DuelPhase::Closed), (teardown, despawn_preview))
            .add_systems(
                Update,
                spawn_leave_button.run_if(in_state(DuelPhase::Finished)),
            )
            .add_systems(OnExit(DuelPhase::Finished), despawn_leave_button);
    }
}

// ------------------------------------------------------------- resources

/// The lobby's state, plus the gateway it is talking to.
#[derive(Resource)]
#[allow(clippy::struct_excessive_bools)] // independent UI toggles and transport/selection state
pub struct LobbyState {
    /// The renderer-free state machine.
    pub lobby: Lobby,
    /// Explicitly selected gateway base URL for this session.
    pub gateway: String,
    pub(crate) gateways: Vec<String>,
    pub(crate) gateway_selected: bool,
    gateway_epoch: u64,
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
    pub(super) confirmation: Option<confirm::Destructive>,
    /// Whether a phone is showing the filter chips. They are three wrapped
    /// rows, which on a phone is most of the screen — the list they filter
    /// would be four rows tall underneath them.
    pub(crate) filters_open: bool,
    pub(crate) stats_open: bool,
    pub(crate) commander_pick: Option<bool>, // false: primary; true: compatible partner
    /// Which half of the builder a phone is showing. Purely a matter of how
    /// much room there is, so it lives here and not in the state machine:
    /// every wider frame shows both halves and never reads it.
    pub(crate) pane: Pane,
    pub(crate) hub: Hub,
    pub(crate) room_chairs: usize,
    /// Whether the settings screen is up, and what it is waiting for.
    settings: SettingsPane,
    /// Offline play, once the player has asked for it.
    ///
    /// `Some` is the whole of "this client has no gateway": every request
    /// the lobby makes is answered by it instead of by HTTP, and the seat it
    /// eventually grants is marked local so the shell installs an in-process
    /// engine rather than dialling a socket.
    pub(crate) offline: Option<offline::Offline>,
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

/// The two tasks in the signed-in hub.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum Hub {
    Play,
    Decks,
}

impl LobbyState {
    /// A signed-out lobby pointed at the configured gateway.
    #[must_use]
    pub fn new() -> Self {
        Self::from_settings(crate::settings::ClientSettings::load())
    }

    /// The same, from settings a caller already has.
    ///
    /// Split out because `ClientSettings::load` answers the defaults under
    /// this crate's own tests — deliberately, so that a test cannot pass or
    /// fail on whose machine it runs — and what the stored settings *do* to
    /// this screen is then exactly what no test could reach.
    #[must_use]
    pub fn from_settings(stored: crate::settings::ClientSettings) -> Self {
        let lang = stored.lang;
        let mut lobby = Lobby::new();
        // The lobby draws itself in this language; `lang` below is the code
        // the *catalog* is asked for. One setting, two readers.
        lobby.set_lang(Lang::of(&lang));
        // The address that signed in here last. Retyping it every launch is
        // the kind of small toll that is paid a hundred times and noticed
        // once — and with it filled in the caret can start where the only
        // thing still missing actually is.
        if !stored.last_email.is_empty() {
            lobby.set_field(Field::Email, &stored.last_email);
            lobby.focus_on(Field::Password);
        }
        lobby.set_gateway_ready(false);
        lobby.set_registration_enabled(false);
        lobby.set_field(Field::Gateway, &crate::settings::gateway_url());
        let gateways = stored
            .gateways
            .into_iter()
            .filter_map(|g| gateway::normalize(&g))
            .collect();
        Self {
            gateways,
            gateway_selected: false,
            gateway_epoch: 0,
            lobby,
            gateway: crate::settings::gateway_url(),
            lang,
            connected: false,
            confirm_leave: false,
            confirmation: None,
            filters_open: false,
            stats_open: false,
            commander_pick: None,
            pane: Pane::Cards,
            hub: Hub::Play,
            room_chairs: MIN_CHAIRS,
            settings: SettingsPane::Closed,
            offline: None,
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
    /// Ignore replies from a gateway selection that has since changed.
    Remote(u64, Box<Reply>),
    /// The outcome of a [`LobbyRequest`].
    Event(LobbyEvent),
    /// Public catalog completion; never starts a second fallback lookup.
    PrintingCatalog(LobbyEvent),
    /// `GET /auth/config` said whether sign-ups are open.
    Registration { enabled: bool, art_cache: bool },
    /// The gateway no longer honours the account token we hold.
    Expired,
}

/// What the shell should make of a successful response body.
#[derive(Clone)]
enum Expect {
    Library(client_core::lobby::library::Request),
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
    /// Something at a table changed. The body says what the whole lobby
    /// looks like, which is not the page this client is reading — so it is
    /// the fact that is taken, and the page is asked for again.
    Moved,
    /// A seat handover.
    Seat,
    /// A chair given up; the gateway answers `204` with no body.
    Left,
}

mod confirm;
pub(crate) mod dock;
mod editing;
mod feed;
mod gateway;
mod http;
mod library_ui;
pub(crate) mod offline;
mod preview;
mod print_catalog;
mod systems;
pub(crate) mod thumbnails;
mod ui;

/// The end screen's keyboard marker, for the probe and for nothing else.
///
/// A re-export rather than `pub(crate) mod ui`, because what `devctl` needs
/// is this one component and none of the several hundred items beside it.
/// `Press` is already re-exported below with the rest of the lobby's
/// vocabulary; this is the half that was private, and it is the half that
/// matters: `devctl`'s `exits` row reports, for every on-screen `Press`,
/// whether it *also* carries `DuelExit`. The two readers of these buttons
/// disagree about which entity is the control — `systems::leave_keys`
/// filters by this marker while `systems::leave_clicks` walks the clicked
/// entity's ancestry — so without the pair a caller cannot tell a way out
/// the keyboard is blind to from one that does not exist (#135).
///
/// Carries `devctl`'s own `cfg` and not a narrower one: the marker is
/// exported for the probe, so it is exported exactly when the probe is
/// compiled. Without this the default build fails on an unused import —
/// which is the same fault as the one that put this commit here, in the
/// other direction, and it was found the same way: by compiling both.
#[cfg(all(feature = "dev-control", not(target_arch = "wasm32")))]
pub(crate) use ui::DuelExit;

#[cfg(test)]
mod tests;

use http::{ask_about_registration, dispatch};
use preview::{Hovered, despawn_preview, hovers, preview};
use systems::{
    came_back, clicks, keyboard, leave_clicks, leave_keys, poll, scrolls, softkeys, waiting, watch,
};
use ui::{despawn_leave_button, spawn_camera, spawn_leave_button, teardown, ui};

// The vocabulary the lobby's own halves share, and that `buildui` and
// `settingsui` build their screens out of. Re-exported here so the split
// into files stays an internal matter: every other module still says
// `crate::lobby::button`.

pub(crate) use preview::{hover_of_card, hover_of_entry};
use systems::Scrollable;
pub(crate) use systems::{List, Press, Scrolled};
pub(crate) use ui::{
    FieldLook, FieldTail, Frame, Metrics, button, chip, heading, note, panel, print_mark, row,
    scroller, spacer, text_field,
};
