//! Shared run-item presentation for the shop, inventory, table drawer, stack,
//! and game log.
use crate::{
    itemart::{Emphasis, Portrait},
    shellkit::{
        Role,
        controls::{self, Kit, Live},
        px_fixed, tokens,
    },
};
use baylee_client_core::{
    echoes::domain::v2::{catalog::Rarity, shop::Goods},
    i18n::Lang,
};
use bevy::{
    prelude::*,
    ui::{BackgroundGradient, ColorStop, Gradient, LinearGradient},
};

pub(crate) mod ladder;
mod layout;
pub(crate) mod motion;
pub(crate) mod prose;
mod rules;
mod runic;
mod scenery;
pub(crate) use layout::{badge, coin, drag_ghost, empty, frame, shelf_frost};
pub(crate) mod words;
pub(crate) use scenery::Decor;

/// The Tower interface's systems: its scenery, and the level ladder held
/// Shift opens on an item's overlay.
pub(crate) fn install(app: &mut App) {
    scenery::install(app);
    ladder::install(app);
}
#[cfg(test)]
mod tests;

pub(crate) struct Item<'a> {
    pub goods: Goods,
    pub name: &'a str,
    pub description: &'a str,
    pub caption: Option<&'a str>,
    pub hint: Option<&'a str>,
    pub size: f32,
    pub emphasis: Emphasis,
    pub selected: bool,
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

/// Builds a HUD-compatible [`Kit`] when a table caller only holds `fonts` and
/// `lang` (`UiScale` already scales table pixels).
#[must_use]
pub(crate) fn kit_for_hud(fonts: &crate::hud::UiFonts, lang: Lang) -> Kit<'_> {
    Kit {
        fonts,
        m: crate::shellkit::ShellMetrics::of(
            crate::shellkit::Viewport::desktop(1280.0, 720.0),
            crate::shellkit::TextSize::default(),
        ),
        german: lang == Lang::De,
    }
}

/// The icon is the object: optically centered square `Portrait` inside `face`,
/// with any count badge anchored absolutely at bottom-right so it never shifts
/// the centered portrait.
pub(crate) fn button(
    commands: &mut Commands,
    kit: Kit,
    item: Item<'_>,
    live: Live<'_>,
    action: impl Bundle,
) -> Entity {
    let accent = color(item.goods);
    let rest_bg = if item.selected {
        tokens::SELECTED
    } else {
        Color::srgba(0.032, 0.030, 0.036, 0.82)
    };
    let rest_border = if item.selected {
        tokens::GOLD
    } else {
        accent.with_alpha(0.46)
    };
    let face = commands
        .spawn((
            Node {
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                width: kit.m.px(item.size),
                height: kit.m.px(item.size),
                padding: UiRect::all(kit.m.px(2.0)),
                border: UiRect::all(px_fixed(if item.selected { 2.0 } else { 1.0 })),
                border_radius: BorderRadius::all(kit.m.px(10.0)),
                ..default()
            },
            Pickable::IGNORE,
            crate::shellkit::surfaces::elevation_control(),
            BackgroundColor(rest_bg),
            BorderColor::all(rest_border),
            crate::ambience::Feel::tinting_to(rest_bg, tokens::HOVER),
            crate::ambience::FeelBorder {
                rest: rest_border,
                hot: if item.selected { tokens::INK } else { accent },
                press: tokens::GOLD,
            },
            crate::ambience::FeelShadow,
        ))
        .id();
    commands.spawn((
        Portrait(item.goods),
        item.emphasis,
        Node {
            width: percent(100),
            height: percent(100),
            ..default()
        },
        ChildOf(face),
        Pickable::IGNORE,
    ));
    if let Some(caption) = item.caption {
        badge(commands, face, kit, caption, accent);
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

/// Spawns a standalone floating rich Item Detail Overlay (`FloatingTooltip`,
/// `Visibility::Hidden`, `Display::None`) for `goods`, ready to pair with
/// `(controls::Tooltip(tip), controls::FocusTooltip)` on any hoverable UI node.
pub(crate) fn item_tooltip(commands: &mut Commands, kit: Kit, goods: Goods, lang: Lang) -> Entity {
    let name = words::name(goods, lang);
    let description = words::description(goods, lang);
    detail(
        commands,
        kit,
        &Item {
            goods,
            name: &name,
            description: &description,
            caption: None,
            hint: None,
            size: 64.0,
            emphasis: Emphasis::None,
            selected: false,
        },
        Live::Yes,
    )
}

/// Spawns [`item_tooltip`] and attaches `(Tooltip(tip), FocusTooltip)` to
/// `owner` (safely despawning `tip` if `owner` was despawned before flush).
pub(crate) fn attach_item_tooltip(
    commands: &mut Commands,
    owner: Entity,
    kit: Kit,
    goods: Goods,
    lang: Lang,
) -> Entity {
    let tip = item_tooltip(commands, kit, goods, lang);
    commands.queue(move |world: &mut World| {
        if let Ok(mut ent) = world.get_entity_mut(owner) {
            ent.insert((
                controls::Tooltip(tip),
                controls::FocusTooltip,
                bevy::picking::hover::PickingInteraction::default(),
            ));
        } else if let Ok(orphan) = world.get_entity_mut(tip) {
            orphan.despawn();
        }
    });
    tip
}

/// A read-only table icon with the exact same elevated Item Detail Overlay.
/// Table callers already apply their HUD scale; `size` is in that coordinate space.
pub(crate) fn inspectable(
    commands: &mut Commands,
    goods: Goods,
    fonts: &crate::hud::UiFonts,
    lang: Lang,
    size: f32,
) -> Entity {
    let kit = kit_for_hud(fonts, lang);
    let tip = item_tooltip(commands, kit, goods, lang);
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
            controls::FocusTooltip,
            bevy::picking::hover::PickingInteraction::default(),
            Pickable::default(),
        ))
        .id()
}

