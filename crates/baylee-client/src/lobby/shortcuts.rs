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
    let modal = state.confirmation.is_some()
        || state.lobby.deleting_account().is_some()
        || state.lobby.library().page.is_some()
        || (screen == Context::Builder && builder_modal(state));
    let menu = state.front_menu
        || state.completion.is_some()
        || state.header_menu.is_some()
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
    state: Res<LobbyState>,
    entrance: Res<super::entrance::Entrance>,
    journey: Option<Res<crate::arrival::Journey>>,
    desk: Option<Res<crate::report::ReportDesk>>,
    mut shell: ResMut<ShellStack>,
) {
    let held = journey.as_ref().is_some_and(|j| j.active())
        || entrance.active()
        || desk.is_some_and(|d| d.holds_keyboard());
    let now = if held {
        ShellStack::default()
    } else {
        stack_of(&state)
    };
    if *shell != now {
        *shell = now;
    }
}

/// The press a shell action means on the lobby's current screen, if it has
/// a door there yet.
pub(super) fn press_for(state: &LobbyState, action: ShellAction) -> Option<Press> {
    let hub = matches!(state.lobby.screen(), Screen::Table)
        && state.lobby.awaiting().is_none()
        && state.lobby.library().page.is_none();
    let builder = matches!(state.lobby.screen(), Screen::Build)
        && !state.settings.is_open()
        && state.confirmation.is_none()
        && state.build.menu.is_none()
        && !builder_modal(state);
    match action {
        ShellAction::GoSettings | ShellAction::OpenSettings
            if !state.settings.is_open()
                && !matches!(state.lobby.screen(), Screen::Build | Screen::Seated(_)) =>
        {
            Some(Press::Settings(SettingsPress::OpenSettings))
        }
        ShellAction::GoPlay | ShellAction::GoDecks if state.settings.is_open() => {
            Some(Press::Settings(SettingsPress::CloseSettings))
        }
        ShellAction::GoPlay if hub => Some(Press::Hub(HubPress::Tab(Hub::Play))),
        ShellAction::GoDecks if hub => Some(Press::Hub(HubPress::Tab(Hub::Decks))),
        ShellAction::Refresh if hub => Some(Press::Hub(HubPress::Refresh)),
        ShellAction::NewDeck if hub && state.hub == Hub::Decks => {
            Some(Press::Hub(HubPress::NewDeck))
        }
        ShellAction::Search if hub => Some(Press::Shared(SharedPress::Focus(Field::Search))),
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
pub(super) fn run_fired(
    mut fired: MessageReader<ShellFired>,
    mut state: ResMut<LobbyState>,
    mut prefs: ResMut<crate::prefs::Prefs>,
    mut scrolled: ResMut<Scrolled>,
    mailbox: Res<Mailbox>,
    mut settings: Option<ResMut<crate::settings::ClientSettings>>,
) {
    for ShellFired(action) in fired.read() {
        // Settings closes first; the hub tab follows on the same key.
        for _ in 0..2 {
            let Some(press) = press_for(&state, *action) else {
                break;
            };
            let closing = press == Press::Settings(SettingsPress::CloseSettings);
            let cx = Cx {
                state: &mut state,
                prefs: &mut prefs,
                scrolled: &mut scrolled,
                mailbox: &mailbox,
                settings: &mut settings,
            };
            match press {
                Press::Hub(press) => press.handle(cx),
                Press::Settings(press) => press.handle(cx),
                Press::Shared(press) => press.handle(cx),
                Press::Header(press) => press.handle(cx),
                Press::Build(press) => press.handle(cx),
                _ => {}
            }
            if !closing {
                break;
            }
        }
    }
}
