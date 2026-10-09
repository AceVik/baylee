//! Keyboard focus on the shell's own controls (`KEYBOARD.md` §1).
//!
//! Every focusable control the kit draws carries a [`Stop`]: the name of
//! the [`TabOrder`] table it belongs to and its place in it. The table is
//! the one source: the walker moves through it, and the tests read the
//! same table (§9.5) — a drawn stop the table does not name, or a name
//! nothing draws, fails them.
//!
//! - **Tab / Shift+Tab** walk the active table in its order, wrapping; a
//!   modal table (a sheet, the `?` overlay) is the active one while any of
//!   its stops is drawn, so Tab cycles inside it (§2.4).
//! - A **composite** is several items under one name (a tablist, a radio
//!   group, a toolbar): one Tab stop, its items walked with the arrows,
//!   Home and End (§1.4).
//! - **A pointer press** focuses the stop it lands in and hides the ring;
//!   a key shows it (`InputFocusVisible`, the browser's `:focus-visible`,
//!   §1.2). The ring is an [`Outline`] in `ACCENT`, 2 px wide and 2 px out,
//!   toggled by colour so a focus move inserts nothing.
//! - A stop that is also a [`ShellField`] owns typing while focused: every
//!   printable key and the editing keys (§2.3).
//! - **Enter / Space** on a stop that is not a field, and a click, say
//!   [`Activated`]; what that does is the drawing screen's.
//!
//! Not `bevy_input_focus`'s `TabNavigationPlugin`: its window-wide Tab
//! observer would fight the table's Tab (next phase). The walker here acts
//! only while a table with drawn stops is on screen and no duel is.

use bevy::input::ButtonState;
use bevy::input::keyboard::{Key, KeyboardInput};
use bevy::input_focus::{FocusCause, InputFocus, InputFocusVisible};
use bevy::prelude::*;

use super::metrics::px_fixed;
use super::tokens;

/// One screen's or sheet's focus order (`KEYBOARD.md` §1.3: the human copy
/// of these tables).
#[derive(Debug)]
pub struct TabOrder {
    /// The table's name, which every [`Stop`] drawn for it names.
    pub name: &'static str,
    /// The stops, in reading order.
    pub stops: &'static [&'static str],
    /// A sheet, menu or overlay: while any of its stops is drawn it is the
    /// active table and Tab cycles inside it.
    pub modal: bool,
}

/// Every table the kit knows, so a stop's table name finds its order.
pub const TABLES: &[&TabOrder] = &[
    &super::overlay::OVERLAY_ORDER,
    &crate::lobby::front::keys::GATEWAY,
    &crate::lobby::front::keys::SIGN_IN,
    &crate::lobby::front::keys::CREATE,
    &crate::lobby::front::keys::GUEST,
    &crate::lobby::front::keys::TERMS,
    &crate::lobby::front::keys::ABOUT,
    &crate::settingsui::keys::SETTINGS,
    &crate::settingsui::keys::PROFILE_SHEET,
    // The lobby's screens and sheets (WP2, WP3; `lobby::orders`).
    &crate::lobby::orders::PLAY,
    &crate::lobby::orders::DECKS,
    &crate::lobby::orders::ROOM,
    &crate::lobby::orders::CREATE,
    &crate::lobby::orders::HISTORY,
    &crate::lobby::orders::PREVIEW,
    &crate::lobby::orders::PICKER,
    &crate::lobby::orders::CHAIR,
    &crate::lobby::orders::MENU,
    &crate::buildui::BUILDER_ORDER,
    &crate::buildui::BUILDER_SHEET_ORDER,
    // The report sheet, over the lobby or the table (window B).
    &crate::report::REPORT,
    &crate::report::REPORT_CONFIRM,
    #[cfg(any(test, all(feature = "dev-control", not(target_arch = "wasm32"))))]
    &super::gallery::GALLERY_ORDER,
];

/// The table named `name`.
#[must_use]
pub fn table(name: &str) -> Option<&'static TabOrder> {
    TABLES.iter().copied().find(|t| t.name == name)
}

