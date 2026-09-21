//! What a window's width decides, and when the retained tree is rebuilt at all. `Frame::of` and `Metrics::of` answer without a window; above them a phone gives every target its 44 pixels, drops the gateway address it has no room for, folds the builder to one pane and its filter chips away, and crossing a breakpoint is a new layout rather than a resize. The other half is the tree's quiet life — merely taking `&mut LobbyState` marks it changed, so a frame in which nothing happened must leave the roots alone — and the 2D camera the lobby brings of its own. Which controls a screen offers is that screen's part; this one only asks whether the width reaches them.

#[allow(clippy::wildcard_imports)] // this module's own vocabulary
use super::*;

#[test]
fn the_lobby_brings_its_own_camera() {
    let mut app = headless();
    let mut query = app
        .world_mut()
        .query_filtered::<Entity, (With<Camera>, With<LobbyScreen>)>();
    assert_eq!(query.iter(app.world()).count(), 1);
}

/// The retained tree is rebuilt from Bevy's change detection, and merely
/// taking `&mut` out of a `ResMut` marks it changed. A key handler that
/// reached for the lobby on every quiet frame therefore rebuilt the whole
/// screen sixty times a second — which is what this caught.
#[test]
fn a_quiet_frame_does_not_rebuild_the_tree() {
    let mut app = headless();
    let before = roots(&mut app);
    app.update();
    app.update();
    assert_eq!(roots(&mut app), before, "the retained tree survived");
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
    {
        let mut state = app.world_mut().resource_mut::<LobbyState>();
        state.lobby.apply(LobbyEvent::LoggedIn {
            token: "tok".to_string(),
        });
        // A real address: `headless` empties it so the startup probe reaches
        // nothing, and an empty one would make the assertions below match any
        // empty label on the screen rather than this one.
        state.gateway = "http://gw.example:28766".to_string();
    }
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
fn a_phone_shows_one_half_of_the_builder_at_a_time() {
    let mut app = headless();
    stocked(&mut app);
    sized(&mut app, 390.0);
    app.world_mut()
        .resource_mut::<LobbyState>()
        .lobby
        .build_deck();
    app.update();
    let cards = presses(&mut app);
    assert!(cards.contains(&Press::AddCard(0)), "the pool is showing");
    assert!(
        !cards.contains(&Press::SetZone(Zone::Side)),
        "and the deck is not: {cards:?}"
    );
    assert!(
        cards.contains(&Press::ShowPane(Pane::Deck)),
        "with a way over"
    );
    // The chips are folded away, or the list under them would be four
    // rows tall.
    assert!(
        !cards.contains(&Press::SetKind(Some("Creature"))),
        "{cards:?}"
    );
    assert!(cards.contains(&Press::ToggleFilters), "but reachable");
    app.world_mut().resource_mut::<LobbyState>().filters_open = true;
    app.update();
    assert!(
        presses(&mut app).contains(&Press::SetKind(Some("Creature"))),
        "unfolded, every filter is there"
    );
    app.world_mut().resource_mut::<LobbyState>().filters_open = false;

    app.world_mut().resource_mut::<LobbyState>().pane = Pane::Deck;
    app.update();
    let list = presses(&mut app);
    assert!(list.contains(&Press::SetZone(Zone::Side)), "{list:?}");
    assert!(!list.contains(&Press::AddCard(0)), "{list:?}");

    // Both halves are reachable on a desktop at once.
    sized(&mut app, 1400.0);
    app.update();
    let both = presses(&mut app);
    assert!(both.contains(&Press::AddCard(0)) && both.contains(&Press::SetZone(Zone::Side)));
}

#[test]
fn issue_186_library_replaces_editable_ui_and_keeps_navigation_on_phone() {
    use client_core::lobby::library::{HouseDeck, Reply};
    let mut app = headless();
    {
        let mut state = app.world_mut().resource_mut::<LobbyState>();
        state.lobby.apply(LobbyEvent::LoggedIn {
            token: "test".into(),
        });
        state.lobby.apply(LobbyEvent::Games(GameListing::default()));
        state.lobby.browse_house();
        state
            .lobby
            .apply(LobbyEvent::Library(Reply::House(vec![HouseDeck {
                id: "h1".into(),
                name: "Starter".into(),
                format: "commander".into(),
                description: String::new(),
                version: 1,
                cards: 70,
                sideboard: 0,
                commanders: vec!["Aang and Katara".into()],
            }])));
    }
    for width in [390.0, 900.0, 1400.0] {
        sized(&mut app, width);
        app.update();
        let controls = presses(&mut app);
        for wanted in [
            Press::CloseLibrary,
            Press::PreviewHouse(0),
            Press::CopyHouse(0),
        ] {
            assert!(controls.contains(&wanted), "{wanted:?} missing at {width}");
        }
        assert!(!controls.iter().any(|p| matches!(
            p,
            Press::EditDeck(_) | Press::DeleteDeck(_) | Press::SaveDeck
        )));
        let before = roots(&mut app);
        app.update();
        assert_eq!(roots(&mut app), before, "idle library must retain its tree");
    }
    press(&mut app, Press::CloseLibrary);
    assert!(
        app.world()
            .resource::<LobbyState>()
            .lobby
            .library()
            .page
            .is_none()
    );
}
