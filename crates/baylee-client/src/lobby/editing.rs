//! Desktop field gestures share the same buffer used by the browser input.
#[allow(clippy::wildcard_imports)]
use super::*;
use crate::buildui::Nav;

pub(super) struct Paste {
    field: BuildField,
    epoch: u64,
    read: bevy::clipboard::ClipboardRead,
}

/// A paste into a sign-in or table field that the clipboard has not
/// answered yet. It lands only in the field it was asked for, and only while
/// the caret has not been placed again since.
pub(super) struct FormPaste {
    pub(super) field: Field,
    pub(super) epoch: u64,
    pub(super) read: bevy::clipboard::ClipboardRead,
}

/// Lands a paste the clipboard has now answered, if it has. Touches the
/// lobby only when there is text to land, so a quiet frame marks nothing
/// changed.
pub(super) fn land_form_paste(state: &mut ResMut<LobbyState>, paste: &mut Option<FormPaste>) {
    let Some(answer) = paste.as_mut().and_then(|p| p.read.poll_result()) else {
        return;
    };
    let Some(FormPaste { field, epoch, .. }) = paste.take() else {
        return;
    };
    if let Ok(text) = answer
        && state.lobby.focus() == field
        && state.lobby.focus_epoch() == epoch
        && state.lobby.typing_here()
    {
        // `insert` drops the line breaks a copied key brings along.
        state.lobby.insert(&text);
    }
}

/// The builder's keys (`KEYBOARD.md` §7.7): typing into the box that has
/// the caret, or walking a list; Esc closes the innermost thing or clears
/// the search and never leaves the builder. Save, Import, Export, `/` and
/// Back are the shell keymap's (`shortcuts::press_for`), and Tab is the
/// kit's walker; neither is read here.
#[allow(clippy::too_many_lines)] // one batch of keys, each by where the keyboard is
pub(super) fn builder_keys(
    keys: &mut MessageReader<KeyboardInput>,
    codes: &ButtonInput<KeyCode>,
    state: &mut ResMut<LobbyState>,
    scrolled: &mut Scrolled,
    mut clipboard: Option<&mut bevy::clipboard::Clipboard>,
    paste: &mut Option<Paste>,
) {
    let picking = state.lobby.builder().picker().is_some();
    if picking {
        let searching = state
            .lobby
            .builder()
            .picker()
            .is_some_and(baylee_client_core::deckbuilder::Picker::set_open);
        if codes.just_pressed(KeyCode::Escape) {
            if searching {
                state.lobby.builder_mut().picker_close_sets();
            } else {
                state.lobby.room_close_print();
            }
            keys.clear();
            return;
        }
        if searching
            && (codes.just_pressed(KeyCode::ArrowDown) || codes.just_pressed(KeyCode::ArrowUp))
        {
            state
                .lobby
                .builder_mut()
                .picker_move_set(codes.just_pressed(KeyCode::ArrowDown));
            keys.clear();
            return;
        }
        if searching && codes.just_pressed(KeyCode::Enter) {
            state.lobby.builder_mut().picker_choose_set();
            keys.clear();
            return;
        }
        if !searching {
            if codes.just_pressed(KeyCode::ArrowLeft) {
                state.lobby.builder_mut().picker_step(-1);
            }
            if codes.just_pressed(KeyCode::ArrowRight) {
                state.lobby.builder_mut().picker_step(1);
            }
            keys.clear();
            return;
        }
    }
    let typing = state.build.nav == Nav::Field;
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
        return;
    }
    if let Some(text) = pasted
        && typing
    {
        state
            .lobby
            .builder_mut()
            .edit_buffer(field, |buffer| buffer.insert(&text));
    }
    if state.lobby.builder().panel().is_some() && field == BuildField::Search && typing {
        // The filter builder holds the search's words while it is open; Esc
        // still closes it.
        if codes.just_pressed(KeyCode::Escape) {
            escape(state, scrolled);
        }
        keys.clear();
        return;
    }
    let pressed: Vec<KeyboardInput> = keys
        .read()
        .filter(|k| k.state.is_pressed())
        .cloned()
        .collect();
    for key in pressed {
        if key.logical_key == Key::Escape {
            if !key.repeat {
                escape(state, scrolled);
            }
            continue;
        }
        if key.key_code == KeyCode::F2 {
            state.lobby.builder_mut().focus_on(BuildField::Name);
            state
                .lobby
                .builder_mut()
                .edit_buffer(BuildField::Name, crate::buildui::select_all);
            crate::buildui::move_nav(state, Nav::Field);
            continue;
        }
        // The card sheet answers its own keys (KEYBOARD §7.7: ←→ the list it
        // came from, Enter Add, ⇧Enter Add to the other).
        if let Some(slot) = state.lobby.builder().inspecting() {
            sheet_key(state, codes, &key, slot);
            continue;
        }
        if state.build.covered() || state.confirm_leave {
            continue;
        }
        match state.build.nav {
            Nav::Field => type_key(
                state,
                codes,
                scrolled,
                clipboard.as_deref_mut(),
                paste,
                &key,
            ),
            Nav::Pool(at) => pool_key(state, codes, &key, at),
            Nav::Deck(at) => deck_key(state, codes, &key, at),
            Nav::Idle => {}
        }
    }
}

