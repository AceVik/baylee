//! The retained header must follow the body when entering or leaving a run.
use super::*;
use crate::lobby::echoes::Confirmation;
use crate::shellkit::focus::{FocusReport, Stop};

#[test]
fn tower_keys_survive_the_retained_header_and_confirmation() {
    let (mut app, _jobs) = screen(Lang::En);
    front_keys::clears_a_despawned_focus(&mut app);
    {
        let mut state = app.world_mut().resource_mut::<LobbyState>();
        state.echoes.opened = true;
        // Hold the confirmation before rendering: the queued account worker
        // intentionally never answers this headless fixture's background reads.
        state.echoes.confirmation = Some(Confirmation::Withdrawal);
    }
    settle(&mut app);
    app.update();
    let brands: Vec<_> = app
        .world_mut()
        .query::<&Stop>()
        .iter(app.world())
        .filter(|stop| stop.id == "baylee")
        .copied()
        .collect();
    assert_eq!(brands, vec![Stop::new("echoes", "baylee")]);
    assert!(
        app.world()
            .resource::<crate::shellkit::keys::ShellStack>()
            .stack
            .modal
    );
    front_keys::press_key(&mut app, KeyCode::Digit2, Key::Character("2".into()), &[]);
    assert_eq!(app.world().resource::<LobbyState>().hub, Hub::Challenges);
    assert!(
        app.world()
            .resource::<LobbyState>()
            .echoes
            .confirmation
            .is_some()
    );
    assert_eq!(
        front_keys::focused(&app),
        Some(Stop::new("echoes-confirm", "cancel"))
    );
    front_keys::tab(&mut app, false);
    assert_eq!(front_keys::focused_id(&app), Some("confirm"));
    front_keys::tab(&mut app, false);
    assert_eq!(front_keys::focused_id(&app), Some("cancel"));
    front_keys::press_key(&mut app, KeyCode::Enter, Key::Enter, &[]);
    assert!(
        app.world()
            .resource::<LobbyState>()
            .echoes
            .confirmation
            .is_none()
    );
    front_keys::tab(&mut app, false);
    assert_eq!(app.world().resource::<FocusReport>().table, Some("echoes"));
    assert!(
        !app.world()
            .resource::<crate::shellkit::keys::ShellStack>()
            .stack
            .modal
    );
    // The overview draws no Refresh any more (only `draw/shop/options.rs`
    // does), so the body's stop here is its pages.
    for _ in 0..12 {
        if front_keys::focused_id(&app) == Some("tabs") {
            break;
        }
        front_keys::tab(&mut app, false);
    }
    assert_eq!(front_keys::focused_id(&app), Some("tabs"));
    assert_eq!(app.world().resource::<FocusReport>().table, Some("echoes"));
}

#[test]
fn online_activity_tiles_share_the_retained_headers_focus_table() {
    let (mut app, _jobs) = screen(Lang::En);
    stocked(&mut app);
    {
        let mut state = app.world_mut().resource_mut::<LobbyState>();
        state.gateway_selected = true;
        state.echoes.tower_answered(true);
        state.lobby.apply(LobbyEvent::Games(GameListing::default()));
    }
    press(
        &mut app,
        crate::lobby::header::nav_press(crate::lobby::header::NavTab::Play),
    );
    settle(&mut app);
    app.update();
    assert_eq!(app.world().resource::<FocusReport>().table, Some("play"));
    for _ in 0..24 {
        if front_keys::focused_id(&app) == Some("tower") {
            break;
        }
        front_keys::tab(&mut app, false);
    }
    assert_eq!(front_keys::focused(&app), Some(Stop::new("play", "tower")));
}
