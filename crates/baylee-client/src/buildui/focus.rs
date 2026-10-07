//! The builder's keyboard model on screen (`KEYBOARD.md` §1.3, §7.7): the
//! kit's focus (`InputFocus`, moved by Tab and a pointer) and the builder's
//! own [`Nav`] (moved by `/`, the arrows, F2) kept as one; the cursor's row
//! lit and scrolled into view; a menu placed under its opener.

use super::rows::{DeckRowAt, PoolRowAt, ROW_GROUND};
use super::sheets::PlacedMenu;
use super::virtual_rows::{VirtualList, is_pool, place_of};
use super::{BUILDER, BuildMenu, Nav};
use crate::lobby::LobbyState;
use crate::shellkit::focus::Stop;
use crate::shellkit::tokens;
use baylee_client_core::deckbuilder::BuildField;
use baylee_client_core::lobby::Screen;
use bevy::input_focus::{FocusCause, InputFocus, InputFocusVisible};
use bevy::prelude::*;

/// Marks the `⋯` a menu opens from, so the menu can stand under it.
#[derive(Component, Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) struct MenuOpener(pub(crate) BuildMenu);

/// The stop a keyboard place is drawn as.
fn stop_of(nav: Nav, field: BuildField) -> Option<&'static str> {
    match nav {
        Nav::Field if field == BuildField::Name => Some("title"),
        Nav::Field => Some("search"),
        Nav::Pool(_) => Some("pool"),
        Nav::Deck(_) => Some("deck"),
        Nav::Idle => None,
    }
}

/// The keyboard place a focused stop means.
fn nav_of(id: &str, now: Nav, renaming: bool) -> Nav {
    match id {
        "search" => Nav::Field,
        "title" if renaming => Nav::Field,
        "pool" => match now {
            Nav::Pool(at) => Nav::Pool(at),
            _ => Nav::Pool(0),
        },
        "deck" => match now {
            Nav::Deck(at) => Nav::Deck(at),
            _ => Nav::Deck(0),
        },
        _ => Nav::Idle,
    }
}

/// What [`follow`] remembers between frames.
#[derive(Default)]
pub(crate) struct Followed {
    epoch: u64,
    focus: Option<Entity>,
    stop: Option<Stop>,
}

/// Keeps the kit's focus and the builder's [`Nav`] one: a key that moved
/// `Nav` moves the focus to its stop; Tab or a press that moved the focus
/// moves `Nav`; a redraw that took the focused stop away gives the focus to
/// the stop that replaced it.
pub(crate) fn follow(
    mut state: ResMut<LobbyState>,
    mut focus: ResMut<InputFocus>,
    mut visible: ResMut<InputFocusVisible>,
    stops: Query<(Entity, &Stop)>,
    mut seen: Local<Followed>,
) {
    if !matches!(state.lobby.screen(), Screen::Build) {
        *seen = Followed::default();
        return;
    }
    let find = |id: &str, table: &str, item: Option<u8>| {
        stops
            .iter()
            .filter(|(_, s)| s.table == table && s.id == id && item.is_none_or(|i| s.item == i))
            .map(|(e, _)| e)
            .next()
    };
    let field = state.lobby.builder().focus();
    // The keyboard model moved: the focus follows it.
    if state.build.nav_epoch != seen.epoch {
        seen.epoch = state.build.nav_epoch;
        let target = stop_of(state.build.nav, field).and_then(|id| find(id, BUILDER, None));
        if let Some(entity) = target
            && focus.get() != Some(entity)
        {
            focus.set(entity, FocusCause::Navigated);
            if matches!(state.build.nav, Nav::Pool(_) | Nav::Deck(_)) && !visible.0 {
                visible.0 = true;
            }
        }
        seen.focus = focus.get();
        seen.stop = seen.focus.and_then(|e| stops.get(e).ok().map(|(_, s)| *s));
        return;
    }
    let now = focus.get();
    if now != seen.focus {
        let stop = now.and_then(|e| stops.get(e).ok().map(|(_, s)| *s));
        match stop {
            // Tab moved it onto a builder stop: the keyboard model follows. A
            // pointer's press (the ring hidden) leaves that to the press's own
            // handler: a redraw between press and release would take the
            // control away from under the click.
            Some(stop) if stop.table == BUILDER && visible.0 => {
                let renaming = state.build.nav == Nav::Field && field == BuildField::Name;
                let nav = nav_of(stop.id, state.build.nav, renaming);
                if stop.id == "search" && field != BuildField::Search {
                    state.lobby.builder_mut().focus_on(BuildField::Search);
                }
                if nav != state.build.nav {
                    state.build.nav = nav;
                }
                seen.stop = Some(stop);
            }
            Some(stop) => seen.stop = Some(stop),
            // The focused stop went away with a redraw: give the focus to
            // the one drawn in its place.
            None => {
                let again = seen.stop.and_then(|s| find(s.id, s.table, Some(s.item)));
                if let Some(entity) = again {
                    focus.set(entity, FocusCause::Navigated);
                }
            }
        }
        seen.focus = focus.get();
    }
}

