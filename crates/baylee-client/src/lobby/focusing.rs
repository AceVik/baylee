//! The keyboard on Play, Decks, the room and their sheets (WP2, WP3;
//! `KEYBOARD.md` §1): the kit's walker moves a ring over the [`Stop`]s
//! these screens draw (`orders`), and this module is the lobby's side of it.
//!
//! - **Enter / Space** on a focused control is its press; on a tile or a
//!   row, its [`Primary`] (a deck tile: Use for next game, the next game's
//!   own tile: Edit; a house tile: Preview). A pointer's click is the
//!   `clicks` system's and is not answered twice.
//! - **A field and the ring agree**: the walker landing on the search box
//!   puts the lobby's caret there; the caret going into a field (`/`, a
//!   tap) moves the ring onto it; the ring leaving a field takes the caret
//!   out, so a letter typed with a tile focused is a shortcut, not a search.
//! - **A rebuilt screen keeps its focus**: the tree is rebuilt on every
//!   change, and the stop that had the ring gets it back by its name.
//! - **A tile's keys**: `Delete` (`⌫` on macOS) deletes with an Undo,
//!   `⇧F10` and the context-menu key open its `⋯` (`KEYBOARD.md` §7.6).
//! - **Initial focus** (§1.7): Play's first table row (else Create table),
//!   Decks' next-game tile (else the first), the room's first seat.

use super::menus::ShellMenu;
use super::press::Cx;
#[allow(clippy::wildcard_imports)] // the lobby's own vocabulary
use super::*;
use crate::shellkit::focus::{Activated, Current, Stop};
use bevy::input::ButtonState;
use bevy::input_focus::{FocusCause, InputFocus, InputFocusVisible};

/// The stop that last had the ring, so a rebuilt screen gives it back. It is
/// written as soon as the ring moves (`keys_press`, the frame the walker
/// moved it) and not only after the frame: a press that the move made can
/// rebuild the tree in that same frame.
#[derive(Resource, Default)]
pub(crate) struct Kept(Option<Stop>);

/// What Enter or Space means on a focused control that is not itself a
/// button: a tile, a table row.
#[derive(Component, Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) struct Primary(pub Press);

/// A deck tile, by its index in the deck list: `Delete` and `⇧F10` act on it.
#[derive(Component, Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) struct TileOf(pub usize);

