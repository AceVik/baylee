//! The gateway operator's legal pages (`privacy_url`, `imprint_url` in `/info`): linked under the front door's notice and on the settings screen when the gateway names them and never otherwise, and the one line beside creating an account or a guest that says where the gateway tells what it keeps. None of these tests presses a link: a press opens the system's browser.

#[allow(clippy::wildcard_imports)] // this module's own vocabulary
use super::*;

use super::super::source::{LegalPage, PrivacyNotice};
use baylee_client_core::lobby::gateway_info::GatewayInfo;

const PRIVACY: &str = "https://hall.example/datenschutz";
const IMPRINT: &str = "https://hall.example/impressum";

/// The chosen gateway answers `/info` naming these pages, as this build's
/// own gateway would.
fn naming(app: &mut App, privacy: Option<&str>, imprint: Option<&str>) {
    {
        let mut state = app.world_mut().resource_mut::<LobbyState>();
        let gateway = state.gateway.clone();
        state.probes.insert(
            gateway,
            Probe::Known(GatewayInfo {
                name: None,
                version: String::new(),
                protocol_version: baylee_protocol::PROTOCOL_VERSION,
                view_version: baylee_view::VIEW_VERSION,
                source: None,
                privacy: privacy.map(str::to_string),
                imprint: imprint.map(str::to_string),
            }),
        );
    }
    app.update();
}

/// Whether `entity` is part of the drawn lobby: a control spawned and
/// never attached is no link a player can see.
fn drawn(app: &App, mut entity: Entity) -> bool {
    loop {
        if app.world().get::<LobbyRoot>(entity).is_some() {
            return true;
        }
        match app.world().get::<ChildOf>(entity) {
            Some(parent) => entity = parent.parent(),
            None => return false,
        }
    }
}

/// How many drawn controls open `page`.
fn links(app: &mut App, page: LegalPage) -> usize {
    let mut query = app.world_mut().query::<(Entity, &Press)>();
    let found: Vec<Entity> = query
        .iter(app.world())
        .filter(|(_, press)| **press == Press::OpenLegal(page))
        .map(|(entity, _)| entity)
        .collect();
    found.into_iter().filter(|&e| drawn(app, e)).count()
}

/// How many privacy notices are drawn.
fn notices(app: &mut App) -> usize {
    let mut query = app
        .world_mut()
        .query_filtered::<Entity, With<PrivacyNotice>>();
    let found: Vec<Entity> = query.iter(app.world()).collect();
    found.into_iter().filter(|&e| drawn(app, e)).count()
}

/// The gateway says whether it takes guests, as its `/auth/config` would.
fn taking_guests(app: &mut App) {
    app.insert_resource(crate::settings::ClientSettings::default());
    app.world()
        .resource::<Mailbox>()
        .0
        .lock()
        .expect("mailbox")
        .push(Reply::Remote(
            0,
            Box::new(Reply::Registration {
                enabled: true,
                art_cache: false,
                guests: true,
            }),
        ));
    app.update();
    settle(app);
}

#[test]
fn the_front_door_links_the_legal_pages_only_when_the_gateway_names_them() {
    for width in [1400.0, 390.0] {
        let mut app = headless();
        sized(&mut app, width);
        settle(&mut app);
        assert_eq!(
            (
                links(&mut app, LegalPage::Privacy),
                links(&mut app, LegalPage::Imprint)
            ),
            (0, 0),
            "{width} px: no gateway has answered, so there is nothing to link"
        );

        naming(&mut app, Some(PRIVACY), None);
        assert_eq!(links(&mut app, LegalPage::Privacy), 1, "{width} px");
        assert_eq!(links(&mut app, LegalPage::Imprint), 0, "{width} px");

        naming(&mut app, Some(PRIVACY), Some(IMPRINT));
        let drawn = labels(&mut app);
        for words in [Phrase::PrivacyLink, Phrase::ImprintLink] {
            assert!(
                drawn.contains(&words.text(Lang::En).to_string()),
                "{width} px: {words:?} in {drawn:?}"
            );
        }
        assert_eq!(links(&mut app, LegalPage::Imprint), 1, "{width} px");

        // On the gateway form too: the notice stands under every panel.
        to_gateway_face(&mut app);
        assert_eq!(links(&mut app, LegalPage::Privacy), 1, "{width} px");
        assert_eq!(links(&mut app, LegalPage::Imprint), 1, "{width} px");

        // Past the check on the way in, as a gateway's answer never is: the
        // check at the door still keeps it from being a link.
        naming(&mut app, Some("javascript:alert(1)"), Some("file:///etc"));
        assert_eq!(links(&mut app, LegalPage::Privacy), 0, "{width} px");
        assert_eq!(links(&mut app, LegalPage::Imprint), 0, "{width} px");
    }
}

