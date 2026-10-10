//! The report form (#309, #310, #314) running in the lobby's own app: the
//! key that opens it, the keys it keeps from the screens under it, its
//! boxes, a paste, each answer of the gateway as the form says it, and the
//! crash question. What a report carries and what an answer means are
//! client-core's (`bugreport`); what is here is the wiring between them
//! and a player's hands.

#[allow(clippy::wildcard_imports)]
use super::*;

use crate::report::{DeskPress, DeskRoot, ReportDesk, gateway_answers};
use crate::settings::ClientSettings;
use baylee_client_core::bugreport::{
    Category, CrashConsent, CrashFile, CrashRecord, MAX_TEXT_CHARS, RecordConsent, Status,
};
use baylee_client_core::prefs::{Action, Chord};

/// The lobby's headless app with a settings file, which the form keeps its
/// consent in and without which it does not run.
fn with_settings() -> App {
    let mut app = headless();
    app.insert_resource(ClientSettings::default());
    app.update();
    app
}

/// Presses `code` for one frame, as a player's finger does: down, a frame,
/// up. A test's `ButtonInput` is never cleared by an input plugin, so a key
/// left down would still be `just_pressed` on the next frame.
fn tap_key(app: &mut App, code: KeyCode) {
    app.world_mut()
        .resource_mut::<ButtonInput<KeyCode>>()
        .press(code);
    app.update();
    let mut keys = app.world_mut().resource_mut::<ButtonInput<KeyCode>>();
    keys.release(code);
    keys.clear();
}

/// The form open and drawn. It waits up to 30 frames for a screenshot no
/// renderer here will take, so a test waits them out.
fn open_form(app: &mut App) {
    tap_key(app, KeyCode::F8);
    for _ in 0..32 {
        app.update();
    }
    assert!(desk(app).open, "F8 opened the form");
    assert!(drawn(app), "the form is drawn");
}

fn desk(app: &App) -> &ReportDesk {
    app.world().resource::<ReportDesk>()
}

fn desk_mut(app: &mut App) -> Mut<'_, ReportDesk> {
    app.world_mut().resource_mut::<ReportDesk>()
}

fn drawn(app: &mut App) -> bool {
    let mut roots = app.world_mut().query_filtered::<Entity, With<DeskRoot>>();
    roots.iter(app.world()).next().is_some()
}

fn keys(app: &mut App, inputs: impl IntoIterator<Item = KeyboardInput>) {
    let mut messages = app.world_mut().resource_mut::<Messages<KeyboardInput>>();
    for input in inputs {
        messages.write(input);
    }
    app.update();
}

/// The form's buttons, by what they do.
fn desk_presses(app: &mut App) -> Vec<(Entity, DeskPress)> {
    let mut query = app.world_mut().query::<(Entity, &DeskPress)>();
    query.iter(app.world()).map(|(e, p)| (e, *p)).collect()
}

/// Clicks the form's button that `pick` accepts. The form's buttons answer
/// through an observer, so the click is triggered on the entity rather than
/// written as a message.
fn click_desk(app: &mut App, what: &str, pick: impl Fn(&DeskPress) -> bool) {
    let found: Vec<(Entity, DeskPress)> = desk_presses(app)
        .into_iter()
        .filter(|(_, p)| pick(p))
        .collect();
    assert_eq!(found.len(), 1, "buttons that are {what}: {found:?}");
    app.world_mut().trigger(aimed(
        found[0].0,
        Click {
            button: PointerButton::Primary,
            hit: bevy::picking::backend::HitData::new(Entity::PLACEHOLDER, 0.0, None, None),
            duration: std::time::Duration::ZERO,
            count: 1,
        },
    ));
    app.update();
}

/// Every text the form's tree shows, in one string.
fn form_words(app: &mut App) -> String {
    let roots: Vec<Entity> = {
        let mut q = app.world_mut().query_filtered::<Entity, With<DeskRoot>>();
        q.iter(app.world()).collect()
    };
    let mut out = String::new();
    let mut stack = roots;
    while let Some(entity) = stack.pop() {
        if let Some(text) = app.world().get::<Text>(entity) {
            // A paragraph of spans (the text box) reads as one line: its
            // root and then its spans, in order.
            out.push_str(&text.0);
            for span in spans(app, entity) {
                out.push_str(&span);
            }
            out.push('\n');
        }
        if let Some(children) = app.world().get::<Children>(entity) {
            stack.extend(children.iter());
        }
    }
    out
}

/// The spans under a text entity, in order.
fn spans(app: &App, text: Entity) -> Vec<String> {
    app.world()
        .get::<Children>(text)
        .map(|children| {
            children
                .iter()
                .filter_map(|child| app.world().get::<TextSpan>(child).map(|s| s.0.clone()))
                .collect()
        })
        .unwrap_or_default()
}

/// The report box's paragraph and its three spans.
fn box_spans(app: &mut App) -> (Entity, Vec<String>) {
    let text = {
        let mut q = app
            .world_mut()
            .query_filtered::<Entity, With<crate::report::DeskText>>();
        q.single(app.world()).expect("one paragraph in the box")
    };
    (text, spans(app, text))
}

/// The report box's caret: its entity and what it wears.
fn box_caret(app: &mut App) -> (Entity, bool, Color) {
    let mut q = app
        .world_mut()
        .query::<(Entity, &crate::report::DeskCaret, &BackgroundColor)>();
    let (entity, caret, colour) = q.single(app.world()).expect("one caret in the box");
    (entity, caret.placed, colour.0)
}

/// `F8` opens the form over the sign-in screen, what is typed goes into its
/// box and not into the field under it, and `Esc` shuts it.
#[test]
fn f8_opens_the_form_and_its_keys_do_not_reach_the_screen_under_it() {
    let mut app = with_settings();
    assert!(!desk(&app).open);
    open_form(&mut app);
    keys(&mut app, [typed('h'), typed('i')]);
    assert_eq!(desk(&app).form().text.text(), "hi");
    assert_eq!(
        app.world()
            .resource::<LobbyState>()
            .lobby
            .field(Field::Username),
        "",
        "a key the form took also reached the sign-in field"
    );
    keys(&mut app, [pressed(KeyCode::Escape, Key::Escape)]);
    assert!(!desk(&app).open, "Esc shut the form");
    app.update();
    assert!(!drawn(&mut app), "and it is gone from the screen");
    // And the field under it has the keyboard back.
    keys(&mut app, [typed('x')]);
    assert_eq!(
        app.world()
            .resource::<LobbyState>()
            .lobby
            .field(Field::Username),
        "x"
    );
    assert_eq!(desk(&app).form().text.text(), "hi", "what was typed stays");
}

