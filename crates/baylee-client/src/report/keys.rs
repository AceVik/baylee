//! The report sheet's keys and presses (window B): typing into the text,
//! the suggestions' keys, a paste, the body's wheel, and what each of the
//! sheet's controls does when clicked or activated by a key.

use baylee_client_core::bugreport::{CrashConsent, RecordConsent};
use baylee_client_core::i18n::Lang;
use baylee_client_core::textbuf::{Dir, Step};
use bevy::input::keyboard::{Key, KeyboardInput};
use bevy::input::mouse::{MouseScrollUnit, MouseWheel};
use bevy::prelude::*;

use super::form::{DeskPress, DeskScroll, part_lines};
use super::{Answers, ReportDesk};
use crate::lobby::LobbyState;
use crate::settings::ClientSettings;
use crate::shellkit::controls::Disabled;

/// Hands this frame's keys to the form. `true` when `Esc` closed it.
///
/// `elsewhere`: the keyboard's focus stands on another of the sheet's
/// controls (the ring shows): Tab put it there, so Enter and Space are that
/// control's (the kit's walker says so), and the text takes no keys.
///
/// With suggestions up, `↑ ↓` move in them, `Enter` or `Tab` take one, and
/// `Esc` puts them away and leaves what was typed.
///
/// `Ctrl`/`Cmd`+`V` asks `clipboard` for its text; natively the answer is
/// there at once, in a browser it comes a frame or more later, so the read
/// waits on the desk and [`take_the_paste`] lands it whenever it arrives.
/// Without a clipboard (a headless test) the chord does nothing, and in
/// particular does not type a `v`.
pub(super) fn typing(
    desk: &mut ReportDesk,
    codes: &ButtonInput<KeyCode>,
    typed: &mut MessageReader<KeyboardInput>,
    mut clipboard: Option<&mut bevy::clipboard::Clipboard>,
    elsewhere: bool,
) -> bool {
    let shift = codes.any_pressed([KeyCode::ShiftLeft, KeyCode::ShiftRight]);
    let word = codes.any_pressed([
        KeyCode::AltLeft,
        KeyCode::AltRight,
        KeyCode::ControlLeft,
        KeyCode::ControlRight,
    ]);
    let line = codes.any_pressed([KeyCode::SuperLeft, KeyCode::SuperRight]);
    let command = line || codes.any_pressed([KeyCode::ControlLeft, KeyCode::ControlRight]);
    let reach = if line {
        Step::Line
    } else if word {
        Step::Word
    } else {
        Step::Char
    };
    let mut closed = false;
    if desk.form.confirming {
        // The words are what is being confirmed: they do not change under
        // the confirmation. `Esc` goes back to them; Enter sends, unless the
        // keyboard stands on Back.
        for key in typed.read() {
            if key.state.is_pressed() && key.logical_key == Key::Escape {
                desk.form.confirming = false;
            } else if key.state.is_pressed() && key.logical_key == Key::Enter && !elsewhere {
                desk.send_by_key = Some(DeskPress::ConfirmSend);
            }
        }
        return false;
    }
    for key in typed.read() {
        if !key.state.is_pressed() {
            continue;
        }
        if suggestion_key(desk, &key.logical_key, command) {
            continue;
        }
        if elsewhere {
            match &key.logical_key {
                Key::Escape => closed = true,
                Key::Enter if command => desk.send_by_key = Some(DeskPress::Send),
                _ => {}
            }
            continue;
        }
        let text = &mut desk.form.text;
        match &key.logical_key {
            Key::Escape => closed = true,
            Key::Backspace => text.delete_back(),
            Key::Delete => text.delete_forward(),
            Key::ArrowLeft => text.move_caret(reach, Dir::Left, shift),
            Key::ArrowRight => text.move_caret(reach, Dir::Right, shift),
            Key::Home => text.move_caret(Step::Line, Dir::Left, shift),
            Key::End => text.move_caret(Step::Line, Dir::Right, shift),
            // Ctrl/Cmd+Enter sends (`KEYBOARD.md` W9 step 3); Enter alone
            // is a line break.
            Key::Enter if command => {
                desk.send_by_key = Some(DeskPress::Send);
                continue;
            }
            Key::Enter => text.replace_selection("\n"),
            // Tab walks the sheet (the kit's walker), and types nothing.
            Key::Tab => continue,
            Key::Character(c) if command => {
                if c.eq_ignore_ascii_case("a") {
                    text.select_all();
                } else if c.eq_ignore_ascii_case("v")
                    && let Some(clipboard) = clipboard.as_deref_mut()
                {
                    desk.paste = Some(clipboard.fetch_text());
                }
                continue;
            }
            _ => {
                let Some(typed) = key.text.as_ref() else {
                    continue;
                };
                let printable: String = typed.chars().filter(|c| !c.is_control()).collect();
                if printable.is_empty() {
                    continue;
                }
                text.replace_selection(&printable);
            }
        }
        desk.form.edited();
    }
    closed
}

