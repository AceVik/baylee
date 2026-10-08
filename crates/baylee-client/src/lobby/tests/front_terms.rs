//! The terms sheet as the shell runs it (WG-1; `DESIGN-v5` §11, §17 WP1;
//! `KEYBOARD.md` §7.9, §9.6): a stale sign-in raises it, Esc ×10 leaves it
//! up and the session signed in, a guest is asked before Not now signs it
//! out, Accept is enabled only at the end and records the device's copy.

#[allow(clippy::wildcard_imports)]
use super::*;

use super::super::front::terms::TermsReply;
use super::front_keys::{focused_id, press_key};
use baylee_client_core::i18n::Lang;
use baylee_client_core::lobby::KeptGuest;
use baylee_client_core::terms::{Sheet, TermsDoc};
use bevy::ecs::system::RunSystemOnce;

fn post(app: &mut App, reply: Reply) {
    app.world()
        .resource::<Mailbox>()
        .0
        .lock()
        .expect("mailbox")
        .push(reply);
    app.update();
    app.update();
    app.update();
}

fn doc(markdown: &str) -> TermsDoc {
    TermsDoc {
        version: "v1".into(),
        updated: Some("2026-10-07".into()),
        markdown: markdown.into(),
        lang: None,
    }
}

/// A lobby a new guest has just signed in to, at a gateway whose terms it
/// has not accepted: the sheet is up, its text arrived.
fn signed_in_stale(guest: bool, markdown: &str) -> App {
    let mut app = headless();
    app.insert_resource(crate::settings::ClientSettings::default());
    app.add_systems(
        PreUpdate,
        |mut focus: ResMut<bevy::input_focus::InputFocus>, alive: Query<()>| {
            if focus.get().is_some_and(|e| alive.get(e).is_err()) {
                focus.clear();
            }
        },
    );
    {
        let mut state = app.world_mut().resource_mut::<LobbyState>();
        state.lobby.set_guests_enabled(true);
    }
    post(&mut app, Reply::Terms(TermsReply::Stale(true)));
    if guest {
        post(
            &mut app,
            Reply::Event(LobbyEvent::GuestIn(KeptGuest {
                token: "guest-tok".into(),
                handle: "Wanda#0001".into(),
            })),
        );
    } else {
        post(
            &mut app,
            Reply::Event(LobbyEvent::LoggedIn {
                token: "tok".into(),
                username: Some("ada".into()),
            }),
        );
    }
    assert!(
        app.world().resource::<LobbyState>().terms.up(),
        "a stale sign-in raises the sheet"
    );
    post(
        &mut app,
        Reply::Terms(TermsReply::Doc {
            asked: Lang::En,
            doc: doc(markdown),
        }),
    );
    app.update();
    app
}

/// The owner's case (08.10.2026): switching the interface's language while
/// the sheet is up asks for the text in the new one; the old text stays
/// until it comes, an answer in the language left behind is dropped, and
/// the new text must be read to its end again.
#[test]
fn switching_language_on_the_sheet_asks_for_the_text_again() {
    let mut app = signed_in_stale(false, "# Terms\n\nPlay fair.");
    assert_eq!(app.world().resource::<LobbyState>().terms.lang(), Lang::En);
    app.world_mut()
        .resource_mut::<LobbyState>()
        .lobby
        .set_lang(Lang::De);
    app.update();
    let state = app.world().resource::<LobbyState>();
    assert_eq!(state.terms.lang(), Lang::De, "not asked again in German");
    assert!(
        matches!(sheet(&app), Sheet::Reading(_)),
        "the old text stays"
    );
    post(
        &mut app,
        Reply::Terms(TermsReply::Doc {
            asked: Lang::En,
            doc: doc("# Terms, late"),
        }),
    );
    assert!(
        matches!(sheet(&app), Sheet::Reading(r) if r.doc.markdown == "# Terms\n\nPlay fair."),
        "an answer in the language left behind replaced the text"
    );
    let german = TermsDoc {
        lang: Some("de".into()),
        ..doc("# Bedingungen\n\nSei fair.")
    };
    post(
        &mut app,
        Reply::Terms(TermsReply::Doc {
            asked: Lang::De,
            doc: german.clone(),
        }),
    );
    assert!(matches!(sheet(&app), Sheet::Reading(r) if r.doc == german));
}

fn sheet(app: &App) -> Sheet {
    app.world().resource::<LobbyState>().terms.sheet().clone()
}

fn signed_in(app: &App) -> bool {
    app.world().resource::<LobbyState>().lobby.token().is_some()
}