/// Esc in the builder (`KEYBOARD.md` §2.5, §7.7): the innermost thing that
/// is open closes, else a renaming ends, else the search clears — and with
/// nothing left, nothing: Esc never leaves the builder.
fn escape(state: &mut ResMut<LobbyState>, scrolled: &mut Scrolled) {
    if state.build.menu.is_some() {
        state.build.menu = None;
    } else if state.confirm_leave {
        state.confirm_leave = false;
    } else if state.lobby.builder().inspecting().is_some() {
        state.lobby.builder_mut().stop_inspecting();
    } else if state.build.stats_sheet {
        state.build.stats_sheet = false;
    } else if state.build.syntax {
        state.build.syntax = false;
    } else if state.build.rail {
        state.build.rail = false;
    } else if state.lobby.builder().panel().is_some() {
        state.lobby.builder_mut().close_panel();
    } else if state.build.nav == Nav::Field && state.lobby.builder().focus() == BuildField::Name {
        state.lobby.builder_mut().focus_on(BuildField::Search);
        crate::buildui::move_nav(state, Nav::Idle);
    } else if !state.lobby.builder().text().is_empty() {
        state.lobby.builder_mut().set_text("");
        scrolled.set(List::Pool, 0.0);
    }
}

/// Where a `+` lands: the list the deck side shows, or with Shift the other.
fn target(state: &LobbyState, other: bool) -> Zone {
    let shown = state
        .build
        .zone()
        .unwrap_or_else(|| state.lobby.builder().zone());
    match (shown, other) {
        (zone, false) => zone,
        (Zone::Main, true) => Zone::Side,
        (Zone::Side, true) => Zone::Main,
    }
}

fn shifted(codes: &ButtonInput<KeyCode>) -> bool {
    codes.any_pressed([KeyCode::ShiftLeft, KeyCode::ShiftRight])
}

/// The next cursor of a list of `n` for a walking key, or `None` when the
/// key is not one.
/// Read by the physical key: Home, End and the page keys carry no
/// character, and a harness (or a layout) may send them unnamed.
fn walk(key: &KeyboardInput, at: usize, n: usize) -> Option<usize> {
    const PAGE: usize = 8;
    let last = n.saturating_sub(1);
    Some(match key.key_code {
        KeyCode::ArrowDown => (at + 1).min(last),
        KeyCode::ArrowUp => at.saturating_sub(1),
        KeyCode::Home => 0,
        KeyCode::End => last,
        KeyCode::PageDown => (at + PAGE).min(last),
        KeyCode::PageUp => at.saturating_sub(PAGE),
        _ => return None,
    })
}

/// The row's menu: the context-menu key, or Shift+F10.
fn menu_key(key: &KeyboardInput, codes: &ButtonInput<KeyCode>) -> bool {
    key.key_code == KeyCode::ContextMenu || (key.key_code == KeyCode::F10 && shifted(codes))
}

/// A key on a pool row.
fn pool_key(
    state: &mut ResMut<LobbyState>,
    codes: &ButtonInput<KeyCode>,
    key: &KeyboardInput,
    at: usize,
) {
    let shown: Vec<usize> = {
        let deck = state.lobby.builder();
        deck.results()
            .iter()
            .copied()
            .filter(|slot| match state.commander_pick {
                Some(true) => deck.can_partner(*slot),
                Some(false) => deck.card(*slot).is_some_and(|card| card.commander),
                None => true,
            })
            .collect()
    };
    if shown.is_empty() {
        return;
    }
    let at = at.min(shown.len() - 1);
    let slot = shown[at];
    // ↑ on the first row goes back up to the search (KEYBOARD §7.7).
    if key.key_code == KeyCode::ArrowUp && at == 0 {
        state.lobby.builder_mut().focus_on(BuildField::Search);
        crate::buildui::move_nav(state, Nav::Field);
        return;
    }
    if let Some(to) = walk(key, at, shown.len()) {
        crate::buildui::move_nav(state, Nav::Pool(to));
        return;
    }
    let added = |state: &mut ResMut<LobbyState>, zone| {
        if !state.lobby.builder_mut().add(slot, zone) {
            state.lobby.tell_refusal(Phrase::NoRoomForCopy, &[]);
        }
    };
    match &key.logical_key {
        Key::Enter => {
            let zone = target(state, shifted(codes));
            added(state, zone);
        }
        Key::Space => state.lobby.builder_mut().inspect(slot),
        _ if menu_key(key, codes) => {
            state.build.menu = Some(crate::buildui::BuildMenu::Pool(slot));
        }
        Key::Character(ch) if ch == "+" => {
            let zone = target(state, false);
            added(state, zone);
        }
        Key::Character(ch) if ch == "-" => {
            let zone = target(state, false);
            state.lobby.builder_mut().remove(slot, zone);
        }
        _ => {}
    }
}