/// A focusable control: its table, its name in the table, and — for a
/// composite — which of its items this is.
#[derive(Component, Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct Stop {
    /// The [`TabOrder`] it belongs to.
    pub table: &'static str,
    /// Its name in that table.
    pub id: &'static str,
    /// The item within a composite (0 for a plain control).
    pub item: u8,
}

impl Stop {
    /// A plain stop.
    #[must_use]
    pub const fn new(table: &'static str, id: &'static str) -> Self {
        Self { table, id, item: 0 }
    }

    /// Item `item` of the composite `id`.
    #[must_use]
    pub const fn item(table: &'static str, id: &'static str, item: u8) -> Self {
        Self { table, id, item }
    }
}

/// Whether a composite's item holds the composite's place: Tab enters the
/// composite at the one that does (the selected tab, the chosen radio);
/// without one, at the first item.
#[derive(Component, Clone, Copy, PartialEq, Eq, Debug, Default)]
pub struct Current(pub bool);

/// A one-line text field the kit edits itself (the gallery's, the `?`
/// overlay's search). The lobby's fields keep their own editor until their
/// screens move onto the kit.
#[derive(Component, Clone, PartialEq, Eq, Debug, Default)]
pub struct ShellField {
    /// What is typed.
    pub value: String,
    /// Shown while nothing is.
    pub hint: String,
}

impl ShellField {
    /// A field holding `value`, showing `hint` while empty.
    #[must_use]
    pub fn new(value: &str, hint: &str) -> Self {
        Self {
            value: value.to_string(),
            hint: hint.to_string(),
        }
    }
}

/// The label inside a field that shows its text (or its hint).
#[derive(Component)]
pub struct FieldWords;

/// A screen's own editor has the keys (the settings screen's seat panel,
/// whose boxes walk with Tab themselves): the walker leaves Tab alone.
/// Written every frame by the screen, before the walker runs.
#[derive(Resource, Clone, Copy, PartialEq, Eq, Debug, Default)]
pub struct WalkerYields(pub bool);

/// The stop focus last stood on. A screen that rebuilds its tree draws the
/// same stop on a new entity, and focus follows it there. Whoever moves
/// focus outside the walker (a screen's initial focus) may write it too.
#[derive(Resource, Clone, Copy, PartialEq, Eq, Debug, Default)]
pub struct Remembered(pub Option<Stop>);

/// A focused stop was activated: Enter or Space on it, or a click.
#[derive(Message, Clone, Copy, PartialEq, Eq, Debug)]
pub struct Activated {
    /// The stop's entity.
    pub entity: Entity,
    /// Its table and name.
    pub stop: Stop,
    /// Enter or Space, not a click. A screen whose controls already answer
    /// a click (the lobby's `Press`) acts only on these, so a click is never
    /// answered twice.
    pub by_key: bool,
}

/// Where focus stands, for `/state` and the tests: the active table, the
/// focused stop's name and item, and whether the ring shows.
#[derive(Resource, Clone, Copy, PartialEq, Eq, Debug, Default)]
pub struct FocusReport {
    /// The active table, when one has drawn stops.
    pub table: Option<&'static str>,
    /// The focused stop.
    pub stop: Option<Stop>,
    /// Whether the ring is drawn.
    pub visible: bool,
    /// A kit field has focus (it owns every printable key).
    pub field: bool,
}

pub(super) fn install(app: &mut App) {
    app.init_resource::<InputFocus>()
        .init_resource::<InputFocusVisible>()
        .init_resource::<FocusReport>()
        .init_resource::<Remembered>()
        .init_resource::<WalkerYields>()
        .add_message::<Activated>()
        .add_message::<Pointer<Press>>()
        .add_message::<Pointer<Click>>()
        .add_observer(give_a_ring)
        .add_systems(
            Update,
            (pointer_focus, edit_fields, walk, activate)
                .chain()
                .in_set(FocusSystems),
        )
        // After every system that moves focus this frame (the walker, the
        // overlay that takes it, a screen's initial focus), so the ring and
        // the report say where it ended up, not where it was.
        .add_systems(PostUpdate, (draw_ring, report).chain());
}

/// The kit's focus systems, which the shell's key resolver runs after: a
/// key typed into a focused field is the field's before any shortcut is
/// asked about it.
#[derive(SystemSet, Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct FocusSystems;

