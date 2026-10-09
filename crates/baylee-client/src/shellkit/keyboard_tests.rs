//! The shell keyboard's acceptance on the gallery (`KEYBOARD.md` §9; the
//! shell design §17 WP0b-2): the Tab walk read from the `TabOrder` table,
//! a modal sheet that cycles, no printable key firing while a field has
//! focus, the text-size chords, focus given back on close, and the ring
//! that follows the input.

use bevy::input::ButtonState;
use bevy::input::keyboard::{Key, KeyboardInput};
use bevy::input_focus::{InputFocus, InputFocusVisible};
use bevy::prelude::*;

use baylee_client_core::shellkeys::{Context, ShellAction, Stack};

use super::focus::{self, FocusReport, ShellField, Stop};
use super::gallery::{GALLERY_ORDER, Gallery};
use super::keys::{self, ShellLog, ShellStack};
use super::overlay::{self, OVERLAY_ORDER, Overlay};
use super::{InputClass, TextSize, tokens};

/// The kit over an open gallery, as the client runs it, minus a window.
fn app() -> App {
    let mut app = App::new();
    app.add_plugins(bevy::state::app::StatesPlugin)
        .init_state::<crate::DuelPhase>()
        .init_resource::<ButtonInput<KeyCode>>()
        .add_message::<KeyboardInput>()
        .add_message::<bevy::input::mouse::MouseWheel>()
        .insert_resource(crate::settings::ClientSettings::default())
        .insert_resource(crate::hud::UiFonts::default())
        .init_resource::<InputClass>()
        .insert_resource(Gallery { open: true });
    super::gallery::install(&mut app);
    focus::install(&mut app);
    keys::install(&mut app);
    overlay::install(&mut app);
    // Play is the screen whose bare keys are the most (`c`, `/`, `1 2 3`):
    // the strictest stack to type under.
    app.insert_resource(ShellStack {
        stack: Stack {
            screen: Some(Context::Play),
            ..Stack::default()
        },
        live: true,
    });
    app.update();
    app.update();
    app
}

fn command() -> KeyCode {
    if keys::mac() {
        KeyCode::SuperLeft
    } else {
        KeyCode::ControlLeft
    }
}

/// One key as a real keyboard sends it: the held keys in `ButtonInput`, the
/// event with its logical key and its text.
fn key(app: &mut App, code: KeyCode, logical: Key, held: &[KeyCode]) {
    {
        let mut codes = app.world_mut().resource_mut::<ButtonInput<KeyCode>>();
        codes.reset_all();
        for k in held {
            codes.press(*k);
        }
        codes.press(code);
    }
    let text = match &logical {
        Key::Character(t) => Some(t.clone()),
        Key::Space => Some(" ".into()),
        _ => None,
    };
    for state in [ButtonState::Pressed, ButtonState::Released] {
        app.world_mut().write_message(KeyboardInput {
            key_code: code,
            logical_key: logical.clone(),
            state,
            text: text.clone(),
            repeat: false,
            window: Entity::PLACEHOLDER,
        });
    }
    app.update();
    app.world_mut()
        .resource_mut::<ButtonInput<KeyCode>>()
        .reset_all();
    app.update();
}

fn tab(app: &mut App, back: bool) {
    let held: &[KeyCode] = if back { &[KeyCode::ShiftLeft] } else { &[] };
    key(app, KeyCode::Tab, Key::Tab, held);
}

fn focused(app: &App) -> Option<Stop> {
    app.world().resource::<FocusReport>().stop
}

fn drawn(app: &mut App, table: &str) -> Vec<&'static str> {
    let mut ids: Vec<&'static str> = app
        .world_mut()
        .query::<&Stop>()
        .iter(app.world())
        .filter(|s| s.table == table)
        .map(|s| s.id)
        .collect();
    ids.sort_unstable();
    ids.dedup();
    ids
}