/// M4-1 (§9.6): Esc ×10 on the terms sheet — still signed in, the sheet
/// still up, focus on Not now, nothing stored and nothing sent.
#[test]
fn escape_ten_times_on_the_terms_leaves_everything_as_it_was() {
    let mut app = signed_in_stale(false, "# Terms\n\nPlay fair.");
    assert_eq!(
        focused_id(&app),
        Some("text"),
        "the sheet opens on its text"
    );
    let before = sheet(&app);
    for _ in 0..10 {
        press_key(&mut app, KeyCode::Escape, Key::Escape, &[]);
    }
    assert!(signed_in(&app), "Esc never signs out");
    assert_eq!(sheet(&app), before, "the sheet as it was");
    assert_eq!(
        focused_id(&app),
        Some("not-now"),
        "Esc moves focus to Not now"
    );
    assert!(
        app.world()
            .resource::<crate::settings::ClientSettings>()
            .terms
            .is_empty(),
        "nothing stored"
    );
}

/// A guest pressing Not now is asked first (C3-3), and stays on Stay; the
/// second press signs it out with nothing stored.
#[test]
fn a_guest_is_asked_before_not_now_signs_it_out() {
    let mut app = signed_in_stale(true, "# Terms\n\nPlay fair.");
    assert!(app.world().resource::<LobbyState>().lobby.guest());
    click(&mut app, Press::Front(FrontPress::TermsNotNow));
    assert!(signed_in(&app), "asked, not signed out");
    assert!(matches!(sheet(&app), Sheet::Reading(r) if r.asking_guest));
    click(&mut app, Press::Front(FrontPress::TermsStay));
    assert!(matches!(sheet(&app), Sheet::Reading(r) if !r.asking_guest));
    click(&mut app, Press::Front(FrontPress::TermsNotNow));
    click(&mut app, Press::Front(FrontPress::TermsNotNow));
    assert!(!signed_in(&app), "the second press signs the guest out");
    assert!(!app.world().resource::<LobbyState>().terms.up());

    // An account is signed out at once.
    let mut account = signed_in_stale(false, "# Terms\n\nPlay fair.");
    click(&mut account, Press::Front(FrontPress::TermsNotNow));
    assert!(!signed_in(&account));
}

/// Accept waits for the end; End (with focus on the text) reaches it, and
/// the gateway's answer closes the sheet and keeps the device's copy.
#[test]
fn accept_waits_for_the_end_and_keeps_the_devices_copy() {
    let long = "A paragraph of the terms.\n\n".repeat(80);
    let mut app = signed_in_stale(false, &long);
    click(&mut app, Press::Front(FrontPress::TermsAccept));
    assert!(
        matches!(sheet(&app), Sheet::Reading(r) if !r.sending),
        "nothing sent before the end"
    );
    press_key(&mut app, KeyCode::End, Key::End, &[]);
    assert!(app.world().resource::<LobbyState>().terms.can_accept());
    press_key(&mut app, KeyCode::Enter, Key::Enter, &[]);
    assert!(matches!(sheet(&app), Sheet::Reading(r) if r.sending));
    post(&mut app, Reply::Terms(TermsReply::Accepted("v1".into())));
    assert!(!app.world().resource::<LobbyState>().terms.up());
    let settings = app.world().resource::<crate::settings::ClientSettings>();
    assert_eq!(
        settings.terms.values().next().map(String::as_str),
        Some("v1")
    );
}

/// A sign-in the gateway does not mark stale raises nothing.
#[test]
fn an_accepted_account_is_not_asked() {
    let mut app = headless();
    post(&mut app, Reply::Terms(TermsReply::Stale(false)));
    post(
        &mut app,
        Reply::Event(LobbyEvent::LoggedIn {
            token: "tok".into(),
            username: Some("ada".into()),
        }),
    );
    assert!(!app.world().resource::<LobbyState>().terms.up());
}

/// The sheet holds the screen: a press behind it does nothing.
#[test]
fn a_press_behind_the_sheet_does_nothing() {
    let mut app = signed_in_stale(false, "# Terms");
    let hub = app.world().resource::<LobbyState>().hub;
    click(&mut app, Press::Hub(HubPress::Tab(Hub::Decks)));
    assert_eq!(app.world().resource::<LobbyState>().hub, hub);
    assert!(app.world().resource::<LobbyState>().terms.up());
}

/// Presses `press` through the lobby's one door for a press.
fn click(app: &mut App, press: Press) {
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
    app.update();
    app.update();
}
