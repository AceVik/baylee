//! The chat's keys (the beta.8 social design §12.1, the chat window design
//! §4.2): the doors at the table (`/`, `R`, `Ctrl`/`Cmd`+`Enter`), and,
//! while the composer (or one of the chat's boxes) has the caret, every key
//! but the F-keys.
//!
//! Who else may hold the keys asks [`super::ChatUi::holds_keyboard`]: the
//! table's `KeyClaims`, the shell's `KitHolds`, its walker and its stack.

use baylee_client_core::prefs::Action;
use baylee_client_core::social::{ChatPhrase, ComposerKey, ComposerOutcome, Intent};
use baylee_client_core::textbuf::{Dir, Step, TextBuffer};
use bevy::input::keyboard::{Key, KeyboardInput};
use bevy::prelude::*;
use bevy::ui::RelativeCursorPosition;

use super::window::{self, ChatSurface};
use super::{ChatUi, Entry, Field, OsEntropy, SocialHost, now_ms};
use crate::{Duel, DuelPhase};

/// A clipboard read under way for a paste.
#[derive(Resource, Default)]
pub(crate) struct Paste(Option<bevy::clipboard::ClipboardRead>);

/// Installs the keys.
pub(crate) fn install(app: &mut App) {
    app.add_message::<KeyboardInput>()
        .init_resource::<Paste>()
        .add_systems(PreUpdate, unswallow.after(bevy::input::InputSystems))
        .add_systems(
            Update,
            (
                doors_by_key.before(crate::input::keyboard),
                typing.before(crate::input::keyboard),
                let_go_on_a_press_elsewhere,
                link_a_right_pressed_card,
                feed_candidates,
                age_the_hint,
                copy_out,
            )
                .chain(),
        );
}

/// The frame after the composer let go, the keys are the table's again.
fn unswallow(mut ui: ResMut<ChatUi>) {
    if ui.swallow {
        ui.swallow = false;
    }
}

/// `/`, `R` and `Ctrl`/`Cmd`+`Enter` at the table open the composer
/// (state 3) with `/`, `/r <last> ` or nothing typed, when nothing else
/// holds the keys (§4.2).
#[allow(clippy::too_many_arguments)] // a Bevy system: what may hold the keys
fn doors_by_key(
    codes: Res<ButtonInput<KeyCode>>,
    mut typed: MessageReader<KeyboardInput>,
    prefs: Option<Res<crate::prefs::Prefs>>,
    mut host: ResMut<SocialHost>,
    mut ui: ResMut<ChatUi>,
    duel: Option<ResMut<Duel>>,
    phase: Option<Res<State<DuelPhase>>>,
    desk: Option<Res<crate::report::ReportDesk>>,
    lobby: Option<Res<crate::LobbyState>>,
) {
    if ui.typing || !host.active() {
        return;
    }
    let at_table = phase.is_some_and(|p| *p.get() != DuelPhase::Closed);
    let Some(mut duel) = duel.filter(|_| at_table) else {
        return;
    };
    // The settings over the table (`lobby::table_settings`) have every key:
    // an `R` typed into their search is a letter, not a reply.
    if desk.is_some_and(|d| d.holds_keyboard())
        || lobby.is_some_and(|l| l.settings_over_table())
        || duel.browser.is_typing()
        || duel.arrangement_menu.is_some()
        || duel.ending().is_some()
    {
        return;
    }
    let slash = typed.read().any(|k| {
        k.state.is_pressed() && matches!(&k.logical_key, Key::Character(c) if c.as_str() == "/")
    });
    let binds = prefs
        .as_deref()
        .map(|p| crate::keys::Binds::new(&codes, p.keymap()));
    let reply = binds
        .as_ref()
        .is_some_and(|b| b.just_pressed(Action::ChatReply));
    let open = binds
        .as_ref()
        .is_some_and(|b| b.just_pressed(Action::ChatOpen));
    let prefix = if slash {
        "/"
    } else if reply {
        "/r"
    } else if open {
        ""
    } else {
        return;
    };
    let ui = &mut *ui;
    if !(duel.log_open && ui.tab.is_chat()) {
        window::open_on_chat(&mut host, ui, Some(&mut duel));
    }
    if let Some(machine) = host.machine.as_mut() {
        machine.open_composer(prefix);
    }
    ui.typing = true;
    ui.field = Field::Composer;
    // The key that opened the box is not a key for the table.
    ui.swallow = true;
}