/// Enter or Space on a focused control: its [`Primary`], else its press.
#[allow(clippy::too_many_arguments)] // a Bevy system: every one is an injection
pub(super) fn keys_press(
    mut activated: MessageReader<Activated>,
    mut clicks: MessageReader<Pointer<Click>>,
    parents: Query<&ChildOf>,
    presses: Query<&Press>,
    primaries: Query<&Primary>,
    disabled: Query<(), With<crate::shellkit::controls::Disabled>>,
    focus: Res<InputFocus>,
    stops: Query<&Stop>,
    codes: Option<Res<ButtonInput<KeyCode>>>,
    mut was: Local<Option<Stop>>,
    mut kept: ResMut<Kept>,
    mut state: ResMut<LobbyState>,
    mut prefs: ResMut<crate::prefs::Prefs>,
    mut scrolled: ResMut<Scrolled>,
    mailbox: Res<Mailbox>,
    mut settings: Option<ResMut<crate::settings::ClientSettings>>,
) {
    // A click activates its stop too; that press is `clicks`' own.
    let clicked: Vec<Entity> = clicks
        .read()
        .flat_map(|click| {
            std::iter::successors(Some(click.entity), |e| {
                parents.get(*e).ok().map(ChildOf::parent)
            })
        })
        .collect();
    let enter = codes
        .as_deref()
        .is_some_and(|c| c.any_just_pressed([KeyCode::Enter, KeyCode::NumpadEnter]));
    let mut fired: Vec<Press> = activated
        .read()
        .filter(|a| !clicked.contains(&a.entity) && !disabled.contains(a.entity))
        // The front door and the settings screen answer their own keys
        // (`front::keys::activate_by_key`): a press is never run twice.
        .filter(|a| !super::front::keys::ours(a.stop.table))
        .filter_map(|a| {
            // Enter on a choice of the Create-table sheet is the sheet's
            // default button (`KEYBOARD.md` W2 step 6); Space chooses.
            if enter && radio(&a.stop, presses.get(a.entity).ok()) {
                return Some(Press::Play(super::play::PlayPress::Open));
            }
            primaries
                .get(a.entity)
                .map(|p| p.0)
                .ok()
                .or_else(|| presses.get(a.entity).ok().copied())
        })
        .collect();
    // The sheet's choices are radio groups: an arrow that moves the ring
    // along one chooses what it lands on (`KEYBOARD.md` W2 steps 3-5).
    let now = focus.get().and_then(|f| stops.get(f).ok().map(|s| (f, *s)));
    if let Some((entity, stop)) = now
        && let Some(before) = *was
        && before != stop
        && before.table == stop.table
        && before.id == stop.id
        && let Ok(press) = presses.get(entity)
        && radio(&stop, Some(press))
        && !clicked.contains(&entity)
    {
        fired.push(*press);
    }
    // A move along the same screen is remembered at once (a press it made
    // may rebuild the tree this frame); a move into a menu is left to
    // `follow_focus`, which keeps the opener.
    if let Some((_, stop)) = now
        && was.is_some_and(|w| w.table == stop.table)
        && kept.0 != Some(stop)
    {
        kept.0 = Some(stop);
    }
    *was = now.map(|(_, s)| s);
    for press in fired {
        super::press::run(
            press,
            Cx {
                state: &mut state,
                prefs: &mut prefs,
                scrolled: &mut scrolled,
                mailbox: &mailbox,
                settings: &mut settings,
            },
        );
    }
}

/// A choice in one of the Create-table sheet's radio groups (players, the
/// template, the clock), not a stepper's buttons.
fn radio(stop: &Stop, press: Option<&Press>) -> bool {
    use super::play::PlayPress;
    stop.table == super::orders::CREATE.name
        && matches!(
            press,
            Some(Press::Play(
                PlayPress::Players(_) | PlayPress::Template(_) | PlayPress::Clock(_)
            ))
        )
}

/// The field a focused control types into, if it is a field's box.
fn field_of(entity: Entity, presses: &Query<&Press>) -> Option<Field> {
    match presses.get(entity).ok()? {
        Press::Shared(SharedPress::Focus(field)) => Some(*field),
        _ => None,
    }
}