/// The key is the keymap's, not `F8`'s: rebound, the old key does nothing
/// and the new one opens it.
#[test]
fn the_report_key_follows_the_keymap() {
    let mut app = with_settings();
    app.world_mut()
        .resource_mut::<crate::prefs::Prefs>()
        .edit()
        .keymap
        .bind(Action::Report, vec![Chord::key("F9")]);
    tap_key(&mut app, KeyCode::F8);
    app.update();
    assert!(!desk(&app).open, "F8 still opened it after the rebinding");
    tap_key(&mut app, KeyCode::F9);
    // The key asks; the form opens on the frame after, as it does for F8.
    app.update();
    assert!(desk(&app).open, "the new key did not open it");
}

/// Over the end screen the form has the keys: `Esc` shuts the form and
/// does not also leave the table, and `Enter` is a line break in the text
/// and not "play again".
#[test]
fn over_the_end_screen_the_form_s_keys_are_not_the_sheet_s_answers() {
    let mut app = with_settings();
    stocked(&mut app);
    {
        let mut state = app.world_mut().resource_mut::<LobbyState>();
        state.lobby.host(GameMode::Ai);
        state.lobby.apply(LobbyEvent::Seated(SeatHandover {
            game_id: "g1".to_string(),
            seat: 0,
            seat_token: "st".to_string(),
            local: false,
        }));
        state.connected = true;
    }
    phase(&mut app, DuelPhase::Finished);
    assert!(presses(&mut app).contains(&Press::End(EndPress::PlayAgain)));
    open_form(&mut app);
    let closes = |app: &mut App| {
        app.world_mut()
            .resource_mut::<Messages<DuelCommand>>()
            .drain()
            .filter(|c| matches!(c, DuelCommand::Close))
            .count()
    };
    closes(&mut app);

    app.world_mut()
        .resource_mut::<ButtonInput<KeyCode>>()
        .press(KeyCode::Enter);
    keys(&mut app, [pressed(KeyCode::Enter, Key::Enter)]);
    let mut input = app.world_mut().resource_mut::<ButtonInput<KeyCode>>();
    input.release(KeyCode::Enter);
    input.clear();
    assert_eq!(closes(&mut app), 0, "Enter in the form played again");
    assert_eq!(desk(&app).form().text.text(), "\n");

    app.world_mut()
        .resource_mut::<ButtonInput<KeyCode>>()
        .press(KeyCode::Escape);
    keys(&mut app, [pressed(KeyCode::Escape, Key::Escape)]);
    assert!(!desk(&app).open, "Esc shut the form");
    assert_eq!(
        closes(&mut app),
        0,
        "the Esc that shut the form left the table"
    );
    assert_eq!(
        app.world().resource::<LobbyState>().lobby.rematch_wanted(),
        None
    );
}

/// The corner button stands while a table is up, playing or finished, over
/// the end screen and under the form; a click on it opens the form, over the
/// end screen too; and it is gone once the table is.
#[test]
fn the_corner_button_stands_at_the_table_and_opens_the_form() {
    use crate::report::ReportCorner;
    let corners = |app: &mut App| {
        let mut q = app
            .world_mut()
            .query_filtered::<(Entity, &GlobalZIndex), With<ReportCorner>>();
        q.iter(app.world())
            .map(|(e, z)| (e, z.0))
            .collect::<Vec<_>>()
    };
    let click = |app: &mut App, button: Entity| {
        app.world_mut().trigger(aimed(
            button,
            Click {
                button: PointerButton::Primary,
                hit: bevy::picking::backend::HitData::new(Entity::PLACEHOLDER, 0.0, None, None),
                duration: std::time::Duration::ZERO,
                count: 1,
            },
        ));
        for _ in 0..32 {
            app.update();
        }
    };
    let mut app = with_settings();
    assert!(corners(&mut app).is_empty(), "a button over the lobby");

    phase(&mut app, DuelPhase::Playing);
    app.update();
    let standing = corners(&mut app);
    assert_eq!(standing.len(), 1, "one button at the table");
    let (button, rung) = standing[0];
    // Where the panels that keep clear of it think it is.
    let node = app.world().get::<Node>(button).expect("a node").clone();
    let window = Vec2::new(1280.0, 720.0);
    let corner = crate::hud::report_corner(window);
    assert_eq!(
        (node.right, node.top, node.width, node.height),
        (
            px(window.x - corner.max.x),
            px(corner.min.y),
            px(corner.width()),
            px(corner.height())
        ),
        "the button stands where `report_corner` says"
    );
    assert!(
        rung > crate::hud::G_PREVIEW_OVER_FINISH,
        "the button is over the end screen and what stands on it"
    );
    click(&mut app, button);
    assert!(desk(&app).open, "the button opened the form");
    let shade = {
        let mut q = app
            .world_mut()
            .query_filtered::<&GlobalZIndex, With<DeskRoot>>();
        q.iter(app.world()).map(|z| z.0).collect::<Vec<_>>()
    };
    assert!(
        shade.iter().all(|&z| z > rung),
        "the form stands over the button: {shade:?} against {rung}"
    );
    click_desk(&mut app, "close", |p| matches!(p, DeskPress::Close));
    assert!(!desk(&app).open);

    phase(&mut app, DuelPhase::Finished);
    app.update();
    let standing = corners(&mut app);
    assert_eq!(standing.len(), 1, "the button outlives the game");
    click(&mut app, standing[0].0);
    assert!(desk(&app).open, "and opens the form over the end screen");
    click_desk(&mut app, "close", |p| matches!(p, DeskPress::Close));

    phase(&mut app, DuelPhase::Closed);
    app.update();
    assert!(
        corners(&mut app).is_empty(),
        "the button outlived the table"
    );
}

/// `Ctrl`+`V` asks the clipboard, and what it answers lands at the caret,
/// cut to the room the limit leaves; the `v` is not typed.
#[test]
fn a_paste_lands_in_the_box_cut_to_the_limit() {
    let mut app = with_settings();
    open_form(&mut app);
    // No clipboard in a headless app: the chord asks nothing and types
    // nothing.
    app.world_mut()
        .resource_mut::<ButtonInput<KeyCode>>()
        .press(KeyCode::ControlLeft);
    keys(&mut app, [typed('v')]);
    assert_eq!(desk(&app).form().text.text(), "", "the chord typed a v");

    desk_mut(&mut app).clipboard_answers("line one\r\nline two");
    app.update();
    assert_eq!(desk(&app).form().text.text(), "line one\nline two");

    desk_mut(&mut app).clipboard_answers(&"z".repeat(MAX_TEXT_CHARS * 2));
    app.update();
    assert_eq!(desk(&app).form().chars(), MAX_TEXT_CHARS);
    assert!(!desk(&app).form().over_limit());
}

