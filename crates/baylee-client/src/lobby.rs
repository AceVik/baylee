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
use baylee_client_core::i18n::{Lang, Phrase};
use baylee_client_core::images::FinishTreatment;
use baylee_client_core::lobby::gateway_info::Probe;
use baylee_client_core::lobby::gateway_list;
use baylee_client_core::lobby::{
    Field, GameMode, GameQuery, GameSummary, Lobby, LobbyEvent, LobbyRequest, MAX_CHAIRS,
    MIN_CHAIRS, Registration, Screen, SeatKind, Tab, Tone,
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

#[cfg(not(target_arch = "wasm32"))]
pub(crate) mod central;

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

mod plugin;

/// Which of Central's forms went off the stage last, so that it leaves drawn
/// as it stood, with its own way out and no other.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
#[cfg_attr(target_arch = "wasm32", allow(dead_code))] // a browser has no Central to leave
pub(crate) enum LeftCentral {
    /// None of them: nothing of Central's is on its way out.
    No,
    /// A form a gateway or a link asked for, with its `‹`.
    WayBack,
    /// The form a launch opens, with its door past the sign-in
    /// (`central::without_account`) and no `‹`.
    Door,
}

/// The lobby's state, plus the gateway it is talking to.
#[derive(Resource)]
#[allow(clippy::struct_excessive_bools)] // independent UI toggles and transport/selection state
pub struct LobbyState {
    #[cfg(not(target_arch = "wasm32"))]
    pub(crate) central: Option<central::State>,
    /// The session's current bearer, for what outlives one: a replay.
    pub(crate) bearer: crate::replay::Bearer,
    /// The renderer-free state machine.
    pub lobby: Lobby,
    /// Explicitly selected gateway base URL for this session.
    pub gateway: String,
    pub(crate) gateways: Vec<String>,
    pub(crate) gateway_selected: bool,
    pub(super) left_central: LeftCentral,
    gateway_epoch: u64,
    /// What each address said when asked about itself, by address.
    ///
    /// Keyed by the address and not by [`Self::gateway_epoch`]: every saved
    /// address is asked at once, and an answer belongs to its address
    /// whichever one is selected by the time it lands.
    pub(crate) auth_probes: Arc<std::sync::atomic::AtomicUsize>,
    pub(crate) probes: std::collections::HashMap<String, Probe>,
    /// The typed address being asked before it is saved.
    pub(crate) adding: Option<String>,
    /// How often each saved address was signed in to, which orders
    /// [`Self::gateways`]. Written back to the settings file on every use.
    pub(crate) uses: baylee_client_core::lobby::gateway_use::GatewayUses,
    /// The guest this device holds at each gateway, by address (#269), as
    /// the settings file keeps it. Handed to the lobby when its gateway is
    /// chosen.
    pub(crate) guests: std::collections::BTreeMap<String, client_core::lobby::KeptGuest>,
    /// The saved gateway the arrow keys are on, if they have been used.
    pub(crate) gateway_cursor: Option<usize>,
    /// Which compatibility/online slice of the gateway list is shown.
    pub(crate) gateway_filter: baylee_client_core::lobby::gateway_list::GatewayFilter,
    /// How each compatibility group of the gateway list is ordered.
    pub(crate) gateway_sort: baylee_client_core::lobby::gateway_list::GatewaySort,
    /// Which gateway compatibility groups are folded (`Compatible`,
    /// `Incompatible`, `Offline`). The lower two start folded.
    pub(crate) gateway_collapsed: [bool; 3],
    /// Whether the front door's gear menu is open.
    pub(crate) front_menu: bool,
    /// The source address as a QR code, for the front door's colophon
    /// (#299). `None` until one is made, and for an address that fails the
    /// check at the door.
    pub(crate) source_code: Option<source::Code>,
    /// The Discord invitation as a QR code, for the front door's top-right
    /// tile; made once, `None` until then.
    pub(crate) discord_code: Option<source::Code>,
    /// The `WhatsApp` community invitation as a QR code, for the front door's
    /// top-right tile; made once, `None` until then.
    pub(crate) whatsapp_code: Option<source::Code>,
    /// Baylee's website (`https://baylee.app`) as a QR code; made once.
    pub(crate) website_code: Option<source::Code>,
    /// Which community QR dialog is open over the front door.
    pub(crate) community_dialog: Option<front::CommunityKind>,
    /// Whether the chosen gateway mirrors card art, as its `/auth/config`
    /// said. Used only while signed in there (`systems::art_follows_the_session`).
    pub(crate) art_cache: bool,
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
    /// The history sheet over the builder asked "Discard changes?" before a
    /// restore would replace the unsaved edits (`DESIGN` §C.3, Q-C3).
    pub(crate) confirm_restore: bool,
    pub(super) confirmation: Option<confirm::Destructive>,
    /// A gateway link that found nobody signed in opened Central's sign-in;
    /// back from it, the same link asks about this device's list instead
    /// (`links`).
    pub(super) link_asked_central: bool,
    /// The pairing links this client claimed lately (`central::pairing`):
    /// the same link again is dropped without a word.
    #[cfg(not(target_arch = "wasm32"))]
    pub(crate) claimed_pairings: central::pairing::Claimed,
    /// The deck builder's own view state: its tabs, panes, menus, the
    /// keyboard's place and the save state (`crate::buildui::BuildUi`).
    pub(crate) build: crate::buildui::BuildUi,
    /// The drafts kept on this device (`crate::buildui::draft`).
    pub(crate) drafts: crate::buildui::draft::Drafts,
    /// The deck saves on their way to the gateway.
    pub(crate) deck_saves: baylee_client_core::deckbuilder::saves::SaveLine,
    /// What the import and export dialogs asked of the clipboard and the
    /// file system this frame, for `buildui::transfer::act` to carry out.
    pub(crate) transfer_asks: Vec<crate::buildui::transfer::Ask>,
    pub(crate) commander_pick: Option<bool>, // false: primary; true: compatible partner
    pub(crate) hub: Hub,
    pub(crate) room_deck_seat: Option<u32>,
    /// The seat whose optional starting-position editor is expanded.
    pub(crate) room_setup_seat: Option<u8>,
    pub(crate) room_card_edit: Option<(u8, usize)>,
    /// Whether the settings screen is up, and what it is waiting for.
    settings: SettingsPane,
    /// The language-model seat's panel on the settings screen, opened the
    /// first time the screen is up (`crate::seatpanel`).
    pub(crate) seat: crate::seatpanel::SeatDesk,
    /// The language models this client seats at the table it hosts
    /// (`crate::tableseats`).
    pub(crate) llm: crate::tableseats::TableSeats,
    /// Offline play, once the player has asked for it.
    ///
    /// `Some` is the whole of "this client has no gateway": every request
    /// the lobby makes is answered by it instead of by HTTP, and the seat it
    /// eventually grants is marked local so the shell installs an in-process
    /// engine rather than dialling a socket.
    pub(crate) offline: Option<offline::Offline>,
    /// Local deck store used when signed in to Baylee Central before a gateway
    /// is selected, so Decks and the Deckbuilder work immediately in the lobby.
    pub(crate) local_decks: Option<offline::Offline>,
    /// The header popover that is open (WP0b-3).
    pub(crate) header_menu: Option<header::HeaderMenu>,
    /// The lobby feed was live and is being dialled again: the reconnecting
    /// strip. Written by `feed::feed` only when it turns.
    pub(crate) feed_down: bool,
    /// Retry now was pressed: the feed dials at once.
    pub(crate) retry_feed: bool,
    /// What the player may have missed (`strips::Bell`).
    pub(crate) bell: client_core::lobby::strips::Bell,
    /// Unread social items (chat messages + incoming friend/table/group requests)
    /// routed into the header notification bell.
    pub(crate) social_unread: usize,
    /// Notice lines from social (unread messages, friend requests, table invites)
    /// shown inside the notification bell popover.
    pub(crate) social_notices: Vec<String>,
    /// Asks a header press made of systems it cannot reach (the overlay,
    /// the report form), carried out by `header::carry_out`.
    pub(crate) shell_asks: Vec<header::ShellAsk>,
    /// The chat's buttons in the header (beta.8): `None` with no social
    /// session; `Some(friends)` with one, `friends` for a registered
    /// account. Written by `social` only when it changes.
    pub(crate) chat_header: Option<bool>,
    /// One successful sign-in password, consumed by the local vault worker.
    pub(crate) vault_login: Option<crate::social::vault_access::Login>,
    /// The terms sheet (WG-1, `DESIGN-v5` §11): up after a sign-in the
    /// gateway marks stale, or a kept guest's return to newer terms.
    pub(crate) terms: client_core::terms::Terms,
    /// What the sign-in that is landing said: `terms_stale`, read off the
    /// answer before its event (`front::terms::follow_the_session`).
    pub(crate) terms_stale: Option<bool>,
    /// Whose terms the sheet shows: the gateway's, or Baylee Central's own
    /// first when the sign-in said they wait (`front::terms`).
    pub(crate) terms_subject: front::terms::Subject,
    /// The About sheet is up (the front door's text row, WP1).
    pub(crate) about_open: bool,
    pub(crate) memorial: client_core::memorial::Album,
    /// Caps Lock looks on: a letter arrived upper case with Shift up (the
    /// platform tells the key, not the lock). Shown under a password.
    pub(crate) caps_lock: bool,
    /// The settings screen's section, query and asks (WP5).
    pub(crate) settings_view: SettingsView,
    /// The menu open from a `⋯` or a caret (`menus`).
    pub(crate) menu: Option<menus::ShellMenu>,
    /// The Decks screen's own choices (WP3).
    pub(crate) decks: decks::DecksUi,
    /// The Play screen's own choices and sheets (WP2).
    pub(crate) play: play::PlayUi,
    /// The Replays tab, and a replay being opened.
    pub(crate) replays: replays::ReplaysUi,
    pub(crate) challenges: challenges::Ui,
    /// The Undo the toast lane offers (S-10).
    pub(crate) undo: Option<decks::Undo>,
    /// The tours as they were before the bubble's box or Restart tours,
    /// for the toast's Undo (TOURS.md §1.7).
    pub(crate) tours_before: Option<baylee_client_core::tour::Tours>,
    /// The player holds a chair at a waiting room and has stepped away
    /// from it to another screen (M-7): the seated strip shows, and Return
    /// brings the room back.
    pub(crate) room_away: bool,
    /// The room's chair sheet (a language model, desktop builds), by seat.
    pub(crate) chair_sheet: Option<u32>,
    /// Whether the host's choice of a list stands open.
    pub(crate) lists_sheet: bool,
    /// The chair sheet lists the gateway's hosted models instead.
    pub(crate) hosted_sheet: bool,
    /// The room's seats show their side as a chip that steps on (Set teams).
    pub(crate) teams_edit: bool,
    /// Copy invite was pressed in this room.
    pub(crate) invite_copied: bool,
    /// Text a press asked to put on the clipboard (`parts::write_clipboard`).
    pub(crate) clipboard_out: Option<String>,
}

/// The settings screen's own state (WP5): which section is shown, what the
/// search holds, a shortcut's refused rebind, and asks for systems a press
/// cannot reach (the display trial, the clipboard).
#[derive(Clone, Debug, Default, PartialEq)]
pub(crate) struct SettingsView {
    /// The section shown.
    pub(crate) section: baylee_client_core::settings_map::Section,
    /// A shortcut's rebind refused: the action, the chord and why.
    pub(crate) refused: Option<(
        baylee_client_core::shellkeys::ShellAction,
        baylee_client_core::shellkeys::ShellChord,
        baylee_client_core::shellkeys::Refused,
    )>,
    /// Keep (`true`) or revert the display mode on trial.
    pub(crate) display_answer: Option<bool>,
    /// Text to put on the clipboard.
    pub(crate) copy: Option<String>,
    /// The graphics in force, as last seen (a device that never chose
    /// starts from its GPU's preset when it first moves a knob).
    pub(crate) in_use: baylee_client_core::graphics::Graphics,
    /// A language-model profile's sheet is open over the list.
    pub(crate) profile_sheet: bool,
    /// Its Advanced fields are shown.
    pub(crate) profile_advanced: bool,
    /// The screen stands over a table, opened from its game menu
    /// (`table_settings`): only the sections `Section::at_table` admits are
    /// offered. Set while the table plays, and put down with it.
    pub(crate) over_table: bool,
    /// The card cache's block.
    pub(crate) fill: card_fill::FillView,
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
    /// Showing, with one shortcut's row listening (Settings › Controls).
    RebindingShell(baylee_client_core::shellkeys::ShellAction),
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

    /// The shortcut waiting for a key, if any.
    const fn capturing_shell(self) -> Option<baylee_client_core::shellkeys::ShellAction> {
        match self {
            Self::RebindingShell(action) => Some(action),
            _ => None,
        }
    }
}

/// The two tasks in the signed-in hub.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum Hub {
    Play,
    /// Multiplayer discovery, reached from the account's home.
    Tables,
    /// Explicit connection selection, never the signed-in landing page.
    Gateways,
    Decks,
    /// Watching games again (desktop only, `replays::SHOWN`).
    Replays,
    Challenges,
}

