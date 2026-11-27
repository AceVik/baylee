//! The builder's keyboard model on screen (`KEYBOARD.md` §1.3, §7.7): the
//! kit's focus (`InputFocus`, moved by Tab and a pointer) and the builder's
//! own [`Nav`] (moved by `/`, the arrows, F2) kept as one; the cursor's row
//! lit and scrolled into view; a menu placed under its opener.

use super::cardwindow::{CARD, first_stop};
use super::rows::{DeckRowAt, PoolRowAt, ROW_GROUND};
use super::sheets::PlacedMenu;
use super::virtual_rows::{VirtualList, is_pool, place_of};
use super::{BUILDER, BUILDER_SHEET, BuildMenu, Nav};
use crate::lobby::LobbyState;
use crate::shellkit::focus::Stop;
use crate::shellkit::tokens;
use baylee_client_core::deckbuilder::{BuildField, Picker};
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
#[allow(clippy::too_many_lines)] // Unified Escape dismiss + Nav <-> InputFocus synchronization.
pub(crate) fn follow(
    mut state: ResMut<LobbyState>,
    mut focus: ResMut<InputFocus>,
    mut visible: ResMut<InputFocusVisible>,
    stops: Query<(Entity, &Stop)>,
    codes: Option<Res<ButtonInput<KeyCode>>>,
    mut seen: Local<Followed>,
) {
    if !matches!(state.lobby.screen(), Screen::Build) {
        *seen = Followed::default();
        return;
    }
    if state.build.confirm_sell.is_some()
        && codes
            .as_deref()
            .is_some_and(|c| c.just_pressed(KeyCode::Escape))
    {
        state.build.confirm_sell = None;
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
    // "Discard changes?" and Tower card-sale confirmation open with the safe
    // cancel/keep button focused (KEYBOARD §2.5).
    if state.confirm_leave || state.build.confirm_sell.is_some() {
        let on_sheet = focus
            .get()
            .and_then(|e| stops.get(e).ok())
            .is_some_and(|(_, s)| s.table == BUILDER_SHEET);
        if !on_sheet && let Some(keep) = find("keep", BUILDER_SHEET, None) {
            focus.set(keep, FocusCause::Navigated);
            seen.focus = Some(keep);
            seen.stop = Some(Stop::new(BUILDER_SHEET, "keep"));
        }
        return;
    }
    // The card window is modal: it opens on its door's stop (§1.7), keeps
    // the stop a redraw replaced, and says whether the strip has the focus
    // (← → step the printings there, the cards elsewhere).
    if let Some(door) = state.lobby.builder().picker().map(Picker::door) {
        let on_window = focus
            .get()
            .and_then(|e| stops.get(e).ok())
            .filter(|(_, s)| s.table == CARD)
            .map(|(_, s)| *s);
        let strip = on_window.is_some_and(|s| s.id == "printings");
        if state.build.on_strip != strip {
            state.bypass_change_detection().build.on_strip = strip;
        }
        if let Some(stop) = on_window {
            seen.focus = focus.get();
            seen.stop = Some(stop);
        } else {
            let again = seen
                .stop
                .filter(|s| s.table == CARD)
                .and_then(|s| find(s.id, CARD, Some(s.item)));
            let target = again.or_else(|| find(first_stop(door), CARD, None));
            if let Some(entity) = target {
                focus.set(entity, FocusCause::Navigated);
                seen.focus = Some(entity);
                seen.stop = stops.get(entity).ok().map(|(_, s)| *s);
            }
        }
        return;
    }
    if state.build.on_strip {
        state.bypass_change_detection().build.on_strip = false;
    }
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
    mut menus: Query<(
        Entity,
        &PlacedMenu,
        &mut Node,
        &mut Visibility,
        &ComputedNode,
    )>,
    children: Query<&Children>,
    mut tree: ParamSet<(
        Query<(&MenuOpener, &ComputedNode, &UiGlobalTransform)>,
        Query<&mut UiGlobalTransform>,
    )>,
    (windows, ui): (Query<&Window>, Option<Res<UiScale>>),
) {
    let Some(window) = windows.iter().next() else {
        return;
    };
    let (width, height) = crate::hud::scale::desktop_dimensions(window, ui.as_deref());
    for (entity, menu, mut node, mut shown, laid) in &mut menus {
        if *shown != Visibility::Hidden {
            continue;
        }
        let scale = laid.inverse_scale_factor();
        let size = laid.size() * scale;
        if size.y <= 0.0 {
            continue;
        }
        let anchor = {
            let openers = tree.p0();
            openers
                .iter()
                .find(|(opener, ..)| opener.0 == menu.0)
                .map(|(_, at, place)| {
                    let s = at.inverse_scale_factor();
                    let half = at.size() * s * 0.5;
                    let mid = place.translation * s;
                    Rect::from_corners(mid - half, mid + half)
                })
        };
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
        if scale > 0.0 {
            let mut transforms = tree.p1();
            if let Ok(cur_at) = transforms.get(entity) {
                let cur_min = cur_at.translation * scale - size * 0.5;
                let delta = (Vec2::new(left, top) - cur_min) / scale;
                for target in std::iter::once(entity).chain(children.iter_descendants(entity)) {
                    if let Ok(mut t) = transforms.get_mut(target) {
                        *t = UiGlobalTransform::from(bevy::math::Affine2 {
                            matrix2: t.matrix2,
                            translation: t.translation + delta,
                        });
                    }
                }
            }
        }
        *shown = Visibility::Inherited;
    }
}

/// Places the card window's Set dropdown next to its search field once both
/// have been laid out, outside the modal's clipped scrolling body and above
/// its footer. Flips above the field when there is not enough room below.
#[allow(clippy::type_complexity)] // popup, anchor and transform queries
pub(crate) fn place_set_popup(
    mut popups: Query<
        (Entity, &mut Node, &mut Visibility, &ComputedNode),
        With<super::print_picker::PickerSetPopup>,
    >,
    anchors: Query<(&ComputedNode, &UiGlobalTransform), With<super::print_picker::PickerSetAnchor>>,
    children: Query<&Children>,
    mut transforms: Query<&mut UiGlobalTransform, Without<super::print_picker::PickerSetAnchor>>,
    (windows, ui): (Query<&Window>, Option<Res<UiScale>>),
) {
    let Some(window) = windows.iter().next() else {
        return;
    };
    let Some((anchor_laid, anchor_at)) = anchors.iter().next() else {
        return;
    };
    let anchor_scale = anchor_laid.inverse_scale_factor();
    let anchor_size = anchor_laid.size() * anchor_scale;
    if anchor_size.x <= 0.0 || anchor_size.y <= 0.0 {
        return;
    }
    let anchor_mid = anchor_at.translation * anchor_scale;
    let anchor_rect = Rect::from_corners(
        anchor_mid - anchor_size * 0.5,
        anchor_mid + anchor_size * 0.5,
    );
    let (width, height) = crate::hud::scale::desktop_dimensions(window, ui.as_deref());
    for (entity, mut node, mut shown, laid) in &mut popups {
        let scale = laid.inverse_scale_factor();
        let size = laid.size() * scale;
        if size.y <= 0.0 {
            continue;
        }
        let pop_w = anchor_size.x;
        let left = anchor_rect.min.x.clamp(8.0, (width - pop_w - 8.0).max(8.0));
        let below = anchor_rect.max.y + 4.0;
        let above = anchor_rect.min.y - size.y - 4.0;
        let top = if below + size.y <= height - 8.0 {
            below
        } else if above >= 8.0 {
            above
        } else {
            (height - size.y - 8.0).max(8.0)
        };
        let changed = !matches!(node.left, Val::Px(x) if (x - left).abs() <= 0.5)
            || !matches!(node.top, Val::Px(y) if (y - top).abs() <= 0.5)
            || !matches!(node.width, Val::Px(w) if (w - pop_w).abs() <= 0.5);
        if changed {
            node.left = Val::Px(left);
            node.top = Val::Px(top);
            node.width = Val::Px(pop_w);
        }
        if (*shown == Visibility::Hidden || changed)
            && scale > 0.0
            && let Ok(cur_at) = transforms.get(entity)
        {
            let cur_min = cur_at.translation * scale - size * 0.5;
            let delta = (Vec2::new(left, top) - cur_min) / scale;
            for target in std::iter::once(entity).chain(children.iter_descendants(entity)) {
                if let Ok(mut t) = transforms.get_mut(target) {
                    *t = UiGlobalTransform::from(bevy::math::Affine2 {
                        matrix2: t.matrix2,
                        translation: t.translation + delta,
                    });
                }
            }
        }
        if *shown != Visibility::Inherited {
            *shown = Visibility::Inherited;
        }
    }
}

/// Shortens `full` (whose full measured pixel width is `full_width`) with a
/// trailing `…` so it fits within `clip_width`.
#[must_use]
#[allow(
    clippy::cast_precision_loss,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss
)]
pub(crate) fn elide_to_fit(full: &str, full_width: f32, clip_width: f32) -> String {
    if full_width <= clip_width + 0.5 {
        return full.to_string();
    }
    let chars: Vec<char> = full.chars().collect();
    if chars.is_empty() {
        return String::new();
    }
    let avg = (full_width / chars.len() as f32).max(1.0);
    let ellipsis_w = avg * 1.15;
    let target = (clip_width - ellipsis_w).max(0.0);
    let keep = ((target / full_width) * chars.len() as f32)
        .floor()
        .max(0.0) as usize;
    let keep = keep.min(chars.len().saturating_sub(1));
    let prefix: String = chars[..keep].iter().collect();
    let cleaned = prefix.trim_end_matches([' ', ',', ';', ':', '\u{b7}', '-', '\u{2014}']);
    format!("{cleaned}\u{2026}")
}