/// What a key is to a text box: the composer's own keys.
fn composer_key(key: &Key, command: bool, shift: bool, alt: bool) -> Option<ComposerKey> {
    Some(match key {
        Key::Enter if shift => ComposerKey::ShiftEnter,
        Key::Enter => ComposerKey::Enter,
        Key::Escape => ComposerKey::Esc,
        Key::ArrowUp => ComposerKey::Up,
        Key::ArrowDown => ComposerKey::Down,
        Key::Tab if shift => ComposerKey::ShiftTab,
        Key::Tab => ComposerKey::Tab,
        Key::PageUp => ComposerKey::PageUp,
        Key::PageDown => ComposerKey::PageDown,
        Key::Home => ComposerKey::Home,
        Key::End => ComposerKey::End,
        Key::ArrowLeft if alt => ComposerKey::WordLeft,
        Key::ArrowRight if alt => ComposerKey::WordRight,
        Key::ArrowLeft => ComposerKey::Left,
        Key::ArrowRight => ComposerKey::Right,
        Key::Backspace if command || alt => ComposerKey::WordBackspace,
        Key::Backspace => ComposerKey::Backspace,
        Key::Delete => ComposerKey::Delete,
        Key::Character(c) if command && c.eq_ignore_ascii_case("a") => ComposerKey::SelectAll,
        Key::Character(c) if command && c.eq_ignore_ascii_case("c") => ComposerKey::Copy,
        Key::Character(c) if command && c.eq_ignore_ascii_case("x") => ComposerKey::Cut,
        Key::Character(c) if alt && c.len() == 1 && c.chars().all(|ch| ch.is_ascii_digit()) => {
            ComposerKey::AltDigit(c.parse().unwrap_or(0))
        }
        _ => return None,
    })
}