impl LobbyState {
    #[cfg(not(target_arch = "wasm32"))]
    pub(crate) fn central_handle(&self) -> Option<central::Handle> {
        self.central.as_ref().and_then(|state| state.handle.clone())
    }

    /// Whether the settings screen is up.
    #[must_use]
    pub(crate) fn settings_open(&self) -> bool {
        self.settings.is_open()
    }

    /// Whether the settings screen is up over a table (`table_settings`).
    #[must_use]
    pub(crate) fn settings_over_table(&self) -> bool {
        self.settings_view.over_table && self.settings.is_open()
    }

    /// Whether a section may be shown where the screen stands: over a
    /// table, only the ones that leave the table where it is.
    #[must_use]
    pub(crate) fn settings_offered(
        &self,
        section: baylee_client_core::settings_map::Section,
    ) -> bool {
        !self.settings_view.over_table || section.at_table()
    }

    /// The table action waiting for a key on the settings screen.
    #[must_use]
    pub(crate) fn settings_capturing(&self) -> Option<baylee_client_core::prefs::Action> {
        self.settings.capturing()
    }

    /// The shortcut waiting for a key on the settings screen.
    #[must_use]
    pub(crate) fn shell_capturing(&self) -> Option<baylee_client_core::shellkeys::ShellAction> {
        self.settings.capturing_shell()
    }

