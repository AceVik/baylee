//! Opening the Decks tab re-reads the decks (RC: a deck saved elsewhere did
//! not show until the player signed in again).
//!
//! The press is run through the lobby's one door for a press and the lobby
//! is looked at before any frame runs, so no reply from a thread can land in
//! between: `busy`/`refreshing` is exactly "a `ListDecks` was asked".

#[allow(clippy::wildcard_imports)]
use super::*;

use bevy::ecs::system::RunSystemOnce;

/// A signed-in, stocked lobby on `hub`, with nothing in flight.
fn on(hub: Hub) -> App {
    let mut app = headless();
    stocked(&mut app);
    app.world_mut().resource_mut::<LobbyState>().hub = hub;
    let lobby = &app.world().resource::<LobbyState>().lobby;
    assert!(
        !lobby.busy(),
        "the rig starts with a request in flight; the tests below could not tell"
    );
    app
}

/// Presses `press` through the lobby's one door, without running a frame.
fn click_now(app: &mut App, press: Press) {
    app.world_mut()
        .run_system_once(
            move |mut state: ResMut<LobbyState>,
                  mut prefs: ResMut<crate::prefs::Prefs>,
                  mut scrolled: ResMut<Scrolled>,
                  mailbox: Res<Mailbox>,
                  mut settings: Option<ResMut<crate::settings::ClientSettings>>| {
                let cx = Cx {
                    state: &mut state,
                    prefs: &mut prefs,
                    scrolled: &mut scrolled,
                    mailbox: &mailbox,
                    settings: &mut settings,
                };
                super::super::clicks::run(press, cx);
            },
        )
        .expect("the press ran");
}

/// Coming to Decks from either other tab asks the gateway for the decks
/// again (a `ListDecks` refresh, which keeps the list standing while it is
/// re-read), and shows the tab.
#[test]
fn coming_to_the_decks_tab_re_reads_the_decks() {
    for from in [Hub::Play, Hub::Replays] {
        let mut app = on(from);
        click_now(&mut app, Press::Hub(HubPress::Tab(Hub::Decks)));
        let state = app.world().resource::<LobbyState>();
        assert_eq!(state.hub, Hub::Decks, "from {from:?}: the tab is shown");
        assert!(
            state.lobby.refreshing(),
            "from {from:?}: opening Decks asked for the decks again, quietly"
        );
    }
}

/// Pressing Decks while already on it asks nothing, and the other tabs ask
/// nothing for decks when they are opened: the refresh belongs to arriving
/// on Decks only, so a press that changes nothing stays free.
#[test]
fn the_decks_tab_pressed_on_the_decks_tab_asks_nothing_and_other_tabs_do_not_ask() {
    let mut app = on(Hub::Decks);
    click_now(&mut app, Press::Hub(HubPress::Tab(Hub::Decks)));
    let state = app.world().resource::<LobbyState>();
    assert_eq!(state.hub, Hub::Decks);
    assert!(!state.lobby.busy(), "already there: nothing was asked");

    let mut app = on(Hub::Decks);
    click_now(&mut app, Press::Hub(HubPress::Tab(Hub::Play)));
    let state = app.world().resource::<LobbyState>();
    assert_eq!(state.hub, Hub::Play);
    assert!(
        !state.lobby.busy(),
        "leaving Decks for Play asked for something"
    );
}