/// A key on a deck row.
fn deck_key(
    state: &mut ResMut<LobbyState>,
    codes: &ButtonInput<KeyCode>,
    key: &KeyboardInput,
    at: usize,
) {
    let order = crate::buildui::deck_order(state);
    if order.is_empty() {
        return;
    }
    let cursor = at.min(order.len() - 1);
    if let Some(to) = walk(key, cursor, order.len()) {
        crate::buildui::move_nav(state, Nav::Deck(to));
        return;
    }
    let row = order[cursor];
    let zone = state.lobby.builder().zone();
    match &key.logical_key {
        Key::Enter if shifted(codes) => {
            let to = match zone {
                Zone::Main => Zone::Side,
                Zone::Side => Zone::Main,
            };
            state.lobby.builder_mut().move_entry(row, zone, to);
        }
        Key::Enter => add_row(state, row, zone),
        Key::Character(ch) if ch == "+" => add_row(state, row, zone),
        Key::Character(ch) if ch == "-" => {
            state.lobby.builder_mut().remove_at(row, zone);
        }
        Key::Space => {
            if let Some(entry) = state.lobby.builder().entries(zone).get(row) {
                let slot = entry.slot;
                state.lobby.builder_mut().inspect(slot);
            }
        }
        _ if menu_key(key, codes) => {
            state.build.menu = Some(crate::buildui::BuildMenu::Deck(row));
        }
        _ => {}
    }
    // A row that went keeps the cursor on the list.
    let left = crate::buildui::deck_order(state).len();
    if left > 0 && cursor >= left {
        crate::buildui::move_nav(state, Nav::Deck(left - 1));
    }
}

/// One more copy of a deck row, in its printing.
fn add_row(state: &mut ResMut<LobbyState>, row: usize, zone: Zone) {
    if let Some(entry) = state.lobby.builder().entries(zone).get(row).cloned()
        && !state
            .lobby
            .builder_mut()
            .add_print(entry.slot, zone, entry.print)
    {
        state.lobby.tell_refusal(Phrase::NoRoomForCopy, &[]);
    }
}

/// A key on the card sheet.
fn sheet_key(
    state: &mut ResMut<LobbyState>,
    codes: &ButtonInput<KeyCode>,
    key: &KeyboardInput,
    slot: usize,
) {
    match &key.logical_key {
        Key::Enter => {
            let zone = target(state, shifted(codes));
            if !state.lobby.builder_mut().add(slot, zone) {
                state.lobby.tell_refusal(Phrase::NoRoomForCopy, &[]);
            }
        }
        Key::ArrowLeft | Key::ArrowRight => {
            let results = state.lobby.builder().results().to_vec();
            if let Some(at) = results.iter().position(|s| *s == slot) {
                let to = if key.logical_key == Key::ArrowLeft {
                    at.checked_sub(1)
                } else {
                    (at + 1 < results.len()).then_some(at + 1)
                };
                if let Some(to) = to {
                    state.lobby.builder_mut().inspect(results[to]);
                    if matches!(state.build.nav, Nav::Pool(_)) {
                        crate::buildui::move_nav(state, Nav::Pool(to));
                    }
                }
            }
        }
        _ => {}
    }
}