    /// A shortcut's refused rebind: the action and why.
    #[must_use]
    pub(crate) fn shell_refusal(
        &self,
    ) -> Option<(
        baylee_client_core::shellkeys::ShellAction,
        baylee_client_core::shellkeys::Refused,
    )> {
        self.settings_view
            .refused
            .as_ref()
            .map(|(a, _, why)| (*a, *why))
    }

    /// The settings section shown.
    #[must_use]
    pub(crate) fn settings_section(&self) -> baylee_client_core::settings_map::Section {
        self.settings_view.section
    }

    /// Shows a settings section, if it is offered where the screen stands
    /// ([`Self::settings_offered`]): every road to a section comes here, the
    /// type-ahead and a search result as much as the nav.
    pub(crate) fn set_settings_section(
        &mut self,
        section: baylee_client_core::settings_map::Section,
    ) {
        if self.settings_offered(section) {
            self.settings_view.section = section;
        }
    }

    /// What the settings search holds.
    #[must_use]
    pub(crate) fn settings_query(&self) -> &str {
        self.lobby.field(Field::SettingsSearch)
    }

    /// Sets the settings search.
    pub(crate) fn set_settings_query(&mut self, query: String) {
        if self.settings_query() != query {
            self.lobby.set_field(Field::SettingsSearch, &query);
        }
    }

