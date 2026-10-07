//! A closed beta's key (#317), as the shell does it: what `/auth/config`
//! says is read, the key's box drawn only where the gateway wants a key, the
//! request carrying it only then, and the gateway's two refusals said as
//! sentences.

#[allow(clippy::wildcard_imports)]
use super::*;

const KEY: &str = "BAYLEE-7K3M-Q9TX-2HVD-4WNC";

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

/// The sign-in face of a gateway that answered `/auth/config` with `body`.
fn at(body: &str) -> App {
    let mut app = headless();
    app.insert_resource(crate::settings::ClientSettings::default());
    let reply = auth_config(body).expect("an /auth/config body");
    post(&mut app, Reply::Remote(0, Box::new(reply)));
    settle(&mut app);
    app
}

fn registering(app: &mut App) {
    app.world_mut()
        .resource_mut::<LobbyState>()
        .lobby
        .toggle_registering();
    settle(app);
}

fn key_box_drawn(app: &mut App) -> bool {
    presses(app).contains(&Press::Shared(SharedPress::Focus(Field::InviteKey)))
}

#[test]
fn what_auth_config_says_is_read_with_and_without_the_new_field() {
    let read = |body: &str| match auth_config(body) {
        Some(Reply::Registration {
            registration,
            guests,
            ..
        }) => (registration, guests),
        _ => panic!("{body} is read"),
    };
    // A gateway from before closed betas: its flag alone decides.
    assert_eq!(
        read(r#"{"registration_enabled":true}"#),
        (Registration::Open, false)
    );
    assert_eq!(
        read(r#"{"registration_enabled":false,"guests_enabled":true}"#),
        (Registration::Off, true)
    );
    // A closed beta says so, and still says "enabled" for the clients that
    // read only that.
    assert_eq!(
        read(r#"{"registration_enabled":true,"registration":"invite","guests_enabled":true}"#),
        (Registration::Invite, true)
    );
    assert_eq!(
        read(r#"{"registration_enabled":true,"registration":"open"}"#),
        (Registration::Open, false)
    );
    assert!(auth_config("<html>").is_none(), "not a gateway's answer");
}

#[test]
fn the_box_is_drawn_only_at_a_closed_beta() {
    // An open gateway, and one from before keys: the form as it was.
    for body in [
        r#"{"registration_enabled":true,"guests_enabled":true}"#,
        r#"{"registration_enabled":true,"registration":"open","guests_enabled":true}"#,
    ] {
        let mut app = at(body);
        assert!(!key_box_drawn(&mut app), "{body}");
        registering(&mut app);
        assert!(!key_box_drawn(&mut app), "{body}");
        assert!(!labels(&mut app).contains(&"CLOSED BETA KEY".to_string()));
    }

    let mut app =
        at(r#"{"registration_enabled":true,"registration":"invite","guests_enabled":true}"#);
    assert!(!key_box_drawn(&mut app), "never on the sign-in face (WP1)");
    app.world_mut()
        .resource_mut::<LobbyState>()
        .lobby
        .open_guest_face();
    settle(&mut app);
    assert!(key_box_drawn(&mut app), "for a new guest");
    app.world_mut()
        .resource_mut::<LobbyState>()
        .lobby
        .back_to_sign_in();
    settle(&mut app);
    registering(&mut app);
    assert!(key_box_drawn(&mut app), "for a new account");
    let labels = labels(&mut app);
    assert!(
        labels.contains(&"CLOSED BETA KEY".to_string()),
        "{labels:?}"
    );
    assert!(
        labels
            .iter()
            .any(|l| l.starts_with("This gateway is a closed beta")),
        "{labels:?}"
    );
}

#[test]
fn a_key_typed_into_the_box_lands_there() {
    let mut app =
        at(r#"{"registration_enabled":true,"registration":"invite","guests_enabled":true}"#);
    registering(&mut app);
    press(
        &mut app,
        Press::Shared(SharedPress::Focus(Field::InviteKey)),
    );
    {
        let mut messages = app.world_mut().resource_mut::<Messages<KeyboardInput>>();
        for ch in "7k3m".chars() {
            messages.write(typed(ch));
        }
    }
    app.update();
    let lobby = &app.world().resource::<LobbyState>().lobby;
    assert_eq!(lobby.field(Field::InviteKey), "7k3m");
    assert_eq!(lobby.field(Field::Username), "", "and nowhere else");
}

#[test]
fn the_key_goes_out_only_when_there_is_one() {
    let register = |invite_key: Option<&str>| {
        let (request, _) = build(
            "http://gw",
            None,
            "en",
            LobbyRequest::Register {
                username: "alice".to_string(),
                display_name: "Alice".to_string(),
                password: "hunter22".to_string(),
                invite_key: invite_key.map(str::to_string),
            },
        );
        body(&request)
    };
    assert_eq!(register(Some(KEY))["invite_key"], KEY);
    // An older gateway is sent exactly what it was always sent.
    assert_eq!(
        register(None),
        serde_json::json!({
            "username": "alice",
            "display_name": "Alice",
            "password": "hunter22",
            "lang": "en",
        })
    );

    let (request, _) = build(
        "http://gw",
        None,
        "en",
        LobbyRequest::PlayAsGuest {
            display_name: None,
            invite_key: Some(KEY.to_string()),
        },
    );
    assert_eq!(request.url, "http://gw/auth/guest");
    assert_eq!(body(&request)["invite_key"], KEY);
    let (request, _) = build(
        "http://gw",
        None,
        "en",
        LobbyRequest::PlayAsGuest {
            display_name: None,
            invite_key: None,
        },
    );
    assert!(body(&request).get("invite_key").is_none());
}

#[test]
fn the_gateways_refusals_are_said_as_sentences() {
    let invalid = answer(403, r#"{"error":"this closed beta key is not valid"}"#);
    let needed = answer(
        403,
        r#"{"error":"this gateway is a closed beta: a new account or guest needs a closed beta key. If there is no field for one, update Baylee"}"#,
    );
    assert_eq!(
        gateway_error(Lang::En, &invalid),
        "this closed beta key is not valid — check it, or ask for a new one"
    );
    assert_eq!(
        gateway_error(Lang::De, &invalid),
        "dieser Beta-Schlüssel ist nicht gültig — prüf ihn oder frag nach einem neuen"
    );
    assert_eq!(
        gateway_error(Lang::En, &needed),
        Phrase::InviteKeyNeeded.text(Lang::En)
    );
    assert_eq!(
        gateway_error(Lang::De, &needed),
        Phrase::InviteKeyNeeded.text(Lang::De)
    );

    // And the refusal stands on the form, where the player is.
    let mut app =
        at(r#"{"registration_enabled":true,"registration":"invite","guests_enabled":true}"#);
    registering(&mut app);
    post(
        &mut app,
        Reply::Event(LobbyEvent::Failed(gateway_error(Lang::En, &invalid))),
    );
    settle(&mut app);
    assert!(labels(&mut app).contains(
        &"this closed beta key is not valid — check it, or ask for a new one".to_string()
    ));
}
