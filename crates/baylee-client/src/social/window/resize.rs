//! The lobby window's resize corner and its per-device size.

#[allow(clippy::wildcard_imports)] // the window's own vocabulary
use super::*;
use crate::settings::ChatWindowSize;
use bevy::picking::events::{PointerDrag, PointerDragEnd};
use bevy::picking::hover::HoverMap;
use bevy::picking::pointer::{PointerButton, PointerId};
use bevy::window::{CursorIcon, PrimaryWindow, SystemCursorIcon};

const MIN_W: f32 = 320.0;
const MIN_H: f32 = 240.0;
const MARGIN: f32 = 24.0;
pub(super) const HANDLE: f32 = 18.0;

/// The top-right corner the player drags.
#[derive(Component)]
pub(crate) struct ResizeHandle;

/// The size chosen in this session and the drag currently changing it.
#[derive(Resource, Default)]
pub(crate) struct LobbyChatSizing {
    pub(super) resized: Option<ChatWindowSize>,
    start: Option<ChatWindowSize>,
}

/// The responsive default or the player's size, clamped to this window.
pub(crate) fn lobby_panel_size(
    stored: Option<ChatWindowSize>,
    window: Vec2,
    zoom: f32,
) -> ChatWindowSize {
    let zoom = if zoom.is_finite() && zoom > 0.0 {
        zoom
    } else {
        1.0
    };
    let available = ((window - Vec2::splat(MARGIN)) / zoom).max(Vec2::ZERO);
    let default = ChatWindowSize {
        width: LOBBY_W,
        height: lobby_panel_height(window.y),
    };
    let stored = stored.unwrap_or(default);
    let clean = |value: f32, fallback: f32| {
        if value.is_finite() { value } else { fallback }
    };
    let (min_w, min_h) = (MIN_W.min(available.x), MIN_H.min(available.y));
    ChatWindowSize {
        width: clean(stored.width, default.width).clamp(min_w, available.x.max(min_w)),
        height: clean(stored.height, default.height).clamp(min_h, available.y.max(min_h)),
    }
}

/// Draws a large, quiet corner target without spending a text glyph on it.
pub(super) fn handle(commands: &mut Commands) -> Entity {
    let mark = commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                right: zpx(4.0),
                top: zpx(4.0),
                width: zpx(8.0),
                height: zpx(8.0),
                border: UiRect {
                    right: zpx(2.0),
                    top: zpx(2.0),
                    ..UiRect::ZERO
                },
                ..default()
            },
            BorderColor::all(LOBBY_INKS.soft.with_alpha(0.65)),
            Pickable::IGNORE,
        ))
        .id();
    commands
        .spawn((
            ResizeHandle,
            ZIndex(2),
            Node {
                position_type: PositionType::Absolute,
                right: zpx(0.0),
                top: zpx(0.0),
                width: zpx(HANDLE),
                height: zpx(HANDLE),
                ..default()
            },
            Pickable::default(),
        ))
        .add_child(mark)
        .id()
}

/// Resizes from the corner's total drag distance; releasing persists once.
#[allow(clippy::too_many_arguments)] // a Bevy system: drag input, geometry and persisted sizing
pub(super) fn resize(
    drags: Option<MessageReader<PointerDrag>>,
    ends: Option<MessageReader<PointerDragEnd>>,
    handles: Query<(), With<ResizeHandle>>,
    ui: Res<ChatUi>,
    windows: Query<&Window>,
    scale: Option<Res<UiScale>>,
    zoom: Res<super::super::zoom::Zoom>,
    mut sizing: ResMut<LobbyChatSizing>,
    mut settings: Option<ResMut<crate::settings::ClientSettings>>,
) {
    let (Some(mut drags), Some(mut ends)) = (drags, ends) else {
        return;
    };
    let window = windows.iter().next().map_or(Vec2::new(1280.0, 720.0), |w| {
        crate::hud::scale::desktop_space(w, scale.as_deref())
    });
    for drag in drags.read() {
        if drag.button != PointerButton::Primary
            || !handles.contains(drag.entity)
            || !ui.open
            || ui.minimised
        {
            continue;
        }
        let current = lobby_panel_size(
            sizing
                .resized
                .or(settings.as_deref().and_then(|s| s.chat_window)),
            window,
            zoom.0,
        );
        let start = *sizing.start.get_or_insert(current);
        let scale = scale.as_deref().map_or(1.0, |scale| scale.0) * zoom.0;
        let asked = ChatWindowSize {
            width: start.width + drag.distance.x / scale,
            height: start.height - drag.distance.y / scale,
        };
        let next = lobby_panel_size(Some(asked), window, zoom.0);
        sizing.resized = Some(next);
        if let Some(settings) = settings.as_deref_mut() {
            settings.chat_window = Some(next);
        }
    }
    for end in ends.read() {
        if end.button != PointerButton::Primary || !handles.contains(end.entity) {
            continue;
        }
        sizing.start = None;
        if sizing.resized.is_some()
            && let Some(settings) = settings.as_deref()
        {
            settings.save();
        }
    }
}

/// Keeps the diagonal resize cursor over the handle after the kit's pointer.
pub(super) fn cursor(
    mut commands: Commands,
    hovers: Option<Res<HoverMap>>,
    handles: Query<(), With<ResizeHandle>>,
    windows: Query<(Entity, Option<&CursorIcon>), With<PrimaryWindow>>,
) {
    let over = hovers.as_deref().is_some_and(|hover| {
        hover
            .get(&PointerId::Mouse)
            .is_some_and(|hits| hits.keys().any(|entity| handles.contains(*entity)))
    });
    if !over {
        return;
    }
    let want = CursorIcon::System(SystemCursorIcon::NeswResize);
    for (window, now) in &windows {
        if now != Some(&want) {
            commands.entity(window).insert(want.clone());
        }
    }
}

#[cfg(test)]
mod tests;
