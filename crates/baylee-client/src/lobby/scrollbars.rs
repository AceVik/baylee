//! Native Bevy scrollbar gestures, with offsets retained across pane updates.
use super::{List, Metrics, Scrolled, scrolling::Scrollable};
use bevy::prelude::*;
use bevy::ui::{percent, px};
use bevy::ui_widgets::{ControlOrientation, Scrollbar, ScrollbarThumb};

/// Standard scrollbar track width on desktop vs touch/compact frames.
pub(crate) const fn bar_width(wide: bool) -> f32 {
    if wide { 12.0 } else { 8.0 }
}

/// Puts `list` and a scrollbar for it side by side in `parent`; returns the
/// bar's track.
pub(crate) fn attach(
    commands: &mut Commands,
    parent: Entity,
    list: Entity,
    metrics: Metrics,
) -> Entity {
    let host = commands
        .spawn((
            Node {
                width: percent(100),
                flex_grow: 1.0,
                flex_basis: px(0),
                min_height: px(0),
                column_gap: px(8),
                ..default()
            },
            Pickable::IGNORE,
        ))
        .id();
    commands.entity(list).entry::<Node>().and_modify(|mut n| {
        n.width = Val::Auto;
        n.flex_grow = 1.0;
        n.flex_basis = px(0);
        n.min_width = px(0);
    });
    let track = track(commands, list, metrics.frame == super::Frame::Compact);
    commands.entity(host).add_children(&[list, track]);
    commands.entity(parent).add_child(host);
    track
}

/// Puts `list` and an [`OnlyWhenNeeded`] scrollbar side by side in `parent`;
/// hidden (`Display::None`) while `list` fits so the content keeps 100% width.
pub(crate) fn attach_when_needed(
    commands: &mut Commands,
    parent: Entity,
    list: Entity,
    wide: bool,
) -> Entity {
    let host = commands
        .spawn((
            Node {
                width: percent(100),
                flex_grow: 1.0,
                flex_basis: px(0),
                min_height: px(0),
                column_gap: px(8),
                ..default()
            },
            Pickable::IGNORE,
        ))
        .id();
    commands.entity(list).entry::<Node>().and_modify(|mut n| {
        n.width = Val::Auto;
        n.flex_grow = 1.0;
        n.flex_basis = px(0);
        n.min_width = px(0);
    });
    let bar = track(commands, list, wide);
    commands
        .entity(bar)
        .insert(OnlyWhenNeeded)
        .entry::<Node>()
        .and_modify(|mut node| node.display = Display::None);
    commands.entity(host).add_children(&[list, bar]);
    commands.entity(parent).add_child(host);
    bar
}

/// Wraps `scroller` and an [`OnlyWhenNeeded`] scrollbar in a horizontal band
/// and returns that band entity (used by sheets and dialogs).
pub(crate) fn band_when_needed(commands: &mut Commands, scroller: Entity, wide: bool) -> Entity {
    band(commands, scroller, wide, false)
}

/// A stable-width viewport: the hidden scrollbar keeps its gutter during
/// measurement and updates, so rebuilding content cannot move its controls.
pub(crate) fn band_stable(commands: &mut Commands, scroller: Entity, wide: bool) -> Entity {
    band(commands, scroller, wide, true)
}

#[derive(Component)]
pub(crate) struct StableGutter;

