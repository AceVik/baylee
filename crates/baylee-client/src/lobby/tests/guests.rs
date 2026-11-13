//! Playing as a guest (#269), as the shell does it: the entry on the
//! sign-in face, the route it asks, the guest kept in the settings file per
//! gateway, and the two ways a guest ends — signed out after a question, and
//! ended by the gateway.

#[allow(clippy::wildcard_imports)]
use super::*;

use baylee_client_core::lobby::{Face, KeptGuest};

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
                registration: Registration::Open,
                art_cache: false,
                guests: true,
                central: false,
                guests_only: false,
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

/// The sign-in face of a gateway that answered `/auth/config` with `body`.
fn answered(body: &str) -> App {
    let mut app = headless();
    app.insert_resource(crate::settings::ClientSettings::default());
    let reply = auth_config(body).expect("an /auth/config body");
    post(&mut app, Reply::Remote(0, Box::new(reply)));
    settle(&mut app);
    app
}

/// What a front door draws: its presses and its words, in a fixed order.
fn drawn_door(app: &mut App) -> (Vec<Press>, Vec<String>) {
    let mut words = labels(app);
    words.sort();
    (presses(app), words)
}

/// A gateway that lets in guests alone has neither a sign-in nor a sign-up
/// to offer: both would only fail, so the face is its doors. So has any gateway
/// that does not take Central's accounts, whatever else it says.
#[test]
fn a_gateway_of_guests_alone_offers_no_form() {
    let guests = r#"{"registration_enabled":false,"registration":"open",
        "guests_enabled":true,"central":false,"auth_mode":"guests"}"#;
    let mut app = answered(guests);
    let found = presses(&mut app);
    assert!(
        found.contains(&Press::Front(FrontPress::GuestFace)),
        "{found:?}"
    );
    assert!(
        !found.contains(&Press::Front(FrontPress::ToggleRegistering)),
        "{found:?}"
    );
    assert!(
        !found.contains(&Press::Front(FrontPress::Submit)),
        "{found:?}"
    );
    let lobby = &mut app.world_mut().resource_mut::<LobbyState>().lobby;
    assert_eq!(lobby.submit(), None, "Enter sends nothing");

    // Nothing a gateway says draws the form: a hostile one that left out its
    // mode, or says it takes sign-ups, would be asking for a password.
    for hostile in [
        r#"{"registration_enabled":true,"guests_enabled":true}"#,
        r#"{"registration_enabled":true,"guests_enabled":false}"#,
        r#"{"registration_enabled":true,"central":false,"auth_mode":"central"}"#,
    ] {
        let found = presses(&mut answered(hostile));
        assert!(
            !found.contains(&Press::Front(FrontPress::Submit)),
            "{hostile}: {found:?}"
        );
        assert!(
            !found.contains(&Press::Front(FrontPress::ToggleRegistering)),
            "{hostile}: {found:?}"
        );
    }

    let central = r#"{"registration_enabled":false,"guests_enabled":true,
        "central":true,"auth_mode":"mixed"}"#;
    let read = auth_config(central);
    assert!(matches!(
        read,
        Some(Reply::Registration {
            guests_only: false,
            ..
        })
    ));
}

/// An older gateway that still says `invite` is drawn the front door its
/// other fields ask for, as if it had said `open`: on the sign-in face, the
/// guest's and the create-account face, the same controls and the same
/// words, and a new guest is asked for with nothing but its name.
#[test]
fn an_older_gateways_invite_draws_the_door_an_open_one_does() {
    for central in [false, true] {
        let body = |word: &str| {
            format!(
                r#"{{"registration_enabled":true,"registration":"{word}","guests_enabled":true,"central":{central}}}"#
            )
        };
        let mut invite = answered(&body("invite"));
        let mut open = answered(&body("open"));
        assert_eq!(
            drawn_door(&mut invite),
            drawn_door(&mut open),
            "the sign-in face, central {central}"
        );

        for app in [&mut invite, &mut open] {
            app.world_mut()
                .resource_mut::<LobbyState>()
                .lobby
                .open_guest_face();
            settle(app);
        }
        assert_eq!(
            drawn_door(&mut invite),
            drawn_door(&mut open),
            "the guest's face, central {central}"
        );
        let lobby = &mut invite.world_mut().resource_mut::<LobbyState>().lobby;
        assert_eq!(lobby.face(), Face::Guest);
        assert_eq!(
            lobby.play_as_guest(),
            Some(LobbyRequest::PlayAsGuest { display_name: None })
        );

        let mut invite = answered(&body("invite"));
        for app in [&mut invite, &mut open] {
            let mut state = app.world_mut().resource_mut::<LobbyState>();
            state.lobby.back_to_sign_in();
            state.lobby.toggle_registering();
            settle(app);
        }
        assert_eq!(
            drawn_door(&mut invite),
            drawn_door(&mut open),
            "the create-account face, central {central}"
        );
    }
}

