//! The terms sheet as the shell runs it (WG-1; `DESIGN-v5` §11, §17 WP1;
//! `KEYBOARD.md` §7.9, §9.6): a stale sign-in raises it, Esc ×10 leaves it
//! up and the session signed in. An account may decide later, which signs
//! it out; Accept is enabled only at the end and records the device's copy.

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
        digest: None,
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
/// still up, focus on the text, nothing stored and nothing sent.
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
        Some("text"),
        "Escape returns to reading without a destructive action"
    );
    assert!(
        app.world()
            .resource::<crate::settings::ClientSettings>()
            .terms
            .is_empty(),
        "nothing stored"
    );
}

/// An account may decide later (owner, 20.10.2026: later signs out,
/// nothing accepted); a guest may not, since a guest that signs out loses
/// its decks. Failing or fetching, the sheet offers only Retry or nothing.
#[test]
fn terms_offer_later_to_an_account_and_no_hidden_sign_out_action() {
    for guest in [false, true] {
        let mut app = signed_in_stale(guest, "# Terms\n\nPlay fair.");
        let reading = if guest {
            vec!["accept", "decline"]
        } else {
            vec!["accept", "decline", "later"]
        };
        for expected in [reading, vec!["retry"], vec![]] {
            let mut query = app
                .world_mut()
                .query::<(&Press, &crate::shellkit::focus::Stop)>();
            let mut ids: Vec<_> = query
                .iter(app.world())
                .filter(|(_, stop)| stop.table == "terms")
                .map(|(_, stop)| stop.id)
                .filter(|id| *id != "impressum")
                .collect();
            ids.sort_unstable();
            assert_eq!(ids, expected, "guest={guest}, sheet={:?}", sheet(&app));
            assert!(signed_in(&app));
            if matches!(sheet(&app), Sheet::Reading(_)) {
                post(&mut app, Reply::Terms(TermsReply::FetchFailed));
            } else if matches!(sheet(&app), Sheet::Failed) {
                click(&mut app, Press::Front(FrontPress::TermsRetry));
                app.update();
            }
        }
    }
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

/// Types `text` as keys, one character at a time.
fn type_keys(app: &mut App, text: &str) {
    for ch in text.chars() {
        press_key(
            app,
            KeyCode::KeyA,
            Key::Character(ch.to_string().into()),
            &[],
        );
    }
}

/// The owner's case (08.10.2026): Decline and delete account, by keyboard
/// alone. Tab reaches it; Enter opens the account deletion's own
/// confirmation over the sheet (the password again), Esc there cancels
/// back to the sheet with focus on Decline, and the confirmed deletion goes
/// out as Settings' does and ends at the front door.
#[test]
fn decline_deletes_the_account_through_its_own_confirmation_by_keyboard() {
    let mut app = signed_in_stale(false, "# Terms\n\nPlay fair.");
    for _ in 0..6 {
        if focused_id(&app) == Some("decline") {
            break;
        }
        press_key(&mut app, KeyCode::Tab, Key::Tab, &[]);
    }
    assert_eq!(focused_id(&app), Some("decline"), "Tab never reached it");
    press_key(&mut app, KeyCode::Enter, Key::Enter, &[]);
    {
        let state = app.world().resource::<LobbyState>();
        let deletion = state.lobby.deleting_account().expect("the confirmation");
        assert!(
            deletion.refusal.is_none() && !state.lobby.busy(),
            "Enter on Decline also confirmed it"
        );
        assert!(state.terms.up(), "the sheet stays under it");
    }
    assert_eq!(focused_id(&app), None, "a stop of the sheet kept the focus");

    // Esc is the confirmation's Cancel: back to the sheet, still signed in.
    press_key(&mut app, KeyCode::Escape, Key::Escape, &[]);
    app.update();
    assert!(
        app.world()
            .resource::<LobbyState>()
            .lobby
            .deleting_account()
            .is_none()
    );
    assert!(signed_in(&app), "Esc signed out");
    assert!(matches!(sheet(&app), Sheet::Reading(_)));
    assert_eq!(focused_id(&app), Some("decline"));

    // Again, with the password typed into the confirmation's own box.
    press_key(&mut app, KeyCode::Enter, Key::Enter, &[]);
    type_keys(&mut app, "pw");
    press_key(&mut app, KeyCode::Enter, Key::Enter, &[]);
    {
        // Sent: still on its way, or already answered by the test's
        // gateway, which is no gateway — never refused here for want of
        // the password, which is the one refusal the client makes itself.
        let state = app.world().resource::<LobbyState>();
        let deletion = state.lobby.deleting_account().expect("still up while sent");
        let unsent = Phrase::DeleteAccountNeedsPassword.text(Lang::En);
        assert!(
            (state.lobby.busy() || deletion.refusal.is_some())
                && deletion.refusal.as_deref() != Some(unsent),
            "not sent: {deletion:?}"
        );
    }
    post(&mut app, Reply::Event(LobbyEvent::AccountDeleted));
    let state = app.world().resource::<LobbyState>();
    assert!(!signed_in(&app));
    assert!(!state.terms.up(), "the sheet outlived the account");
    assert!(matches!(state.lobby.screen(), Screen::SignIn { .. }));
}

/// A guest declines with no password: its confirmation sends at once.
#[test]
fn a_guest_declines_and_is_deleted_without_a_password() {
    let mut app = signed_in_stale(true, "# Terms\n\nPlay fair.");
    click(&mut app, Press::Front(FrontPress::TermsDecline));
    assert!(
        app.world()
            .resource::<LobbyState>()
            .lobby
            .deleting_account()
            .is_some()
    );
    assert!(!app.world().resource::<LobbyState>().lobby.busy());
    click(
        &mut app,
        Press::Settings(SettingsPress::ConfirmAccountDeletion),
    );
    {
        // A guest's deletion is refused by nothing on this side: it is on
        // its way, or the test's gateway (none) has answered it.
        let lobby = &app.world().resource::<LobbyState>().lobby;
        assert!(
            lobby.busy()
                || lobby
                    .deleting_account()
                    .is_some_and(|d| d.refusal.is_some()),
            "a guest's deletion was not sent"
        );
    }
    post(&mut app, Reply::Event(LobbyEvent::AccountDeleted));
    let state = app.world().resource::<LobbyState>();
    assert!(!signed_in(&app));
    assert!(state.lobby.kept_guest().is_none(), "the guest is kept");
    assert!(!state.terms.up());
}

/// Every text drawn now: `Text` roots with their spans, joined.
fn drawn(app: &mut App) -> String {
    let mut out = String::new();
    let mut roots = app.world_mut().query::<&Text>();
    for text in roots.iter(app.world()) {
        out.push_str(&text.0);
        out.push('\n');
    }
    let mut spans = app.world_mut().query::<&TextSpan>();
    for span in spans.iter(app.world()) {
        out.push_str(&span.0);
        out.push('\n');
    }
    out
}

/// The stops the sheet offers now, but the legal notice's link.
fn stops(app: &mut App) -> Vec<&'static str> {
    let mut query = app
        .world_mut()
        .query::<(&Press, &crate::shellkit::focus::Stop)>();
    let mut ids: Vec<_> = query
        .iter(app.world())
        .filter(|(_, stop)| stop.table == "terms")
        .map(|(_, stop)| stop.id)
        .filter(|id| *id != "impressum")
        .collect();
    ids.sort_unstable();
    ids
}