/// A key while the caret is in the search or the name.
#[allow(clippy::too_many_lines)] // the text editing keys, one arm each
fn type_key(
    state: &mut ResMut<LobbyState>,
    codes: &ButtonInput<KeyCode>,
    scrolled: &mut Scrolled,
    mut clipboard: Option<&mut bevy::clipboard::Clipboard>,
    paste: &mut Option<Paste>,
    key: &KeyboardInput,
) {
    let field = state.lobby.builder().focus();
    let epoch = state.lobby.builder().focus_epoch();
    // ↓ in the search, with no suggestion open, steps down into the list.
    if field == BuildField::Search && key.key_code == KeyCode::ArrowDown {
        if !state.lobby.builder().results().is_empty() {
            crate::buildui::move_nav(state, Nav::Pool(0));
        }
        return;
    }
    if key.logical_key == Key::Enter {
        match field {
            // Enter in the search adds the top result (today's behaviour,
            // KEYBOARD §7.7).
            BuildField::Search => {
                let zone = target(state, shifted(codes));
                let builder = state.lobby.builder_mut();
                if let Some(slot) = builder.results().first().copied() {
                    builder.add(slot, zone);
                }
            }
            // Enter on the name ends the renaming; the keyboard is free.
            BuildField::Name => {
                state.lobby.builder_mut().focus_on(BuildField::Search);
                crate::buildui::move_nav(state, Nav::Idle);
            }
            BuildField::PickerSet => {}
        }
        return;
    }
    let shift = shifted(codes);
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
    state
        .lobby
        .builder_mut()
        .edit_buffer(field, |buffer| match &key.logical_key {
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
            Key::Tab | Key::Escape | Key::Enter => {}
            Key::Character(ch) if command => match ch.to_lowercase().as_str() {
                "a" => buffer.select_all(),
                "c" | "x" => {
                    if let (Some(range), Some(cb)) = (buffer.selection(), clipboard.as_deref_mut())
                    {
                        let copied = cb.set_text(buffer.text()[range].to_owned()).is_ok();
                        if copied && ch.eq_ignore_ascii_case("x") {
                            buffer.replace_selection("");
                        }
                    }
                }
                "v" => {
                    if let Some(cb) = clipboard.as_mut() {
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
                _ => {}
            },
            _ => {
                if let Some(text) = &key.text
                    && !text.chars().any(char::is_control)
                {
                    buffer.insert(text);
                }
            }
        });
    if before != state.lobby.builder().focused_text() && field == BuildField::Search {
        scrolled.set(List::Pool, 0.0);
    }
}

/// The import and export dialogs' keys.
///
/// Nothing is typed into either dialog (the box shows what was pasted), so
/// only chords and navigation keys are read, and from `codes`, which the
/// browser's typing field does not take away: the dialogs answer the same
/// keys on every platform. Returns whether it used the frame's keys.
pub(super) fn transfer_keys(
    codes: &ButtonInput<KeyCode>,
    state: &mut ResMut<LobbyState>,
    scrolled: &mut Scrolled,
) -> bool {
    use crate::buildui::transfer::Ask;
    use baylee_client_core::deckbuilder::transfer::{Stage, Transfer};
    let command = codes.any_pressed([
        KeyCode::SuperLeft,
        KeyCode::SuperRight,
        KeyCode::ControlLeft,
        KeyCode::ControlRight,
    ]);
    let chord = |key| command && codes.just_pressed(key);
    // Opening either is the shell keymap's (`Ctrl/Cmd+I`, `Ctrl/Cmd+E`,
    // rebindable; `shortcuts::press_for`); this answers the open dialog.
    let Some(open) = state.lobby.builder().transfer() else {
        return false;
    };
    let (importing, ready, done) = match open {
        Transfer::Import(it) => (true, it.ready(), matches!(it.stage(), Stage::Done(_))),
        Transfer::Export(_) => (false, false, false),
    };
    if codes.just_pressed(KeyCode::Escape) {
        state.lobby.builder_mut().close_transfer();
    } else if importing {
        if chord(KeyCode::KeyV) {
            state.transfer_asks.push(Ask::Paste);
        } else if codes.just_pressed(KeyCode::Enter) && ready {
            let lang = state.lobby.lang();
            state.lobby.builder_mut().import_confirm(lang);
            scrolled.set(List::Deck, 0.0);
        } else if codes.just_pressed(KeyCode::Enter) && done {
            // A finished import's one button is Close, and Enter presses it.
            state.lobby.builder_mut().close_transfer();
        }
    } else if chord(KeyCode::KeyC) {
        state.transfer_asks.push(Ask::Copy);
    } else if chord(KeyCode::KeyS) {
        if crate::buildui::transfer::can_save_files() {
            state.transfer_asks.push(Ask::Save);
        }
    } else if codes.just_pressed(KeyCode::ArrowLeft) {
        state.lobby.builder_mut().export_step(false);
    } else if codes.just_pressed(KeyCode::ArrowRight) {
        state.lobby.builder_mut().export_step(true);
    }
    true
}
