//! Original card-back ornament and a clear next step for an empty lobby.
use super::Metrics;
use crate::hud::{UiFonts, palette, tf};
use bevy::prelude::*;

/// A quiet, centred composition; actions are attached by the caller so they
/// follow the actual deck/search state, never a decorative promise.
pub(super) fn state(
    commands: &mut Commands,
    fonts: &UiFonts,
    metrics: Metrics,
    title: &str,
    description: &str,
) -> Entity {
    let root = commands
        .spawn((
            Node {
                width: percent(100),
                min_height: px(if metrics.stacked() { 240 } else { 280 }),
                flex_grow: 1.0,
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                row_gap: px(metrics.gap),
                padding: UiRect::axes(px(4), px(32)),
                ..default()
            },
            Pickable::IGNORE,
        ))
        .id();
    let art = ornament(commands);
    commands.entity(root).add_child(art);
    for (words, size, colour) in [
        (title, metrics.head, palette::INK),
        (description, metrics.text, palette::MUTED),
    ] {
        let text = commands
            .spawn((
                Text::new(words.to_owned()),
                tf(fonts, size),
                TextColor(colour),
                TextLayout::justify(Justify::Center),
                Node {
                    max_width: px(400),
                    width: percent(100),
                    margin: UiRect::bottom(px(4)),
                    ..default()
                },
                Pickable::IGNORE,
            ))
            .id();
        commands.entity(root).add_child(text);
    }
    root
}

/// Three unbranded, geometric card backs: no card art, borrowed emblem or font.
fn ornament(commands: &mut Commands) -> Entity {
    let root = commands
        .spawn((
            Node {
                width: px(138),
                height: px(106),
                flex_shrink: 0.0,
                margin: UiRect::bottom(px(12)),
                ..default()
            },
            Pickable::IGNORE,
        ))
        .id();
    for (left, angle, top) in [(16.0, -0.22, 18.0), (68.0, 0.22, 18.0), (42.0, 0.0, 7.0)] {
        let card = commands
            .spawn((
                Node {
                    position_type: PositionType::Absolute,
                    left: px(left),
                    top: px(top),
                    width: px(54),
                    height: px(76),
                    align_items: AlignItems::Center,
                    justify_content: JustifyContent::Center,
                    border: UiRect::all(px(1)),
                    border_radius: BorderRadius::all(px(7)),
                    ..default()
                },
                BackgroundColor(palette::PANEL),
                BorderColor::all(palette::DOCK_EDGE),
                UiTransform {
                    rotation: Rot2::radians(angle),
                    ..default()
                },
                Pickable::IGNORE,
            ))
            .id();
        let inlay = commands
            .spawn((
                Node {
                    width: px(19),
                    height: px(19),
                    border: UiRect::all(px(1)),
                    ..default()
                },
                BorderColor::all(palette::DOCK_EDGE),
                BackgroundColor(palette::PANEL_LIT),
                UiTransform {
                    rotation: Rot2::radians(std::f32::consts::FRAC_PI_4),
                    ..default()
                },
                Pickable::IGNORE,
            ))
            .id();
        commands.entity(card).add_child(inlay);
        commands.entity(root).add_child(card);
    }
    root
}
