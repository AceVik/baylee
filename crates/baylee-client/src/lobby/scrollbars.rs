//! Native Bevy scrollbar gestures, with offsets retained across pane updates.
use super::{List, Metrics, Scrolled, systems::Scrollable};
use bevy::prelude::*;
use bevy::ui::{percent, px};
use bevy::ui_widgets::{ControlOrientation, Scrollbar, ScrollbarThumb};

pub(crate) fn attach(commands: &mut Commands, parent: Entity, list: Entity, metrics: Metrics) {
    let host = commands
        .spawn((
            Node {
                width: percent(100),
                flex_grow: 1.0,
                flex_basis: px(0),
                min_height: px(0),
                column_gap: px(10),
                ..default()
            },
            Pickable::IGNORE,
        ))
        .id();
    commands.entity(list).entry::<Node>().and_modify(|mut n| {
        n.width = Val::Auto;
        n.flex_basis = px(0);
        n.min_width = px(0);
    });
    let track = commands
        .spawn((
            Node {
                width: px(if metrics.frame == super::Frame::Phone {
                    16
                } else {
                    12
                }),
                height: percent(100),
                flex_shrink: 0.0,
                border_radius: BorderRadius::all(px(6)),
                ..default()
            },
            BackgroundColor(Color::srgba(0.08, 0.12, 0.18, 0.8)),
            Scrollbar::new(list, ControlOrientation::Vertical, 30.0),
        ))
        .id();
    let thumb = commands
        .spawn((
            ScrollbarThumb {
                border_radius: BorderRadius::all(px(6)),
                border: UiRect::all(px(1)),
            },
            BackgroundColor(Color::srgb(0.42, 0.47, 0.55)),
            BorderColor::all(Color::srgb(0.57, 0.58, 0.55)),
            Pickable::default(),
        ))
        .id();
    commands.entity(track).add_child(thumb);
    commands.entity(host).add_children(&[list, track]);
    commands.entity(parent).add_child(host);
}

pub(super) fn remember(
    rows: Query<(&ScrollPosition, &Scrollable), Changed<ScrollPosition>>,
    mut memory: ResMut<Scrolled>,
) {
    for (position, which) in &rows {
        if matches!(which.0, List::Deck | List::Pool)
            && (memory.get(which.0) - position.y).abs() > f32::EPSILON
        {
            memory.set(which.0, position.y);
        }
    }
}