#[test]
fn the_entry_is_drawn_only_for_a_gateway_that_takes_guests() {
    let mut closed = headless();
    assert!(!presses(&mut closed).contains(&Press::Front(FrontPress::GuestFace)));
    assert!(!presses(&mut closed).contains(&Press::Shared(SharedPress::Focus(Field::GuestName))));

    // The sign-in face offers the guest's door (WP1); the name is on the
    // guest's own face, one press on.
    let mut open = welcoming();
    let found = presses(&mut open);
    assert_eq!(
        found
            .iter()
            .filter(|p| **p == Press::Front(FrontPress::GuestFace))
            .count(),
        1,
        "{found:?}"
    );
    assert!(!found.contains(&Press::Shared(SharedPress::Focus(Field::GuestName))));
    assert!(labels(&mut open).contains(&"Play as guest".to_string()));
    open.world_mut()
        .resource_mut::<LobbyState>()
        .lobby
        .open_guest_face();
    settle(&mut open);
    assert!(presses(&mut open).contains(&Press::Shared(SharedPress::Focus(Field::GuestName))));
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
    assert!(!presses(&mut app).contains(&Press::Shared(SharedPress::Focus(Field::GuestName))));
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
    sign_out_from_the_header(&mut app);
    assert!(matches!(
        app.world().resource::<LobbyState>().confirmation,
        Some(super::confirm::Destructive::SignOutGuest { to_gateways: false })
    ));
    assert_eq!(kept_in_the_file(&app), Some(kept()), "not yet");
    assert!(labels(&mut app).contains(&"Sign out Casper#0007?".to_string()));

    tap_control(&mut app, "the answer", |p| {
        *p == Press::Shared(SharedPress::ConfirmDestructive)
    });
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
    sign_out_from_the_header(&mut app);
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
    // The name and the language are the whole body: nothing else goes out.
    let (request, _) = build(
        "http://gw",
        None,
        "en",
        LobbyRequest::PlayAsGuest { display_name: None },
    );
    assert_eq!(
        body(&request),
        serde_json::json!({ "display_name": null, "lang": "en" })
    );

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

/// A Central account that goes in as a guest here is the guest: the pill says
/// so, and Central's record is gone, so `GET /me` is asked who that is.
#[test]
fn a_guest_is_not_the_central_account_it_came_from() {
    let mut app = welcoming();
    let mut state = app.world_mut().resource_mut::<LobbyState>();
    state.lobby.set_me(baylee_client_core::lobby::strips::Me {
        handle: "Anna#0001".to_string(),
        guest: false,
    });
    state.lobby.set_field(Field::Username, "Anna");
    state.lobby.apply(LobbyEvent::GuestIn(kept()));
    assert_eq!(crate::lobby::handle(&state).as_deref(), Some("Casper#0007"));
    assert_eq!(state.lobby.me(), None);
}

/// An account signed in here is not the guest this device keeps for the
/// gateway, before `GET /me` has answered as much as after.
#[test]
fn an_account_is_not_the_guest_kept_for_its_gateway() {
    let mut app = welcoming();
    let mut state = app.world_mut().resource_mut::<LobbyState>();
    state.lobby.keep_guest(Some(kept()));
    state.lobby.set_field(Field::Username, "Anna");
    state.lobby.apply(LobbyEvent::LoggedIn {
        token: "tok".to_string(),
        username: None,
    });
    assert_eq!(crate::lobby::handle(&state).as_deref(), Some("Anna"));
    state.lobby.set_me(baylee_client_core::lobby::strips::Me {
        handle: "Anna#0001".to_string(),
        guest: false,
    });
    assert_eq!(crate::lobby::handle(&state).as_deref(), Some("Anna#0001"));
}

/// Choosing the next gateway carries Central's own record along, and never a
/// guest's: that guest belongs to the gateway being left.
#[test]
fn choosing_a_gateway_leaves_a_guests_record_behind() {
    let mut state = LobbyState::new();
    state.gateways = vec!["https://one.test".into(), "https://two.test".into()];
    let (handle, _jobs) = super::super::central::Handle::queued_for_test();
    state.central.as_mut().expect("Central is on").handle = Some(handle);
    state.lobby.set_me(baylee_client_core::lobby::strips::Me {
        handle: "Anna#0001".to_string(),
        guest: false,
    });
    assert!(state.select_gateway(0));
    assert_eq!(
        state.lobby.me().map(|me| me.handle.as_str()),
        Some("Anna#0001")
    );
    state.lobby.set_me(baylee_client_core::lobby::strips::Me {
        handle: "Casper#0007".to_string(),
        guest: true,
    });
    assert!(state.select_gateway(1));
    assert_eq!(state.lobby.me(), None);
}