/// Every box starts ticked, clears and ticks on a click, and the answer is
/// kept in the device's settings, where the next form reads it.
#[test]
fn an_unticked_box_is_kept_in_the_settings() {
    let mut app = with_settings();
    open_form(&mut app);
    for category in Category::ALL {
        assert!(
            app.world()
                .resource::<ClientSettings>()
                .reports
                .allows(category),
            "{category:?} starts ticked"
        );
        click_desk(
            &mut app,
            "that box",
            |p| matches!(p, DeskPress::Toggle(c) if *c == category),
        );
        assert!(
            !app.world()
                .resource::<ClientSettings>()
                .reports
                .allows(category),
            "{category:?} did not clear"
        );
    }
    // Shut and opened again, the boxes are as they were left.
    keys(&mut app, [pressed(KeyCode::Escape, Key::Escape)]);
    app.update();
    open_form(&mut app);
    click_desk(&mut app, "the log box", |p| {
        matches!(p, DeskPress::Toggle(Category::Log))
    });
    let consent = &app.world().resource::<ClientSettings>().reports;
    assert!(consent.allows(Category::Log), "a tick is kept too");
    assert!(
        !consent.allows(Category::System),
        "un-ticking is the revocation"
    );
}

/// A tick or a kind is redrawn in place, never the whole form (§10 #9 of
/// the shell design) — unless the preview is open, whose text it changes.
#[test]
fn a_ticked_box_redraws_its_mark_and_not_the_form() {
    let mut app = with_settings();
    open_form(&mut app);
    let mark = |app: &mut App| {
        let (entity, _) = desk_presses(app)
            .into_iter()
            .find(|(_, p)| matches!(p, DeskPress::Toggle(Category::System)))
            .expect("the system box");
        let first = app
            .world()
            .get::<Children>(entity)
            .expect("a box has a mark")[0];
        app.world().get::<Text>(first).expect("the mark").0.clone()
    };
    let before = (desk(&app).redraws, mark(&mut app));
    click_desk(&mut app, "the system box", |p| {
        matches!(p, DeskPress::Toggle(Category::System))
    });
    click_desk(&mut app, "the improvement kind", |p| {
        matches!(
            p,
            DeskPress::Kind(baylee_client_core::bugreport::Kind::Improvement)
        )
    });
    assert_eq!(desk(&app).redraws, before.0, "a tick redrew the whole form");
    assert_ne!(mark(&mut app), before.1, "the tick was not drawn");
    // Under the preview the payload is on screen, and it changes with a box.
    click_desk(&mut app, "the preview", |p| matches!(p, DeskPress::Preview));
    let shown = desk(&app).redraws;
    click_desk(&mut app, "the system box", |p| {
        matches!(p, DeskPress::Toggle(Category::System))
    });
    assert_eq!(
        desk(&app).redraws,
        shown + 1,
        "the preview kept a stale payload"
    );
}

/// The lobby has no table, so the game, the log and the picture have
/// nothing behind them: the boxes say so, and ticked they send nothing.
#[test]
fn a_box_over_nothing_says_so_and_its_tick_sends_nothing() {
    let mut app = with_settings();
    app.world_mut().resource_mut::<ClientSettings>().reports =
        baylee_client_core::bugreport::Consent::everything();
    open_form(&mut app);
    let words = form_words(&mut app);
    let nothing = Phrase::ReportCatNothing.text(Lang::En);
    assert!(
        words.matches(nothing).count() >= 2,
        "the game and the log say they have nothing: {words}"
    );
    click_desk(&mut app, "the preview", |p| matches!(p, DeskPress::Preview));
    let words = form_words(&mut app);
    assert!(
        words.contains("\"system\""),
        "the preview shows the ticked system part"
    );
    for absent in ["\"game\"", "\"log\"", "\"screenshot\""] {
        assert!(
            !words.contains(absent),
            "the preview shows {absent}: {words}"
        );
    }
}

/// Each answer the gateway can give is said in the form in its own
/// sentence, and the text survives every refusal.
#[test]
fn each_answer_of_the_gateway_is_said_in_the_form() {
    let mut app = with_settings();
    open_form(&mut app);
    keys(&mut app, [typed('o'), typed('w')]);
    for (status, body, said) in [
        (
            400,
            r#"{"error":"kind is not one of the five"}"#,
            "kind is not one of the five".to_string(),
        ),
        (
            401,
            "",
            Phrase::ReportSignInAgain.text(Lang::En).to_string(),
        ),
        (413, "", Phrase::ReportTooLarge.text(Lang::En).to_string()),
        (429, "", Phrase::ReportTooMany.text(Lang::En).to_string()),
        (
            502,
            "",
            Phrase::ReportNotPassedOn.text(Lang::En).to_string(),
        ),
        (
            503,
            r#"{"error":"reports are not configured"}"#,
            Phrase::ReportsUnavailable.text(Lang::En).to_string(),
        ),
        (0, "", Phrase::ReportUnreachable.text(Lang::En).to_string()),
        (
            201,
            r#"{"report_id":"r-77"}"#,
            Phrase::ReportSent.fill(Lang::En, &["r-77"]),
        ),
    ] {
        desk_mut(&mut app).form_mut().status = Status::Sending;
        gateway_answers(app.world(), status, body);
        app.update();
        app.update();
        let words = form_words(&mut app);
        assert!(
            words.contains(&said),
            "{status}: {said:?} is not in {words}"
        );
        if status != 201 {
            assert_eq!(
                desk(&app).form().text.text(),
                "ow",
                "{status} lost the text"
            );
        }
    }
    assert!(
        desk(&app).form().text.is_empty(),
        "a received report empties the box"
    );
}

