//! The lobby's half of the shell keymap (`KEYBOARD.md` §2, §7.1): it says
//! what is open, and it answers the screen moves a shell key asks for.
//!
//! The stack is read off the lobby's own state, conservatively: wherever
//! the lobby types into a field today (the sign-in form, Play's search box,
//! the builder) the field owns every printable key, so only the chords that
//! edit no text reach the keymap there (`Ctrl/Cmd + = − 0`, `Ctrl/Cmd+/`,
//! `Ctrl/Cmd+,`, `Ctrl/Cmd+R`). Each screen's package gives its screen a
//! focus model of its own (§1.7) and with it the bare keys.
//!
//! The doors this module opens: `1 2 3` and `Ctrl/Cmd+,` (Play, Decks,
//! Settings), Refresh, New deck, Search (the search box takes the caret),
//! and `r`, the seated strip's Return.
//! The others resolve and are logged (`/state.shell.fired`); their doors are
//! the screens' own, built by the packages that own those screens.

use super::press::Cx;
#[allow(clippy::wildcard_imports)] // the lobby's own vocabulary
use super::*;
use crate::shellkit::keys::{ShellFired, ShellStack};
use baylee_client_core::shellkeys::{Context, ShellAction, Stack};

/// What the lobby has open, as the resolver's stack.
pub(super) fn stack_of(state: &LobbyState) -> ShellStack {
    let screen = match state.lobby.screen() {
        Screen::Seated(_) => {
            return ShellStack::default();
        }
        _ if state.settings.is_open() => Context::Settings,
        Screen::SignIn { .. } => Context::Front,
        Screen::Build => Context::Builder,
        Screen::Table if state.lobby.awaiting().is_some() => Context::Room,
        Screen::Table => match state.hub {
            Hub::Play => Context::Play,
            Hub::Decks => Context::Decks,
        },
    };
    // The house list is the Decks screen's data, not a sheet (WP3); a
    // deck's history, a house deck's cards, the Create-table sheet and the
    // deck picker are.
    let modal = state.confirmation.is_some()
        || state.terms.up()
        || state.about_open
        || (state.settings_open() && state.settings_view.profile_sheet)
        || state.lobby.deleting_account().is_some()
        || matches!(
            state.lobby.library().page,
            Some(client_core::lobby::library::Page::History(_))
        )
        || state.decks.preview.is_some()
        || state.play.sheet.is_some()
        || state.play.picker
        || state.chair_sheet.is_some()
        || (screen == Context::Builder && builder_modal(state));
    let menu = state.front_menu
        || state.header_menu.is_some()
        || state.menu.is_some()
        || (screen == Context::Builder
            && (state.build.menu.is_some() || state.build.syntax || state.build.rail));
    // Settings types only into its own boxes (the key capture, the seat
    // panel); behind it the lobby's caret is idle (`keyboard` returns early).
    let field = if screen == Context::Settings {
        state.settings.capturing().is_some() || state.seat.typing()
    } else if screen == Context::Builder {
        // Only while the caret is in the search or the name: on a list row
        // the bare keys are the keymap's (`/`, `?`, `r`).
        state.build.nav == crate::buildui::Nav::Field
    } else {
        state.lobby.typing_here()
            && (!matches!(state.lobby.screen(), Screen::Table)
                || super::keyboard::field_drawn(state))
    };
    ShellStack {
        stack: Stack {
            screen: Some(screen),
            modal,
            menu,
            field,
            typeahead: false,
        },
        live: true,
    }
}

/// Whether something of the builder's stands over it as a sheet: the
/// printing picker, import or export, the card, "Discard changes?", the
/// phone's Stats.
fn builder_modal(state: &LobbyState) -> bool {
    let deck = state.lobby.builder();
    deck.picker().is_some()
        || deck.transfer().is_some()
        || deck.inspecting().is_some()
        || state.confirm_leave
        || state.build.stats_sheet
}

/// Writes [`ShellStack`] from the lobby every frame, and stands it down
/// while the arrival, the entrance or the report form holds the keyboard.
pub(super) fn write_stack(
    mut yields: ResMut<crate::shellkit::focus::WalkerYields>,
    focus: Res<crate::shellkit::focus::FocusReport>,
    state: Res<LobbyState>,
    entrance: Res<super::entrance::Entrance>,
    journey: Option<Res<crate::arrival::Journey>>,
    desk: Option<Res<crate::report::ReportDesk>>,
    mut shell: ResMut<ShellStack>,
) {
    let held = journey.as_ref().is_some_and(|j| j.active())
        || entrance.active()
        || desk.is_some_and(|d| d.holds_keyboard());
    // The seat panel's boxes walk with Tab themselves.
    // So does the account deletion's confirmation over the terms sheet
    // (its Decline): its password box is a lobby field with no stop, and
    // the walker would take Tab back to the sheet under it.
    let seat = (state.settings_open() && state.seat.typing())
        || (state.terms.up() && state.lobby.deleting_account().is_some());
    if yields.0 != seat {
        yields.0 = seat;
    }
    let mut now = if held {
        ShellStack::default()
    } else {
        stack_of(&state)
    };
    // Settings' nav takes letters as type-ahead (`KEYBOARD.md` §1.5);
    // digits and `?` stay shortcuts.
    if state.settings_open()
        && focus
            .stop
            .is_some_and(|s| s.table == crate::settingsui::keys::SETTINGS.name && s.id == "nav")
    {
        now.stack.typeahead = true;
    }
    if *shell != now {
        *shell = now;
    }
}

