//! Desktop field gestures share the same buffer used by the browser input.
#[allow(clippy::wildcard_imports)]
use super::*;

pub(super) struct Paste {
    field: BuildField,
    epoch: u64,
    read: bevy::clipboard::ClipboardRead,
}

#[allow(clippy::too_many_lines)] // one batch, with a single refilter after all text events
pub(super) fn builder_keys(
    keys: &mut MessageReader<KeyboardInput>,
    codes: &ButtonInput<KeyCode>,
    state: &mut ResMut<LobbyState>,
    scrolled: &mut Scrolled,
    mut clipboard: Option<&mut bevy::clipboard::Clipboard>,
    paste: &mut Option<Paste>,
) -> bool {
    if state.lobby.builder().picker().is_some() {
        keys.clear();
        if codes.just_pressed(KeyCode::Escape) {
            state.lobby.builder_mut().close_picker();
        }
        return false;
    }
    let field = state.lobby.builder().focus();
    let epoch = state.lobby.builder().focus_epoch();
    let pasted = paste
        .as_mut()
        .and_then(|p| p.read.poll_result().map(|r| (p.field, p.epoch, r)));
    if pasted.is_some() {
        *paste = None;
    }
    let pasted =
        pasted.and_then(|(f, e, r)| (f == field && e == epoch).then_some(r.ok()).flatten());
    if keys.is_empty() && pasted.is_none() {
        return false;
    }
    if state.lobby.builder().panel().is_some() && field == BuildField::Search {
        keys.clear();
        return false;
    }
    let shift = codes.any_pressed([KeyCode::ShiftLeft, KeyCode::ShiftRight]);
    let command = codes.any_pressed([
        KeyCode::SuperLeft,
        KeyCode::SuperRight,
        KeyCode::ControlLeft,
        KeyCode::ControlRight,
    ]);
    let reach = if codes.any_pressed([KeyCode::SuperLeft, KeyCode::SuperRight]) {
        Reach::Line
    } else if codes.any_pressed([
        KeyCode::AltLeft,
        KeyCode::AltRight,
        KeyCode::ControlLeft,
        KeyCode::ControlRight,
    ]) {
        Reach::Word
    } else {
        Reach::Char
    };
    let before = state.lobby.builder().focused_text().to_owned();
    let mut tab = false;
    let mut enter = false;
    let mut save = false;
    state.lobby.builder_mut().edit_buffer(field, |buffer| {
        if let Some(text) = pasted {
            buffer.insert(&text);
        }
        for key in keys.read().filter(|k| k.state.is_pressed()) {
            match &key.logical_key {
                Key::ArrowLeft => buffer.move_caret(reach, Dir::Left, shift),
                Key::ArrowRight => buffer.move_caret(reach, Dir::Right, shift),
                Key::Home => buffer.move_caret(Reach::Line, Dir::Left, shift),
                Key::End => buffer.move_caret(Reach::Line, Dir::Right, shift),
                Key::Backspace | Key::Delete => {
                    if buffer.selection().is_none() && reach != Reach::Char {
                        buffer.move_caret(
                            reach,
                            if key.logical_key == Key::Backspace {
                                Dir::Left
                            } else {
                                Dir::Right
                            },
                            true,
                        );
                    }
                    if key.logical_key == Key::Backspace {
                        buffer.delete_back();
                    } else {
                        buffer.delete_forward();
                    }
                }
                Key::Tab => tab = true,
                Key::Enter => enter = true,
                Key::Escape => buffer.deselect(),
                Key::Character(ch) if command => match ch.to_lowercase().as_str() {
                    "a" => buffer.select_all(),
                    "c" | "x" => {
                        if let (Some(range), Some(cb)) =
                            (buffer.selection(), clipboard.as_deref_mut())
                        {
                            let copied = cb.set_text(buffer.text()[range].to_owned()).is_ok();
                            if copied && ch.eq_ignore_ascii_case("x") {
                                buffer.replace_selection("");
                            }
                        }
                    }
                    "v" => {
                        if let Some(cb) = clipboard.as_deref_mut() {
                            let mut read = cb.fetch_text();
                            if let Some(result) = read.poll_result() {
                                if let Ok(text) = result {
                                    buffer.insert(&text);
                                }
                            } else {
                                *paste = Some(Paste { field, epoch, read });
                            }
                        }
                    }
                    "s" => save = true,
                    _ => {}
                },
                _ => {
                    if let Some(text) = &key.text {
                        buffer.insert(text);
                    }
                }
            }
        }
    });
    if before != state.lobby.builder().focused_text() && field == BuildField::Search {
        scrolled.set(List::Pool, 0.0);
    }
    if tab {
        state.lobby.builder_mut().cycle_focus();
    }
    if enter && field == BuildField::Search {
        let builder = state.lobby.builder_mut();
        if let Some(slot) = builder.results().first().copied() {
            builder.add(slot, builder.zone());
        }
    }
    save
}