/// A send the size limit trimmed says so under the status line.
#[test]
fn a_trimmed_send_says_what_was_left_out() {
    let mut app = with_settings();
    open_form(&mut app);
    {
        let mut desk = desk_mut(&mut app);
        desk.form_mut().text = baylee_client_core::textbuf::TextBuffer::new("big");
        desk.gathered_mut().screenshot = Some(baylee_client_core::bugreport::Screenshot {
            width: 1,
            height: 1,
            png_base64: "A".repeat(baylee_client_core::bugreport::MAX_CLIENT_BYTES),
        });
    }
    app.world_mut()
        .resource_mut::<ClientSettings>()
        .reports
        .screenshot = true;
    let consent = app.world().resource::<ClientSettings>().reports.clone();
    let gathered = {
        let mut desk = desk_mut(&mut app);
        desk.gathered_mut().clone()
    };
    let json = desk_mut(&mut app)
        .form_mut()
        .prepare(
            &gathered,
            &consent,
            &[],
            baylee_client_core::bugreport::Via::Gateway,
        )
        .expect("sendable once trimmed");
    assert!(!json.contains("AAAA"), "the picture went out");
    assert!(desk(&app).form().trimmed.screenshot);
    app.update();
    let words = form_words(&mut app);
    assert!(
        words.contains(Phrase::ReportTrimmed.text(Lang::En)),
        "the form did not say the picture was left out: {words}"
    );
}

fn a_crash() -> String {
    CrashFile {
        gateway: Some("http://127.0.0.1:1".into()),
        record: CrashRecord {
            message: "index out of bounds".into(),
            backtrace: Some("   0: baylee_client::main".into()),
            ..CrashRecord::default()
        },
        ..CrashFile::default()
    }
    .to_text()
}

/// A crash never asked about is asked about, once, in a question that has
/// the keyboard; "send" is remembered and the crash waits for the courier,
/// "never" is remembered and the crash is dropped.
#[test]
fn the_crash_question_is_asked_and_its_answer_kept() {
    for (send, answer) in [(true, CrashConsent::Send), (false, CrashConsent::Never)] {
        let mut app = with_settings();
        let remove = desk_mut(&mut app).found(CrashConsent::Unasked, Some(&a_crash()));
        assert!(!remove, "an unanswered crash is kept");
        app.update();
        assert!(desk(&app).asking() && desk(&app).holds_keyboard());
        let words = form_words(&mut app);
        assert!(
            words.contains(Phrase::CrashAskTitle.text(Lang::En)),
            "{words}"
        );
        // The question keeps the keys from the sign-in field.
        keys(&mut app, [typed('q')]);
        assert_eq!(
            app.world()
                .resource::<LobbyState>()
                .lobby
                .field(Field::Username),
            ""
        );
        if send {
            click_desk(&mut app, "send crash reports", |p| {
                matches!(p, DeskPress::CrashSend)
            });
        } else {
            click_desk(&mut app, "never", |p| matches!(p, DeskPress::CrashNever));
        }
        assert_eq!(
            app.world().resource::<ClientSettings>().reports.crashes,
            answer
        );
        assert!(!desk(&app).asking());
        assert_eq!(desk(&app).crash().is_some(), send, "{answer:?}");
        app.update();
        assert!(!drawn(&mut app), "the question is gone");
    }
}

/// Answered before, the question is not asked again: a device that said
/// "never" deletes the file, one that said "send" keeps the crash for the
/// courier, and a torn file is deleted unasked.
#[test]
fn an_answered_device_is_not_asked_again() {
    let mut app = with_settings();
    assert!(desk_mut(&mut app).found(CrashConsent::Never, Some(&a_crash())));
    assert!(!desk(&app).asking() && desk(&app).crash().is_none());
    assert!(!desk_mut(&mut app).found(CrashConsent::Send, Some(&a_crash())));
    assert!(!desk(&app).asking() && desk(&app).crash().is_some());
    let mut app = with_settings();
    assert!(desk_mut(&mut app).found(CrashConsent::Unasked, Some("{\"torn")));
    assert!(!desk(&app).asking());
    assert!(!desk_mut(&mut app).found(CrashConsent::Unasked, None));
    assert!(!desk(&app).asking());
}

/// With "send" on, the courier sends the crash once the lobby is signed in
/// at the gateway the crash names, and only then.
#[test]
fn the_courier_sends_a_crash_once_signed_in_where_it_belongs() {
    let mut app = with_settings();
    app.world_mut()
        .resource_mut::<ClientSettings>()
        .reports
        .crashes = CrashConsent::Send;
    desk_mut(&mut app).found(CrashConsent::Send, Some(&a_crash()));
    app.update();
    assert!(!desk(&app).crash_tried(), "sent without a session");
    {
        let mut state = app.world_mut().resource_mut::<LobbyState>();
        state.gateway = "http://127.0.0.1:1".into();
        state.lobby.apply(LobbyEvent::LoggedIn {
            token: "5e55105e55105e55105e55105e55105e".into(),
            username: None,
        });
    }
    app.update();
    assert!(
        desk(&app).crash_tried(),
        "not sent once signed in where it belongs"
    );
}

/// The owner's PS on report 01a0e3ec (#320): the box had no visible caret.
///
/// It was `▏` spliced into the text, a glyph neither shipped face has. The
/// caret is now a bar of its own, and the text is three spans — before the
/// caret, the selection, after it — that it stands between; no glyph of the
/// text stands for it.
#[test]
fn the_box_s_caret_is_a_bar_between_the_spans_and_no_glyph() {
    let mut app = with_settings();
    open_form(&mut app);
    keys(&mut app, [typed('h'), typed('i')]);
    keys(&mut app, [pressed(KeyCode::ArrowLeft, Key::ArrowLeft)]);
    let (_, parts) = box_spans(&mut app);
    // The tail ends in the one space the caret's reading leans on.
    assert_eq!(parts, ["h", "", "i "], "head, selection, tail");
    let words = form_words(&mut app);
    assert!(
        !words.contains('\u{258f}'),
        "no caret glyph in the text: {words:?}"
    );
    let (caret, _, _) = box_caret(&mut app);
    let node = app.world().get::<Node>(caret).expect("the caret is a node");
    assert_eq!(node.position_type, PositionType::Absolute);
    assert!(
        matches!(node.width, Val::Px(w) if w > 0.0),
        "a bar with a width"
    );

    // A selection is its own span, painted, with the caret at its held end.
    app.world_mut()
        .resource_mut::<ButtonInput<KeyCode>>()
        .press(KeyCode::ShiftLeft);
    keys(&mut app, [pressed(KeyCode::ArrowLeft, Key::ArrowLeft)]);
    let (text, parts) = box_spans(&mut app);
    assert_eq!(parts, ["", "h", "i "]);
    let selected = app.world().get::<Children>(text).expect("spans")[1];
    assert!(
        app.world()
            .get::<bevy::text::TextBackgroundColor>(selected)
            .is_some(),
        "the selection is painted"
    );
}