/// §9.5: the gallery draws exactly the stops its table names, Tab visits
/// them in the table's order and wraps, Shift+Tab walks it back.
#[test]
fn tab_walks_the_gallery_in_its_tables_order() {
    let mut app = app();
    let mut named: Vec<&str> = GALLERY_ORDER.stops.to_vec();
    named.sort_unstable();
    assert_eq!(
        drawn(&mut app, GALLERY_ORDER.name),
        named,
        "a drawn stop missing from the table, or a table entry nothing draws"
    );
    let mut walked = Vec::new();
    for _ in GALLERY_ORDER.stops {
        tab(&mut app, false);
        walked.push(focused(&app).expect("Tab focuses a stop").id);
    }
    assert_eq!(walked, GALLERY_ORDER.stops);
    tab(&mut app, false);
    assert_eq!(
        focused(&app).map(|s| s.id),
        Some(GALLERY_ORDER.stops[0]),
        "wraps"
    );
    let mut back = Vec::new();
    for _ in GALLERY_ORDER.stops {
        tab(&mut app, true);
        back.push(focused(&app).expect("Shift+Tab focuses a stop").id);
    }
    let mut reversed: Vec<&str> = GALLERY_ORDER.stops.to_vec();
    reversed.reverse();
    assert_eq!(back, reversed);
}

/// A composite is one stop entered at its current item, walked by arrows.
#[test]
fn a_composite_is_one_stop_walked_by_the_arrows() {
    let mut app = app();
    while focused(&app).map(|s| s.id) != Some("sizes") {
        tab(&mut app, false);
    }
    assert_eq!(
        focused(&app).map(|s| s.item),
        Some(3),
        "enters at the chosen item"
    );
    key(&mut app, KeyCode::ArrowRight, Key::ArrowRight, &[]);
    assert_eq!(focused(&app).map(|s| s.item), Some(4));
    key(&mut app, KeyCode::Home, Key::Home, &[]);
    assert_eq!(focused(&app).map(|s| s.item), Some(0));
    tab(&mut app, false);
    assert_eq!(
        focused(&app).map(|s| s.id),
        Some("toggle-on"),
        "Tab leaves the composite"
    );
}

/// The ring is drawn after a key and not after a pointer press (§1.2, §9.12).
#[test]
fn the_ring_follows_the_input() {
    let mut app = app();
    tab(&mut app, false);
    let focus = app.world().resource::<InputFocus>().get().expect("focused");
    let ring = app
        .world()
        .get::<Outline>(focus)
        .expect("a stop has a ring");
    assert_eq!(ring.color, tokens::ACCENT);
    assert_eq!(ring.width, super::px_fixed(tokens::RING_WIDTH));
    assert_eq!(ring.offset, super::px_fixed(tokens::RING_OFFSET));
    app.world_mut().resource_mut::<InputFocusVisible>().0 = false;
    app.update();
    let ring = app.world().get::<Outline>(focus).expect("still a stop");
    assert_eq!(ring.color, Color::NONE, "a pointer's focus draws no ring");
}

fn focus_on(app: &mut App, table: &str, id: &str) {
    let entity = app
        .world_mut()
        .query::<(Entity, &Stop)>()
        .iter(app.world())
        .find(|(_, s)| s.table == table && s.id == id)
        .map(|(e, _)| e)
        .expect("drawn");
    app.world_mut()
        .resource_mut::<InputFocus>()
        .set(entity, bevy::input_focus::FocusCause::Navigated);
    app.update();
}

fn value(app: &mut App, table: &str, id: &str) -> String {
    app.world_mut()
        .query::<(&Stop, &ShellField)>()
        .iter(app.world())
        .find(|(s, _)| s.table == table && s.id == id)
        .map(|(_, f)| f.value.clone())
        .expect("a field")
}

