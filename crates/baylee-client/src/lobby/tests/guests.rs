//! Playing as a guest (#269), as the shell does it: the entry on the
//! sign-in face, the route it asks, the guest kept in the settings file per
//! gateway, and the two ways a guest ends — signed out after a question, and
//! ended by the gateway.

#[allow(clippy::wildcard_imports)]
use super::*;

use baylee_client_core::lobby::KeptGuest;

/// The guest a gateway handed out, as it is kept.
fn kept() -> KeptGuest {
    KeptGuest {
        token: "guest-tok".to_string(),
        handle: "Casper#0007".to_string(),
    }
}

/// Posts a reply as the gateway would have, and lets it land.
fn post(app: &mut App, reply: Reply) {
    app.world()
        .resource::<Mailbox>()
        .0
        .lock()
        .expect("mailbox")
        .push(reply);
    app.update();
}

/// The sign-in face of a gateway that takes guests, with a settings file.
fn welcoming() -> App {
    let mut app = headless();
    app.insert_resource(crate::settings::ClientSettings::default());
    post(
        &mut app,
        Reply::Remote(
            0,
            Box::new(Reply::Registration {
                enabled: true,
                art_cache: false,
                guests: true,
            }),
        ),
    );
    settle(&mut app);
    app
}

/// A guest in, at the tables.
fn in_as_a_guest() -> App {
    let mut app = welcoming();
    post(&mut app, Reply::Event(LobbyEvent::GuestIn(kept())));
    settle(&mut app);
    app
}

fn kept_in_the_file(app: &App) -> Option<KeptGuest> {
    let gateway = app.world().resource::<LobbyState>().gateway.clone();
    app.world()
        .resource::<crate::settings::ClientSettings>()
        .guests
        .get(&gateway)
        .cloned()
}

#[test]
fn the_entry_is_drawn_only_for_a_gateway_that_takes_guests() {
    let mut closed = headless();
    assert!(!presses(&mut closed).contains(&Press::PlayAsGuest));
    assert!(!presses(&mut closed).contains(&Press::Focus(Field::GuestName)));

    let mut open = welcoming();
    let found = presses(&mut open);
    assert_eq!(
        found.iter().filter(|p| **p == Press::PlayAsGuest).count(),
        1,
        "{found:?}"
    );
    assert!(found.contains(&Press::Focus(Field::GuestName)));
    assert!(labels(&mut open).contains(&"Play as guest".to_string()));
}

/// The guest this device keeps here is offered back by its handle, with no
/// name to type: it has one.
#[test]
fn a_kept_guest_is_offered_back_by_its_handle() {
    let mut app = welcoming();
    {
        let mut state = app.world_mut().resource_mut::<LobbyState>();
        let gateway = state.gateway.clone();
        state.guests.insert(gateway, kept());
        state.lobby.keep_guest(Some(kept()));
    }
    settle(&mut app);
    assert!(labels(&mut app).contains(&"Continue as Casper#0007".to_string()));
    assert!(!presses(&mut app).contains(&Press::Focus(Field::GuestName)));
}

/// Choosing a gateway hands the lobby the guest kept for it, and only for it.
#[test]
fn choosing_a_gateway_hands_the_lobby_its_own_guest() {
    let mut state = LobbyState::from_settings(crate::settings::ClientSettings {
        gateways: vec!["https://one.test".into(), "https://two.test".into()],
        guests: std::iter::once(("https://two.test".to_string(), kept())).collect(),
        ..crate::settings::ClientSettings::default()
    });
    let at = |state: &LobbyState, url: &str| {
        state.gateways.iter().position(|g| g == url).expect("saved")
    };
    let one = at(&state, "https://one.test");
    assert!(state.select_gateway(one));
    assert_eq!(state.lobby.kept_guest(), None);
    let two = at(&state, "https://two.test");
    assert!(state.select_gateway(two));
    assert_eq!(state.lobby.kept_guest(), Some(&kept()));
}

#[test]
fn a_new_guest_is_kept_for_its_gateway_and_told_what_a_guest_is() {
    let mut app = in_as_a_guest();
    assert_eq!(kept_in_the_file(&app), Some(kept()));
    let state = app.world().resource::<LobbyState>();
    assert!(state.lobby.guest());
    assert_eq!(
        state.uses.of(&state.gateway).count,
        1,
        "a use of the gateway"
    );
    assert!(
        labels(&mut app)
            .iter()
            .any(|label| label.starts_with("You are playing as a guest.")),
        "the notice is on the tables screen"
    );
}

