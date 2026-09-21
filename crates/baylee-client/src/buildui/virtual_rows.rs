//! Keep row geometry stable while mounting controls only near the viewport.
use super::{Frame, LobbyState, Metrics, UiFonts, deck_row, pool_row};
use bevy::prelude::*;
use bevy::ui::{CalculatedClip, percent, px};

#[derive(Clone, Copy)]
pub(super) enum Row {
    Pool(usize),
    Deck(usize),
}

#[derive(Component)]
pub(crate) struct VirtualRow {
    row: Row,
    metrics: Metrics,
}

pub(super) fn spawn(
    commands: &mut Commands,
    state: &LobbyState,
    fonts: &UiFonts,
    metrics: Metrics,
    row: Row,
    initially_visible: bool,
) -> Entity {
    let height = match (row, metrics.frame) {
        (Row::Pool(_), Frame::Phone) => 156.0,
        (Row::Pool(_), _) => 112.0,
        (Row::Deck(_), Frame::Phone) => 128.0,
        (Row::Deck(_), _) => 86.0,
    };
    let entity = commands
        .spawn((
            Node {
                width: percent(100),
                height: px(height),
                min_height: px(height),
                flex_shrink: 0.0,
                ..default()
            },
            VirtualRow { row, metrics },
        ))
        .id();
    if initially_visible {
        mount(commands, state, fonts, entity, row, metrics);
    }
    entity
}

fn mount(
    commands: &mut Commands,
    state: &LobbyState,
    fonts: &UiFonts,
    parent: Entity,
    row: Row,
    metrics: Metrics,
) {
    let content = match row {
        Row::Pool(slot) => pool_row(commands, state, fonts, metrics, slot),
        Row::Deck(at) => deck_row(commands, state, fonts, metrics, at),
    };
    if let Some(content) = content {
        commands.entity(parent).add_child(content);
    }
}

fn near_viewport(bounds: Rect, clip: Rect) -> bool {
    !bounds
        .intersect(Rect::from_corners(
            clip.min - Vec2::Y * 160.0,
            clip.max + Vec2::Y * 160.0,
        ))
        .is_empty()
}

#[allow(clippy::type_complexity)] // Geometry and existing children of virtual rows.
pub(crate) fn update(
    mut commands: Commands,
    state: Res<LobbyState>,
    fonts: Res<UiFonts>,
    rows: Query<(
        Entity,
        &VirtualRow,
        &ComputedNode,
        &UiGlobalTransform,
        &CalculatedClip,
        Option<&Children>,
    )>,
) {
    for (entity, row, node, transform, clip, children) in &rows {
        if node.size.min_element() <= 0.0 {
            continue;
        }
        let visible = near_viewport(
            Rect::from_center_size(transform.translation, node.size),
            clip.clip,
        );
        if visible && children.is_none() {
            mount(&mut commands, &state, &fonts, entity, row.row, row.metrics);
        } else if !visible && let Some(children) = children {
            for child in children {
                commands.entity(*child).despawn();
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn overscan_keeps_nearby_rows_but_excludes_distant_rows() {
        let clip = Rect::new(0.0, 200.0, 400.0, 600.0);
        assert!(near_viewport(Rect::new(0.0, 100.0, 400.0, 180.0), clip));
        assert!(near_viewport(Rect::new(0.0, 590.0, 400.0, 670.0), clip));
        assert!(!near_viewport(Rect::new(0.0, 900.0, 400.0, 980.0), clip));
    }
}