/// A key while suggestions stand: `↑ ↓` move, `Enter` or `Tab` take,
/// `Esc` puts them away. `true` when the key was theirs.
fn suggestion_key(desk: &mut ReportDesk, key: &Key, command: bool) -> bool {
    let Some(list) = desk.form.suggestions(&desk.gathered.refs) else {
        return false;
    };
    match key {
        Key::ArrowUp => desk.form.choose(&list, -1),
        Key::ArrowDown => desk.form.choose(&list, 1),
        Key::Enter | Key::Tab if !command => {
            let ReportDesk { form, gathered, .. } = desk;
            form.take(&gathered.refs, None);
            desk.refocus = true;
        }
        Key::Escape => desk.form.dismiss(&desk.gathered.refs),
        _ => return false,
    }
    true
}

/// Lands a paste the clipboard has answered, cut to the room the limit
/// leaves ([`baylee_client_core::bugreport::ReportForm::paste`]). A read
/// that failed (an empty or non-text clipboard) is dropped without a word:
/// nothing was pasted, and the box shows that.
pub(super) fn take_the_paste(desk: &mut ReportDesk) {
    let Some(result) = desk
        .paste
        .as_mut()
        .and_then(bevy::clipboard::ClipboardRead::poll_result)
    else {
        return;
    };
    desk.paste = None;
    if let Ok(text) = result
        && desk.open
    {
        desk.form.paste(&text);
    }
}

/// Scrolls the sheet's body under the wheel.
pub(super) fn scroll(
    desk: Res<ReportDesk>,
    mut wheel: MessageReader<MouseWheel>,
    mut column: Query<&mut ScrollPosition, With<DeskScroll>>,
) {
    let delta: f32 = wheel
        .read()
        .map(|w| match w.unit {
            MouseScrollUnit::Line => w.y * 40.0,
            MouseScrollUnit::Pixel => w.y,
        })
        .sum();
    if !desk.open || delta == 0.0 {
        return;
    }
    for mut position in &mut column {
        position.y = (position.y - delta).max(0.0);
    }
}

/// A control on the form was clicked: an observer on every entity, so a
/// press meant for the form is the form's alone.
#[allow(clippy::too_many_arguments)] // an observer: one reader per thing a press may change
pub(super) fn pressed(
    mut click: On<Pointer<Click>>,
    presses: Query<&DeskPress, Without<Disabled>>,
    mut desk: ResMut<ReportDesk>,
    mut settings: ResMut<ClientSettings>,
    holders: super::Holders,
    answers: Res<Answers>,
    lobby: Option<Res<LobbyState>>,
    mut clipboard: Option<ResMut<bevy::clipboard::Clipboard>>,
) {
    let Ok(&press) = presses.get(click.entity) else {
        return;
    };
    click.propagate(false);
    act(
        press,
        &mut desk,
        &mut settings,
        &holders,
        &answers,
        lobby.as_deref(),
        clipboard.as_deref_mut(),
    );
}

/// What a press does, by pointer or by key (the kit's `Activated`).
pub(super) fn act(
    press: DeskPress,
    desk: &mut ReportDesk,
    settings: &mut ClientSettings,
    holders: &super::Holders,
    answers: &Answers,
    lobby: Option<&LobbyState>,
    clipboard: Option<&mut bevy::clipboard::Clipboard>,
) {
    match press {
        DeskPress::Kind(kind) => {
            desk.form.kind = kind;
            desk.form.edited();
        }
        DeskPress::Toggle(category) => {
            super::consent_mut(settings).toggle(category);
            settings.save();
            desk.form.edited();
        }
        DeskPress::Crashes => {
            let consent = super::consent_mut(settings);
            consent.crashes = if consent.crashes == CrashConsent::Send {
                CrashConsent::Never
            } else {
                CrashConsent::Send
            };
            settings.save();
        }
        DeskPress::Preview => desk.form.preview = !desk.form.preview,
        DeskPress::Attachments => {
            settings.report_attachments_closed = !settings.report_attachments_closed;
            settings.save();
        }
        DeskPress::Send => super::ask_to_send(desk, holders, settings, answers),
        DeskPress::ConfirmSend => super::send(desk, holders, settings, answers),
        DeskPress::ConfirmBack => desk.form.confirming = false,
        DeskPress::Record => {
            desk.form.send_record = !desk.form.send_record;
            desk.form.edited();
        }
        DeskPress::RecordNever => {
            let consent = super::consent_mut(settings);
            consent.record = if consent.record == RecordConsent::Never {
                RecordConsent::Ask
            } else {
                RecordConsent::Never
            };
            settings.save();
            desk.form.send_record = false;
            desk.form.edited();
        }
        DeskPress::Close => {
            desk.open = false;
            desk.swallow = true;
        }
        DeskPress::CrashSend | DeskPress::CrashNever => {
            let send = matches!(press, DeskPress::CrashSend);
            super::answer_the_crash_question(desk, settings, send);
        }
        DeskPress::Take(row) => {
            let ReportDesk { form, gathered, .. } = desk;
            form.take(&gathered.refs, Some(row));
            desk.refocus = true;
        }
        DeskPress::CopyText => {
            let route = super::route(lobby, settings);
            let lang = Lang::of(&settings.lang);
            let mut text = desk.form.text.text().to_string();
            text.push_str("\n\n");
            for (line, _) in part_lines(desk, settings, &route, lang) {
                text.push_str("\u{b7} ");
                text.push_str(&line);
                text.push('\n');
            }
            desk.copied = Some(clipboard.is_some_and(|c| c.set_text(text).is_ok()));
        }
    }
}