/// The other half of the PS: the text ran out of the box to the right.
///
/// The box's paragraph is as wide as the box and wraps (a word too long for
/// a line breaks inside itself rather than overflowing), and the box has a
/// height past which it scrolls instead of growing without end.
#[test]
fn the_box_wraps_its_text_and_scrolls_past_its_height() {
    let mut app = with_settings();
    open_form(&mut app);
    keys(&mut app, "word ".repeat(40).chars().map(typed));
    let (text, _) = box_spans(&mut app);
    let layout = app.world().get::<TextLayout>(text).expect("a layout");
    assert_eq!(
        layout.linebreak,
        bevy::text::LineBreak::WordOrCharacter,
        "wraps, and breaks a word no line can hold"
    );
    let node = app.world().get::<Node>(text).expect("a node");
    assert_eq!(node.width, percent(100), "as wide as the box, no wider");
    let boxed = {
        let mut q = app
            .world_mut()
            .query_filtered::<&Node, With<crate::report::DeskBox>>();
        q.single(app.world()).expect("one text box").clone()
    };
    assert_eq!(boxed.overflow.y, OverflowAxis::Scroll, "scrolls down");
    assert!(
        matches!((boxed.min_height, boxed.max_height), (Val::Px(lo), Val::Px(hi)) if hi > lo),
        "grows with its text up to a height, then scrolls"
    );
}

/// A rebuild keeps the box where it was scrolled: every keystroke rebuilds
/// the form, and a long text thrown back to its first line on each one
/// would hide the line being typed.
#[test]
fn a_keystroke_keeps_the_box_where_it_was_scrolled() {
    let mut app = with_settings();
    open_form(&mut app);
    desk_mut(&mut app).set_box_scroll(57.0);
    keys(&mut app, [typed('x')]);
    let scrolled = {
        let mut q = app
            .world_mut()
            .query_filtered::<&ScrollPosition, With<crate::report::DeskBox>>();
        q.single(app.world()).expect("one text box").y
    };
    assert!((scrolled - 57.0).abs() < f32::EPSILON, "{scrolled}");
}

/// The caret blinks at the rate every text box here blinks at, lit again
/// the moment it moves, and holds still under `reduce_motion`.
#[test]
fn the_box_s_caret_blinks_unless_asked_to_hold_still() {
    fn colours(app: &mut App, frames: usize) -> Vec<Color> {
        (0..frames)
            .map(|_| {
                app.update();
                box_caret(app).2
            })
            .collect()
    }
    let mut app = with_settings();
    app.insert_resource(bevy::time::TimeUpdateStrategy::ManualDuration(
        std::time::Duration::from_millis(200),
    ));
    open_form(&mut app);
    // An empty box is laid out as soon as it is drawn: its caret stands at
    // the start, where no text has to be measured to find it.
    let (_, placed, _) = box_caret(&mut app);
    assert!(placed, "the empty box's caret is placed");
    let seen = colours(&mut app, 8);
    assert!(seen.contains(&palette::INK), "lit: {seen:?}");
    assert!(seen.contains(&Color::NONE), "and dark: {seen:?}");

    app.world_mut()
        .resource_mut::<crate::prefs::Prefs>()
        .edit()
        .reduce_motion = true;
    let still = colours(&mut app, 8);
    assert!(
        still.iter().all(|c| *c == palette::INK),
        "reduce_motion holds it lit: {still:?}"
    );
}

/// A feedback service only this test's settings name, on a port nothing
/// listens on: the build's own default never decides what a test sees.
const SERVICE: &str = "http://127.0.0.1:9";

/// The settings with direct reports going to [`SERVICE`] (`Some`) or
/// turned off (`None`).
fn service(app: &mut App, url: Option<&str>) {
    app.world_mut()
        .resource_mut::<ClientSettings>()
        .feedback_url = Some(url.unwrap_or_default().to_string());
}

/// Signed in nowhere and knowing no service, the form says so and Send is
/// off (the kit's disabled button: focusable, saying why, answering
/// nothing); knowing one, it says the report goes there.
#[test]
fn signed_in_nowhere_the_form_says_where_a_report_would_go() {
    let mut app = with_settings();
    service(&mut app, None);
    open_form(&mut app);
    keys(&mut app, [typed('x')]);
    let words = form_words(&mut app);
    assert!(
        words.contains(Phrase::ReportNeedsSession.text(Lang::En)),
        "{words}"
    );
    let sends: Vec<Entity> = desk_presses(&mut app)
        .into_iter()
        .filter(|(_, p)| matches!(p, DeskPress::Send))
        .map(|(e, _)| e)
        .collect();
    assert!(
        !sends.is_empty()
            && sends.iter().all(|e| app
                .world()
                .get::<crate::shellkit::controls::Disabled>(*e)
                .is_some()),
        "nowhere to send it, nothing to press"
    );
    let before = desk(&app).form().status.clone();
    app.world_mut().trigger(aimed(
        sends[0],
        Click {
            button: PointerButton::Primary,
            hit: bevy::picking::backend::HitData::new(Entity::PLACEHOLDER, 0.0, None, None),
            duration: std::time::Duration::ZERO,
            count: 1,
        },
    ));
    app.update();
    assert_eq!(
        desk(&app).form().status,
        before,
        "a dead Send did something"
    );

    service(&mut app, Some(SERVICE));
    app.update();
    let words = form_words(&mut app);
    assert!(
        words.contains(&Phrase::ReportGoesDirect.fill(Lang::En, &[SERVICE])),
        "{words}"
    );
    assert!(!words.contains(Phrase::ReportNeedsSession.text(Lang::En)));
}