fn band(commands: &mut Commands, scroller: Entity, wide: bool, stable: bool) -> Entity {
    let w = bar_width(wide);
    let track = track(commands, scroller, wide);
    commands.entity(track).insert((
        OnlyWhenNeeded,
        Node {
            width: px(w),
            height: percent(100),
            flex_shrink: 0.0,
            display: if stable { Display::Flex } else { Display::None },
            margin: UiRect::right(px(6)),
            border_radius: BorderRadius::all(px(w * 0.5)),
            ..default()
        },
    ));
    if stable {
        commands
            .entity(track)
            .insert((StableGutter, Visibility::Hidden));
    }
    let host = commands
        .spawn((
            Node {
                width: percent(100),
                flex_direction: FlexDirection::Row,
                flex_shrink: 1.0,
                min_height: px(0),
                overflow: Overflow::clip_y(),
                column_gap: px(6),
                ..default()
            },
            Pickable::IGNORE,
        ))
        .id();
    commands.entity(host).add_children(&[scroller, track]);
    commands.queue(move |world: &mut World| {
        let mut transferred_max_height = Val::Auto;
        if let Some(mut node) = world.get_mut::<Node>(scroller) {
            if node.max_height != Val::Auto && node.max_height != percent(100) {
                transferred_max_height = node.max_height;
            }
            node.flex_grow = 1.0;
            node.flex_basis = px(0);
            node.min_width = px(0);
            node.max_height = percent(100);
        }
        if transferred_max_height != Val::Auto
            && let Some(mut host_node) = world.get_mut::<Node>(host)
        {
            host_node.max_height = transferred_max_height;
        }
    });
    host
}

/// The bar itself for `list`, a track with its thumb, wider for a finger
/// (`wide`); the caller stands it beside the list.
pub(crate) fn track(commands: &mut Commands, list: Entity, wide: bool) -> Entity {
    let w = bar_width(wide);
    let radius = w * 0.5;
    let track = commands
        .spawn((
            Node {
                width: px(w),
                height: percent(100),
                flex_shrink: 0.0,
                border_radius: BorderRadius::all(px(radius)),
                ..default()
            },
            BackgroundColor(Color::srgba(0.08, 0.12, 0.18, 0.72)),
            Scrollbar::new(list, ControlOrientation::Vertical, 44.0),
        ))
        .id();
    let thumb = commands
        .spawn((
            ScrollbarThumb {
                border_radius: BorderRadius::all(px(radius)),
                border: UiRect::all(px(1)),
            },
            Node {
                width: percent(100),
                border_radius: BorderRadius::all(px(radius)),
                ..default()
            },
            BackgroundColor(Color::srgb(0.42, 0.49, 0.60)),
            BorderColor::all(Color::srgb(0.56, 0.64, 0.75)),
            UiTransform::default(),
            crate::ambience::Feel::tinting_to(
                Color::srgb(0.42, 0.49, 0.60),
                Color::srgb(0.58, 0.68, 0.82),
            ),
            Pickable::default(),
        ))
        .id();
    commands.entity(track).add_child(thumb);
    track
}

/// A scrollbar shown only while its list is taller than its view: the front
/// door's page (owner, 09.10.2026: a form taller than the window scrolls,
/// with a bar that says so), which most windows hold whole.
#[derive(Component, Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) struct OnlyWhenNeeded;

/// Shows an [`OnlyWhenNeeded`] bar while its list runs past its view, and
/// hides it otherwise; written only when that changes.
///
/// Taken out of the layout while hidden (`Display::None`), so a page that
/// fits keeps its whole width and stays centred.
pub(crate) fn show_when_needed(
    mut bars: Query<
        (&Scrollbar, &mut Node, &mut Visibility, Has<StableGutter>),
        With<OnlyWhenNeeded>,
    >,
    lists: Query<&ComputedNode>,
) {
    for (bar, mut node, mut visibility, stable) in &mut bars {
        let Ok(list) = lists.get(bar.target) else {
            continue;
        };
        let runs_over = list.content_size.y > list.size.y + 1.0;
        let display = if runs_over || stable {
            Display::Flex
        } else {
            Display::None
        };
        if node.display != display {
            node.display = display;
        }
        if stable {
            let next = if runs_over {
                Visibility::Inherited
            } else {
                Visibility::Hidden
            };
            if *visibility != next {
                *visibility = next;
            }
        }
    }
}

pub(super) fn remember(
    rows: Query<(&ScrollPosition, &Scrollable), Changed<ScrollPosition>>,
    mut memory: ResMut<Scrolled>,
) {
    for (position, which) in &rows {
        if !matches!(which.0, List::PickerSets)
            && (memory.get(which.0) - position.y).abs() > f32::EPSILON
        {
            memory.set(which.0, position.y);
        }
    }
}
