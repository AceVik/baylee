//! What the lobby rebuilds, and what it must not (§10 of the shell design).
//!
//! A rebuild despawns and respawns every node on the screen, so it is
//! counted (`UiRebuilds`, read live by `devctl`'s `/state.ui_rebuilds`) and
//! held to: nothing for a press that changes nothing, nothing for a caret
//! that moves, no veil over a list that is only being read again.

#[allow(clippy::wildcard_imports)]
use super::*;

fn rebuilt(app: &App) -> u64 {
    app.world().resource::<UiRebuilds>().total
}

/// The lobby signed in with a deck, on the table screen, drawn.
fn signed_in() -> App {
    let mut app = headless();
    stocked(&mut app);
    for _ in 0..3 {
        app.update();
    }
    assert_eq!(
        app.world().resource::<LobbyState>().lobby.screen(),
        &Screen::Table
    );
    app
}

/// Taps an entity of the test's own carrying `press`, so a press can be
/// tried whether or not this screen happens to draw it.
fn tap_press(app: &mut App, press: Press) {
    let entity = app.world_mut().spawn(press).id();
    tap(app, entity);
    app.update();
    app.world_mut().entity_mut(entity).despawn();
}

/// §10 #1: every write a press makes is guarded, so a press that changes
/// nothing marks nothing — not the lobby's state, whose change rebuilds the
/// tree, and not the account's preferences, whose edit is also sent to the
/// gateway.
#[test]
fn a_press_that_changes_nothing_marks_nothing() {
    let mut app = signed_in();
    let (hub, lang) = {
        let state = app.world().resource::<LobbyState>();
        (state.hub, state.lobby.lang())
    };
    let (sky, sound, air) = {
        let prefs = app.world().resource::<crate::prefs::Prefs>().all();
        (prefs.sky, prefs.sound, prefs.atmosphere)
    };
    let quiet = [
        Press::Hub(HubPress::Tab(hub)),
        Press::Settings(SettingsPress::PickSky(sky)),
        Press::Settings(SettingsPress::PickSound(sound)),
        Press::Settings(SettingsPress::PickAtmosphere(air)),
        Press::Shared(SharedPress::PickLang(lang)),
        Press::Settings(SettingsPress::CloseSettings),
    ];
    for press in quiet {
        let state = app.world().resource_ref::<LobbyState>().last_changed();
        let prefs = app
            .world()
            .resource_ref::<crate::prefs::Prefs>()
            .last_changed();
        let before = rebuilt(&app);
        tap_press(&mut app, press);
        app.update();
        assert_eq!(
            app.world().resource_ref::<LobbyState>().last_changed(),
            state,
            "{press:?} marked the lobby changed"
        );
        assert_eq!(
            app.world()
                .resource_ref::<crate::prefs::Prefs>()
                .last_changed(),
            prefs,
            "{press:?} marked the preferences changed"
        );
        assert_eq!(rebuilt(&app), before, "{press:?} rebuilt the tree");
    }
    // And the count is not blind: a press that does change something is one
    // rebuild.
    let other = if hub == Hub::Play {
        Hub::Decks
    } else {
        Hub::Play
    };
    let before = rebuilt(&app);
    tap_press(&mut app, Press::Hub(HubPress::Tab(other)));
    assert_eq!(rebuilt(&app), before + 1, "a real change was not drawn");
}

/// §10 #3: an arrow key moves the caret and redraws its field's runs in
/// place; the rest of the screen is not rebuilt for it.
#[test]
fn a_moving_caret_redraws_its_field_and_nothing_else() {
    let mut app = signed_in();
    press(&mut app, Press::Shared(SharedPress::Focus(Field::Search)));
    {
        let mut messages = app.world_mut().resource_mut::<Messages<KeyboardInput>>();
        for ch in "abc".chars() {
            messages.write(typed(ch));
        }
    }
    app.update();
    app.update();
    let runs = |app: &mut App| {
        let mut query = app
            .world_mut()
            .query_filtered::<&Text, With<super::super::FieldRun>>();
        query
            .iter(app.world())
            .map(|t| t.0.clone())
            .collect::<Vec<_>>()
    };
    let typed_runs = runs(&mut app);
    assert_eq!(
        typed_runs,
        vec!["abc".to_string()],
        "the field holds what was typed"
    );
    let before = rebuilt(&app);
    {
        let mut messages = app.world_mut().resource_mut::<Messages<KeyboardInput>>();
        messages.write(pressed(KeyCode::ArrowLeft, Key::ArrowLeft));
    }
    app.update();
    app.update();
    assert_eq!(rebuilt(&app), before, "the caret rebuilt the screen");
    let mut moved = runs(&mut app);
    moved.sort();
    assert_eq!(
        moved,
        vec!["ab".to_string(), "c".to_string()],
        "the runs were not redrawn around the caret"
    );
    assert_eq!(
        app.world()
            .resource::<LobbyState>()
            .lobby
            .buffer(Field::Search)
            .cursor(),
        2
    );
}

/// §10 #7: re-reading the lists on screen raises no veil; the lists keep
/// standing until the answer replaces them.
#[test]
fn a_refresh_raises_no_veil() {
    let mut app = signed_in();
    press(&mut app, Press::Hub(HubPress::Refresh));
    let state = app.world().resource::<LobbyState>();
    assert!(state.lobby.busy(), "the refresh went out");
    assert!(state.lobby.refreshing(), "it is a refresh");
    assert_eq!(
        app.world().resource::<crate::loading::Loading>().what(),
        None,
        "a veil stood over a list being read again"
    );
}