    /// The display trial's answer, taken once.
    pub(crate) fn take_display_answer(&mut self) -> Option<bool> {
        if self.settings_view.display_answer.is_some() {
            self.settings_view.display_answer.take()
        } else {
            None
        }
    }

    /// A signed-out lobby starting at Baylee Central sign-in on native
    /// builds, so the account's gateways load once signed in.
    #[must_use]
    pub fn new() -> Self {
        let mut state = Self::from_settings(crate::settings::ClientSettings::load());
        #[cfg(not(target_arch = "wasm32"))]
        central::enable(&mut state);
        state
    }

    /// The same, from settings a caller already has.
    ///
    /// Split out because `ClientSettings::load` answers the defaults under
    /// this crate's own tests — deliberately, so that a test cannot pass or
    /// fail on whose machine it runs — and what the stored settings *do* to
    /// this screen is then exactly what no test could reach.
    #[must_use]
    #[allow(clippy::too_many_lines)]
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
        if !stored.last_username.is_empty() {
            lobby.set_field(Field::Username, &stored.last_username);
            lobby.focus_on(Field::Password);
        }
        lobby.set_gateway_ready(false);
        lobby.set_registration(Registration::Off);
        let raw = crate::settings::gateway_url();
        // In a browser, the gateway that served the page (#327); natively
        // `None`, which changes nothing below. The live gateway's former
        // address, configured or serving the page, is the live gateway
        // (`gateway_list::moved`): its row, its guest, its terms.
        let page = gateway_list::page_gateway(crate::settings::page_origin().as_deref(), &raw)
            .map(|page| gateway_list::moved(&page).to_string());
        let configured = gateway_list::moved(&raw).to_string();
        let mut gateways = gateway_list::with_page(
            gateway_list::with_pinned(
                stored
                    .gateways
                    .into_iter()
                    .filter_map(|g| gateway::normalize(&g))
                    .collect(),
            ),
            page.as_deref(),
        );
        // The address field offers the configured gateway to save, unless
        // it is saved already (in a release that is the pinned one).
        if !gateways.contains(&configured) {
            lobby.set_field(Field::Gateway, &configured);
        }
        // Ordered once, here, and not while the list is on screen: a row
        // that moved under the pointer would be a row chosen by mistake.
        stored.gateway_uses.order(&mut gateways);
        // A release puts the keyboard's row on the live gateway, and a page
        // on the gateway that served it, so Enter plays there; choosing is
        // still the player's step.
        let gateway_cursor =
            gateway_list::starting_row(&gateways, !cfg!(debug_assertions), page.as_deref());
        Self {
            #[cfg(not(target_arch = "wasm32"))]
            central: None,
            gateways,
            uses: stored.gateway_uses,
            guests: stored.guests,
            gateway_cursor,
            gateway_filter: baylee_client_core::lobby::gateway_list::GatewayFilter::All,
            gateway_sort: baylee_client_core::lobby::gateway_list::GatewaySort::LastUsed,
            gateway_collapsed: [false, true, true],
            front_menu: false,
            source_code: None,
            discord_code: None,
            whatsapp_code: None,
            website_code: None,
            community_dialog: None,
            art_cache: false,
            gateway_selected: false,
            left_central: LeftCentral::No,
            gateway_epoch: 0,
            bearer: crate::replay::Bearer::default(),
            auth_probes: Arc::default(),
            probes: std::collections::HashMap::new(),
            adding: None,
            lobby,
            gateway: crate::settings::gateway_url(),
            lang,
            connected: false,
            confirm_leave: false,
            confirm_restore: false,
            confirmation: None,
            link_asked_central: false,
            #[cfg(not(target_arch = "wasm32"))]
            claimed_pairings: central::pairing::Claimed::default(),
            build: crate::buildui::BuildUi::default(),
            drafts: crate::buildui::draft::Drafts::default(),
            deck_saves: baylee_client_core::deckbuilder::saves::SaveLine::default(),
            transfer_asks: Vec::new(),
            commander_pick: None,
            hub: Hub::Play,
            room_deck_seat: None,
            room_setup_seat: None,
            room_card_edit: None,
            settings: SettingsPane::Closed,
            seat: crate::seatpanel::SeatDesk::default(),
            llm: crate::tableseats::TableSeats::default(),
            offline: None,
            local_decks: None,
            header_menu: None,
            feed_down: false,
            retry_feed: false,
            bell: client_core::lobby::strips::Bell::default(),
            social_unread: 0,
            social_notices: Vec::new(),
            shell_asks: Vec::new(),
            chat_header: None,
            vault_login: None,
            terms: client_core::terms::Terms::default(),
            terms_stale: None,
            terms_subject: front::terms::Subject::Gateway,
            about_open: false,
            memorial: client_core::memorial::Album::default(),
            caps_lock: false,
            settings_view: SettingsView::default(),
            menu: None,
            decks: decks::DecksUi::default(),
            play: play::PlayUi::default(),
            replays: replays::ReplaysUi::default(),
            challenges: challenges::Ui::default(),
            undo: None,
            tours_before: None,
            room_away: false,
            chair_sheet: None,
            lists_sheet: false,
            hosted_sheet: false,
            teams_edit: false,
            invite_copied: false,
            clipboard_out: None,
        }
    }
}