/// Lights the cursor's row: the pool's, or the deck's.
#[allow(clippy::type_complexity)] // two kinds of row, one ground
pub(crate) fn light_the_cursor(
    state: Res<LobbyState>,
    mut pool: Query<(&PoolRowAt, &mut crate::ambience::Feel), Without<DeckRowAt>>,
    mut deck: Query<(&DeckRowAt, &mut crate::ambience::Feel), Without<PoolRowAt>>,
) {
    let nav = state.build.nav;
    for (at, mut feel) in &mut pool {
        let want = if nav == Nav::Pool(at.0) {
            tokens::SELECTED
        } else {
            ROW_GROUND
        };
        if feel.base != want {
            feel.base = want;
        }
    }
    for (at, mut feel) in &mut deck {
        let want = if nav == Nav::Deck(at.0) {
            tokens::SELECTED
        } else {
            ROW_GROUND
        };
        if feel.base != want {
            feel.base = want;
        }
    }
}

/// Scrolls the cursor's row into view when the cursor moves.
pub(crate) fn scroll_to_the_cursor(
    state: Res<LobbyState>,
    lists: Query<(&VirtualList, &ChildOf)>,
    mut scrollers: Query<(&mut ScrollPosition, &ComputedNode)>,
    mut last: Local<Option<Nav>>,
) {
    let nav = state.build.nav;
    if *last == Some(nav) {
        return;
    }
    *last = Some(nav);
    let (cursor, pool) = match nav {
        Nav::Pool(at) => (at, true),
        Nav::Deck(at) => (at, false),
        _ => return,
    };
    for (list, parent) in &lists {
        if is_pool(list) != pool {
            continue;
        }
        let Some((top, height)) = place_of(list, cursor) else {
            continue;
        };
        let Ok((mut position, node)) = scrollers.get_mut(parent.parent()) else {
            continue;
        };
        let view = node.size().y * node.inverse_scale_factor();
        if view <= 0.0 {
            continue;
        }
        let y = position.y;
        let want = if top < y {
            top
        } else if top + height > y + view {
            top + height - view
        } else {
            y
        };
        if (want - y).abs() > 0.5 {
            position.y = want.max(0.0);
        }
    }
}

/// Places a menu under its opener's right edge, flipped above it at the
/// window's foot, once both have been laid out; then shows it.
#[allow(clippy::type_complexity)] // the menu and the button it opened from
pub(crate) fn place_menus(
    mut menus: Query<(&PlacedMenu, &mut Node, &mut Visibility, &ComputedNode)>,
    openers: Query<(&MenuOpener, &ComputedNode, &UiGlobalTransform)>,
    windows: Query<&Window>,
) {
    let Some(window) = windows.iter().next() else {
        return;
    };
    let (width, height) = (window.width(), window.height());
    for (menu, mut node, mut shown, laid) in &mut menus {
        if *shown != Visibility::Hidden {
            continue;
        }
        let scale = laid.inverse_scale_factor();
        let size = laid.size() * scale;
        if size.y <= 0.0 {
            continue;
        }
        let anchor =
            openers
                .iter()
                .find(|(opener, ..)| opener.0 == menu.0)
                .map(|(_, at, place)| {
                    let s = at.inverse_scale_factor();
                    let half = at.size() * s * 0.5;
                    let mid = place.translation * s;
                    Rect::from_corners(mid - half, mid + half)
                });
        let (left, top) = match anchor {
            Some(rect) => {
                let below = rect.max.y + 4.0;
                let top = if below + size.y > height - 8.0 {
                    (rect.min.y - size.y - 4.0).max(8.0)
                } else {
                    below
                };
                (
                    (rect.max.x - size.x).clamp(8.0, (width - size.x - 8.0).max(8.0)),
                    top,
                )
            }
            None => ((width - size.x - 16.0).max(8.0), 64.0),
        };
        node.left = Val::Px(left);
        node.top = Val::Px(top);
        *shown = Visibility::Inherited;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_stop_and_a_keyboard_place_name_each_other() {
        for (nav, field) in [
            (Nav::Field, BuildField::Search),
            (Nav::Field, BuildField::Name),
            (Nav::Pool(3), BuildField::Search),
            (Nav::Deck(2), BuildField::Search),
        ] {
            let id = stop_of(nav, field).expect("a place with a stop");
            let renaming = field == BuildField::Name;
            assert_eq!(nav_of(id, nav, renaming), nav, "{id}");
        }
        assert_eq!(stop_of(Nav::Idle, BuildField::Search), None);
        assert_eq!(nav_of("save", Nav::Pool(2), false), Nav::Idle);
        assert_eq!(
            nav_of("title", Nav::Idle, false),
            Nav::Idle,
            "Tab onto the title does not start renaming"
        );
    }
}