/// Later signs an account out with nothing accepted or stored (owner,
/// 20.10.2026: "später, dann wird er abgemeldet").
#[test]
fn later_signs_an_account_out_and_stores_nothing() {
    let mut app = signed_in_stale(false, "# Terms\n\nPlay fair.");
    click(&mut app, Press::Front(FrontPress::TermsLater));
    app.update();
    assert!(!signed_in(&app), "Later signs out");
    assert!(!app.world().resource::<LobbyState>().terms.up());
    assert!(
        app.world()
            .resource::<crate::settings::ClientSettings>()
            .terms
            .is_empty(),
        "nothing stored"
    );
}

/// A Baylee account's sheet: the client's own chat note is part of the text
/// to read, the acceptance is kept at Central, declining deletes only its
/// data on this gateway, and that names no password.
#[test]
fn a_baylee_account_reads_the_chat_note_and_deletes_only_its_data_here() {
    let mut app = signed_in_stale(false, "# Terms\n\nPlay fair.");
    post(&mut app, Reply::Terms(TermsReply::CentralBound));
    let words = drawn(&mut app);
    for phrase in [
        Phrase::TermsChatTitle,
        Phrase::TermsChatNote,
        Phrase::TermsStoredCentral,
        Phrase::TermsDeleteHere,
        Phrase::TermsLater,
    ] {
        assert!(words.contains(phrase.text(Lang::En)), "{phrase:?}");
    }
    assert!(!words.contains(Phrase::TermsDecline.text(Lang::En)));
    click(&mut app, Press::Front(FrontPress::TermsDecline));
    app.update();
    let lobby = &app.world().resource::<LobbyState>().lobby;
    assert!(lobby.deleting_account().is_some());
    assert!(
        !lobby.deletion_needs_password(),
        "a Baylee account names none here"
    );
    let words = drawn(&mut app);
    assert!(words.contains(Phrase::DeleteGatewayDataHint.text(Lang::En)));
}