/// Every stop gets its ring hidden, so focusing it later changes a colour
/// and inserts nothing.
fn give_a_ring(added: On<Add, Stop>, mut commands: Commands) {
    commands.entity(added.entity).insert(Outline::new(
        px_fixed(tokens::RING_WIDTH),
        px_fixed(tokens::RING_OFFSET),
        Color::NONE,
    ));
}

fn table_open(phase: Option<&State<crate::DuelPhase>>) -> bool {
    phase.is_none_or(|p| *p.get() == crate::DuelPhase::Closed)
}

/// The active table among the drawn stops: a modal one if any of its stops
/// is drawn, else the page's.
fn active(stops: &[(Entity, Stop)]) -> Option<&'static TabOrder> {
    let drawn = |modal: bool| {
        stops
            .iter()
            .filter_map(|(_, s)| table(s.table))
            .find(|t| t.modal == modal)
    };
    drawn(true).or_else(|| drawn(false))
}

/// The entry of each composite of `order` that is drawn, in the table's
/// order: its [`Current`] item, else its first.
fn entries(
    order: &TabOrder,
    stops: &[(Entity, Stop)],
    current: &Query<&Current>,
) -> Vec<(Entity, Stop)> {
    order
        .stops
        .iter()
        .filter_map(|id| {
            let mut items: Vec<&(Entity, Stop)> = stops
                .iter()
                .filter(|(_, s)| s.table == order.name && s.id == *id)
                .collect();
            items.sort_by_key(|(_, s)| s.item);
            items
                .iter()
                .find(|(e, _)| current.get(*e).is_ok_and(|c| c.0))
                .or_else(|| items.first())
                .map(|found| **found)
        })
        .collect()
}

/// The stop `forward` (or back) of `from` in `order`, wrapping.
#[must_use]
pub fn next_in(
    order: &TabOrder,
    drawn: &[&'static str],
    from: Option<&str>,
    forward: bool,
) -> Option<&'static str> {
    let walk: Vec<&'static str> = order
        .stops
        .iter()
        .copied()
        .filter(|id| drawn.contains(id))
        .collect();
    if walk.is_empty() {
        return None;
    }
    let at = from.and_then(|f| walk.iter().position(|id| *id == f));
    let index = match (at, forward) {
        (None, true) => 0,
        (None, false) => walk.len() - 1,
        (Some(i), true) => (i + 1) % walk.len(),
        (Some(i), false) => (i + walk.len() - 1) % walk.len(),
    };
    Some(walk[index])
}

fn held(codes: &ButtonInput<KeyCode>, keys: [KeyCode; 2]) -> bool {
    codes.any_pressed(keys)
}

