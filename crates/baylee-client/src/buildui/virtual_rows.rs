#![allow(
    clippy::cast_precision_loss,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss
)] // Bounded UI pixel coordinates and row indices.
//! Keep row geometry stable while mounting controls only near the viewport.
use super::{Frame, LobbyState, Metrics, UiFonts, deck_row, pool_row};
use bevy::prelude::*;
use bevy::ui::{CalculatedClip, percent, px};

#[derive(Clone, Copy)]
pub(super) enum Row {
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
        (Row::Deck(_), Frame::Phone) => 144.0,
        (Row::Deck(_), _) => 90.0,
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
        &ChildOf,
        Option<&Children>,
    )>,
    scrollers: Query<(&ScrollPosition, &ComputedNode)>,
) {
    for (entity, row, node, transform, clip, parent, children) in &rows {
        if node.size.min_element() <= 0.0 {
            continue;
        }
        let visible = near_viewport(
            Rect::from_center_size(
                transform.translation - Vec2::Y * scroll_delta(parent, &scrollers),
                node.size,
            ),
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

/// A full catalog's geometry costs one entity, irrespective of its row count.
#[derive(Component)]
pub(crate) struct VirtualPool {
    slots: Vec<usize>,
    metrics: Metrics,
    mounted: std::collections::BTreeMap<usize, Entity>,
}

fn pool_pitch(metrics: Metrics) -> f32 {
    if metrics.frame == Frame::Phone {
        184.0
    } else {
        112.0
    }
}

pub(super) fn pool(
    commands: &mut Commands,
    state: &LobbyState,
    fonts: &UiFonts,
    metrics: Metrics,
    slots: Vec<usize>,
) -> Entity {
    let pitch = pool_pitch(metrics);
    let entity = commands
        .spawn((
            Node {
                width: percent(100),
                height: px(pitch * slots.len() as f32),
                flex_shrink: 0.0,
                ..default()
            },
            Pickable::IGNORE,
        ))
        .id();
    let mut list = VirtualPool {
        slots,
        metrics,
        mounted: default(),
    };
    for at in 0..list.slots.len().min(10) {
        mount_pool(commands, state, fonts, entity, &mut list, at);
    }
    commands.entity(entity).insert(list);
    entity
}

fn mount_pool(
    commands: &mut Commands,
    state: &LobbyState,
    fonts: &UiFonts,
    parent: Entity,
    list: &mut VirtualPool,
    at: usize,
) {
    let Some(row) = pool_row(commands, state, fonts, list.metrics, list.slots[at]) else {
        return;
    };
    let pitch = pool_pitch(list.metrics);
    commands
        .entity(row)
        .entry::<Node>()
        .and_modify(move |mut n| {
            n.position_type = PositionType::Absolute;
            n.top = px(at as f32 * pitch);
            n.height = px(pitch - 6.0);
        });
    commands.entity(parent).add_child(row);
    list.mounted.insert(at, row);
}

pub(crate) fn update_pool(
    mut commands: Commands,
    state: Res<LobbyState>,
    fonts: Res<UiFonts>,
    mut lists: Query<(
        Entity,
        &mut VirtualPool,
        &ComputedNode,
        &UiGlobalTransform,
        &CalculatedClip,
        &ChildOf,
    )>,
    scrollers: Query<(&ScrollPosition, &ComputedNode)>,
) {
    for (entity, mut list, node, transform, clip, parent) in &mut lists {
        if node.size.min_element() <= 0.0 {
            continue;
        }
        let top = transform.translation.y - node.size.y * 0.5 - scroll_delta(parent, &scrollers);
        let scale = node.inverse_scale_factor();
        let pitch = pool_pitch(list.metrics);
        let start = (((clip.clip.min.y - top) * scale - 160.0).max(0.0) / pitch).floor() as usize;
        let end = ((((clip.clip.max.y - top) * scale + 160.0).max(0.0) / pitch).ceil() as usize)
            .min(list.slots.len());
        let removed: Vec<_> = list
            .mounted
            .keys()
            .copied()
            .filter(|at| *at + 2 < start || *at >= end.saturating_add(2))
            .collect();
        for at in removed {
            if let Some(row) = list.mounted.remove(&at) {
                commands.entity(row).despawn();
            }
        }
        for at in start..end {
            if !list.mounted.contains_key(&at) {
                mount_pool(&mut commands, &state, &fonts, entity, &mut list, at);
            }
        }
    }
}

// Account for this frame's scroll input before Bevy computes new transforms.
fn scroll_delta(parent: &ChildOf, scrollers: &Query<(&ScrollPosition, &ComputedNode)>) -> f32 {
    scrollers
        .get(parent.parent())
        .map_or(0.0, |(position, node)| {
            position.y / node.inverse_scale_factor().max(f32::EPSILON) - node.scroll_position.y
        })
}