/// §9.2: every character a shell chord is bound to, typed into every kit
/// field, lands in the field and fires nothing; AltGr+Q on a German Windows
/// layout types `@` and fires nothing.
#[test]
fn no_printable_key_fires_while_a_field_has_focus() {
    let mut app = app();
    let bound: [(KeyCode, &str); 13] = [
        (KeyCode::Slash, "/"),
        (KeyCode::Minus, "?"),
        (KeyCode::KeyC, "c"),
        (KeyCode::KeyN, "n"),
        (KeyCode::KeyR, "r"),
        (KeyCode::KeyE, "e"),
        (KeyCode::BracketRight, "+"),
        (KeyCode::Minus, "-"),
        (KeyCode::Digit1, "1"),
        (KeyCode::Digit2, "2"),
        (KeyCode::Digit3, "3"),
        (KeyCode::Digit0, "0"),
        (KeyCode::Equal, "="),
    ];
    for field in ["search", "search-full", "sheet-name"] {
        focus_on(&mut app, GALLERY_ORDER.name, field);
        for (code, ch) in bound {
            let before = value(&mut app, GALLERY_ORDER.name, field);
            key(&mut app, code, Key::Character(ch.into()), &[]);
            assert_eq!(
                value(&mut app, GALLERY_ORDER.name, field),
                format!("{before}{ch}"),
                "{ch} did not land in {field}"
            );
        }
        let before = value(&mut app, GALLERY_ORDER.name, field);
        key(
            &mut app,
            KeyCode::KeyQ,
            Key::Character("@".into()),
            &[KeyCode::ControlLeft, KeyCode::AltRight],
        );
        assert_eq!(
            value(&mut app, GALLERY_ORDER.name, field),
            format!("{before}@")
        );
    }
    assert!(
        app.world().resource::<ShellLog>().fired.is_empty(),
        "fired while typing: {:?}",
        app.world().resource::<ShellLog>().fired
    );
    // The same keys with nothing focused do fire: the test could fail.
    app.world_mut().resource_mut::<InputFocus>().clear();
    app.update();
    key(&mut app, KeyCode::KeyC, Key::Character("c".into()), &[]);
    key(&mut app, KeyCode::Digit2, Key::Character("2".into()), &[]);
    assert_eq!(
        app.world().resource::<ShellLog>().fired,
        [ShellAction::CreateTable, ShellAction::GoDecks]
    );
}

/// `Ctrl/Cmd+=` steps the text size from inside a field too (it edits no
/// text), and the gallery redraws at the new step.
#[test]
fn the_text_size_chord_passes_a_field() {
    let mut app = app();
    focus_on(&mut app, GALLERY_ORDER.name, "search");
    key(
        &mut app,
        KeyCode::Equal,
        Key::Character("=".into()),
        &[command()],
    );
    assert_eq!(
        app.world()
            .resource::<crate::settings::ClientSettings>()
            .text_size,
        TextSize::Xl
    );
    assert_eq!(
        value(&mut app, GALLERY_ORDER.name, "search"),
        "",
        "= was typed"
    );
}

/// The `?` overlay: `Ctrl/Cmd+/` opens it over the gallery, focus lands in
/// its search without the opening key typed, Tab cycles inside it (a modal
/// table), Esc closes it, and focus goes back to what had it.
#[test]
fn the_overlay_is_modal_and_gives_focus_back() {
    let mut app = app();
    focus_on(&mut app, GALLERY_ORDER.name, "edit");
    let opener = app.world().resource::<InputFocus>().get();
    key(
        &mut app,
        KeyCode::Slash,
        Key::Character("/".into()),
        &[command()],
    );
    assert!(app.world().resource::<Overlay>().open);
    assert_eq!(
        focused(&app).map(|s| (s.table, s.id)),
        Some((OVERLAY_ORDER.name, "search"))
    );
    assert_eq!(
        value(&mut app, OVERLAY_ORDER.name, "search"),
        "",
        "the opening key was typed"
    );
    assert_eq!(
        app.world().resource::<FocusReport>().table,
        Some(OVERLAY_ORDER.name)
    );
    let mut walked = Vec::new();
    for _ in 0..OVERLAY_ORDER.stops.len() {
        tab(&mut app, false);
        walked.push(focused(&app).expect("focused").id);
    }
    assert_eq!(
        walked,
        ["list", "edit-keys", "close", "search"],
        "cycles inside"
    );
    // A bare `c` in its search is typed, not Create table; `?` too.
    key(&mut app, KeyCode::KeyC, Key::Character("c".into()), &[]);
    assert_eq!(value(&mut app, OVERLAY_ORDER.name, "search"), "c");
    // Esc clears the search first, then closes.
    key(&mut app, KeyCode::Escape, Key::Escape, &[]);
    assert!(app.world().resource::<Overlay>().open);
    assert_eq!(value(&mut app, OVERLAY_ORDER.name, "search"), "");
    key(&mut app, KeyCode::Escape, Key::Escape, &[]);
    assert!(!app.world().resource::<Overlay>().open);
    app.update();
    assert_eq!(app.world().resource::<InputFocus>().get(), opener);
    assert!(
        !app.world()
            .resource::<ShellLog>()
            .fired
            .contains(&ShellAction::CreateTable),
        "c fired under the overlay"
    );
}