/// Every key while one of the chat's boxes has the caret.
#[allow(clippy::too_many_arguments, clippy::too_many_lines)] // a Bevy system: the keys and where they go
fn typing(
    codes: Res<ButtonInput<KeyCode>>,
    mut typed: MessageReader<KeyboardInput>,
    mut host: ResMut<SocialHost>,
    mut ui: ResMut<ChatUi>,
    mut duel: Option<ResMut<Duel>>,
    mut paste: ResMut<Paste>,
    mut clipboard: Option<ResMut<bevy::clipboard::Clipboard>>,
    phase: Option<Res<State<DuelPhase>>>,
) {
    let ui = &mut *ui;
    // A paste the clipboard answered.
    if let Some(result) = paste
        .0
        .as_mut()
        .and_then(bevy::clipboard::ClipboardRead::poll_result)
    {
        paste.0 = None;
        if let Ok(text) = result {
            paste_into(&mut host, ui, &text);
        }
    }
    // Keys that arrived while the box did not have the caret, or on the
    // frame it took it (the key that opened it), are not the box's.
    if !ui.typing || !host.active() || ui.swallow {
        typed.clear();
        return;
    }
    let shift = codes.any_pressed([KeyCode::ShiftLeft, KeyCode::ShiftRight]);
    let alt = codes.any_pressed([KeyCode::AltLeft, KeyCode::AltRight]);
    let command = codes.any_pressed([
        KeyCode::ControlLeft,
        KeyCode::ControlRight,
        KeyCode::SuperLeft,
        KeyCode::SuperRight,
    ]);
    let at_table = phase.is_some_and(|p| *p.get() != DuelPhase::Closed);
    let events: Vec<KeyboardInput> = typed.read().cloned().collect();
    for key in events {
        if !key.state.is_pressed() {
            continue;
        }
        // The F-keys are the table's still (F6, F7, F8).
        if matches!(
            key.logical_key,
            Key::F1
                | Key::F2
                | Key::F3
                | Key::F4
                | Key::F5
                | Key::F6
                | Key::F7
                | Key::F8
                | Key::F9
                | Key::F10
                | Key::F11
                | Key::F12
        ) {
            continue;
        }
        if command && matches!(&key.logical_key, Key::Character(c) if c.eq_ignore_ascii_case("v")) {
            if let Some(clipboard) = clipboard.as_deref_mut() {
                paste.0 = Some(clipboard.fetch_text());
            }
            continue;
        }
        if ui.field != Field::Composer {
            other_box(&mut host, ui, &key, command, shift);
            continue;
        }
        if let Some(composer_key) = composer_key(&key.logical_key, command, shift, alt) {
            let Some(machine) = host.machine.as_mut() else {
                return;
            };
            let (outcome, effects) = machine.key(composer_key, now_ms(), &mut OsEntropy);
            host.carry_out(effects);
            match outcome {
                ComposerOutcome::Close => {
                    ui.typing = false;
                    ui.swallow = true;
                }
                ComposerOutcome::SwitchTab(n) => {
                    if let Some(machine) = host.machine() {
                        let tabs = window::drawn_tabs(machine, at_table);
                        if let Some(tab) = usize::from(n)
                            .checked_sub(1)
                            .and_then(|i| tabs.get(i).copied())
                        {
                            window::select_tab(&mut host, ui, tab);
                            if let Some(duel) = duel.as_deref_mut() {
                                duel.log_open = true;
                            }
                        }
                    }
                }
                ComposerOutcome::Copy(text) => host.copy = Some(text),
                ComposerOutcome::Submit => {
                    // What was sent shows on the tab it was sent from; a
                    // whisper from the Table tab is read on Private and All.
                    ui.follow = true;
                }
                ComposerOutcome::Nothing
                | ComposerOutcome::Changed
                | ComposerOutcome::ScrollHistory(_) => {}
            }
            continue;
        }
        if command || alt && !matches!(&key.logical_key, Key::Character(_)) {
            continue;
        }
        if let Some(text) = key.text.as_ref() {
            let printable: String = text.chars().filter(|c| !c.is_control()).collect();
            if !printable.is_empty()
                && let Some(machine) = host.machine.as_mut()
            {
                machine.composer_mut().typed(&printable);
            }
        }
    }
}

/// A paste lands in the box that has the caret.
fn paste_into(host: &mut SocialHost, ui: &mut ChatUi, text: &str) {
    match ui.field {
        Field::Composer => {
            if let Some(machine) = host.machine.as_mut() {
                let (_, effects) = machine.key(
                    ComposerKey::Paste(text.to_string()),
                    now_ms(),
                    &mut OsEntropy,
                );
                host.carry_out(effects);
            }
        }
        Field::Find => ui.find.replace_selection(&one_line(text)),
        Field::Entry => {
            if let Some((_, buffer)) = ui.entry.as_mut() {
                buffer.replace_selection(&one_line(text));
            }
        }
    }
}

fn one_line(text: &str) -> String {
    text.chars().filter(|c| !c.is_control()).take(64).collect()
}