#[test]
fn the_settings_screen_links_the_legal_pages_the_gateway_names() {
    let mut app = headless();
    stocked(&mut app);
    sized(&mut app, 1400.0);
    app.update();
    press(&mut app, Press::FrontMenu);
    press(&mut app, Press::OpenSettings);
    assert!(app.world().resource::<LobbyState>().settings.is_open());
    assert_eq!(links(&mut app, LegalPage::Privacy), 0, "none named");
    assert_eq!(links(&mut app, LegalPage::Imprint), 0, "none named");

    naming(&mut app, Some(PRIVACY), Some(IMPRINT));
    assert!(app.world().resource::<LobbyState>().settings.is_open());
    assert_eq!(links(&mut app, LegalPage::Privacy), 1);
    assert_eq!(links(&mut app, LegalPage::Imprint), 1);
}

/// Creating an account is told where the gateway says what it keeps, in
/// one line above the button; signing in to one creates nothing and is not.
#[test]
fn creating_an_account_is_told_where_the_privacy_statement_is() {
    let mut app = headless();
    naming(&mut app, Some(PRIVACY), Some(IMPRINT));
    settle(&mut app);
    assert_eq!(notices(&mut app), 0, "signing in creates nothing");

    press(&mut app, Press::ToggleRegistering);
    settle(&mut app);
    assert_eq!(notices(&mut app), 1, "the registration form");
    assert!(
        labels(&mut app).contains(&Phrase::PrivacyNotice.text(Lang::En).to_string()),
        "the line says what it links"
    );
    // The line is itself a link to the statement: under the notice, and
    // the line.
    assert_eq!(links(&mut app, LegalPage::Privacy), 2);

    // A gateway that names no statement gets no line.
    naming(&mut app, None, Some(IMPRINT));
    assert_eq!(notices(&mut app), 0, "nothing to point at");
}

/// A guest is an account too: the new guest's entry carries the line, once
/// a face even where the registration form is drawn beside it, and a guest
/// this device keeps, coming back, creates nothing.
#[test]
fn a_new_guest_is_told_where_the_privacy_statement_is() {
    let mut app = headless();
    taking_guests(&mut app);
    assert!(presses(&mut app).contains(&Press::PlayAsGuest));
    assert_eq!(notices(&mut app), 0, "the gateway has named no statement");

    naming(&mut app, Some(PRIVACY), None);
    assert_eq!(
        notices(&mut app),
        1,
        "the guest's entry on the sign-in face"
    );

    press(&mut app, Press::ToggleRegistering);
    settle(&mut app);
    assert_eq!(notices(&mut app), 1, "one line for both ways in");

    press(&mut app, Press::ToggleRegistering);
    settle(&mut app);
    {
        let mut state = app.world_mut().resource_mut::<LobbyState>();
        state
            .lobby
            .keep_guest(Some(baylee_client_core::lobby::KeptGuest {
                token: "guest-tok".to_string(),
                handle: "Casper#0007".to_string(),
            }));
    }
    settle(&mut app);
    assert!(labels(&mut app).contains(&"Continue as Casper#0007".to_string()));
    assert_eq!(notices(&mut app), 0, "a kept guest is not created again");
}