impl Default for LobbyState {
    fn default() -> Self {
        Self::new()
    }
}

impl LobbyState {
    pub(crate) fn displayed_gateways(&self) -> &[String] {
        #[cfg(not(target_arch = "wasm32"))]
        if let Some(central) = &self.central
            && self.gateways.is_empty()
            && !central.local_gateways.is_empty()
        {
            return &central.local_gateways;
        }
        &self.gateways
    }

    /// This device's own gateways: the list on screen, or the one Central's
    /// sign-in set aside while the account's is shown.
    pub(crate) fn device_gateways(&self) -> &[String] {
        #[cfg(not(target_arch = "wasm32"))]
        if let Some(central) = &self.central {
            return &central.local_gateways;
        }
        &self.gateways
    }

    pub(crate) fn central_pending(&self) -> bool {
        #[cfg(not(target_arch = "wasm32"))]
        {
            self.central
                .as_ref()
                .is_some_and(|state| state.handle.is_none())
        }
        #[cfg(target_arch = "wasm32")]
        {
            false
        }
    }

    /// Central's sign-in is shown because the chosen gateway (or a link)
    /// asked for it, so there is a way back to the device's own gateways.
    pub(crate) fn central_bound(&self) -> bool {
        #[cfg(not(target_arch = "wasm32"))]
        {
            self.central
                .as_ref()
                .is_some_and(|state| state.handle.is_none() && state.has_way_back())
        }
        #[cfg(target_arch = "wasm32")]
        {
            false
        }
    }

    /// Central is off on a client that has it: the device's own gateways
    /// stand alone, and a press brings the sign-in back
    /// (`central::back_to_sign_in`).
    pub(crate) fn central_off(&self) -> bool {
        #[cfg(not(target_arch = "wasm32"))]
        {
            self.central.is_none()
        }
        #[cfg(target_arch = "wasm32")]
        {
            false
        }
    }

