//! UI feature-visibility contracts across Offline, Guest, Local-Account, and
//! Central-Account sessions: unavailable features are omitted from headers,
//! menus, sheets, and toolbars rather than offered as failing buttons.

use super::super::header::{HeaderPress, NavTab, offered_nav};
use super::super::replays::ReplaysPress;
#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_client_core::lobby::KeptGuest;
use baylee_client_core::replay::library::SourceFilter;

fn offline_app() -> App {
    let mut app = headless();
    sized(&mut app, 1400.0);
    {
        let mut state = app.world_mut().resource_mut::<LobbyState>();
        let _ = state.lobby.play_offline();
        state.lobby.apply(LobbyEvent::Decks(vec![DeckSummary {
            id: "d1".to_string(),
            name: "Allytifact".to_string(),
            cards: 60,
            ..Default::default()
        }]));
        state.lobby.apply(LobbyEvent::Games(GameListing::default()));
        state.lobby.apply(LobbyEvent::Pool {
            cards: pool_cards(),
            has_text: true,
        });
    }
    settle(&mut app);
    app
}

fn guest_app() -> App {
    let mut app = headless();
    sized(&mut app, 1400.0);
    {
        let mut state = app.world_mut().resource_mut::<LobbyState>();
        state.lobby.apply(LobbyEvent::GuestIn(KeptGuest {
            token: "guest-tok".to_string(),
            handle: "Casper#0007".to_string(),
        }));
        state.lobby.apply(LobbyEvent::Decks(vec![DeckSummary {
            id: "d1".to_string(),
            name: "Allytifact".to_string(),
            cards: 60,
            ..Default::default()
        }]));
        state.lobby.apply(LobbyEvent::Seats(
            baylee_client_core::lobby::resume::Listing::default(),
        ));
        state.lobby.apply(LobbyEvent::Games(GameListing::default()));
    }
    settle(&mut app);
    app
}

#[test]
fn offline_and_guest_sessions_never_offer_contacts_or_challenges_in_header() {
    for mut app in [offline_app(), guest_app()] {
        let state = app.world().resource::<LobbyState>();
        assert!(!state.can_use_contacts());
        assert!(!state.can_enter_challenges());
        assert!(!offered_nav(state).contains(&NavTab::Challenges));
        assert!(state.chat_header.is_none());

        let found = presses(&mut app);
        assert!(
            !found.contains(&Press::Header(HeaderPress::Friends)),
            "Friends button must not render when contacts are unavailable: {found:?}"
        );
        assert!(
            !found.contains(&Press::Header(HeaderPress::Chat)),
            "Chat button must not render when contacts are unavailable: {found:?}"
        );
    }
}

#[cfg(not(target_arch = "wasm32"))]
#[test]
fn challenges_nav_is_hidden_for_local_gateway_and_shown_for_central_accounts() {
    let mut local = headless();
    sized(&mut local, 1400.0);
    stocked(&mut local);
    app_settle(&mut local);
    assert!(
        !offered_nav(local.world().resource::<LobbyState>()).contains(&NavTab::Challenges),
        "non-Central gateway accounts must not see Central Challenges nav"
    );

    let mut central_app = headless();
    sized(&mut central_app, 1400.0);
    {
        let mut state = central_app.world_mut().resource_mut::<LobbyState>();
        central::restored_for_test(&mut state, Vec::new());
    }
    app_settle(&mut central_app);
    assert!(
        offered_nav(central_app.world().resource::<LobbyState>()).contains(&NavTab::Challenges),
        "Central accounts must see Challenges nav"
    );
}

fn app_settle(app: &mut App) {
    app.update();
    settle(app);
}

#[test]
fn offline_play_and_create_table_sheet_hide_online_only_controls() {
    let mut app = offline_app();
    let home_presses = presses(&mut app);
    assert!(
        !home_presses.contains(&Press::Shared(SharedPress::Focus(Field::Search))),
        "offline Play must not draw the online table search bar: {home_presses:?}"
    );

    press(&mut app, Press::Play(PlayPress::CreateTable));
    let sheet_presses = presses(&mut app);
    assert!(
        !sheet_presses.contains(&Press::Play(PlayPress::Spectators)),
        "offline CreateTable must hide AllowSpectators: {sheet_presses:?}"
    );
    assert!(
        !sheet_presses.contains(&Press::Shared(SharedPress::Focus(Field::RoomPassword))),
        "offline CreateTable must hide RoomPassword: {sheet_presses:?}"
    );
    assert!(
        !sheet_presses
            .iter()
            .any(|p| matches!(p, Press::Play(PlayPress::StepClock(_)))),
        "offline CreateTable must hide multiplayer turn clock: {sheet_presses:?}"
    );
    let words = labels(&mut app);
    let lang = app.world().resource::<LobbyState>().lobby.lang();
    assert!(
        !words.iter().any(|w| w == Phrase::CreatePublic.text(lang)),
        "offline CreateTable must not wear a Public badge: {words:?}"
    );
}

#[cfg(not(target_arch = "wasm32"))]
#[test]
fn offline_replays_omit_server_source_tabs_while_online_replays_offer_them() {
    let mut offline = offline_app();
    offline.world_mut().resource_mut::<LobbyState>().hub = Hub::Replays;
    app_settle(&mut offline);
    let off_presses = presses(&mut offline);
    assert!(
        !off_presses.contains(&Press::Replays(ReplaysPress::Source(SourceFilter::Online))),
        "offline Replays must not offer an Online source tab: {off_presses:?}"
    );
    assert!(
        !off_presses.contains(&Press::Replays(ReplaysPress::Source(SourceFilter::House))),
        "offline Replays must not offer a redundant House source tab: {off_presses:?}"
    );
    assert!(
        off_presses.contains(&Press::Replays(ReplaysPress::Source(
            SourceFilter::Archived
        ))),
        "offline Replays still offers local Archived filter toggle: {off_presses:?}"
    );

    let mut online = headless();
    sized(&mut online, 1400.0);
    stocked(&mut online);
    online.world_mut().resource_mut::<LobbyState>().hub = Hub::Replays;
    app_settle(&mut online);
    let on_presses = presses(&mut online);
    assert!(
        on_presses.contains(&Press::Replays(ReplaysPress::Source(SourceFilter::Online))),
        "online Replays offers Online source tab: {on_presses:?}"
    );
}

#[test]
fn deck_history_menu_item_requires_online_gateway_session() {
    let mut offline = offline_app();
    to_decks(&mut offline);
    press(
        &mut offline,
        Press::Shared(SharedPress::OpenMenu(ShellMenu::Deck(0))),
    );
    let off_presses = presses(&mut offline);
    assert!(
        !off_presses.contains(&Press::Decks(DecksPress::History(0))),
        "offline deck menu must hide gateway-backed History: {off_presses:?}"
    );

    let mut online = headless();
    sized(&mut online, 1400.0);
    stocked(&mut online);
    to_decks(&mut online);
    press(
        &mut online,
        Press::Shared(SharedPress::OpenMenu(ShellMenu::Deck(0))),
    );
    let on_presses = presses(&mut online);
    assert!(
        on_presses.contains(&Press::Decks(DecksPress::History(0))),
        "online deck menu offers History: {on_presses:?}"
    );
}
