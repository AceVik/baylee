//! Header press handling.
#[allow(clippy::wildcard_imports)]
use super::*;

impl HeaderPress {
    /// What a press on the header or a strip does.
    pub(in crate::lobby) fn handle(self, cx: Cx<'_, '_, '_, '_, '_>) {
        match self {
            HeaderPress::Gateway => toggle(cx.state, HeaderMenu::Gateway),
            HeaderPress::Language => toggle(cx.state, HeaderMenu::Language),
            HeaderPress::Account => toggle(cx.state, HeaderMenu::Account),
            HeaderPress::CardFill => toggle(cx.state, HeaderMenu::CardFill),
            HeaderPress::Bell => {
                toggle(cx.state, HeaderMenu::Bell);
                if cx.state.bell.unread() > 0 {
                    cx.state.bell.read_all();
                }
            }
            HeaderPress::CloseMenu => {
                cx.state.header_menu = None;
            }
            HeaderPress::DismissToast(ToastId::Undo) => cx.state.undo = None,
            HeaderPress::DismissToast(id) => cx.state.shell_asks.push(ShellAsk::DismissToast(id)),
            HeaderPress::Shortcuts => {
                cx.state.header_menu = None;
                cx.state.shell_asks.push(ShellAsk::Shortcuts);
            }
            HeaderPress::Report => {
                cx.state.header_menu = None;
                cx.state.shell_asks.push(ShellAsk::Report);
            }
            HeaderPress::Friends | HeaderPress::Chat => {
                let ask = if self == HeaderPress::Chat {
                    ShellAsk::Chat
                } else {
                    ShellAsk::Friends
                };
                if cx.state.header_menu.is_none() && !cx.state.front_menu {
                    cx.state.bypass_change_detection().shell_asks.push(ask);
                } else {
                    cx.state.header_menu = None;
                    cx.state.front_menu = false;
                    cx.state.shell_asks.push(ask);
                }
            }
            HeaderPress::ChatPrivacy => cx.state.shell_asks.push(ShellAsk::ChatPrivacy),
            HeaderPress::SwitchGateway => {
                cx.state.header_menu = None;
                if cx.state.central_ready() {
                    let Cx {
                        state,
                        prefs,
                        scrolled,
                        mailbox,
                        settings,
                    } = cx;
                    super::clicks::switch_gateway(state, prefs, scrolled, mailbox, settings);
                    return;
                }
                HubPress::SignOut.handle(cx);
            }
            HeaderPress::SignOut => {
                cx.state.header_menu = None;
                HubPress::SignOut.handle(cx);
            }
            HeaderPress::Quit => {
                cx.state.header_menu = None;
                cx.state.shell_asks.push(ShellAsk::Quit);
            }
            HeaderPress::Nav(n) => {
                cx.state.header_menu = None;
                cx.state.front_menu = false;
                if cx.state.lobby.awaiting().is_some() && !cx.state.room_away {
                    cx.state.room_away = true;
                }
                if cx.state.settings.is_open() {
                    cx.state.settings = SettingsPane::Closed;
                }
                if cx.state.library_open() {
                    cx.state.lobby.close_library();
                }
                let hub = match NAV.get(usize::from(n)) {
                    Some(NavTab::Decks) => Hub::Decks,
                    Some(NavTab::Replays) => Hub::Replays,
                    _ => Hub::Play,
                };
                HubPress::Tab(hub).handle(cx);
            }
            HeaderPress::Return => back_to_the_table(cx),
            HeaderPress::Leave => {
                if let Some(Strip::Seated { index, .. }) = strips::strip(&cx.state.lobby) {
                    RoomPress::LeaveTable(index).handle(cx);
                }
            }
            HeaderPress::RetryNow => {
                cx.state.retry_feed = true;
                HubPress::Refresh.handle(cx);
            }
        }
    }
}

fn toggle(state: &mut LobbyState, menu: HeaderMenu) {
    state.front_menu = false;
    state.header_menu = if state.header_menu == Some(menu) {
        None
    } else {
        Some(menu)
    };
}

/// Return: to the waiting room (close what stands over it — Settings, the
/// house decks, the builder, which asks first when there is something
/// unsaved), or back into the game being played.
fn back_to_the_table(cx: Cx<'_, '_, '_, '_, '_>) {
    match strips::strip(&cx.state.lobby) {
        Some(Strip::Seated { .. }) => {
            if cx.state.room_away {
                cx.state.room_away = false;
            }
            if cx.state.hub != Hub::Play {
                cx.state.hub = Hub::Play;
            }
            if cx.state.settings.is_open() {
                cx.state.settings = SettingsPane::Closed;
            }
            if cx.state.library_open() {
                cx.state.lobby.close_library();
            }
            if matches!(cx.state.lobby.screen(), Screen::Build) {
                BuildPress::CloseBuilder.handle(cx);
            }
        }
        Some(Strip::Playing { game_id, .. }) => {
            let request = cx.state.lobby.return_to_game(&game_id);
            dispatch(cx.state, cx.mailbox, request);
        }
        None => {}
    }
}