    /// Which question stands over the lobby, by kind and never by what it
    /// names (dev-control's `/state`).
    #[cfg(all(feature = "dev-control", not(target_arch = "wasm32")))]
    #[must_use]
    pub(crate) fn asking(&self) -> Option<&'static str> {
        self.confirmation
            .as_ref()
            .map(confirm::Destructive::kind)
            .or_else(|| self.pairing().map(|pairing| pairing.stage.kind()))
    }

    /// The sign-in from baylee.app under way, whose panel stands over the
    /// lobby (`central::pairing`).
    #[cfg(not(target_arch = "wasm32"))]
    pub(crate) fn pairing(&self) -> Option<&central::pairing::Pairing> {
        self.central.as_ref()?.pairing.as_ref()
    }

    /// Whether that panel stands: it takes every press and key but its own.
    pub(crate) fn pairing_up(&self) -> bool {
        #[cfg(not(target_arch = "wasm32"))]
        {
            self.pairing().is_some()
        }
        #[cfg(target_arch = "wasm32")]
        {
            false
        }
    }

    pub(crate) fn central_ready(&self) -> bool {
        #[cfg(not(target_arch = "wasm32"))]
        {
            self.central
                .as_ref()
                .is_some_and(|state| state.handle.is_some())
        }
        #[cfg(target_arch = "wasm32")]
        {
            false
        }
    }

    pub(crate) fn central_terms_pending(&self) -> bool {
        #[cfg(not(target_arch = "wasm32"))]
        {
            central::terms_pending(self)
        }
        #[cfg(target_arch = "wasm32")]
        {
            false
        }
    }

    pub(crate) fn central_admitting(&self) -> bool {
        #[cfg(not(target_arch = "wasm32"))]
        {
            self.central.as_ref().is_some_and(|state| state.admitting)
        }
        #[cfg(target_arch = "wasm32")]
        {
            false
        }
    }

    /// Whether the list offers the guest door of the gateway that turned
    /// this Central account away (`central::guest_door_open`).
    pub(crate) fn central_guest_door(&self) -> bool {
        #[cfg(not(target_arch = "wasm32"))]
        {
            central::guest_door_open(self)
        }
        #[cfg(target_arch = "wasm32")]
        {
            false
        }
    }

    /// Whether a deck can be saved from here: a gateway with a session, or
    /// the device's own store.
    pub(crate) fn saves_decks(&self) -> bool {
        (!self.gateway.is_empty() && self.lobby.token().is_some())
            || self.offline.is_some()
            || self.central_without_gateway()
    }

    /// Whether a deck save is out to this gateway.
    pub(crate) fn deck_save_out(&self) -> bool {
        self.deck_saves.busy(self.gateway_epoch)
    }

    /// Signed in to Baylee Central without an active gateway session.
    pub(crate) fn central_without_gateway(&self) -> bool {
        self.central_ready() && !self.lobby.offline() && self.lobby.token().is_none()
    }

    /// Explicit multiplayer connection selection before gateway admission.
    /// The signed-in home remains available independently of this picker.
    pub(crate) fn showing_gateway_hub(&self) -> bool {
        self.central_without_gateway()
            && self.hub == Hub::Gateways
            && matches!(self.lobby.screen(), Screen::SignIn { .. } | Screen::Table)
    }

    /// Transitions the lobby state machine to [`Screen::Table`] for a Central
    /// session before a gateway is selected, loading local decks.
    fn ensure_central_lobby(&mut self, mailbox: &Mailbox) {
        if self.central_without_gateway() && matches!(self.lobby.screen(), Screen::SignIn { .. }) {
            self.local_decks.get_or_insert_with(offline::Offline::load);
            let lang = self.lobby.lang();
            let mut request = self.lobby.enter_central();
            while let Some(req) = request {
                if let Some(local) = self.local_decks.as_mut() {
                    let event = local.perform(req, lang);
                    request = self.lobby.apply(event);
                } else {
                    http::dispatch(self, mailbox, Some(req));
                    break;
                }
            }
            if self.lobby.decks().is_empty()
                && self.lobby.library().house.is_empty()
                && let Some(req) = self.lobby.browse_house()
            {
                if let Some(local) = self.local_decks.as_mut() {
                    let event = local.perform(req, lang);
                    let next = self.lobby.apply(event);
                    http::dispatch(self, mailbox, next);
                } else {
                    http::dispatch(self, mailbox, Some(req));
                }
            }
        }
    }
}

/// Where a finished HTTP call leaves its answer for the next frame.
///
/// Separate from [`LobbyState`] on purpose: touching it must not count as a
/// change to the lobby, or the UI would rebuild itself every frame.
#[derive(Resource, Clone, Default)]
pub(crate) struct Mailbox(Arc<Mutex<Vec<Reply>>>);

/// Tables the chat asked to sit down at: a friend's invite's Join (the
/// beta.8 social design §11.7), with the invite that stands in for the
/// password.
#[derive(Resource, Default)]
pub(crate) struct ChatJoins(pub(crate) Vec<(String, Option<String>)>);