/// Straight to the service, Send first shows what goes and where; Back
/// returns to the words, and only the confirmation's Send sends, under a
/// device id made for it and kept.
#[test]
fn a_direct_report_is_confirmed_before_it_goes() {
    let mut app = with_settings();
    service(&mut app, Some(SERVICE));
    open_form(&mut app);
    keys(&mut app, [typed('o'), typed('w')]);
    click_desk(&mut app, "send", |p| matches!(p, DeskPress::Send));
    assert!(desk(&app).form().confirming);
    assert_eq!(
        desk(&app).form().status,
        Status::Editing,
        "nothing sent yet"
    );
    let words = form_words(&mut app);
    for said in [
        Phrase::ReportConfirmTitle.text(Lang::En).to_string(),
        Phrase::ReportConfirmTo.fill(Lang::En, &[SERVICE]),
        Phrase::ReportConfirmText.fill(Lang::En, &["2"]),
        Phrase::ReportConfirmDevice.text(Lang::En).to_string(),
    ] {
        assert!(words.contains(&said), "{said:?} is not in {words}");
    }
    assert!(!words.contains(&Phrase::ReportConfirmRecord.fill(Lang::En, &["0"])[..20]));

    // Keys do not reach the words under the confirmation; Esc goes back.
    keys(&mut app, [typed('!')]);
    assert_eq!(desk(&app).form().text.text(), "ow");
    click_desk(&mut app, "back", |p| matches!(p, DeskPress::ConfirmBack));
    assert!(!desk(&app).form().confirming && desk(&app).open);

    assert!(
        app.world()
            .resource::<ClientSettings>()
            .report_device
            .is_none(),
        "no id before the first direct report"
    );
    click_desk(&mut app, "send", |p| matches!(p, DeskPress::Send));
    click_desk(&mut app, "send now", |p| {
        matches!(p, DeskPress::ConfirmSend)
    });
    // On its way, or already refused by the port nobody listens on, in the
    // service's own words: either way it went to the service.
    assert!(
        matches!(
            desk(&app).form().status,
            Status::Sending
                | Status::Failed(baylee_client_core::i18n::Refusal::Said(
                    Phrase::ReportDirectUnreachable
                ))
        ),
        "{:?}",
        desk(&app).form().status
    );
    let device = app
        .world()
        .resource::<ClientSettings>()
        .report_device
        .clone()
        .expect("an id was made for it");
    assert!(baylee_client_core::bugreport::is_device_id(&device));
}

/// A game hosted here offers its record, ticked again at every opening; a
/// ticked record is confirmed with what it shows, and "never" takes the
/// box away and is kept.
#[test]
fn a_local_games_record_is_offered_ticked_and_never_remembered() {
    let mut app = with_settings();
    service(&mut app, Some(SERVICE));
    let host = crate::host::house_duel().expect("the house duel builds");
    app.insert_resource(crate::InstalledHost(Box::new(host)));
    open_form(&mut app);
    let words = form_words(&mut app);
    assert!(
        words.contains(Phrase::ReportRecordBox.text(Lang::En)),
        "{words}"
    );
    assert!(desk(&app).form().send_record, "ticked when it opens");

    click_desk(&mut app, "the record", |p| matches!(p, DeskPress::Record));
    assert!(!desk(&app).form().send_record);
    click_desk(&mut app, "close", |p| matches!(p, DeskPress::Close));
    open_form(&mut app);
    assert!(desk(&app).form().send_record, "ticked at every opening");
    assert_eq!(
        app.world().resource::<ClientSettings>().reports.record,
        RecordConsent::Ask,
        "an answer is kept nowhere"
    );

    keys(&mut app, [typed('x')]);
    click_desk(&mut app, "send", |p| matches!(p, DeskPress::Send));
    let kilobytes = desk_mut(&mut app)
        .gathered_mut()
        .local_record
        .as_ref()
        .expect("gathered")
        .kilobytes()
        .to_string();
    let words = form_words(&mut app);
    assert!(
        words.contains(&Phrase::ReportConfirmRecord.fill(Lang::En, &[&kilobytes])),
        "{words}"
    );
    click_desk(&mut app, "back", |p| matches!(p, DeskPress::ConfirmBack));

    click_desk(&mut app, "never", |p| matches!(p, DeskPress::RecordNever));
    assert_eq!(
        app.world().resource::<ClientSettings>().reports.record,
        RecordConsent::Never
    );
    assert!(!desk(&app).form().send_record);
    assert!(
        !desk_presses(&mut app)
            .iter()
            .any(|(_, p)| matches!(p, DeskPress::Record)),
        "never: no box"
    );
    click_desk(&mut app, "send", |p| matches!(p, DeskPress::Send));
    let words = form_words(&mut app);
    assert!(!words.contains(&Phrase::ReportConfirmRecord.fill(Lang::En, &[&kilobytes])));
}

/// How far the form itself is scrolled.
fn panel_scroll(app: &mut App) -> f32 {
    let mut q = app
        .world_mut()
        .query_filtered::<&ScrollPosition, With<crate::report::DeskScroll>>();
    q.single(app.world()).expect("one form column").y
}

/// A tick rebuilds the form and keeps it where it was scrolled: the
/// record's boxes stand at its foot, and a form thrown back to its top on
/// each tick hid the box just ticked (found playing, 06.10.). A new
/// opening starts at the top again.
#[test]
fn a_tick_keeps_the_form_where_it_was_scrolled() {
    let mut app = with_settings();
    service(&mut app, Some(SERVICE));
    let host = crate::host::house_duel().expect("the house duel builds");
    app.insert_resource(crate::InstalledHost(Box::new(host)));
    open_form(&mut app);
    {
        let mut q = app
            .world_mut()
            .query_filtered::<&mut ScrollPosition, With<crate::report::DeskScroll>>();
        q.single_mut(app.world_mut()).expect("one form column").y = 64.0;
    }
    click_desk(&mut app, "the record", |p| matches!(p, DeskPress::Record));
    assert!(!desk(&app).form().send_record, "the click landed");
    let scrolled = panel_scroll(&mut app);
    assert!((scrolled - 64.0).abs() < f32::EPSILON, "{scrolled}");

    click_desk(&mut app, "close", |p| matches!(p, DeskPress::Close));
    open_form(&mut app);
    assert!(panel_scroll(&mut app).abs() < f32::EPSILON, "a new opening");
}

/// A game hosted here has no gateway to add its record or an id to name
/// it by, so the form does not say it sends either (found playing, 06.10.).
#[test]
fn a_local_game_s_form_does_not_promise_a_gateway_s_record() {
    let mut app = with_settings();
    service(&mut app, Some(SERVICE));
    let host = crate::host::house_duel().expect("the house duel builds");
    app.insert_resource(crate::InstalledHost(Box::new(host)));
    open_form(&mut app);
    // What always goes is said in What is sent (window B), beside the body.
    click_desk(&mut app, "what is sent", |p| {
        matches!(p, DeskPress::Preview)
    });
    let words = form_words(&mut app);
    assert!(
        words.contains(Phrase::ReportAlwaysLocal.text(Lang::En)),
        "{words}"
    );
    assert!(
        !words.contains(Phrase::ReportAlways.text(Lang::En)),
        "{words}"
    );
}

