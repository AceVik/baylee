//! The real lobby schedule must cover the first authenticated frame and
//! release its input again when the visual flight ends.

#[allow(clippy::wildcard_imports)]
use super::*;

#[test]
fn login_flies_over_the_lobby_and_blocks_its_controls_until_arrival() {
    let mut app = headless();
    app.world_mut().spawn(crate::vista::Vista::Front);
    app.insert_resource(bevy::time::TimeUpdateStrategy::ManualDuration(
        std::time::Duration::from_millis(100),
    ));
    app.update();
    app.world_mut()
        .resource_mut::<LobbyState>()
        .lobby
        .apply(LobbyEvent::LoggedIn {
            token: "test-flight".into(),
            username: Some("review".into()),
        });
    app.update();
    assert!(app.world().resource::<crate::vista::FrontScene>().entering);
    // The lobby is already available underneath; its network work need not
    // wait for the flight. But a click must not open a deck behind the veil.
    assert_eq!(
        app.world().resource::<LobbyState>().lobby.screen(),
        &Screen::Table
    );
    tap_control(&mut app, "new deck", |press| {
        *press == Press::Hub(HubPress::NewDeck)
    });
    assert_eq!(
        app.world().resource::<LobbyState>().lobby.screen(),
        &Screen::Table
    );
    for _ in 0..20 {
        app.update();
    }
    let scene = app.world().resource::<crate::vista::FrontScene>();
    assert!(!scene.entering && !scene.shown);
    tap_control(&mut app, "new deck", |press| {
        *press == Press::Hub(HubPress::NewDeck)
    });
    assert_eq!(
        app.world().resource::<LobbyState>().lobby.screen(),
        &Screen::Build
    );
}