/// The drawer's search box and the small entry box: one line each; Enter
/// answers, Esc lets go.
fn other_box(
    host: &mut SocialHost,
    ui: &mut ChatUi,
    key: &KeyboardInput,
    command: bool,
    shift: bool,
) {
    let buffer: &mut TextBuffer = match ui.field {
        Field::Find => &mut ui.find,
        Field::Entry => {
            let Some((_, buffer)) = ui.entry.as_mut() else {
                ui.typing = false;
                return;
            };
            buffer
        }
        Field::Composer => return,
    };
    match &key.logical_key {
        Key::Escape => {
            if ui.field == Field::Entry {
                ui.entry = None;
            }
            ui.typing = false;
            ui.swallow = true;
            ui.field = Field::Composer;
        }
        Key::Enter => {
            answer_box(host, ui);
        }
        Key::Backspace => buffer.delete_back(),
        Key::Delete => buffer.delete_forward(),
        Key::ArrowLeft => buffer.move_caret(Step::Char, Dir::Left, shift),
        Key::ArrowRight => buffer.move_caret(Step::Char, Dir::Right, shift),
        Key::Home => buffer.move_caret(Step::Line, Dir::Left, shift),
        Key::End => buffer.move_caret(Step::Line, Dir::Right, shift),
        Key::Character(c) if command && c.eq_ignore_ascii_case("a") => buffer.select_all(),
        _ if command => {}
        _ => {
            if let Some(text) = key.text.as_ref() {
                let printable: String = text.chars().filter(|c| !c.is_control()).collect();
                if !printable.is_empty() && buffer.text().chars().count() < 64 {
                    buffer.replace_selection(&printable);
                }
            }
        }
    }
}

/// Enter in the drawer's search or the small entry box.
pub(crate) fn answer_box(host: &mut SocialHost, ui: &mut ChatUi) {
    match ui.field {
        Field::Find => {
            let typed = ui.find.text().trim().to_string();
            if typed.contains('#') {
                host.act(Intent::AddFriend { handle: typed });
                ui.find.clear();
                ui.typing = false;
                ui.swallow = true;
            }
        }
        Field::Entry => {
            let Some((entry, buffer)) = ui.entry.take() else {
                return;
            };
            let text = buffer.text().trim().to_string();
            if !text.is_empty() || matches!(entry, Entry::Nickname(_)) {
                let intent = match entry {
                    Entry::NewGroup => Intent::NewGroup { name: text },
                    Entry::Rename(id) => Intent::RenameGroup { id, name: text },
                    Entry::Nickname(tag) => Intent::Nickname {
                        tag,
                        name: (!text.is_empty()).then_some(text),
                    },
                    Entry::InviteToGroup(id) => {
                        let tag = baylee_client_core::social::tag_of(&text)
                            .map(str::to_string)
                            .or_else(|| {
                                host.machine().and_then(|m| {
                                    m.contacts()
                                        .friends
                                        .iter()
                                        .find(|f| f.person.handle.eq_ignore_ascii_case(&text))
                                        .map(|f| f.person.tag.clone())
                                })
                            });
                        match tag {
                            Some(tag) => Intent::InviteToGroup { id, tag },
                            None => return,
                        }
                    }
                };
                host.act(intent);
            }
            ui.typing = false;
            ui.swallow = true;
            ui.field = Field::Composer;
        }
        Field::Composer => {}
    }
}

/// A press anywhere but on the chat's own surfaces takes the keys back
/// (state 3 → 2).
fn let_go_on_a_press_elsewhere(
    mouse: Option<Res<ButtonInput<MouseButton>>>,
    surfaces: Query<(&RelativeCursorPosition, &InheritedVisibility), With<ChatSurface>>,
    mut ui: ResMut<ChatUi>,
) {
    let pressed = mouse.is_some_and(|m| m.just_pressed(MouseButton::Left));
    if !ui.typing || !pressed {
        return;
    }
    let inside = surfaces
        .iter()
        .any(|(at, seen)| seen.get() && at.cursor_over());
    if !inside {
        ui.typing = false;
        if ui.field == Field::Entry {
            ui.entry = None;
        }
        ui.field = Field::Composer;
    }
}