/// Keeps the ring and the lobby's caret in one place, and gives a rebuilt
/// screen back the stop that had the ring.
#[allow(clippy::too_many_arguments)] // a Bevy system: every one is an injection
pub(super) fn follow_focus(
    mut focus: ResMut<InputFocus>,
    stops: Query<(Entity, &Stop)>,
    current: Query<&Current>,
    presses: Query<&Press>,
    mut state: ResMut<LobbyState>,
    mut kept: ResMut<Kept>,
    mut ring_was: Local<Option<Entity>>,
    mut caret_was: Local<Option<(Field, u64)>>,
    mut opener: Local<Option<Stop>>,
) {
    if !matches!(state.lobby.screen(), Screen::Table) || state.settings_open() {
        kept.0 = None;
        return;
    }
    // The account deletion's confirmation over the terms sheet (its
    // Decline) holds the keyboard and no stop: nothing under it gets the
    // ring back while it stands, and the stop kept is Decline's again once
    // it is cancelled (`front::terms::place_sheet_focus`).
    if state.terms.up() && state.lobby.deleting_account().is_some() {
        return;
    }
    let drawn = |stop: &Stop| stops.iter().find(|(_, s)| *s == stop).map(|(e, _)| e);
    // A menu that closed gives the ring back to the control that opened it
    // (`KEYBOARD.md` §1.8): the stop that had it before the menu took it.
    let in_menu = kept.0.is_some_and(|s| s.table == super::orders::MENU.name);
    if state.menu.is_none() && in_menu {
        kept.0 = opener.take();
    } else if state.menu.is_none() {
        *opener = None;
    } else if !in_menu && opener.is_none() {
        *opener = kept.0;
    }
    // A rebuilt screen: the stop that had the ring has a new entity.
    if focus.get().is_none_or(|f| !stops.contains(f))
        && let Some(stop) = kept.0
        && let Some(entity) = drawn(&stop)
    {
        focus.set(entity, FocusCause::Navigated);
    }
    let ring = focus.get().filter(|f| stops.contains(*f));
    if let Some(entity) = ring
        && let Ok((_, stop)) = stops.get(entity)
        && kept.0 != Some(*stop)
    {
        kept.0 = Some(*stop);
    }
    // The caret went into a field (a tap, `/`): the ring follows it.
    let caret = (state.lobby.focus(), state.lobby.focus_epoch());
    let typing = state.lobby.typing_here() && super::keyboard::field_drawn(&state);
    if *caret_was != Some(caret) {
        *caret_was = Some(caret);
        if typing
            && ring.and_then(|e| field_of(e, &presses)) != Some(caret.0)
            && let Some((entity, _)) = stops
                .iter()
                .find(|(e, _)| field_of(*e, &presses) == Some(caret.0))
        {
            focus.set(entity, FocusCause::Navigated);
            *ring_was = Some(entity);
            kept.0 = stops.get(entity).ok().map(|(_, s)| *s);
            return;
        }
    }
    // The ring moved (the walker, a press): the caret follows it into a
    // field, or out of one.
    if *ring_was == ring {
        return;
    }
    *ring_was = ring;
    let Some(entity) = ring else {
        return;
    };
    match field_of(entity, &presses) {
        Some(field) if state.lobby.focus() != field => state.lobby.focus_on(field),
        None if typing => {
            // Out of the field: the caret rests where nothing is drawn, so
            // a letter is a shortcut again (`KEYBOARD.md` §2.7).
            state.lobby.focus_on(Field::Username);
        }
        Some(_) | None => {}
    }
    let _ = &current;
}

/// The initial focus of a screen as it appears (`KEYBOARD.md` §1.7), the
/// ring hidden until a key is pressed.
pub(super) fn initial_focus(
    mut focus: ResMut<InputFocus>,
    report: Res<crate::shellkit::focus::FocusReport>,
    stops: Query<(Entity, &Stop)>,
    current: Query<&Current>,
    state: Res<LobbyState>,
    mut was: Local<Option<&'static str>>,
    mut kept: ResMut<Kept>,
) {
    // A frame between two trees reports no table; that is no new screen.
    if report.table.is_none() || *was == report.table {
        return;
    }
    *was = report.table;
    if focus.get().is_some_and(|f| stops.contains(f)) {
        return;
    }
    // The caret is already in a field the screen draws: that is the focus.
    if state.lobby.typing_here() && super::keyboard::field_drawn(&state) {
        return;
    }
    let first = |table: &str, id: &str| {
        let mut items: Vec<(Entity, Stop)> = stops
            .iter()
            .filter(|(_, s)| s.table == table && s.id == id)
            .map(|(e, s)| (e, *s))
            .collect();
        items.sort_by_key(|(_, s)| s.item);
        items
            .iter()
            .find(|(e, _)| current.get(*e).is_ok_and(|c| c.0))
            .or_else(|| items.first())
            .map(|(e, _)| *e)
    };
    let pick = match report.table {
        Some("play") => first("play", "tables")
            .or_else(|| first("play", "minis"))
            .or_else(|| first("play", "create")),
        Some("decks") => first("decks", "tiles").or_else(|| first("decks", "new")),
        Some("room") => first("room", "seats"),
        Some("create-table") => first("create-table", "name"),
        Some("history") => first("history", "versions"),
        Some("preview") => first("preview", "add"),
        Some("picker") => first("picker", "tiles"),
        Some("menu") => first("menu", "items"),
        _ => None,
    };
    if let Some(entity) = pick {
        focus.set(entity, FocusCause::Navigated);
        // Remembered now: the tree may be rebuilt before the frame ends.
        kept.0 = stops.get(entity).ok().map(|(_, s)| *s);
    }
}

