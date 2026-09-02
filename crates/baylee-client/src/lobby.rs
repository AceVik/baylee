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
        crate::ambience::install(app);
        crate::loading::install(app);
        crate::flip::install(app);
        app.init_resource::<Mailbox>()
            .init_resource::<SoftKeyboard>()
            .init_resource::<Scrolled>()
            .insert_resource(LobbyState::new())
            .add_systems(Startup, ask_about_registration)
            .add_systems(
                Update,
                (
                    poll, watch, softkeys, keyboard, clicks, scrolls, hovers, ui, preview, waiting,
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

mod http;
mod preview;
mod systems;
mod ui;

#[cfg(test)]
mod tests;

use http::{ask_about_registration, dispatch};
use preview::{Hovered, despawn_preview, hovers, preview};
use systems::{came_back, clicks, keyboard, leave_clicks, poll, scrolls, softkeys, waiting, watch};
use ui::{despawn_leave_button, spawn_camera, spawn_leave_button, teardown, ui};

// The vocabulary the lobby's own halves share, and that `buildui` and
// `settingsui` build their screens out of. Re-exported here so the split
// into files stays an internal matter: every other module still says
// `crate::lobby::button`.
use preview::starter_rows;
pub(crate) use preview::{hover_of_card, hover_of_entry};
use systems::Scrollable;
pub(crate) use systems::{List, Press, Scrolled};
pub(crate) use ui::{
    Frame, Metrics, button, chip, heading, note, panel, print_mark, row, scroller, spacer, text_box,
};