/// Replaces the clipped tail of any [`super::ElideText`] whose laid-out width
/// exceeds its `Overflow::clip_x()` parent box with a trailing `…`, without
/// mutating anything on quiet frames at rest.
#[allow(
    clippy::type_complexity,
    clippy::cast_precision_loss,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss
)]
pub(crate) fn elide_cells(
    mut texts: Query<(
        &mut Text,
        &mut super::ElideText,
        &bevy::text::TextLayoutInfo,
        &ChildOf,
    )>,
    clips: Query<&ComputedNode>,
) {
    for (mut text, mut elide, layout, parent) in &mut texts {
        let laid_w = layout.size.x;
        if laid_w <= 0.0 {
            continue;
        }
        let Ok(clip) = clips.get(parent.parent()) else {
            continue;
        };
        let clip_w = clip.size().x;
        if clip_w <= 0.0 {
            continue;
        }
        if elide.full_width <= 0.0 && text.0 == elide.full {
            elide.full_width = laid_w;
        }
        if elide.full_width <= 0.0 {
            continue;
        }
        if laid_w <= clip_w + 0.5 && (text.0 == elide.full || clip_w <= elide.last_clip + 4.0) {
            continue;
        }
        if text.0 == "\u{2026}" && laid_w > clip_w + 0.5 {
            continue;
        }
        let want = if elide.full_width <= clip_w + 0.5 {
            elide.full.clone()
        } else if laid_w > clip_w + 0.5 && text.0 != elide.full {
            let trimmed = text.0.trim_end_matches('\u{2026}');
            let cur_chars: Vec<char> = trimmed.chars().collect();
            let avg = (laid_w / (cur_chars.len() + 1).max(1) as f32).max(1.0);
            let ratio = ((clip_w - avg * 0.6) / laid_w).clamp(0.0, 0.99);
            let next_keep = ((cur_chars.len() as f32 * ratio).floor().max(0.0) as usize)
                .min(cur_chars.len().saturating_sub(1));
            let prefix: String = cur_chars[..next_keep].iter().collect();
            let cleaned = prefix.trim_end_matches([' ', ',', ';', ':', '\u{b7}', '-', '\u{2014}']);
            format!("{cleaned}\u{2026}")
        } else {
            elide_to_fit(&elide.full, elide.full_width, clip_w)
        };
        if (elide.last_clip - clip_w).abs() > 0.5 {
            elide.last_clip = clip_w;
        }
        if text.0 != want {
            text.0 = want;
        }
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

    #[test]
    fn elide_to_fit_keeps_fitting_text_and_ends_overflow_with_ellipsis() {
        assert_eq!(
            elide_to_fit("Lightning Bolt", 100.0, 120.0),
            "Lightning Bolt"
        );
        assert_eq!(
            elide_to_fit("Lightning Bolt", 100.0, 100.0),
            "Lightning Bolt"
        );
        let cut = elide_to_fit("70,000 Light-Years from Home", 220.0, 110.0);
        assert!(cut.ends_with('\u{2026}'), "expected ellipsis: {cut}");
        assert!(cut.chars().count() < "70,000 Light-Years from Home".chars().count());
        assert!(
            !cut.ends_with(" \u{2026}"),
            "no trailing space before ellipsis: {cut}"
        );
    }
}
