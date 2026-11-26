//! The lobby window's place (owner, 26.11.2026: "es soll bitte auch
//! dragable/moveable sein"): the player drags it by its head, wherever in
//! the window it fits, and this device keeps where it was put, as it keeps
//! its size.
//!
//! The place is the window's lower-left corner, `left` and `bottom` in the
//! same unscaled interface pixels as [`ChatWindowSize`]: the corner the
//! window is anchored by, so the resize corner keeps growing it up and to
//! the right from wherever it stands. Its default is where it always stood,
//! docked on the window's foot; put back there, it docks again.

#[allow(clippy::wildcard_imports)] // the window's own vocabulary
use super::*;
use crate::settings::{ChatWindowPlace, ChatWindowSize};
use bevy::picking::events::{PointerDrag, PointerDragEnd};
use bevy::picking::hover::HoverMap;
use bevy::picking::pointer::{PointerButton, PointerId};
use bevy::window::{CursorIcon, PrimaryWindow, SystemCursorIcon};

/// Where the window stands until it is moved: docked on the foot, a
/// little in from the left, as it always stood.
pub(crate) const DOCKED: ChatWindowPlace = ChatWindowPlace {
    left: 12.0,
    bottom: 0.0,
};

/// How close to the foot a dropped window docks on it, in interface pixels.
const DOCK_SNAP: f32 = 10.0;

/// The head the player drags the window by: its free band between the tabs
/// and the tools (a tab or a tool is a press, never a grip).
#[derive(Component, Clone, Copy, Debug, Default)]
pub(crate) struct MoveHandle;

/// The place chosen in this session and the drag currently moving it.
#[derive(Resource, Default)]
pub(crate) struct LobbyChatPlacing {
    pub(super) placed: Option<ChatWindowPlace>,
    start: Option<ChatWindowPlace>,
}

/// Whether the window stands on the window's foot: only its upper corners
/// are round then, as it grows out of the edge.
#[must_use]
pub(crate) fn docked(place: ChatWindowPlace) -> bool {
    place.bottom <= 0.0
}

/// The default or the player's place, held so that a window of `size`
/// stands whole inside `window` (UI pixels at `zoom`): a place kept from a
/// larger screen, or a window grown by its corner, is moved in, never cut.
#[must_use]
pub(crate) fn lobby_panel_place(
    stored: Option<ChatWindowPlace>,
    size: ChatWindowSize,
    window: Vec2,
    zoom: f32,
) -> ChatWindowPlace {
    let zoom = if zoom.is_finite() && zoom > 0.0 {
        zoom
    } else {
        1.0
    };
    let room = (window / zoom - Vec2::new(size.width, size.height)).max(Vec2::ZERO);
    let stored = stored.unwrap_or(DOCKED);
    let clean = |value: f32, fallback: f32| {
        if value.is_finite() { value } else { fallback }
    };
    ChatWindowPlace {
        left: clean(stored.left, DOCKED.left).clamp(0.0, room.x),
        bottom: clean(stored.bottom, DOCKED.bottom).clamp(0.0, room.y),
    }
}

/// The window's room in the lobby: the whole window, less what the
/// contacts drawer takes from its right while the chat is open.
pub(super) fn lobby_room(window: Vec2, open: bool, drawer_room: f32, zoom: f32) -> Vec2 {
    if open && drawer_room > 0.0 {
        Vec2::new((window.x - drawer_room).max(320.0 * zoom), window.y)
    } else {
        window
    }
}

