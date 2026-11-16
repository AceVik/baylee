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
mod rules;
mod scenery;
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
        Rarity::Uncommon => Color::srgb(0.76, 0.85, 0.89),
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
                row_gap: kit.m.px(2.0),
                ..default()
            },
            Pickable::IGNORE,
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
        let label = controls::label(commands, kit, caption, kit.m.text, tokens::INK);
        commands
            .entity(label)
            .insert(crate::hud::tf_bold(kit.fonts, kit.m.text));
        commands.entity(face).add_child(label);
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
