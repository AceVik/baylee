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
//! Settings), Refresh, New deck, and Search (the search box takes the caret).
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
        || state.lobby.library().page.is_some();
    let menu = state.front_menu || state.completion.is_some();
    // Settings types only into its own boxes (the key capture, the seat
    // panel); behind it the lobby's caret is idle (`keyboard` returns early).
    let field = if screen == Context::Settings {
        state.settings.capturing().is_some() || state.seat.typing()
    } else {
        state.lobby.typing_here() || screen == Context::Builder
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
                _ => {}
            }
            if !closing {
                break;
            }
        }
    }
}
