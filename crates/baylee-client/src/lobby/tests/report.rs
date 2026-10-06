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
    assert!(presses(&mut app).contains(&Press::PlayAgain));
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

/// A box ticks and clears on a click, and the answer is kept in the
/// device's settings, where the next form reads it.
#[test]
fn a_ticked_box_is_kept_in_the_settings() {
    let mut app = with_settings();
    open_form(&mut app);
    for category in Category::ALL {
        assert!(
            !app.world()
                .resource::<ClientSettings>()
                .reports
                .allows(category)
        );
        click_desk(
            &mut app,
            "that box",
            |p| matches!(p, DeskPress::Toggle(c) if *c == category),
        );
        assert!(
            app.world()
                .resource::<ClientSettings>()
                .reports
                .allows(category),
            "{category:?} did not tick"
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
    assert!(
        !consent.allows(Category::Log),
        "un-ticking is the revocation"
    );
    assert!(consent.allows(Category::System));
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
/// no button at all; knowing one, it says the report goes there.
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
    assert!(
        !desk_presses(&mut app)
            .iter()
            .any(|(_, p)| matches!(p, DeskPress::Send)),
        "nowhere to send it, nothing to press"
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

/// A game hosted here offers its record, unticked at every opening; a
/// ticked record is confirmed with what it shows, and "never" takes the
/// box away and is kept.
#[test]
fn a_local_games_record_is_offered_unticked_and_never_remembered() {
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
    assert!(!desk(&app).form().send_record, "unticked when it opens");

    click_desk(&mut app, "the record", |p| matches!(p, DeskPress::Record));
    assert!(desk(&app).form().send_record);
    click_desk(&mut app, "close", |p| matches!(p, DeskPress::Close));
    open_form(&mut app);
    assert!(!desk(&app).form().send_record, "unticked at every opening");
    assert_eq!(
        app.world().resource::<ClientSettings>().reports.record,
        RecordConsent::Ask,
        "a yes is kept nowhere"
    );

    click_desk(&mut app, "the record", |p| matches!(p, DeskPress::Record));
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
