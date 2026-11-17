//! The same furniture and slot geometry in the shop, bag and table drawer.
use super::{Decor, Kit, controls, px_fixed, scenery::glint::FrostSurface, tokens};
use bevy::{
    prelude::*,
    ui::{BackgroundGradient, ColorStop, Gradient, LinearGradient},
};

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
                left: px(0.0),
                top: px(0.0),
                width: px(side),
                height: px(side),
                ..default()
            },
            UiTransform {
                translation: Val2::new(px(position.x - side * 0.5), px(position.y - side * 0.5)),
                ..default()
            },
            crate::shellkit::surfaces::popover_shadow(),
            GlobalZIndex(tokens::z::TOOLTIP + 1),
            Pickable::IGNORE,
        ))
        .id()
}

/// Ornate wooden frame (`tower/shop-frame.png`) backed by rich translucent dark
/// obsidian glass, warm inner highlight bevel, and card elevation shadow.
pub(crate) fn frame(commands: &mut Commands, parent: Entity, kit: Kit) -> Entity {
    let box_node = commands
        .spawn((
            Node {
                flex_direction: FlexDirection::Column,
                min_width: px(0),
                padding: UiRect::all(kit.m.px(16.0)),
                row_gap: kit.m.px(8.0),
                border: UiRect::all(px_fixed(1.0)),
                border_radius: BorderRadius::all(kit.m.px(12.0)),
                ..default()
            },
            BackgroundColor(Color::srgba(0.028, 0.024, 0.022, 0.78)),
            BackgroundGradient(vec![Gradient::Linear(LinearGradient {
                angle: std::f32::consts::PI,
                stops: vec![
                    ColorStop::auto(Color::srgba(0.068, 0.056, 0.044, 0.24)),
                    ColorStop::auto(Color::srgba(0.018, 0.015, 0.014, 0.14)),
                ],
                ..default()
            })]),
            BorderColor::all(Color::srgba(0.96, 0.78, 0.44, 0.34)),
            crate::shellkit::surfaces::elevation_card(),
            Pickable::IGNORE,
            ChildOf(parent),
        ))
        .id();
    commands.spawn((
        Decor::Frame,
        Node {
            position_type: PositionType::Absolute,
            left: px(0.0),
            top: px(0.0),
            width: percent(100),
            height: percent(100),
            ..default()
        },
        ZIndex(2),
        Pickable::IGNORE,
        ChildOf(box_node),
    ));
    box_node
}

/// Attaches a full-bleed GPU-clocked crystalline rime-frost overlay inside a
/// frozen market shelf or frame (`Pickable::IGNORE`, zero CPU churn at rest).
#[allow(dead_code)] // Cross-module door for runshop frozen-shelf overlay.
pub(crate) fn shelf_frost(commands: &mut Commands, parent: Entity) -> Entity {
    commands
        .spawn((
            FrostSurface,
            Node {
                position_type: PositionType::Absolute,
                left: px(0.0),
                top: px(0.0),
                width: percent(100),
                height: percent(100),
                border: UiRect::all(px_fixed(1.5)),
                border_radius: BorderRadius::all(px_fixed(10.0)),
                ..default()
            },
            BorderColor::all(Color::srgba(0.56, 0.88, 1.0, 0.58)),
            ZIndex(1),
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
            BorderColor::all(tokens::BORDER.with_alpha(0.55)),
            BackgroundColor(Color::srgba(0.020, 0.019, 0.022, 0.56)),
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
                right: kit.m.px(2.5),
                bottom: kit.m.px(2.5),
                min_width: kit.m.px(19.0),
                padding: UiRect::axes(kit.m.px(4.0), kit.m.px(1.0)),
                justify_content: JustifyContent::Center,
                border: UiRect::all(px_fixed(1.0)),
                border_radius: BorderRadius::all(kit.m.px(5.0)),
                ..default()
            },
            BackgroundColor(Color::srgba(0.035, 0.032, 0.042, 0.94)),
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