/// Tab and Shift+Tab over the active table; arrows, Home and End inside a
/// composite.
#[allow(clippy::too_many_arguments)] // a Bevy system: every one is an injection
fn walk(
    mut keys: MessageReader<KeyboardInput>,
    codes: Option<Res<ButtonInput<KeyCode>>>,
    phase: Option<Res<State<crate::DuelPhase>>>,
    stops: Query<(Entity, &Stop)>,
    current: Query<&Current>,
    fields: Query<(), With<ShellField>>,
    mut focus: ResMut<InputFocus>,
    mut visible: ResMut<InputFocusVisible>,
    mut remembered: ResMut<Remembered>,
    mut last: Local<Option<Entity>>,
    yields: Res<WalkerYields>,
) {
    if yields.0 {
        keys.clear();
        return;
    }
    let drawn: Vec<(Entity, Stop)> = stops.iter().map(|(e, s)| (e, *s)).collect();
    // A screen rebuilt from its state (the lobby redraws its tree on every
    // change) despawns the focused stop and draws the same one anew: focus
    // follows it to the new entity, ring and all, rather than being lost to
    // every keystroke. `bevy_input_focus` clears the focus of a despawned
    // entity itself, so a focus that went to nothing is followed too —
    // while the table it stood in is still the one drawn.
    // (A focus cleared on purpose, its stop still drawn, stays cleared.)
    let lost = match focus.get() {
        Some(f) => stops.get(f).is_err(),
        None => {
            last.is_some_and(|e| stops.get(e).is_err())
                && remembered
                    .0
                    .is_some_and(|r| active(&drawn).is_some_and(|t| t.name == r.table))
        }
    };
    if lost
        && let Some(stop) = remembered.0
        && let Some((entity, _)) = drawn.iter().find(|(_, s)| *s == stop)
    {
        focus.set(*entity, FocusCause::Navigated);
    }
    if let Some((entity, stop)) = focus.get().and_then(|f| stops.get(f).ok()) {
        remembered.0 = Some(*stop);
        *last = Some(entity);
    }
    // At a table only a modal kit sheet walks (a sheet over the table, such
    // as a chooser with a field — the table design's amendment to KEYBOARD
    // §6); the table's own Tab is next phase, and nothing else keeps focus.
    let at_table = !table_open(phase.as_deref());
    let order = active(&drawn).filter(|t| t.modal || !at_table);
    // A focused stop that went away (its screen redrew, its sheet closed)
    // leaves nothing focused rather than a dangling entity.
    if focus.get().is_some()
        && (focus.get().is_some_and(|f| stops.get(f).is_err()) || (at_table && order.is_none()))
    {
        focus.clear();
    }
    let Some(order) = order else {
        keys.clear();
        remembered.0 = None;
        return;
    };
    let ctrl_alt_meta = codes.as_deref().is_some_and(|c| {
        held(c, [KeyCode::ControlLeft, KeyCode::ControlRight])
            || held(c, [KeyCode::AltLeft, KeyCode::AltRight])
            || held(c, [KeyCode::SuperLeft, KeyCode::SuperRight])
    });
    let shift = codes
        .as_deref()
        .is_some_and(|c| held(c, [KeyCode::ShiftLeft, KeyCode::ShiftRight]));
    for key in keys.read() {
        if key.state != ButtonState::Pressed {
            continue;
        }
        let here = focus.get().and_then(|f| stops.get(f).ok().map(|(_, s)| *s));
        match &key.logical_key {
            Key::Tab if !key.repeat && !ctrl_alt_meta => {
                let entries = entries(order, &drawn, &current);
                let ids: Vec<&'static str> = entries.iter().map(|(_, s)| s.id).collect();
                let from = here.filter(|s| s.table == order.name).map(|s| s.id);
                if let Some(id) = next_in(order, &ids, from, !shift)
                    && let Some((entity, _)) = entries.iter().find(|(_, s)| s.id == id)
                {
                    focus.set(*entity, FocusCause::Navigated);
                    visible.0 = true;
                }
            }
            Key::ArrowLeft
            | Key::ArrowRight
            | Key::ArrowUp
            | Key::ArrowDown
            | Key::Home
            | Key::End => {
                let Some(here) = here else { continue };
                if focus.get().is_some_and(|f| fields.contains(f)) {
                    continue;
                }
                let mut items: Vec<(Entity, Stop)> = drawn
                    .iter()
                    .copied()
                    .filter(|(_, s)| s.table == here.table && s.id == here.id)
                    .collect();
                if items.len() < 2 {
                    continue;
                }
                items.sort_by_key(|(_, s)| s.item);
                let at = items
                    .iter()
                    .position(|(_, s)| s.item == here.item)
                    .unwrap_or(0);
                let to = match &key.logical_key {
                    Key::Home => 0,
                    Key::End => items.len() - 1,
                    Key::ArrowLeft | Key::ArrowUp => (at + items.len() - 1) % items.len(),
                    _ => (at + 1) % items.len(),
                };
                focus.set(items[to].0, FocusCause::Navigated);
                visible.0 = true;
            }
            _ => {}
        }
    }
    if let Some((entity, stop)) = focus.get().and_then(|f| stops.get(f).ok()) {
        remembered.0 = Some(*stop);
        *last = Some(entity);
    }
}

/// A pointer press focuses the stop it lands in and hides the ring.
fn pointer_focus(
    mut presses: MessageReader<Pointer<Press>>,
    stops: Query<(), With<Stop>>,
    parents: Query<&ChildOf>,
    mut focus: ResMut<InputFocus>,
    mut visible: ResMut<InputFocusVisible>,
) {
    for press in presses.read() {
        let mut at = Some(press.entity);
        while let Some(e) = at {
            if stops.contains(e) {
                if focus.get() != Some(e) {
                    focus.set(e, FocusCause::Pressed);
                }
                if visible.0 {
                    visible.0 = false;
                }
                break;
            }
            at = parents.get(e).ok().map(ChildOf::parent);
        }
    }
}