/// Sits down where the chat asked, as the listing's Join does.
fn chat_joins(mut asks: ResMut<ChatJoins>, mut state: ResMut<LobbyState>, mailbox: Res<Mailbox>) {
    if asks.0.is_empty() {
        return;
    }
    state.hub = Hub::Play;
    state.room_away = false;
    for (game, invite) in std::mem::take(&mut asks.0) {
        let request = match invite.as_deref() {
            Some(invite) => state.lobby.join_invited(&game, invite),
            None => state.lobby.join(&game),
        };
        http::dispatch(&mut state, &mailbox, request);
    }
}

/// What a finished HTTP call hands back.
enum Reply {
    #[cfg(not(target_arch = "wasm32"))]
    Central(Result<Box<central::Restored>, baylee_account::Error>),
    /// What the worker of a sign-in from baylee.app said, by the pairing's
    /// number (`central::pairing`). Not bound to the gateway's epoch: the
    /// pairing lives in Central's state and ends with it.
    #[cfg(not(target_arch = "wasm32"))]
    Pairing(u64, central::pairing::News),
    /// The gateway refused to renew the bearer: the gateway token may still
    /// carry the session on.
    BearerLost,
    /// A socket closed for good, and this is what the player reads: the
    /// session ends here as after a `401`, but with the reason.
    ClosedSaying(String),
    /// Ignore replies from a gateway selection that has since changed.
    Remote(u64, Box<Reply>),
    /// The outcome of a [`LobbyRequest`].
    Event(LobbyEvent),
    /// A successful authentication's one-shot local-vault handoff.
    VaultLogin(crate::social::vault_access::Login),
    /// Public catalog completion; never starts a second fallback lookup.
    PrintingCatalog(LobbyEvent),
    PoolLanguage(Lang, LobbyEvent),
    /// `GET /auth/config` said who may register.
    Registration {
        registration: Registration,
        art_cache: bool,
        /// Whether it takes guests (#269).
        guests: bool,
        /// Whether it takes Baylee Central sign-ins; a native client then
        /// signs in there through Central (`central::bound`).
        central: bool,
        /// Whether guests are all it takes (`Lobby::guests_only`).
        guests_only: bool,
    },
    /// The gateway no longer honours the account token we hold.
    Expired,
    /// What an address said when asked about itself.
    Gateway {
        url: String,
        probe: Probe,
    },
    /// `GET /me` (WP0b-3).
    Me(client_core::lobby::strips::Me),
    /// `GET /lobby/stats` (WP0b-3).
    Stats(client_core::lobby::strips::LobbyStats),
    /// The terms of use (WG-1): what a sign-in said, and the sheet's
    /// requests' answers (`front::terms`).
    Terms(front::terms::TermsReply),
    /// `GET /auth/config`'s `clocks`: what the Create-table sheet offers.
    Clocks(Vec<client_core::lobby::play::ClockPreset>),
    /// A page of `GET /replays`.
    Replays(client_core::replay::library::ReplayPage),
    /// A page of the archived ones (`?archived=true`).
    ReplaysArchived(client_core::replay::library::ReplayPage),
    /// A replay change the gateway refused (archive, share), in its words.
    ReplayRefused(String),
    /// An auto-save in the deck builder finished. `archived`: the gateway
    /// kept the state it replaced in the deck's history.
    AutoSaved {
        deck_id: Option<String>,
        archived: bool,
    },
    /// An auto-save did not reach the gateway, or was refused; `why` is
    /// `None` where another reply says what happened.
    AutoSaveFailed {
        why: Option<String>,
    },
}

/// What the shell should make of a successful response body.
#[derive(Clone)]
enum Expect {
    /// `GET /banlists`.
    Banlists,
    /// One list and its cards, and what it was asked for.
    Banlist(client_core::lobby::ListFor),
    /// `204`: this list is gone.
    BanlistDeleted(String),
    /// A table's list; a `404` is a table that has none.
    RoomList,
    Library(client_core::lobby::library::Request),
    /// `{"token":…}`.
    LoggedIn,
    /// `{"token":…, "handle":…}` for a new guest (#269).
    Guest,
    /// A session ended; nothing is read, and nothing is answered.
    LoggedOut,
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
    /// The account is gone (#292); the gateway answers `204` with no body.
    AccountDeleted,
    /// A game list.
    Games,
    /// This account's resumable chairs, independent of public lobby pages.
    Seats,
    /// Something at a table changed. The body says what the whole lobby
    /// looks like, which is not the page this client is reading — so it is
    /// the fact that is taken, and the page is asked for again.
    Moved,
    /// A seat handover.
    Seat,
    /// A chair given up; the gateway answers `204` with no body.
    Left,
    /// `GET /lobby/llm-profiles`: `{"profiles": [...]}`.
    HostedProfiles,
}