/// Baylee Central's own terms come first when the sign-in says they wait:
/// the sheet names Central, carries no chat note, and offers accept and
/// later only (Central deletes no account yet, M1).
#[test]
fn central_terms_come_first_and_offer_no_deletion() {
    let mut app = headless();
    app.insert_resource(crate::settings::ClientSettings::default());
    post(&mut app, Reply::Terms(TermsReply::CentralStale));
    post(
        &mut app,
        Reply::Event(LobbyEvent::LoggedIn {
            token: "tok".into(),
            username: None,
        }),
    );
    post(&mut app, Reply::Terms(TermsReply::CentralBound));
    assert!(app.world().resource::<LobbyState>().terms.up());
    post(
        &mut app,
        Reply::Terms(TermsReply::Doc {
            asked: Lang::En,
            doc: doc("# Central terms\n\nBe kind."),
        }),
    );
    let words = drawn(&mut app);
    assert!(words.contains(Phrase::CentralTermsTitle.text(Lang::En)));
    assert!(!words.contains(Phrase::TermsChatNote.text(Lang::En)));
    assert_eq!(stops(&mut app), ["accept", "later"]);
}

/// Central's own terms before any gateway session (the owner's sign-in,
/// 22.10.2026, which failed until they were accepted on the website): the
/// sheet goes up with no token, names Central, and Later ends the Central
/// sign-in, the one session there is.
#[cfg(not(target_arch = "wasm32"))]
#[test]
fn central_terms_before_any_gateway_and_later_ends_the_central_sign_in() {
    let mut app = headless();
    app.insert_resource(crate::settings::ClientSettings::default());
    {
        let mut state = app.world_mut().resource_mut::<LobbyState>();
        super::super::central::enable(&mut state);
        state.central.as_mut().expect("Central on").terms_pending = true;
    }
    app.update();
    {
        let state = app.world().resource::<LobbyState>();
        assert!(state.terms.up(), "the sheet is up");
        assert!(state.lobby.token().is_none(), "with no gateway session");
    }
    post(
        &mut app,
        Reply::Terms(TermsReply::Doc {
            asked: Lang::En,
            doc: doc("# Central terms\n\nBe kind."),
        }),
    );
    let words = drawn(&mut app);
    assert!(words.contains(Phrase::CentralTermsTitle.text(Lang::En)));
    assert_eq!(stops(&mut app), ["accept", "later"]);
    click(&mut app, Press::Front(FrontPress::TermsLater));
    app.update();
    let state = app.world().resource::<LobbyState>();
    assert!(state.central.is_none(), "Later ends the Central sign-in");
    assert!(!state.terms.up());
}

/// A lobby signed in to Central whose own terms wait, with a gateway chosen
/// and the account worker's queue in the test's hand.
#[cfg(not(target_arch = "wasm32"))]
fn central_terms_waiting() -> (App, std::sync::mpsc::Receiver<super::super::central::Job>) {
    let mut app = headless();
    app.insert_resource(crate::settings::ClientSettings::default());
    let (handle, jobs) = super::super::central::Handle::queued_for_test();
    {
        let mut state = app.world_mut().resource_mut::<LobbyState>();
        super::super::central::enable(&mut state);
        // Chosen, as a bound sign-in's gateway is once it lands.
        state.gateway.clear();
        state.gateway_selected = true;
        let central = state.central.as_mut().expect("Central on");
        central.terms_pending = true;
        central.handle = Some(handle);
    }
    app.update();
    assert!(app.world().resource::<LobbyState>().terms.up());
    // The sheet's own request for the text, through the worker.
    assert!(jobs.try_recv().is_ok(), "the text is asked for");
    post(
        &mut app,
        Reply::Terms(TermsReply::Doc {
            asked: Lang::En,
            doc: doc("# Central terms\n\nBe kind."),
        }),
    );
    (app, jobs)
}

/// Accepted at Central, the sheet goes down and the chosen gateway is
/// entered: the one admission there is, after the terms.
#[cfg(not(target_arch = "wasm32"))]
#[test]
fn accepting_central_terms_first_then_enters_the_gateway() {
    let (mut app, jobs) = central_terms_waiting();
    assert!(jobs.try_recv().is_err(), "nothing entered yet");
    post(&mut app, Reply::Terms(TermsReply::Accepted("v1".into())));
    post(&mut app, Reply::Terms(TermsReply::Readmit));
    let state = app.world().resource::<LobbyState>();
    assert!(!state.terms.up(), "the sheet is down");
    assert!(!super::super::central::terms_pending(state));
    assert!(jobs.try_recv().is_ok(), "and the gateway is entered");
}

/// The move that waited for Central's terms failed after the acceptance:
/// Central is signed out of, as at a failed sign-in, and the lobby says so.
#[cfg(not(target_arch = "wasm32"))]
#[test]
fn a_move_that_fails_after_central_terms_ends_the_central_sign_in() {
    let (mut app, jobs) = central_terms_waiting();
    post(&mut app, Reply::Terms(TermsReply::Accepted("v1".into())));
    post(&mut app, Reply::Terms(TermsReply::MigrationFailed));
    let state = app.world().resource::<LobbyState>();
    assert!(!state.terms.up());
    assert!(state.central.is_none(), "signed out of Central");
    assert!(jobs.try_recv().is_err(), "no gateway entered");
    let words = drawn(&mut app);
    assert!(words.contains(Phrase::CentralSignInFailed.text(Lang::En)));
}