/// A right-press on a card while the composer has the caret links it at
/// the caret (§11.9); a card whose identity is not this seat's to share
/// inserts nothing and says so.
#[allow(clippy::too_many_arguments)] // a Bevy system: the card under the pointer and the box
fn link_a_right_pressed_card(
    mouse: Option<Res<ButtonInput<MouseButton>>>,
    mut host: ResMut<SocialHost>,
    mut ui: ResMut<ChatUi>,
    duel: Option<Res<Duel>>,
    texts: Option<Res<crate::cardtext::CardTexts>>,
    lobby: Option<Res<crate::lobby::LobbyState>>,
) {
    if !ui.typing
        || ui.field != Field::Composer
        || !mouse.is_some_and(|m| m.just_pressed(MouseButton::Right))
    {
        return;
    }
    let Some(duel) = duel else {
        return;
    };
    let Some(object) = duel.hovered else {
        return;
    };
    let Some(view) = duel.view.as_ref() else {
        return;
    };
    let lookup = |card: baylee_core::ids::CardIndex, face: u8| {
        texts.as_deref().and_then(|texts| texts.face(card, face))
    };
    let shown = view
        .identities()
        .into_iter()
        .find(|s| s.object == Some(object));
    let Some(shown) = shown else {
        ui.hint = Some((ChatPhrase::PrivateCard, 0.0));
        return;
    };
    let candidates = crate::report::references(Some(&duel), texts.as_deref(), lobby.as_deref());
    let row = candidates
        .cards
        .iter()
        .find(|c| c.object == Some(object))
        .cloned();
    let card = baylee_client_core::social::LinkedCard {
        card: shown.card.index,
        face: shown.card.face,
        print: row.as_ref().and_then(|r| r.print.clone()),
        seen: Some(shown.zone),
        face_down: shown.face_down,
        name: row
            .as_ref()
            .map_or_else(|| shown.name.to_string(), |r| r.name.clone()),
        english: shown.name.to_string(),
    };
    let _ = lookup;
    if let Some(machine) = host.machine.as_mut()
        && let Some(hint) = machine.insert_card(card)
    {
        ui.hint = Some((hint, 0.0));
    }
}

/// What `#` offers: at a table the seat's own view (`refs::candidates`) and
/// the cards its log named; elsewhere the compiled pool. Refreshed when the
/// view changes, never per frame.
fn feed_candidates(
    mut host: ResMut<SocialHost>,
    duel: Option<Res<Duel>>,
    texts: Option<Res<crate::cardtext::CardTexts>>,
    lobby: Option<Res<crate::lobby::LobbyState>>,
    phase: Option<Res<State<DuelPhase>>>,
    mut fed: Local<Option<(bool, u64, usize, u64)>>,
) {
    if !host.active() {
        *fed = None;
        return;
    }
    let at_table = phase.is_some_and(|p| *p.get() != DuelPhase::Closed);
    let duel = duel.filter(|_| at_table);
    let now = (
        at_table,
        duel.as_ref()
            .and_then(|d| d.view.as_ref())
            .map_or(0, |v| v.seq),
        duel.as_ref().map_or(0, |d| d.log.len()),
        texts.as_ref().map_or(0, |t| t.generation()),
    );
    if *fed == Some(now) {
        return;
    }
    // Away from a table the pool is big: loaded once, when the box first
    // opens.
    if !at_table && fed.is_some_and(|f| !f.0) {
        return;
    }
    *fed = Some(now);
    let cards = if at_table {
        crate::report::references(duel.as_deref(), texts.as_deref(), lobby.as_deref()).cards
    } else {
        crate::report::pool_candidates(&[])
    };
    if let Some(machine) = host.machine.as_mut() {
        machine.set_card_candidates(cards);
    }
}

/// A hint shows for three seconds.
fn age_the_hint(time: Res<Time>, mut ui: ResMut<ChatUi>) {
    if let Some((_, age)) = ui.hint.as_mut() {
        *age += time.delta_secs();
        if *age > 3.0 {
            ui.hint = None;
        }
    }
}

/// Puts what the machine asked to copy on the clipboard.
fn copy_out(mut host: ResMut<SocialHost>, clipboard: Option<ResMut<bevy::clipboard::Clipboard>>) {
    let Some(text) = host.copy.take() else {
        return;
    };
    if let Some(mut clipboard) = clipboard {
        let _ = clipboard.set_text(text);
    }
}