mod build_press;
pub(crate) mod card_fill;
mod challenges;
mod clicks;
mod confirm;
pub(crate) mod decks;
pub(crate) mod decksave;
pub(crate) mod dock;
mod editing;
mod end_screen;
mod entrance;
mod feed;
mod field;
mod focusing;
mod header;
#[cfg(test)]
pub(crate) use feed::feed_url;
pub(crate) mod front;
/// The gateway in words (Settings › Network & Gateway).
pub(crate) fn gateway_facts(state: &LobbyState, lang: Lang) -> String {
    state.gateway_facts(lang)
}
/// The connection's state as text.
pub(crate) fn diagnostics(state: &LobbyState) -> String {
    state.diagnostics()
}
mod gateway;
mod hint;
mod history;
pub(crate) mod http;
mod hub;
mod keyboard;
mod library_ui;
#[cfg(not(target_arch = "wasm32"))]
mod links;
pub(crate) mod offline;
mod poolfacts;
mod press;
pub(crate) mod preview;
mod print_catalog;
pub(crate) mod replays;
#[cfg(not(any(target_arch = "wasm32", target_os = "android", target_os = "ios")))]
mod resume;
mod room;
mod scrolling;
pub(crate) use scrolling::WHEEL_LINE;
pub(crate) mod settings_press;
mod shell;
pub(crate) mod shortcuts;
mod source;
mod systems;
pub(crate) mod table_settings;
pub(crate) mod thumbnails;
mod touring;
mod widgets;

// The shell and the makers every screen builds from, under one roof for the
// modules beside them, which read the lobby through `use super::*`.
#[allow(clippy::wildcard_imports)] // the lobby's own vocabulary
use end_screen::*;
#[allow(clippy::wildcard_imports)] // the lobby's own vocabulary
use field::*;
#[allow(clippy::wildcard_imports)] // the lobby's own vocabulary
use hub::*;
#[allow(clippy::wildcard_imports)] // the lobby's own vocabulary
use shell::*;
#[allow(clippy::wildcard_imports, unused_imports)] // the lobby's own vocabulary
use widgets::*;

/// The end screen's keyboard marker, for the probe and for nothing else.
///
/// A re-export rather than `pub(crate) mod end_screen`, because what `devctl` needs
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
pub(crate) use end_screen::DuelExit;
pub(crate) use field::caret_lit;
#[cfg(any(test, all(feature = "dev-control", not(target_arch = "wasm32"))))]
pub(crate) use shell::LobbyRoot;
#[cfg(all(feature = "dev-control", not(target_arch = "wasm32")))]
pub(crate) use shell::UiRebuilds;

#[cfg(test)]
mod tests;

use clicks::clicks;
use http::{ask_about_registration, ask_about_saved_gateways, dispatch};
use keyboard::keyboard;
use preview::{Hovered, despawn_preview, hovers, preview, scroll_the_preview};
use scrolling::scrolls;
use systems::{
    art_follows_the_session, came_back, leave_clicks, leave_keys, lend_the_pool, poll, softkeys,
    text_follows_the_session, waiting, watch,
};

// The vocabulary the lobby's own halves share, and that `buildui` and
// `settingsui` build their screens out of. Re-exported here so the split
// into files stays an internal matter: every other module still says
// `crate::lobby::button`.

pub(crate) use build_press::BuildPress;
pub(crate) use end_screen::EndPress;
pub(crate) use field::{FieldLook, FieldStops, FieldTail, Masked, text_field, text_field_with};
pub(crate) use front::FrontPress;
/// The shell's notice lane: the chat keeps it clear of the friends list and
/// the chat window (`social::drawer::clear_the_lane`).
pub(crate) use header::ToastLane;
pub(crate) use header::{HeaderPress, ShellAsk, handle, tools};
pub(crate) use hint::HoverHint;
pub(crate) use hub::HubPress;
pub(crate) use library_ui::LibraryPress;
pub(crate) use press::{Press, SharedPress};
pub(crate) use preview::{HoverCard, hover_of_card, hover_of_entry, hover_of_index};
pub(crate) use room::RoomPress;
pub(crate) use scrolling::Scrollable;
pub(crate) use scrolling::{List, Scrolled};
pub(crate) use settings_press::SettingsPress;
pub(crate) use shell::{Frame, Metrics};
pub(crate) use widgets::{button, chip, heading, note, panel, print_mark, row, scroller, spacer};

pub(crate) mod deck_frame;
pub(crate) mod scrollbars;

pub(crate) mod button_style;

mod localization;
pub(crate) mod memorial;
mod menus;
pub(crate) mod orders;
mod parts;
mod play;