/// Enter in the overlay runs the highlighted action beneath it.
#[test]
fn the_overlay_runs_what_it_highlights() {
    let mut app = app();
    app.world_mut().resource_mut::<InputFocus>().clear();
    key(
        &mut app,
        KeyCode::Slash,
        Key::Character("?".into()),
        &[KeyCode::ShiftLeft],
    );
    assert!(app.world().resource::<Overlay>().open);
    for ch in ["l", "a", "r", "g"] {
        let code = match ch {
            "l" => KeyCode::KeyL,
            "a" => KeyCode::KeyA,
            "r" => KeyCode::KeyR,
            _ => KeyCode::KeyG,
        };
        key(&mut app, code, Key::Character(ch.into()), &[]);
    }
    key(&mut app, KeyCode::Enter, Key::Enter, &[]);
    assert!(!app.world().resource::<Overlay>().open);
    assert_eq!(
        app.world()
            .resource::<crate::settings::ClientSettings>()
            .text_size,
        TextSize::Xl,
        "Larger text did not run"
    );
}

/// At a table only a modal kit sheet walks and types (a sheet over the
/// table — the table design's amendment to KEYBOARD §6): the page's stops
/// take no focus there, the sheet's do, and a focused kit field holds the
/// keyboard, which the table's keymap asks before it acts.
#[test]
fn at_a_table_only_a_modal_sheet_takes_focus() {
    let mut app = app();
    app.init_resource::<super::KitHolds>()
        .add_systems(Update, super::hold_the_keyboard);
    app.world_mut()
        .resource_mut::<NextState<crate::DuelPhase>>()
        .set(crate::DuelPhase::Playing);
    app.update();
    tab(&mut app, false);
    assert_eq!(focused(&app), None, "the gallery is no sheet");
    app.world_mut().resource_mut::<Overlay>().open = true;
    app.update();
    app.update();
    tab(&mut app, false);
    let stop = focused(&app).expect("the sheet takes focus");
    assert_eq!(stop.table, OVERLAY_ORDER.name);
    focus_on(&mut app, OVERLAY_ORDER.name, "search");
    key(&mut app, KeyCode::KeyW, Key::Character("w".into()), &[]);
    assert_eq!(value(&mut app, OVERLAY_ORDER.name, "search"), "w");
    assert!(app.world().resource::<super::KitHolds>().0);
    assert_eq!(
        app.world().resource::<ShellLog>().resolved,
        0,
        "no shell key at a table"
    );
}

/// A chord is not an activation. `Cmd/Ctrl+Enter` is the room's Start
/// (`KEYBOARD.md` W2 step 9); before this the focused stop took the same
/// Enter as its own press, so in a room whose focus stood on a seat's team
/// chip the chord moved that chair to the next team and the table did not
/// start (beta.6 QA). Plain Enter and Space still activate.
#[test]
fn a_command_chord_does_not_activate_the_focused_stop() {
    let mut app = app();
    app.init_resource::<Activations>()
        .add_systems(Update, count_activations);
    focus_on(&mut app, GALLERY_ORDER.name, "create");
    key(&mut app, KeyCode::Enter, Key::Enter, &[command()]);
    assert_eq!(app.world().resource::<Activations>().0, 0, "Cmd/Ctrl+Enter");
    key(&mut app, KeyCode::Enter, Key::Enter, &[KeyCode::AltLeft]);
    assert_eq!(app.world().resource::<Activations>().0, 0, "Alt+Enter");
    key(&mut app, KeyCode::Enter, Key::Enter, &[]);
    assert_eq!(app.world().resource::<Activations>().0, 1, "plain Enter");
    key(&mut app, KeyCode::Space, Key::Space, &[]);
    assert_eq!(app.world().resource::<Activations>().0, 2, "plain Space");
    key(&mut app, KeyCode::Enter, Key::Enter, &[KeyCode::ShiftLeft]);
    assert_eq!(
        app.world().resource::<Activations>().0,
        2,
        "Shift+Enter is a sheet's second action, not the focused stop's"
    );
}

#[derive(Resource, Default)]
struct Activations(usize);

fn count_activations(mut read: MessageReader<focus::Activated>, mut n: ResMut<Activations>) {
    n.0 += read.read().filter(|a| a.by_key).count();
}