/// Moves the window by the head's total drag distance; releasing keeps the
/// place on this device once. A window dropped near the foot docks there.
#[allow(clippy::too_many_arguments)] // a Bevy system: drag input, geometry and the kept place
pub(super) fn drag(
    drags: Option<MessageReader<PointerDrag>>,
    ends: Option<MessageReader<PointerDragEnd>>,
    handles: Query<(), With<MoveHandle>>,
    ui: Res<ChatUi>,
    windows: Query<&Window>,
    scale: Option<Res<UiScale>>,
    zoom: Res<super::super::zoom::Zoom>,
    slide: Option<Res<super::super::drawer::DrawerSlide>>,
    sizing: Res<LobbyChatSizing>,
    mut placing: ResMut<LobbyChatPlacing>,
    mut settings: Option<ResMut<crate::settings::ClientSettings>>,
) {
    let (Some(mut drags), Some(mut ends)) = (drags, ends) else {
        return;
    };
    let window = windows.iter().next().map_or(Vec2::new(1280.0, 720.0), |w| {
        crate::hud::scale::desktop_space(w, scale.as_deref())
    });
    let room = lobby_room(
        window,
        ui.open,
        slide.as_deref().map_or(0.0, |s| s.room(zoom.0)),
        zoom.0,
    );
    for drag in drags.read() {
        if !handles.contains(drag.entity)
            || drag.button != PointerButton::Primary
            || !ui.open
            || ui.minimised
        {
            continue;
        }
        let size = lobby_panel_size(
            sizing
                .resized
                .or(settings.as_deref().and_then(|s| s.chat_window)),
            room,
            zoom.0,
        );
        let current = lobby_panel_place(
            placing
                .placed
                .or(settings.as_deref().and_then(|s| s.chat_window_at)),
            size,
            room,
            zoom.0,
        );
        let start = *placing.start.get_or_insert(current);
        let scale = scale.as_deref().map_or(1.0, |scale| scale.0) * zoom.0;
        let asked = ChatWindowPlace {
            left: start.left + drag.distance.x / scale,
            bottom: start.bottom - drag.distance.y / scale,
        };
        let mut next = lobby_panel_place(Some(asked), size, room, zoom.0);
        if next.bottom < DOCK_SNAP {
            next.bottom = 0.0;
        }
        placing.placed = Some(next);
        if let Some(settings) = settings.as_deref_mut() {
            settings.chat_window_at = Some(next);
        }
    }
    for end in ends.read() {
        if end.button != PointerButton::Primary || !handles.contains(end.entity) {
            continue;
        }
        placing.start = None;
        if placing.placed.is_some()
            && let Some(settings) = settings.as_deref()
        {
            settings.save();
        }
    }
}

/// The window's clip is a rectangle, so the composer, the window's foot,
/// rounds its own lower corners to the window's inner corners once the
/// window floats (its radius less the border), and squares them again when
/// it docks: its wash never stands past the rim.
pub(super) fn round_the_foot(
    windows: Res<Windows>,
    roots: Query<&Node, With<LobbyChat>>,
    mut feet: Query<&mut Node, Without<LobbyChat>>,
) {
    let (Some(root), Some(foot)) = (windows.lobby.root, windows.lobby.composer) else {
        return;
    };
    let (Ok(root), Ok(mut foot)) = (roots.get(root), feet.get_mut(foot)) else {
        return;
    };
    let inside = |corner: CornerRadius| match corner.x {
        Val::Px(radius) if radius > 1.0 => CornerRadius::from(Val::Px(radius - 1.0)),
        _ => CornerRadius::ZERO,
    };
    let want = BorderRadius {
        bottom_left: inside(root.border_radius.bottom_left),
        bottom_right: inside(root.border_radius.bottom_right),
        ..BorderRadius::ZERO
    };
    if foot.border_radius != want {
        foot.border_radius = want;
    }
}

/// The open hand over the head's free band, the closed one while it moves
/// the window, after the kit's pointer has had its say.
pub(super) fn cursor(
    mut commands: Commands,
    hovers: Option<Res<HoverMap>>,
    handles: Query<(), With<MoveHandle>>,
    placing: Res<LobbyChatPlacing>,
    windows: Query<(Entity, Option<&CursorIcon>), With<PrimaryWindow>>,
) {
    let over = hovers.as_deref().is_some_and(|hover| {
        hover
            .get(&PointerId::Mouse)
            .is_some_and(|hit| hit.keys().any(|entity| handles.contains(*entity)))
    });
    if !over && placing.start.is_none() {
        return;
    }
    let want = CursorIcon::System(if placing.start.is_some() {
        SystemCursorIcon::Grabbing
    } else {
        SystemCursorIcon::Grab
    });
    for (window, now) in &windows {
        if now != Some(&want) {
            commands.entity(window).try_insert(want.clone());
        }
    }
}

#[cfg(test)]
mod tests;