/// The press a shell action means on the lobby's current screen, if it has
/// a door there yet.
pub(super) fn press_for(state: &LobbyState, action: ShellAction) -> Option<Press> {
    let table = matches!(state.lobby.screen(), Screen::Table) && !state.settings.is_open();
    let in_room = table
        && !state.room_away
        && state.lobby.awaiting().is_some_and(|h| {
            state
                .lobby
                .games()
                .iter()
                .any(|g| g.id == h.game_id && g.state == "waiting")
        });
    let hub = table && !in_room && state.lobby.awaiting().is_none_or(|_| state.room_away);
    let sheet = state.play.sheet.is_some()
        || state.play.picker
        || state.decks.preview.is_some()
        || matches!(
            state.lobby.library().page,
            Some(client_core::lobby::library::Page::History(_))
        );
    let play = hub && state.hub == Hub::Play && !sheet;
    let decks = hub && state.hub == Hub::Decks && !sheet;
    let builder = matches!(state.lobby.screen(), Screen::Build)
        && !state.settings.is_open()
        && state.confirmation.is_none()
        && state.build.menu.is_none()
        && !builder_modal(state);
    match action {
        // The screens a key goes to: through the header's nav, which steps
        // away from a room and keeps the seat (M-7).
        ShellAction::GoPlay if table && (in_room || state.hub != Hub::Play) => {
            Some(Press::Header(super::header::HeaderPress::Nav(0)))
        }
        ShellAction::GoDecks if table && (in_room || state.hub != Hub::Decks) => {
            Some(Press::Header(super::header::HeaderPress::Nav(1)))
        }
        ShellAction::Undo if state.undo.is_some() => {
            Some(Press::Decks(super::decks::DecksPress::Undo))
        }
        ShellAction::CreateTable if play => Some(Press::Play(super::play::PlayPress::CreateTable)),
        ShellAction::NewDeck if decks => Some(Press::Decks(super::decks::DecksPress::NewDeck)),
        ShellAction::ImportDeck if decks => Some(Press::Decks(super::decks::DecksPress::Import)),
        ShellAction::Search if decks => Some(Press::Shared(SharedPress::Focus(Field::DeckSearch))),
        ShellAction::StartGame if in_room => super::room::start_press(state),
        ShellAction::GoSettings | ShellAction::OpenSettings
            if !state.settings.is_open()
                && !matches!(state.lobby.screen(), Screen::Build | Screen::Seated(_)) =>
        {
            Some(Press::Settings(SettingsPress::OpenSettings))
        }
        ShellAction::GoPlay | ShellAction::GoDecks if state.settings.is_open() => {
            Some(Press::Settings(SettingsPress::CloseSettings))
        }
        ShellAction::Refresh if hub => Some(Press::Hub(HubPress::Refresh)),
        ShellAction::Search if play => Some(Press::Shared(SharedPress::Focus(Field::Search))),
        // The builder's doors (`KEYBOARD.md` §7.7), while nothing stands
        // over it.
        ShellAction::Search if builder => Some(Press::Build(BuildPress::FocusBuild(
            baylee_client_core::deckbuilder::BuildField::Search,
        ))),
        ShellAction::SaveDeck if builder => Some(Press::Build(BuildPress::SaveDeck)),
        ShellAction::ImportDeck if builder && !state.lobby.busy() => {
            Some(Press::Build(BuildPress::OpenImport))
        }
        ShellAction::ExportDeck if builder && !state.lobby.busy() => {
            Some(Press::Build(BuildPress::OpenExport))
        }
        ShellAction::Back if builder => Some(Press::Build(BuildPress::CloseBuilder)),
        // `r`: the seated strip's Return, wherever the strip stands.
        ShellAction::ReturnToGame if super::header::seated(state).is_some() => {
            Some(Press::Header(super::header::HeaderPress::Return))
        }
        _ => None,
    }
}

/// Answers the shell actions the lobby has doors for, through the same
/// handlers a click reaches. A move to Play or Decks from Settings closes
/// Settings and then switches the hub.
#[allow(clippy::too_many_arguments)] // a Bevy system: every one is an injection
pub(super) fn run_fired(
    mut fired: MessageReader<ShellFired>,
    mut state: ResMut<LobbyState>,
    mut prefs: ResMut<crate::prefs::Prefs>,
    mut scrolled: ResMut<Scrolled>,
    mailbox: Res<Mailbox>,
    mut settings: Option<ResMut<crate::settings::ClientSettings>>,
    focus: Option<Res<bevy::input_focus::InputFocus>>,
    tiles: Query<&super::focusing::TileOf>,
) {
    let tile = focus
        .as_deref()
        .and_then(|f| super::focusing::focused_tile(f, &tiles));
    for ShellFired(action) in fired.read() {
        // Settings closes first; the hub tab follows on the same key.
        for _ in 0..2 {
            // `e` / `F2` on the focused deck tile (`KEYBOARD.md` §7.6).
            let edit = (*action == ShellAction::EditTile)
                .then_some(tile)
                .flatten()
                .map(|i| Press::Decks(super::decks::DecksPress::Edit(i)));
            let Some(press) = edit.or_else(|| press_for(&state, *action)) else {
                break;
            };
            let closing = press == Press::Settings(SettingsPress::CloseSettings);
            super::press::run(
                press,
                Cx {
                    state: &mut state,
                    prefs: &mut prefs,
                    scrolled: &mut scrolled,
                    mailbox: &mailbox,
                    settings: &mut settings,
                },
            );
            if !closing {
                break;
            }
        }
    }
}