/// The attachments stand behind one disclosure (§12): open until it is
/// closed, closed again on the next report (the device remembers), and its
/// count follows a tick without the form being drawn again.
#[test]
fn the_attachments_close_behind_their_count_and_stay_closed() {
    let mut app = with_settings();
    open_form(&mut app);
    let toggles = |app: &mut App| {
        desk_presses(app)
            .iter()
            .filter(|(_, p)| matches!(p, DeskPress::Toggle(_)))
            .count()
    };
    assert!(toggles(&mut app) > 0, "open on a first report");
    let words = form_words(&mut app);
    // Every box starts ticked (owner, 10.10.2026): "N of N".
    let there = words
        .lines()
        .find_map(|line| {
            let (ticked, of) = line.trim().split_once(" of ")?;
            let digits = |s: &str| !s.is_empty() && s.chars().all(|c| c.is_ascii_digit());
            (digits(ticked) && digits(of)).then(|| of.to_string())
        })
        .expect("the count beside the disclosure");
    assert!(words.contains(&format!("{there} of {there}")), "{words}");
    let redraws = desk(&app).redraws;
    click_desk(&mut app, "the system box", |p| {
        matches!(p, DeskPress::Toggle(Category::System))
    });
    assert_eq!(desk(&app).redraws, redraws, "a tick redrew the form");
    assert!(!there.is_empty() && there != "0", "{words}");
    let all: usize = there.parse().expect("a count");
    let one_less = format!("{} of {there}", all - 1);
    assert!(
        form_words(&mut app).contains(&one_less),
        "the count followed"
    );

    click_desk(&mut app, "the disclosure", |p| {
        matches!(p, DeskPress::Attachments)
    });
    assert_eq!(toggles(&mut app), 0, "closed");
    keys(&mut app, [pressed(KeyCode::Escape, Key::Escape)]);
    app.update();
    open_form(&mut app);
    assert_eq!(toggles(&mut app), 0, "still closed on the next report");
    assert!(
        app.world()
            .resource::<ClientSettings>()
            .report_attachments_closed
    );
}

/// `KEYBOARD.md` W9 step 3: Ctrl/Cmd+Enter in the text asks to send (the
/// confirmation comes up) without typing a line break, and Enter on the
/// confirmation sends. Every Enter was a line break, so the form could not
/// be sent from the keyboard (beta.6 QA).
#[test]
fn command_enter_asks_to_send_and_enter_confirms() {
    let command = if crate::shellkit::keys::mac() {
        KeyCode::SuperLeft
    } else {
        KeyCode::ControlLeft
    };
    let mut app = with_settings();
    service(&mut app, Some(SERVICE));
    open_form(&mut app);
    keys(&mut app, [typed('o'), typed('w')]);
    app.world_mut()
        .resource_mut::<ButtonInput<KeyCode>>()
        .press(command);
    keys(&mut app, [pressed(KeyCode::Enter, Key::Enter)]);
    app.world_mut()
        .resource_mut::<ButtonInput<KeyCode>>()
        .release(command);
    app.update();
    assert_eq!(desk(&app).form().text.text(), "ow", "no line break typed");
    assert!(desk(&app).form().confirming, "the confirmation is up");
    keys(&mut app, [pressed(KeyCode::Enter, Key::Enter)]);
    app.update();
    assert!(
        matches!(
            desk(&app).form().status,
            Status::Sending
                | Status::Failed(baylee_client_core::i18n::Refusal::Said(
                    Phrase::ReportDirectUnreachable
                ))
        ),
        "{:?}",
        desk(&app).form().status
    );
}

/// The report's suggestions as `/state.report` names them.
fn suggested(app: &App) -> Vec<String> {
    desk(app).state_json(Lang::En)["suggestions"]
        .as_array()
        .map(|rows| {
            rows.iter()
                .map(|r| r["name"].as_str().unwrap_or_default().to_string())
                .collect()
        })
        .unwrap_or_default()
}

/// Window B, away from a table: the first `#` loads the compiled pool,
/// `#` and a name's start offers it, `Enter` writes it in brackets, the
/// written name is a span that previews its printing when hovered, and the
/// report carries it as a reference.
#[test]
fn a_hash_away_from_a_table_offers_the_pool_and_writes_the_name() {
    let mut app = with_settings();
    // What the picking plugin registers in a running client.
    app.add_message::<Pointer<Over>>()
        .add_message::<Pointer<Out>>()
        .add_message::<Pointer<bevy::picking::events::Press>>()
        .add_message::<Pointer<Release>>();
    open_form(&mut app);
    assert_eq!(desk(&app).gathered_refs_len(), 0, "the pool waits for a #");
    keys(&mut app, "cast #Lightning Bol".chars().map(typed));
    assert!(desk(&app).gathered_refs_len() > 100, "the pool arrived");
    let offered = suggested(&app);
    assert_eq!(
        offered.first().map(String::as_str),
        Some("Lightning Bolt"),
        "{offered:?}"
    );
    // A suggestion stands under the caret, a pointer's popover.
    let mut popovers = app
        .world_mut()
        .query_filtered::<Entity, With<crate::report::DeskSuggest>>();
    assert_eq!(popovers.iter(app.world()).count(), 1);

    keys(&mut app, [pressed(KeyCode::Enter, Key::Enter)]);
    assert_eq!(desk(&app).form().text.text(), "cast [Lightning Bolt] ");
    assert!(suggested(&app).is_empty(), "taken, the list closed");
    app.update();
    let span = {
        let mut links = app
            .world_mut()
            .query::<(Entity, &crate::report::ReportLink)>();
        let found: Vec<Entity> = links.iter(app.world()).map(|(e, _)| e).collect();
        assert_eq!(found.len(), 1, "the name is one reference span");
        found[0]
    };
    app.world_mut().write_message(aimed(
        span,
        Over {
            hit: bevy::picking::backend::HitData::new(Entity::PLACEHOLDER, 0.0, None, None),
        },
    ));
    app.update();
    app.update();
    let mut previews = app.world_mut().query::<&crate::report::ReportPreview>();
    let keys_shown: Vec<String> = previews.iter(app.world()).map(|p| p.key.clone()).collect();
    assert_eq!(keys_shown.len(), 1, "one preview");
    let bolt = baylee_cards::pool::rows()
        .iter()
        .find(|r| r.english_name == "Lightning Bolt")
        .expect("Bolt is in the pool");
    assert_eq!(keys_shown[0], format!("{}#0", bolt.scryfall_id));

    let refs = desk(&app).state_json(Lang::En)["refs"].clone();
    assert_eq!(refs["cards"][0]["text"], "Lightning Bolt");
    assert_eq!(refs["cards"][0]["card"], bolt.index);
    assert_eq!(refs["cards"][0]["at"], serde_json::json!([5, 21]));
}

