//! The same furniture and slot geometry in the shop, bag and table drawer.
use super::{Kit, controls, px_fixed, tokens};
use bevy::prelude::*;

/// A real pointer-following object, outside layout and ignored by hit testing.
pub(crate) fn drag_ghost(
    commands: &mut Commands,
    goods: super::Goods,
    position: Vec2,
    side: f32,
) -> Entity {
    commands
        .spawn((
            super::Portrait(goods),
            Node {
                position_type: PositionType::Absolute,
                left: px(position.x - side * 0.5),
                top: px(position.y - side * 0.5),
                width: px(side),
                height: px(side),
                ..default()
            },
            crate::shellkit::surfaces::popover_shadow(),
            GlobalZIndex(tokens::z::TOOLTIP + 1),
            Pickable::IGNORE,
        ))
        .id()
}

pub(crate) fn frame(commands: &mut Commands, parent: Entity, kit: Kit) -> Entity {
    commands
        .spawn((
            Node {
                flex_direction: FlexDirection::Column,
                min_width: px(0),
                padding: UiRect::all(kit.m.px(12.0)),
                row_gap: kit.m.px(8.0),
                border: UiRect::all(px_fixed(1.0)),
                border_radius: BorderRadius::all(kit.m.px(14.0)),
                ..default()
            },
            BackgroundColor(tokens::CONTROL),
            BorderColor::all(tokens::GOLD.with_alpha(0.35)),
            crate::shellkit::surfaces::elevation_card(),
            Pickable::IGNORE,
            ChildOf(parent),
        ))
        .id()
}

pub(crate) fn empty(
    commands: &mut Commands,
    parent: Entity,
    kit: Kit,
    size: f32,
    icon: char,
) -> Entity {
    let slot = commands
        .spawn((
            Node {
                width: kit.m.px(size),
                height: kit.m.px(size),
                flex_shrink: 0.0,
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                border: UiRect::all(px_fixed(1.0)),
                border_radius: BorderRadius::all(kit.m.px(10.0)),
                ..default()
            },
            BorderColor::all(tokens::BORDER.with_alpha(0.65)),
            BackgroundColor(tokens::OPAQUE.with_alpha(0.6)),
            crate::shellkit::surfaces::elevation_control_disabled(),
            Pickable::default(),
            ChildOf(parent),
        ))
        .id();
    commands.spawn((
        Text::new(icon.to_string()),
        crate::hud::icon_tf(kit.fonts, kit.m.text),
        TextColor(tokens::MUTED.with_alpha(0.28)),
        Pickable::IGNORE,
        ChildOf(slot),
    ));
    slot
}

/// Counts remain inside a slot, including multi-digit charges and potion stacks.
pub(crate) fn badge(commands: &mut Commands, slot: Entity, kit: Kit, text: &str, rim: Color) {
    let badge = commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                right: kit.m.px(3.0),
                bottom: kit.m.px(3.0),
                min_width: kit.m.px(19.0),
                padding: UiRect::axes(kit.m.px(4.0), kit.m.px(1.0)),
                justify_content: JustifyContent::Center,
                border: UiRect::all(px_fixed(1.0)),
                border_radius: BorderRadius::all(kit.m.px(5.0)),
                ..default()
            },
            BackgroundColor(tokens::OPAQUE),
            BorderColor::all(rim),
            Pickable::IGNORE,
            ChildOf(slot),
        ))
        .id();
    let label = controls::label(commands, kit, text, kit.m.small, tokens::INK);
    commands
        .entity(label)
        .insert(crate::hud::tf_bold(kit.fonts, kit.m.small));
    commands.entity(badge).add_child(label);
}

pub(crate) fn coin(commands: &mut Commands, parent: Entity, kit: Kit, amount: u32) -> Entity {
    let band = commands
        .spawn((
            Node {
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                column_gap: kit.m.px(5.0),
                flex_shrink: 0.0,
                ..default()
            },
            Pickable::IGNORE,
            ChildOf(parent),
        ))
        .id();
    commands.spawn((
        Text::new(crate::hud::glyph::COINS.to_string()),
        crate::hud::icon_tf(kit.fonts, kit.m.text),
        TextColor(tokens::GOLD),
        Pickable::IGNORE,
        ChildOf(band),
    ));
    let text = controls::label(commands, kit, &amount.to_string(), kit.m.text, tokens::GOLD);
    commands
        .entity(text)
        .insert(crate::hud::tf_bold(kit.fonts, kit.m.text));
    commands.entity(band).add_child(text);
    band
}