/// `Delete` (`⌫` on macOS) and `⇧F10` / the context-menu key on a focused
/// deck tile (`KEYBOARD.md` §7.6).
#[allow(clippy::too_many_arguments)] // a Bevy system: every one is an injection
pub(super) fn tile_keys(
    mut keys: MessageReader<bevy::input::keyboard::KeyboardInput>,
    codes: Option<Res<ButtonInput<KeyCode>>>,
    focus: Res<InputFocus>,
    visible: Res<InputFocusVisible>,
    tiles: Query<&TileOf>,
    mut state: ResMut<LobbyState>,
    mut prefs: ResMut<crate::prefs::Prefs>,
    mut scrolled: ResMut<Scrolled>,
    mailbox: Res<Mailbox>,
    mut settings: Option<ResMut<crate::settings::ClientSettings>>,
) {
    let Some(tile) = focus.get().and_then(|f| tiles.get(f).ok()).copied() else {
        keys.clear();
        return;
    };
    let shift = codes
        .as_deref()
        .is_some_and(|c| c.any_pressed([KeyCode::ShiftLeft, KeyCode::ShiftRight]));
    let mut fired = None;
    for key in keys.read() {
        if key.state != ButtonState::Pressed || key.repeat || state.menu.is_some() {
            continue;
        }
        fired = match key.key_code {
            KeyCode::Delete => Some(Press::Decks(super::decks::DecksPress::Delete(tile.0))),
            KeyCode::Backspace if crate::shellkit::keys::mac() => {
                Some(Press::Decks(super::decks::DecksPress::Delete(tile.0)))
            }
            KeyCode::F10 if shift => Some(Press::Shared(SharedPress::OpenMenu(ShellMenu::Deck(
                tile.0,
            )))),
            KeyCode::ContextMenu => Some(Press::Shared(SharedPress::OpenMenu(ShellMenu::Deck(
                tile.0,
            )))),
            _ => fired,
        };
    }
    let _ = visible;
    if let Some(press) = fired {
        super::press::run(
            press,
            Cx {
                state: &mut state,
                prefs: &mut prefs,
                scrolled: &mut scrolled,
                mailbox: &mailbox,
                settings: &mut settings,
            },
        );
    }
}

/// The tile the ring is on, for `e` / `F2` (Edit, the shell keymap's).
pub(super) fn focused_tile(focus: &InputFocus, tiles: &Query<&TileOf>) -> Option<usize> {
    focus.get().and_then(|f| tiles.get(f).ok()).map(|t| t.0)
}

/// `↓` in Play's or Decks' search goes to the first table row or deck tile
/// (`KEYBOARD.md` W3 step 2): the search filters as it is typed, and the
/// list under it is where the arrow points.
pub(super) fn down_from_search(
    mut keys: MessageReader<bevy::input::keyboard::KeyboardInput>,
    mut focus: ResMut<InputFocus>,
    stops: Query<(Entity, &Stop)>,
    mut kept: ResMut<Kept>,
) {
    let down = keys
        .read()
        .any(|k| k.state == ButtonState::Pressed && k.key_code == KeyCode::ArrowDown);
    if !down {
        return;
    }
    let Some(here) = focus.get().and_then(|f| stops.get(f).ok()).map(|(_, s)| *s) else {
        return;
    };
    let list = match (here.table, here.id) {
        ("play", "search") => "tables",
        ("decks", "search") => "tiles",
        _ => return,
    };
    let first = stops
        .iter()
        .filter(|(_, s)| s.table == here.table && s.id == list)
        .min_by_key(|(_, s)| s.item);
    if let Some((entity, stop)) = first {
        focus.set(entity, FocusCause::Navigated);
        kept.0 = Some(*stop);
    }
}