/// `Esc` with suggestions up puts them away and keeps what was typed; the
/// next `Esc` closes the sheet, the draft kept.
#[test]
fn esc_puts_the_suggestions_away_before_it_closes_the_sheet() {
    let mut app = with_settings();
    open_form(&mut app);
    keys(&mut app, "#Wrath".chars().map(typed));
    assert!(!suggested(&app).is_empty());
    keys(&mut app, [pressed(KeyCode::Escape, Key::Escape)]);
    assert!(desk(&app).open, "the first Esc is the list's");
    assert!(suggested(&app).is_empty());
    assert_eq!(desk(&app).form().text.text(), "#Wrath");
    keys(&mut app, [pressed(KeyCode::Escape, Key::Escape)]);
    assert!(!desk(&app).open, "the second closes the sheet");
    assert_eq!(desk(&app).form().text.text(), "#Wrath", "the draft stays");
}

/// Away from a table a card's second column is its type line in the
/// sheet's language, and it wraps inside the popover instead of running
/// over its right edge (4K pass, 09.10.2026: "Legendary Artifact Creature —
/// Human" ran out of the box, in English under a German sheet).
#[test]
fn the_hash_popover_s_type_line_is_localized_and_stays_inside() {
    let mut app = headless();
    app.insert_resource(ClientSettings {
        lang: "de".to_string(),
        ..ClientSettings::default()
    });
    app.update();
    app.add_message::<Pointer<Over>>()
        .add_message::<Pointer<Out>>()
        .add_message::<Pointer<bevy::picking::events::Press>>()
        .add_message::<Pointer<Release>>();
    open_form(&mut app);
    keys(&mut app, "#Lightning Bol".chars().map(typed));
    app.update();
    let mut metas = app
        .world_mut()
        .query_filtered::<(&Text, &Node, &TextLayout), With<crate::report::DeskSuggestMeta>>();
    let rows: Vec<(String, Node, TextLayout)> = metas
        .iter(app.world())
        .map(|(t, n, l)| (t.0.clone(), n.clone(), *l))
        .collect();
    assert!(!rows.is_empty(), "the popover has rows");
    let instant = baylee_client_core::deckbuilder::translated_type_line("Instant", Lang::De);
    assert_ne!(instant, "Instant", "the dictionary knows the word");
    assert!(
        rows.iter().any(|(text, _, _)| *text == instant),
        "no row said {instant}: {:?}",
        rows.iter().map(|r| &r.0).collect::<Vec<_>>()
    );
    for (text, node, layout) in &rows {
        assert!(node.flex_shrink > 0.0, "{text} does not give way");
        assert_eq!(node.min_width, Val::Px(0.0), "{text} keeps its width");
        assert_ne!(layout.linebreak, LineBreak::NoWrap, "{text} cannot wrap");
    }
}

/// At a table `#` offers the seat's own view and never the pool: a card in
/// the hand is offered as the seat's, and a card no zone of the view holds
/// is not offered at all, however well it is known to this build.
#[test]
fn at_a_table_the_hash_offers_the_view_and_never_the_pool() {
    use baylee_client_core::test_support::{ViewBuilder, printed, statics};
    let mut app = with_settings();
    app.init_resource::<crate::Duel>();
    let mut view = ViewBuilder::new(2)
        .with_hand(vec![("Lightning Bolt", 1, 7)])
        .with_battlefield(1, [printed(8, 1, "Island", 8)])
        .build();
    view.seats[1].hand_count = 7;
    {
        let mut duel = app.world_mut().resource_mut::<crate::Duel>();
        duel.view = Some(view);
        let mut table = statics(10);
        table.seats.push(baylee_view::SeatIdentity {
            player: baylee_core::ids::PlayerId::new(1),
            display_name: "steady 1".into(),
            is_ai: true,
            away: false,
            team: None,
        });
        duel.statics = Some(table);
    }
    phase(&mut app, DuelPhase::Playing);
    open_form(&mut app);
    keys(&mut app, "#Li".chars().map(typed));
    let state = desk(&app).state_json(Lang::En);
    assert_eq!(state["at_table"], true);
    assert_eq!(state["suggestions"][0]["name"], "Lightning Bolt");
    assert_eq!(state["suggestions"][0]["meta"], "Hand \u{b7} yours");
    keys(&mut app, " #Isl".chars().map(typed));
    let state = desk(&app).state_json(Lang::En);
    assert_eq!(
        state["suggestions"][0]["meta"],
        "Battlefield \u{b7} steady 1"
    );
    keys(&mut app, " #Wrath".chars().map(typed));
    assert!(
        suggested(&app).is_empty(),
        "a card in no zone of the view was offered"
    );
    assert_eq!(desk(&app).gathered_refs_len(), 2, "the view's two, no pool");
    keys(&mut app, " @st".chars().map(typed));
    assert_eq!(suggested(&app), ["steady 1"]);
    keys(&mut app, [pressed(KeyCode::Enter, Key::Enter)]);
    assert!(desk(&app).form().text.text().ends_with("[@steady 1] "));
}

/// The tours' anchors on the sheet (`TOURS.md` §3.1, T31–T33): the sheet,
/// the Attachments disclosure, the route line, and the record's row where a
/// local game's record is offered.
#[test]
fn the_sheet_keeps_the_tours_anchors() {
    let anchors = |app: &mut App| {
        let mut q = app.world_mut().query::<&crate::report::ReportAnchor>();
        let mut ids: Vec<&str> = q.iter(app.world()).map(|a| a.id()).collect();
        ids.sort_unstable();
        ids
    };
    let mut app = with_settings();
    service(&mut app, Some(SERVICE));
    let host = crate::host::house_duel().expect("the house duel builds");
    app.insert_resource(crate::InstalledHost(Box::new(host)));
    open_form(&mut app);
    assert_eq!(
        anchors(&mut app),
        [
            "report_attachments",
            "report_form",
            "report_record_row",
            "report_route"
        ]
    );
}
