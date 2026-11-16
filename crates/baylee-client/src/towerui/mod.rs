//! Shared run-item presentation for the shop, inventory and table drawer.
use crate::{
    itemart::{Emphasis, Portrait},
    shellkit::{
        Role,
        controls::{self, Kit, Live},
        px_fixed, tokens,
    },
};
use baylee_client_core::echoes::domain::v2::{catalog::Rarity, shop::Goods};
use bevy::prelude::*;
mod layout;
mod rules;
mod runic;
mod scenery;
pub(crate) use layout::drag_ghost;
pub(crate) use layout::{badge, coin, empty, frame};
pub(crate) mod words;
pub(crate) use scenery::{Decor, install};
#[cfg(test)]
mod tests;

pub(crate) struct Item<'a> {
    pub goods: Goods,
    pub name: &'a str,
    pub description: &'a str,
    pub caption: Option<&'a str>,
    pub size: f32,
    pub emphasis: Emphasis,
}

pub(crate) fn color(goods: Goods) -> Color {
    let rarity = match goods {
        Goods::Relic(item) => item.rarity,
        Goods::Mythic => Rarity::MythicRare,
        _ => Rarity::Common,
    };
    match rarity {
        Rarity::Common => Color::srgb(0.56, 0.61, 0.64),
        Rarity::Uncommon => Color::srgb(0.51, 0.80, 0.88),
        Rarity::Rare => Color::srgb(0.97, 0.74, 0.34),
        Rarity::MythicRare => Color::srgb(1.0, 0.62, 0.34),
    }
}

/// The icon is the object: no card-shaped body or permanent name/description block.
pub(crate) fn button(
    commands: &mut Commands,
    kit: Kit,
    item: Item<'_>,
    live: Live<'_>,
    action: impl Bundle,
) -> Entity {
    let face = commands
        .spawn((
            Node {
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::Center,
                width: kit.m.px(item.size),
                height: kit.m.px(item.size),
                border_radius: BorderRadius::all(kit.m.px(10.0)),
                ..default()
            },
            Pickable::IGNORE,
            crate::shellkit::surfaces::elevation_control(),
            BackgroundColor(tokens::CONTROL),
            crate::ambience::Feel::tinting_to(tokens::CONTROL, tokens::HOVER),
            crate::ambience::FeelShadow,
        ))
        .id();
    commands.spawn((
        Portrait(item.goods),
        item.emphasis,
        Node {
            width: kit.m.px(item.size),
            height: kit.m.px(item.size),
            ..default()
        },
        ChildOf(face),
        Pickable::IGNORE,
    ));
    if let Some(caption) = item.caption {
        badge(commands, face, kit, caption, color(item.goods));
    }
    let owner = controls::hit(commands, kit, face, action);
    let tip = detail(commands, kit, &item, live);
    commands
        .entity(owner)
        .insert((controls::Tooltip(tip), controls::FocusTooltip));
    if matches!(live, Live::No(_)) {
        commands.entity(owner).insert(controls::Disabled);
    }
    owner
}

/// A read-only table icon with exactly the shop's detail presentation.
/// Table callers already apply their HUD scale; size is in that coordinate space.
pub(crate) fn inspectable(
    commands: &mut Commands,
    goods: Goods,
    fonts: &crate::hud::UiFonts,
    lang: baylee_client_core::i18n::Lang,
    size: f32,
) -> Entity {
    let kit = Kit {
        fonts,
        m: crate::shellkit::ShellMetrics::of(
            crate::shellkit::Viewport::desktop(1280.0, 720.0),
            crate::shellkit::TextSize::default(),
        ),
        german: lang == baylee_client_core::i18n::Lang::De,
    };
    let name = words::name(goods, lang);
    let description = words::description(goods, lang);
    let tip = detail(
        commands,
        kit,
        &Item {
            goods,
            name: &name,
            description: &description,
            caption: None,
            size,
            emphasis: Emphasis::None,
        },
        Live::Yes,
    );
    commands
        .spawn((
            Portrait(goods),
            Node {
                width: px_fixed(size),
                height: px_fixed(size),
                flex_shrink: 0.0,
                ..default()
            },
            controls::Tooltip(tip),
            bevy::picking::hover::PickingInteraction::default(),
            Pickable::default(),
        ))
        .id()
}

fn detail(commands: &mut Commands, kit: Kit, item: &Item<'_>, live: Live<'_>) -> Entity {
    let tip = commands
        .spawn((
            controls::FloatingTooltip,
            Role::Opaque,
            Node {
                display: Display::None,
                position_type: PositionType::Absolute,
                width: kit.m.px(420.0),
                max_width: Val::Vw(90.0),
                padding: UiRect::all(kit.m.px(16.0)),
                flex_direction: FlexDirection::Column,
                row_gap: kit.m.px(10.0),
                border: UiRect::all(px_fixed(1.5)),
                border_radius: BorderRadius::all(px_fixed(12.0)),
                ..default()
            },
            BackgroundColor(tokens::OPAQUE),
            BorderColor::all(color(item.goods)),
            crate::shellkit::surfaces::popover_shadow(),
            GlobalZIndex(tokens::z::TOOLTIP),
            Visibility::Hidden,
            Pickable::IGNORE,
        ))
        .id();
    let header = commands
        .spawn((
            Node {
                align_items: AlignItems::Center,
                column_gap: kit.m.px(10.0),
                ..default()
            },
            Pickable::IGNORE,
            ChildOf(tip),
        ))
        .id();
    commands.spawn((
        Portrait(item.goods),
        Node {
            width: kit.m.px(76.0),
            height: kit.m.px(76.0),
            flex_shrink: 0.0,
            ..default()
        },
        ChildOf(header),
        Pickable::IGNORE,
    ));
    let name = crate::shellkit::surfaces::prose(commands, kit, item.name, false);
    commands.entity(name).insert((
        crate::hud::tf_bold(kit.fonts, kit.m.text),
        TextColor(color(item.goods)),
    ));
    commands.entity(header).add_child(name);
    for line in item.description.lines() {
        let description =
            crate::manaui::spawn_rich(commands, kit.fonts, line, kit.m.small, tokens::INK);
        commands.entity(tip).add_child(description);
    }
    if let Live::No(reason) = live {
        let reason = crate::shellkit::surfaces::prose(commands, kit, reason, true);
        commands.entity(tip).add_child(reason);
    }
    tip
}