/// Whether `key` carries text a field takes: a printable character with no
/// command held (`AltGr` arrives as Ctrl and Alt together and still types).
fn typed_text(key: &KeyboardInput, ctrl: bool, alt: bool, meta: bool) -> Option<String> {
    let text = key.text.as_ref()?;
    if meta || (ctrl && !alt) {
        return None;
    }
    let text: String = text.chars().filter(|c| !c.is_control()).collect();
    (!text.is_empty()).then_some(text)
}

/// The kit's one-line field editor: typing and Backspace into the focused
/// [`ShellField`]; Esc clears a field with text (a search's Esc, §2.5).
fn edit_fields(
    mut keys: MessageReader<KeyboardInput>,
    codes: Option<Res<ButtonInput<KeyCode>>>,
    focus: Res<InputFocus>,
    mut fields: Query<&mut ShellField>,
) {
    let Some(mut field) = focus.get().and_then(|f| fields.get_mut(f).ok()) else {
        keys.clear();
        return;
    };
    let (ctrl, alt, meta) = codes.as_deref().map_or((false, false, false), |c| {
        (
            held(c, [KeyCode::ControlLeft, KeyCode::ControlRight]),
            held(c, [KeyCode::AltLeft, KeyCode::AltRight]),
            held(c, [KeyCode::SuperLeft, KeyCode::SuperRight]),
        )
    });
    for key in keys.read() {
        if key.state != ButtonState::Pressed {
            continue;
        }
        match &key.logical_key {
            Key::Backspace => {
                field.value.pop();
            }
            Key::Escape if !field.value.is_empty() => field.value.clear(),
            Key::Tab | Key::Enter | Key::Escape => {}
            _ => {
                if let Some(text) = typed_text(key, ctrl, alt, meta) {
                    field.value.push_str(&text);
                }
            }
        }
    }
}

/// Enter or Space on a focused stop that is not a field, and every click on
/// a stop, say [`Activated`] — never on one marked
/// [`Disabled`](super::controls::Disabled).
#[allow(clippy::too_many_arguments)] // a Bevy system: every one is an injection
fn activate(
    mut keys: MessageReader<KeyboardInput>,
    mut clicks: MessageReader<Pointer<Click>>,
    phase: Option<Res<State<crate::DuelPhase>>>,
    focus: Res<InputFocus>,
    stops: Query<&Stop>,
    fields: Query<(), With<ShellField>>,
    off: Query<(), With<super::controls::Disabled>>,
    parents: Query<&ChildOf>,
    codes: Option<Res<ButtonInput<KeyCode>>>,
    mut out: MessageWriter<Activated>,
) {
    // At a table a stop answers only inside a modal kit sheet.
    let at_table = !table_open(phase.as_deref());
    let answers = |stop: &Stop| !at_table || table(stop.table).is_some_and(|t| t.modal);
    // A chord is the shell's, never the focused stop's: `Cmd/Ctrl+Enter` is
    // the room's Start, and taken as Enter it pressed whatever had focus.
    let chord = codes.as_deref().is_some_and(|c| {
        c.any_pressed([
            KeyCode::ControlLeft,
            KeyCode::ControlRight,
            KeyCode::SuperLeft,
            KeyCode::SuperRight,
            KeyCode::AltLeft,
            KeyCode::AltRight,
        ])
    });
    for key in keys.read() {
        if key.state != ButtonState::Pressed || key.repeat || chord {
            continue;
        }
        if !matches!(key.logical_key, Key::Enter | Key::Space) {
            continue;
        }
        let Some(entity) = focus.get() else { continue };
        if fields.contains(entity) || off.contains(entity) {
            continue;
        }
        if let Ok(stop) = stops.get(entity).ok().filter(|s| answers(s)).ok_or(()) {
            out.write(Activated {
                entity,
                stop: *stop,
                by_key: true,
            });
        }
    }
    for click in clicks.read() {
        let mut at = Some(click.entity);
        while let Some(e) = at {
            if let Ok(stop) = stops.get(e) {
                if !off.contains(e) && answers(stop) {
                    out.write(Activated {
                        entity: e,
                        stop: *stop,
                        by_key: false,
                    });
                }
                break;
            }
            at = parents.get(e).ok().map(ChildOf::parent);
        }
    }
}

