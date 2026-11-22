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

/// The owner's case (19.11.2026): the foot's three answers are as wide as
/// their words, never equal shares of the row (which cut the longest label
/// at its face), and Decline stands apart from the two ways on.
#[test]
fn the_foot_gives_each_answer_its_own_width_and_sets_decline_apart() {
    let mut app = signed_in_stale(false, "# Terms\n\nPlay fair.");
    let mut query = app
        .world_mut()
        .query::<(&crate::shellkit::focus::Stop, &Node, &ChildOf)>();
    let found: Vec<(&'static str, Val, f32, Entity)> = query
        .iter(app.world())
        .filter(|(stop, ..)| stop.table == "terms")
        .filter(|(stop, ..)| matches!(stop.id, "accept" | "decline" | "later"))
        .map(|(stop, node, parent)| (stop.id, node.flex_basis, node.flex_grow, parent.parent()))
        .collect();
    assert_eq!(found.len(), 3, "{found:?}");
    for (id, basis, grow, _) in &found {
        assert_eq!(*basis, Val::Auto, "{id} is sized by a share, not its words");
        assert!(*grow <= 0.0, "{id} grows into the row");
    }
    let holder = |id: &str| found.iter().find(|f| f.0 == id).expect("stop").3;
    assert_eq!(
        holder("accept"),
        holder("later"),
        "the ways on stand together"
    );
    assert_ne!(holder("accept"), holder("decline"), "Decline stands apart");
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
pub(super) fn click(app: &mut App, press: Press) {
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
    assert!(state.guests.is_empty(), "its dead token is kept");
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
    assert!(drawn(&mut app).contains(Phrase::TermsLater.text(Lang::En)));
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

/// A gateway's sheet comes up on the gateway's word, so a hostile one could
/// raise it at will: its *Later* leaves that gateway, and the player's session
/// at Central stands.
#[cfg(not(target_arch = "wasm32"))]
#[test]
fn later_on_a_gateway_s_sheet_leaves_the_central_sign_in_standing() {
    let mut app = signed_in_stale(false, "# Terms\n\nPlay fair.");
    let (handle, _jobs) = super::super::central::Handle::queued_for_test();
    {
        let mut state = app.world_mut().resource_mut::<LobbyState>();
        super::super::central::enable(&mut state);
        state.central.as_mut().expect("Central on").handle = Some(handle);
    }
    app.update();
    app.update();
    let words = drawn(&mut app);
    assert!(words.contains(Phrase::TermsLaterGateway.text(Lang::En)));
    assert!(!words.contains(Phrase::TermsLater.text(Lang::En)));
    click(&mut app, Press::Front(FrontPress::TermsLater));
    app.update();
    let state = app.world().resource::<LobbyState>();
    assert!(state.central_ready(), "Later ended the Central sign-in");
    assert!(!signed_in(&app), "and it left the gateway");
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
    assert!(
        state.central_admitting(),
        "the chosen gateway is being entered"
    );
    assert!(jobs.try_recv().is_ok(), "and the gateway is entered");
}

/// The move that waited for Central's terms failed after the acceptance:
/// Central is signed out of, as at a failed sign-in, and the lobby says so.
#[cfg(not(target_arch = "wasm32"))]
#[test]
fn a_move_that_fails_after_central_terms_ends_the_central_sign_in() {
    let (mut app, jobs) = central_terms_waiting();
    let old_epoch = app.world().resource::<LobbyState>().gateway_epoch;
    post(&mut app, Reply::Terms(TermsReply::MigrationFailed));
    post(
        &mut app,
        Reply::Remote(old_epoch, Box::new(Reply::Terms(TermsReply::Readmit))),
    );
    let state = app.world().resource::<LobbyState>();
    assert!(!state.terms.up());
    assert!(state.central.is_none(), "signed out of Central");
    assert!(!state.central_admitting() && state.lobby.token().is_none());
    assert!(jobs.try_recv().is_err(), "no gateway entered");
    let words = drawn(&mut app);
    assert!(words.contains(Phrase::CentralSignInFailed.text(Lang::En)));
}

#[cfg(not(target_arch = "wasm32"))]
fn terms_layout_app() -> (App, UiFonts) {
    let (mut app, mut fonts) = crate::shellkit::header::tests::layout_app();
    let mut store = app.world_mut().resource_mut::<Assets<Font>>();
    let mut load = |file: &str| {
        let path = format!("{}/assets/fonts/{file}", env!("CARGO_MANIFEST_DIR"));
        store.add(Font::from_bytes(std::fs::read(path).expect(file)))
    };
    fonts.italic = load("AlegreyaSans-Italic.ttf");
    fonts.medium_italic = load("AlegreyaSans-MediumItalic.ttf");
    (app, fonts)
}

#[cfg(not(target_arch = "wasm32"))]
fn rect_of(world: &World, entity: Entity) -> Rect {
    let cn = world.get::<ComputedNode>(entity).expect("laid out");
    let gt = world
        .get::<UiGlobalTransform>(entity)
        .expect("UI transform");
    let inv = cn.inverse_scale_factor();
    Rect::from_center_size(gt.translation * inv, cn.size() * inv)
}

#[cfg(not(target_arch = "wasm32"))]
fn german_gateway_terms_state(central_bound: bool, read_to_end: bool) -> LobbyState {
    let md = include_str!("../../../../../docs/gateway-legal/gateway-terms.de.md");
    let mut state = LobbyState::from_settings(crate::settings::ClientSettings::default());
    state.lobby.set_lang(Lang::De);
    state.lobby.apply(LobbyEvent::LoggedIn {
        token: "tok".into(),
        username: Some("ada".into()),
    });
    if central_bound {
        state.lobby.set_central_bound(true);
        let (handle, _jobs) = super::super::central::Handle::queued_for_test();
        super::super::central::enable(&mut state);
        state.central.as_mut().expect("Central on").handle = Some(handle);
    }
    state.terms_subject = crate::lobby::front::terms::Subject::Gateway;
    let _ = state.terms.ask(Lang::De);
    state.terms.loaded(
        TermsDoc {
            version: "2026-10-21".into(),
            digest: None,
            updated: Some("2026-10-21".into()),
            markdown: md.into(),
            lang: Some("de".into()),
        },
        Lang::De,
    );
    if read_to_end {
        state.terms.reached_end();
    }
    state
}

#[cfg(not(target_arch = "wasm32"))]
fn spawn_laid_out_sheet(
    app: &mut App,
    state: LobbyState,
    kit: crate::shellkit::controls::Kit,
    w: f32,
    h: f32,
) -> Entity {
    let scrolled = Scrolled::default();
    let root = {
        let mut commands = app.world_mut().commands();
        let root = commands
            .spawn(Node {
                width: Val::Px(w),
                height: Val::Px(h),
                ..default()
            })
            .id();
        crate::lobby::front::terms::sheet(&mut commands, root, &state, kit, &scrolled);
        root
    };
    app.insert_resource(state);
    app.world_mut().flush();
    app.update();
    app.update();
    root
}

/// The terms scroller's `content_size` ends at its last block (list items wrap
/// at the row's width instead of reporting a one-word-per-line overflow ~857 px
/// past the end), and "read to the end" enables Accept right at that real end.
#[cfg(not(target_arch = "wasm32"))]
#[test]
fn the_terms_scroller_ends_at_the_last_block_and_accepts_there() {
    use crate::lobby::front::terms::{TermsProgress, TermsText, read_the_terms};
    use crate::shellkit::{ShellMetrics, TextSize, Viewport};

    let (mut app, fonts) = terms_layout_app();
    app.insert_resource(LobbyState::new()).add_systems(
        PostUpdate,
        read_the_terms.after(bevy::ui::UiSystems::Layout),
    );

    for (w, h, step) in [
        (1920.0, 1080.0, TextSize::M),
        (2560.0, 1440.0, TextSize::M),
        (1280.0, 800.0, TextSize::Xl),
        (844.0, 390.0, TextSize::M),
    ] {
        let kit = crate::shellkit::controls::Kit {
            fonts: &fonts,
            m: ShellMetrics::of(Viewport::desktop(w, h), step),
            german: true,
        };
        let state = german_gateway_terms_state(false, false);
        let root = spawn_laid_out_sheet(&mut app, state, kit, w, h);

        let (scroller, seen, content_h, expected_end) = {
            let mut q = app.world_mut().query_filtered::<(
                Entity,
                &ComputedNode,
                &UiGlobalTransform,
                &Children,
            ), With<TermsText>>();
            let (scroller, cn, gt, kids) = q.single(app.world()).expect("terms scroller");
            let inv = cn.inverse_scale_factor();
            let scroller_top = (gt.translation.y - cn.size().y * 0.5) * inv;
            let last = kids.last().copied().expect("last block");
            let last_bottom_rel = rect_of(app.world(), last).max.y - scroller_top;
            (
                scroller,
                cn.size().y * inv,
                cn.content_size().y * inv,
                last_bottom_rel + kit.m.scaled(18.0),
            )
        };
        assert!(
            (content_h - expected_end).abs() <= 4.0,
            "{w}x{h} {step:?}: content_size.y={content_h:.1} extends past last block end={expected_end:.1}"
        );
        assert!(
            !app.world().resource::<LobbyState>().terms.can_accept(),
            "{w}x{h} {step:?}: unread at top"
        );

        app.world_mut()
            .get_mut::<ScrollPosition>(scroller)
            .expect("scroll pos")
            .y = (expected_end - seen - 24.0).max(0.0);
        app.update();
        assert!(
            !app.world().resource::<LobbyState>().terms.can_accept(),
            "{w}x{h} {step:?}: accepted 24 px before the real end"
        );

        let real_end = (expected_end - seen).max(0.0);
        app.world_mut()
            .get_mut::<ScrollPosition>(scroller)
            .expect("scroll pos")
            .y = real_end;
        app.update();
        assert!(
            app.world().resource::<LobbyState>().terms.can_accept(),
            "{w}x{h} {step:?}: real end ({real_end:.1}) did not enable Accept"
        );
        let bar_w = app
            .world_mut()
            .query_filtered::<&Node, With<TermsProgress>>()
            .single(app.world())
            .expect("progress bar")
            .width;
        assert_eq!(bar_w, Val::Percent(100.0), "{w}x{h} {step:?}");

        app.world_mut().entity_mut(root).despawn();
        app.update();
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn assert_terms_answers_fit(
    app: &mut App,
    foot_rect: Rect,
    tag: &str,
) -> std::collections::HashMap<&'static str, Rect> {
    use crate::shellkit::focus::Stop;

    let mut stops = std::collections::HashMap::new();
    let mut q = app.world_mut().query::<(Entity, &Stop, &Children)>();
    for (entity, stop, kids) in q.iter(app.world()) {
        if stop.table != "terms" || !matches!(stop.id, "decline" | "later" | "accept") {
            continue;
        }
        let face = kids.first().copied().expect("button face");
        let face_cn = app.world().get::<ComputedNode>(face).expect("face cn");
        let inv = face_cn.inverse_scale_factor();
        let face_inner_w = face_cn.content_box().size().x * inv;
        let words = app
            .world()
            .get::<Children>(face)
            .expect("face kids")
            .first()
            .copied()
            .expect("words");
        let text = &app.world().get::<Text>(words).expect("label").0;
        if stop.id == "decline" {
            assert_eq!(text, Phrase::TermsDeleteHere.text(Lang::De));
        }
        let words_w = app
            .world()
            .get::<ComputedNode>(words)
            .expect("words cn")
            .size()
            .x
            * inv;
        let glyph_w = app
            .world()
            .get::<bevy::text::TextLayoutInfo>(words)
            .expect("glyph layout")
            .size
            .x
            * inv;
        assert!(
            words_w <= face_inner_w + 1.0 && glyph_w <= face_inner_w + 1.0,
            "{tag} {}: label {text:?} ({glyph_w:.1}/{words_w:.1}) cut in {face_inner_w:.1}",
            stop.id
        );
        let hit_rect = rect_of(app.world(), entity);
        assert!(
            hit_rect.min.x >= foot_rect.min.x - 1.0
                && hit_rect.max.x <= foot_rect.max.x + 1.0
                && hit_rect.min.y >= foot_rect.min.y - 1.0
                && hit_rect.max.y <= foot_rect.max.y + 1.0,
            "{tag} {}: {hit_rect:?} leaves foot {foot_rect:?}",
            stop.id
        );
        stops.insert(stop.id, hit_rect);
    }
    stops
}

/// "Meine Daten auf diesem Gateway löschen" has room across desktop, narrow and
/// phone viewports without clipping at its button face, footer or sheet.
#[cfg(not(target_arch = "wasm32"))]
#[test]
fn delete_my_data_on_this_gateway_has_room_without_clipping() {
    use crate::shellkit::role::Role;
    use crate::shellkit::surfaces::ModalFoot;
    use crate::shellkit::{ShellMetrics, TextSize, Viewport};

    let (mut app, fonts) = terms_layout_app();
    for (w, h, step, one_row) in [
        (1920.0, 1080.0, TextSize::M, true),
        (2560.0, 1440.0, TextSize::M, true),
        (1280.0, 800.0, TextSize::M, true),
        (1180.0, 820.0, TextSize::M, true),
        (960.0, 700.0, TextSize::M, false),
        (900.0, 800.0, TextSize::M, false),
        (1280.0, 800.0, TextSize::Xl, false),
        (844.0, 390.0, TextSize::M, false),
        (640.0, 360.0, TextSize::M, false),
    ] {
        let tag = format!("{w}x{h} {step:?}");
        let kit = crate::shellkit::controls::Kit {
            fonts: &fonts,
            m: ShellMetrics::of(Viewport::desktop(w, h), step),
            german: true,
        };
        let state = german_gateway_terms_state(true, true);
        let root = spawn_laid_out_sheet(&mut app, state, kit, w, h);

        let surface = app
            .world_mut()
            .query::<(Entity, &Role)>()
            .iter(app.world())
            .find_map(|(e, r)| (*r == Role::Opaque).then_some(e))
            .expect("dialog surface");
        let foot = app
            .world_mut()
            .query_filtered::<Entity, With<ModalFoot>>()
            .single(app.world())
            .expect("modal foot");
        let surface_rect = rect_of(app.world(), surface);
        let foot_rect = rect_of(app.world(), foot);
        assert!(
            surface_rect.min.x >= -0.5
                && surface_rect.min.y >= -0.5
                && surface_rect.max.x <= w + 0.5
                && surface_rect.max.y <= h + 0.5,
            "{tag}: surface {surface_rect:?} leaves viewport {w}x{h}"
        );
        assert!(
            foot_rect.min.x >= surface_rect.min.x - 1.0
                && foot_rect.max.x <= surface_rect.max.x + 1.0
                && foot_rect.max.y <= surface_rect.max.y + 1.0,
            "{tag}: foot {foot_rect:?} clipped by surface {surface_rect:?}"
        );

        let stops = assert_terms_answers_fit(&mut app, foot_rect, &tag);
        assert_eq!(stops.len(), 3, "{tag}");
        for (a, ra) in &stops {
            for (b, rb) in &stops {
                if a < b {
                    let overlap = ra.intersect(*rb);
                    assert!(
                        overlap.width() <= 0.5 || overlap.height() <= 0.5,
                        "{tag}: {a} {ra:?} overlaps {b} {rb:?}"
                    );
                }
            }
        }
        if one_row {
            let (decline, later, accept) = (stops["decline"], stops["later"], stops["accept"]);
            assert!(
                (decline.center().y - accept.center().y).abs() <= 1.0
                    && (later.center().y - accept.center().y).abs() <= 1.0,
                "{tag}: answers wrapped on a wide desktop ({stops:?})"
            );
            assert!(
                decline.max.x <= later.min.x && later.max.x <= accept.min.x,
                "{tag}: answers out of order ({stops:?})"
            );
        }

        app.world_mut().entity_mut(root).despawn();
        app.update();
    }
}

/// While a `ModalScrim` stands over the lobby, non-refusal toasts (notices and
/// bell rings) are held back without ageing, appearing for their full duration
/// once the modal closes; refusal toasts still show immediately.
#[test]
fn modal_scrim_holds_non_refusal_toasts_without_ageing_and_shows_refusals() {
    use std::time::Duration;

    let mut app = signed_in_stale(false, "# Terms\n\nPlay fair.");
    app.world_mut()
        .resource_mut::<Time<Virtual>>()
        .set_max_delta(Duration::from_secs(60));
    app.insert_resource(bevy::time::TimeUpdateStrategy::ManualDuration(
        Duration::from_secs(5),
    ));

    let lane_count = |app: &mut App| {
        app.world_mut()
            .query_filtered::<(), With<super::super::header::ToastLane>>()
            .iter(app.world())
            .count()
    };

    {
        let mut state = app.world_mut().resource_mut::<LobbyState>();
        state.lobby.note_said("the house is warming up");
        state
            .bell
            .ring(baylee_client_core::lobby::strips::BellItem::Joined(
                "Lina#0031".into(),
            ));
    }
    // 20 s > TOAST_SECS (15 s): neither notice nor bell toast covers the modal,
    // and neither ages out behind it.
    for _ in 0..4 {
        app.update();
        assert_eq!(lane_count(&mut app), 0, "non-refusal toast bled over modal");
    }

    app.insert_resource(bevy::time::TimeUpdateStrategy::ManualDuration(
        Duration::from_millis(16),
    ));

    // A refusal while the modal stands DOES show.
    post(
        &mut app,
        Reply::Terms(TermsReply::SendFailed("Gateway busy".into())),
    );
    assert_eq!(lane_count(&mut app), 1, "refusal toast must show on modal");
    let words = drawn(&mut app);
    assert!(words.contains("Gateway busy"), "{words}");
    assert!(
        !words.contains("Lina#0031 sat down at your table"),
        "bell toast must stay held while modal stands: {words}"
    );

    // Clear the refusal and put a fresh notice behind the modal before closing,
    // plus verify the bell toast from 20+ seconds ago still has its full life!
    app.world_mut()
        .resource_mut::<LobbyState>()
        .lobby
        .note_said("the house is warming up");
    app.update();
    assert_eq!(
        lane_count(&mut app),
        0,
        "notice still held while modal stands"
    );

    press_key(&mut app, KeyCode::End, Key::End, &[]);
    press_key(&mut app, KeyCode::Enter, Key::Enter, &[]);
    post(&mut app, Reply::Terms(TermsReply::Accepted("v1".into())));
    app.update();
    assert!(!app.world().resource::<LobbyState>().terms.up());
    assert_eq!(
        lane_count(&mut app),
        1,
        "held toasts appear once modal closes"
    );
    let words = drawn(&mut app);
    assert!(words.contains("The house is warming up"), "{words}");
    assert!(
        words.contains("Lina#0031 sat down at your table"),
        "bell toast aged out while held behind modal: {words}"
    );

    // Once shown, they age normally and leave after 15 s.
    app.insert_resource(bevy::time::TimeUpdateStrategy::ManualDuration(
        Duration::from_secs(8),
    ));
    app.update();
    app.update();
    assert_eq!(lane_count(&mut app), 0, "toasts age out once unheld");
}

#[cfg(not(target_arch = "wasm32"))]
fn toast_layout_app(w: u16, h: u16) -> (App, UiFonts) {
    use crate::lobby::header::{Toasts, stand_over_the_update_card, toast_lane};
    use crate::shellkit::InputClass;
    use bevy::camera::{ComputedCameraValues, RenderTargetInfo};

    let (mut app, fonts) = terms_layout_app();
    let (uw, uh) = (u32::from(w), u32::from(h));
    app.insert_resource(fonts.clone())
        .insert_resource(InputClass::Pointer)
        .insert_resource(crate::settings::ClientSettings::default())
        .init_resource::<Toasts>()
        .add_systems(
            Update,
            (toast_lane, stand_over_the_update_card.after(toast_lane)),
        )
        .add_systems(
            PostUpdate,
            stand_over_the_update_card
                .after(bevy::ui::UiSystems::Layout)
                .before(bevy::ui::UiSystems::PostLayout),
        );
    app.world_mut().spawn((
        Window {
            resolution: bevy::window::WindowResolution::new(uw, uh).with_scale_factor_override(1.0),
            ..default()
        },
        bevy::window::PrimaryWindow,
    ));
    app.world_mut().spawn((
        Camera2d,
        bevy::ui::IsDefaultUiCamera,
        Camera {
            computed: ComputedCameraValues {
                target_info: Some(RenderTargetInfo {
                    physical_size: UVec2::new(uw, uh),
                    scale_factor: 1.0,
                }),
                ..default()
            },
            ..default()
        },
    ));
    (app, fonts)
}

/// A refusal toast shown while the terms `ModalScrim` stands is placed above
/// the modal's foot so it never covers the Accept button.
#[cfg(not(target_arch = "wasm32"))]
#[test]
fn refusal_toast_stands_above_the_terms_modal_foot() {
    use crate::lobby::header::ToastLane;
    use crate::shellkit::focus::Stop;
    use crate::shellkit::surfaces::ModalFoot;
    use crate::shellkit::{ShellMetrics, TextSize, Viewport};

    for (w, h) in [(1280u16, 800u16), (1920u16, 1080u16)] {
        let (mut app, fonts) = toast_layout_app(w, h);
        let mut state = german_gateway_terms_state(false, true);
        state.lobby.say("Gateway busy");

        let (fw, fh) = (f32::from(w), f32::from(h));
        let kit = crate::shellkit::controls::Kit {
            fonts: &fonts,
            m: ShellMetrics::of(Viewport::desktop(fw, fh), TextSize::M),
            german: true,
        };
        let _ = spawn_laid_out_sheet(&mut app, state, kit, fw, fh);
        app.update();

        let lane = app
            .world_mut()
            .query_filtered::<Entity, With<ToastLane>>()
            .single(app.world())
            .expect("refusal toast lane");
        let foot = app
            .world_mut()
            .query_filtered::<Entity, With<ModalFoot>>()
            .single(app.world())
            .expect("terms modal foot");
        let accept = app
            .world_mut()
            .query::<(Entity, &Stop)>()
            .iter(app.world())
            .find_map(|(e, s)| (s.table == "terms" && s.id == "accept").then_some(e))
            .expect("accept stop");

        let lane_rect = rect_of(app.world(), lane);
        let foot_rect = rect_of(app.world(), foot);
        let accept_rect = rect_of(app.world(), accept);
        assert!(
            lane_rect.height() > 0.0 && lane_rect.max.y <= foot_rect.min.y - kit.m.gap + 1.0,
            "{w}x{h}: refusal lane {lane_rect:?} is not above modal foot {foot_rect:?}"
        );
        assert!(
            lane_rect.intersect(accept_rect).is_empty(),
            "{w}x{h}: refusal lane {lane_rect:?} covers Accept {accept_rect:?}"
        );
    }
}