fn detail_elevation(accent: Color) -> BoxShadow {
    let mut layers = crate::shellkit::surfaces::elevation_floating().0;
    layers.push(ShadowStyle {
        color: accent.with_alpha(0.20),
        x_offset: px_fixed(0.0),
        y_offset: px_fixed(2.0),
        spread_radius: px_fixed(0.5),
        blur_radius: px_fixed(20.0),
    });
    BoxShadow(layers)
}

/// A 1 px line in the item's accent across `parent`: between a detail card's
/// header and its effect lines, and over the level ladder.
fn hairline(commands: &mut Commands, parent: Entity, accent: Color) {
    commands.spawn((
        Node {
            width: percent(100),
            height: px_fixed(1.0),
            ..default()
        },
        BackgroundColor(accent.with_alpha(0.22)),
        Pickable::IGNORE,
        ChildOf(parent),
    ));
}

#[allow(clippy::too_many_lines)] // Cohesive detail card: accent bar, header, subline pills, rich body.
fn detail(commands: &mut Commands, kit: Kit, item: &Item<'_>, live: Live<'_>) -> Entity {
    let lang = kit.lang();
    let accent = color(item.goods);
    let tip = commands
        .spawn((
            controls::FloatingTooltip,
            Role::Opaque,
            Node {
                display: Display::None,
                position_type: PositionType::Absolute,
                width: kit.m.px(ladder::REST_WIDTH),
                max_width: Val::Vw(90.0),
                padding: UiRect::new(
                    kit.m.px(16.0),
                    kit.m.px(16.0),
                    kit.m.px(18.0),
                    kit.m.px(16.0),
                ),
                flex_direction: FlexDirection::Column,
                row_gap: kit.m.px(10.0),
                border: UiRect::all(px_fixed(1.5)),
                border_radius: BorderRadius::all(px_fixed(12.0)),
                overflow: Overflow::clip(),
                ..default()
            },
            BackgroundColor(Color::srgba(0.024, 0.027, 0.036, 0.95)),
            BackgroundGradient(vec![Gradient::Linear(LinearGradient {
                angle: std::f32::consts::PI,
                stops: vec![
                    ColorStop::auto(Color::srgba(0.068, 0.076, 0.098, 0.34)),
                    ColorStop::auto(Color::srgba(0.016, 0.018, 0.025, 0.18)),
                ],
                ..default()
            })]),
            BorderColor::all(accent.with_alpha(0.72)),
            detail_elevation(accent),
            GlobalZIndex(tokens::z::TOOLTIP),
            Visibility::Hidden,
            Pickable::IGNORE,
        ))
        .id();

    // Rarity-colored top accent bar across the obsidian-glass crown.
    commands.spawn((
        Node {
            position_type: PositionType::Absolute,
            left: px_fixed(0.0),
            right: px_fixed(0.0),
            top: px_fixed(0.0),
            height: px_fixed(3.0),
            ..default()
        },
        BackgroundColor(accent.with_alpha(0.92)),
        Pickable::IGNORE,
        ChildOf(tip),
    ));

    let header = commands
        .spawn((
            Node {
                align_items: AlignItems::Center,
                column_gap: kit.m.px(12.0),
                ..default()
            },
            Pickable::IGNORE,
            ChildOf(tip),
        ))
        .id();
    commands.spawn((
        Portrait(item.goods),
        item.emphasis,
        Node {
            width: kit.m.px(72.0),
            height: kit.m.px(72.0),
            flex_shrink: 0.0,
            ..default()
        },
        ChildOf(header),
        Pickable::IGNORE,
    ));

    let title_col = commands
        .spawn((
            Node {
                flex_direction: FlexDirection::Column,
                row_gap: kit.m.px(4.0),
                flex_grow: 1.0,
                min_width: px_fixed(0.0),
                ..default()
            },
            Pickable::IGNORE,
            ChildOf(header),
        ))
        .id();
    let title_text = words::clean_title(item.name);
    let name_node = crate::shellkit::surfaces::prose(commands, kit, title_text, false);
    commands.entity(name_node).insert((
        crate::hud::tf_bold(
            kit.fonts,
            crate::shellkit::surfaces::whole_pixels(kit.m.text * 1.05),
        ),
        TextColor(tokens::INK),
    ));
    commands.entity(title_col).add_child(name_node);

    let rarity_sub = words::rarity_subline(item.goods, lang);
    let limit_tag = words::activation_limit_tag(item.goods, lang);
    if rarity_sub.is_some() || limit_tag.is_some() {
        let meta_row = commands
            .spawn((
                Node {
                    align_items: AlignItems::Center,
                    flex_wrap: FlexWrap::Wrap,
                    column_gap: kit.m.px(8.0),
                    row_gap: kit.m.px(4.0),
                    ..default()
                },
                Pickable::IGNORE,
                ChildOf(title_col),
            ))
            .id();
        if let Some(rarity_label) = rarity_sub {
            let sub = crate::shellkit::surfaces::prose(commands, kit, rarity_label, true);
            commands
                .entity(sub)
                .insert(TextColor(accent.with_alpha(0.92)));
            commands.entity(meta_row).add_child(sub);
        }
        if let Some(tag_text) = limit_tag {
            let pill = commands
                .spawn((
                    Node {
                        padding: UiRect::axes(kit.m.px(7.0), kit.m.px(1.5)),
                        border: UiRect::all(px_fixed(1.0)),
                        border_radius: BorderRadius::all(px_fixed(tokens::RADIUS_PILL)),
                        align_items: AlignItems::Center,
                        ..default()
                    },
                    BackgroundColor(Color::srgba(0.96, 0.72, 0.28, 0.14)),
                    BorderColor::all(Color::srgba(0.96, 0.76, 0.36, 0.52)),
                    Pickable::IGNORE,
                    ChildOf(meta_row),
                ))
                .id();
            let pill_label = controls::label(
                commands,
                kit,
                &tag_text,
                kit.m.small * 0.92,
                Color::srgb(0.98, 0.82, 0.46),
            );
            commands
                .entity(pill_label)
                .insert(crate::hud::tf_bold(kit.fonts, kit.m.small * 0.92));
            commands.entity(pill).add_child(pill_label);
        }
    }

    // Subtle hairline separating header identity from color-coded effect lines.
    hairline(commands, tip, accent);

    let body = commands
        .spawn((
            Node {
                flex_direction: FlexDirection::Column,
                row_gap: kit.m.px(6.0),
                ..default()
            },
            Pickable::IGNORE,
            ChildOf(tip),
        ))
        .id();
    for line in item.description.lines().filter(|line| !line.is_empty()) {
        let description = prose::spawn_line(commands, kit.fonts, line, kit.m.small);
        commands.entity(body).add_child(description);
    }
    // Only a relic has levels to compare (held Shift, `ladder`).
    let levels = match item.goods {
        Goods::Relic(definition) => Some((definition, ladder::slot(commands, kit, tip))),
        _ => None,
    };

    if let Some(hint) = item.hint
        && !hint.is_empty()
    {
        let note = crate::shellkit::surfaces::prose(commands, kit, hint, true);
        commands.entity(tip).add_child(note);
    }
    if let Live::No(reason) = live
        && !reason.is_empty()
    {
        let reason = crate::shellkit::surfaces::prose(commands, kit, reason, true);
        commands.entity(tip).add_child(reason);
    }
    if let Some((definition, slot)) = levels {
        ladder::attach(commands, kit, tip, slot, definition);
    }
    tip
}