/// The ring on the focused stop while it is visible; none anywhere else. A
/// field shows its text, or its hint while empty.
#[allow(clippy::type_complexity)] // two queries over one kind of node
fn draw_ring(
    focus: Res<InputFocus>,
    visible: Res<InputFocusVisible>,
    mut rings: Query<(Entity, &mut Outline), With<Stop>>,
    changed: Query<(&ShellField, &Children), Changed<ShellField>>,
    mut words: Query<(&mut Text, &mut TextColor), With<FieldWords>>,
) {
    if focus.is_changed() || visible.is_changed() {
        for (entity, mut ring) in &mut rings {
            let want = if visible.0 && focus.get() == Some(entity) {
                tokens::ACCENT
            } else {
                Color::NONE
            };
            if ring.color != want {
                ring.color = want;
            }
        }
    }
    for (field, children) in &changed {
        for child in children.iter() {
            if let Ok((mut text, mut ink)) = words.get_mut(child) {
                let (shown, colour) = if field.value.is_empty() {
                    (field.hint.as_str(), tokens::MUTED)
                } else {
                    (field.value.as_str(), tokens::INK)
                };
                if text.0 != shown {
                    shown.clone_into(&mut text.0);
                }
                if ink.0 != colour {
                    ink.0 = colour;
                }
            }
        }
    }
}

/// Keeps [`FocusReport`] true for `/state` and the tests.
fn report(
    focus: Res<InputFocus>,
    visible: Res<InputFocusVisible>,
    stops: Query<(Entity, &Stop)>,
    fields: Query<(), With<ShellField>>,
    mut out: ResMut<FocusReport>,
) {
    let drawn: Vec<(Entity, Stop)> = stops.iter().map(|(e, s)| (e, *s)).collect();
    let now = FocusReport {
        table: active(&drawn).map(|t| t.name),
        stop: focus.get().and_then(|f| stops.get(f).ok().map(|(_, s)| *s)),
        visible: visible.0,
        field: focus.get().is_some_and(|f| fields.contains(f)),
    };
    if *out != now {
        *out = now;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const PAGE: TabOrder = TabOrder {
        name: "test-page",
        stops: &["a", "b", "c"],
        modal: false,
    };

    #[test]
    fn the_walk_follows_the_table_and_wraps_both_ways() {
        let drawn = ["a", "b", "c"];
        assert_eq!(next_in(&PAGE, &drawn, None, true), Some("a"));
        assert_eq!(next_in(&PAGE, &drawn, None, false), Some("c"));
        assert_eq!(next_in(&PAGE, &drawn, Some("a"), true), Some("b"));
        assert_eq!(next_in(&PAGE, &drawn, Some("c"), true), Some("a"));
        assert_eq!(next_in(&PAGE, &drawn, Some("a"), false), Some("c"));
        // A stop the screen does not draw is skipped, not stopped at.
        assert_eq!(next_in(&PAGE, &["a", "c"], Some("a"), true), Some("c"));
        assert_eq!(next_in(&PAGE, &[], None, true), None);
    }

    #[test]
    fn altgr_types_and_a_command_chord_does_not() {
        let key = |text: &str| KeyboardInput {
            key_code: KeyCode::KeyQ,
            logical_key: Key::Character(text.into()),
            state: ButtonState::Pressed,
            text: Some(text.into()),
            repeat: false,
            window: Entity::PLACEHOLDER,
        };
        assert_eq!(
            typed_text(&key("@"), true, true, false).as_deref(),
            Some("@")
        );
        assert_eq!(typed_text(&key("q"), true, false, false), None);
        assert_eq!(typed_text(&key("q"), false, false, true), None);
        assert_eq!(
            typed_text(&key("q"), false, false, false).as_deref(),
            Some("q")
        );
    }
}