#[test]
fn a_guest_the_gateway_has_ended_is_forgotten_here_too() {
    let mut app = in_as_a_guest();
    post(&mut app, Reply::Expired);
    assert_eq!(kept_in_the_file(&app), None);
    let state = app.world().resource::<LobbyState>();
    assert!(state.guests.is_empty());
    assert_eq!(
        state.lobby.status(),
        "that guest has ended — play as a new one, or sign in"
    );
}

/// Signing a guest out is the end of it, so it is asked first; the answer
/// forgets the guest here and ends it on the gateway.
#[test]
fn a_guest_is_asked_before_it_signs_out_and_forgotten_after() {
    let mut app = in_as_a_guest();
    tap_control(&mut app, "sign out", |p| *p == Press::SignOut);
    assert!(matches!(
        app.world().resource::<LobbyState>().confirmation,
        Some(super::confirm::Destructive::SignOutGuest)
    ));
    assert_eq!(kept_in_the_file(&app), Some(kept()), "not yet");
    assert!(labels(&mut app).contains(&"Sign out Casper#0007?".to_string()));

    tap_control(&mut app, "the answer", |p| *p == Press::ConfirmDestructive);
    assert_eq!(kept_in_the_file(&app), None);
    let state = app.world().resource::<LobbyState>();
    assert_eq!(state.lobby.token(), None);
    assert!(matches!(state.lobby.screen(), Screen::SignIn { .. }));
}

/// An account signing out is not asked, and leaves the guest kept here
/// alone: it is the account's session that ends.
#[test]
fn an_account_signing_out_leaves_the_kept_guest_alone() {
    let mut app = welcoming();
    {
        let mut state = app.world_mut().resource_mut::<LobbyState>();
        let gateway = state.gateway.clone();
        state.guests.insert(gateway.clone(), kept());
        state.lobby.keep_guest(Some(kept()));
        let mut settings = app
            .world_mut()
            .resource_mut::<crate::settings::ClientSettings>();
        settings.guests.insert(gateway, kept());
    }
    post(
        &mut app,
        Reply::Event(LobbyEvent::LoggedIn {
            token: "tok".to_string(),
            username: Some("alice".to_string()),
        }),
    );
    settle(&mut app);
    tap_control(&mut app, "sign out", |p| *p == Press::SignOut);
    let state = app.world().resource::<LobbyState>();
    assert!(state.confirmation.is_none(), "nothing to ask");
    assert_eq!(state.lobby.token(), None);
    assert_eq!(state.lobby.kept_guest(), Some(&kept()));
    assert_eq!(kept_in_the_file(&app), Some(kept()));
}

#[test]
fn the_guest_routes_carry_what_the_gateway_reads() {
    let (request, _) = build(
        "http://gw",
        None,
        "de",
        LobbyRequest::PlayAsGuest {
            display_name: Some("Casper".to_string()),
        },
    );
    assert_eq!(request.url, "http://gw/auth/guest");
    assert_eq!(
        body(&request),
        serde_json::json!({ "display_name": "Casper", "lang": "de" })
    );
    let (request, _) = build(
        "http://gw",
        None,
        "en",
        LobbyRequest::PlayAsGuest { display_name: None },
    );
    assert_eq!(body(&request)["display_name"], serde_json::Value::Null);

    // Signed with the token it ends, which the lobby has already let go of.
    let (request, _) = build(
        "http://gw",
        None,
        "en",
        LobbyRequest::LogOut {
            token: "guest-tok".to_string(),
        },
    );
    assert_eq!(request.url, "http://gw/auth/logout");
    assert_eq!(request.method, ehttp::Method::POST);
    assert_eq!(
        request.headers.get("Authorization"),
        Some("Bearer guest-tok")
    );

    assert_eq!(
        decode(
            Lang::En,
            Expect::Guest,
            &answer(
                200,
                r#"{"token":"t","expires_at":1,"guest":true,"handle":"Guest#0001"}"#
            ),
        ),
        LobbyEvent::GuestIn(KeptGuest {
            token: "t".to_string(),
            handle: "Guest#0001".to_string(),
        })
    );
}
