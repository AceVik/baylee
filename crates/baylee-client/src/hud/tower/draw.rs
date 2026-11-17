#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_client_core::echoes::domain::v2::catalog::{Potion, Rarity};
use shellkit::focus::Stop;
use shellkit::{controls, px_fixed};

#[allow(clippy::too_many_lines)]
pub(super) fn draw(
    commands: &mut Commands,
    kit: Kit,
    duel: &Duel,
    canvas: Vec2,
    lang: Lang,
    scroll_y: f32,
) {
    if !duel.tower_inventory {
        // The source is run equipment, not a card on the felt. Keep a hidden
        // layout-only anchor box after closing the inventory so the shared
        // ability chooser places itself cleanly without drawing a second
        // portrait over AbilitySheet's text or flashing before/after it.
        if let Some(object) = duel.ability_menu
            && can_use(duel, object)
            && let Some(source) = duel.view.as_ref().and_then(|view| view.object(object))
            && crate::itemart::identity::run_source(source).is_some()
        {
            let size = kit.m.scale * 64.0;
            commands.spawn((
                Root,
                super::super::sheet::AbilityAnchor(object),
                Node {
                    position_type: PositionType::Absolute,
                    right: px_fixed(24.0),
                    bottom: px_fixed(super::super::hand::HAND_ZONE_H + 64.0),
                    width: px_fixed(size),
                    height: px_fixed(size),
                    flex_shrink: 0.0,
                    ..default()
                },
                Visibility::Hidden,
                Pickable::IGNORE,
            ));
        }
        return;
    }
    commands.spawn((
        Root,
        Node {
            position_type: PositionType::Absolute,
            left: px_fixed(0.0),
            right: px_fixed(0.0),
            top: px_fixed(0.0),
            bottom: px_fixed(0.0),
            ..default()
        },
        BackgroundColor(Color::NONE),
        GlobalZIndex(super::super::Z_MENU - 1),
        UiButton,
        MenuButton {
            action: MenuAction::ToggleTowerItems,
        },
    ));
    let root = commands
        .spawn((
            Root,
            Node {
                position_type: PositionType::Absolute,
                right: px_fixed(16.0),
                bottom: px_fixed(
                    super::super::hand::HAND_ZONE_H - super::super::hand::LEDGE_H + 56.0,
                ),
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::End,
                row_gap: kit.m.px(8.0),
                ..default()
            },
            GlobalZIndex(super::super::Z_MENU),
            Pickable::IGNORE,
        ))
        .id();
    let radius = kit.m.px(14.0);
    let shell = commands
        .spawn((
            shellkit::Role::Opaque,
            Node {
                width: kit.m.px(464.0),
                max_width: px_fixed((canvas.x - 32.0).max(280.0)),
                max_height: px_fixed((canvas.y - 104.0).max(240.0)),
                flex_direction: FlexDirection::Column,
                border: UiRect::all(px_fixed(1.0)),
                border_radius: BorderRadius::all(radius),
                overflow: Overflow::clip(),
                ..default()
            },
            BackgroundColor(Color::NONE),
            crate::social::rows::dialog_glass(),
            BorderColor::all(tokens::GOLD.with_alpha(0.52)),
            shellkit::surfaces::popover_shadow(),
            Pickable::default(),
            ChildOf(root),
        ))
        .id();
    let rim = commands
        .spawn(crate::social::glass::layer(
            crate::social::glass::Glass::Panel,
            radius,
        ))
        .id();
    commands.entity(shell).add_child(rim);
    commands.spawn((
        towerui::Decor::Frame,
        Node {
            position_type: PositionType::Absolute,
            left: px_fixed(0.0),
            top: px_fixed(0.0),
            width: percent(100),
            height: percent(100),
            ..default()
        },
        ZIndex(2),
        Pickable::IGNORE,
        ChildOf(shell),
    ));
    title(commands, shell, kit, lang);
    let panel = commands
        .spawn((
            Panel,
            crate::hud::scroll::Scrolls,
            ScrollPosition(Vec2::new(0.0, scroll_y)),
            Node {
                width: percent(100),
                flex_grow: 1.0,
                min_height: px_fixed(0.0),
                overflow: Overflow::scroll_y(),
                flex_direction: FlexDirection::Column,
                row_gap: kit.m.px(10.0),
                padding: UiRect::new(
                    kit.m.px(16.0),
                    kit.m.px(16.0),
                    kit.m.px(12.0),
                    kit.m.px(16.0),
                ),
                ..default()
            },
            ZIndex(1),
            Pickable::default(),
            ChildOf(shell),
        ))
        .id();
    seats(commands, panel, kit, duel, lang);
}

fn close(commands: &mut Commands, parent: Entity, kit: Kit, lang: Lang) {
    let toggle = controls::button_compact(
        commands,
        kit,
        Some(super::super::glyph::CLOSE),
        Phrase::ShellClose.text(lang),
        Weight::Secondary,
        Live::Yes,
        (
            UiButton,
            MenuButton {
                action: MenuAction::ToggleTowerItems,
            },
        ),
    );
    commands
        .entity(toggle)
        .insert(Stop::new(keys::ORDER.name, "close"));
    commands.entity(parent).add_child(toggle);
}

fn title(commands: &mut Commands, parent: Entity, kit: Kit, lang: Lang) {
    let row = commands
        .spawn((
            Node {
                width: percent(100),
                flex_shrink: 0.0,
                justify_content: JustifyContent::SpaceBetween,
                align_items: AlignItems::Center,
                column_gap: kit.m.px(8.0),
                padding: UiRect::new(
                    kit.m.px(18.0),
                    kit.m.px(16.0),
                    kit.m.px(14.0),
                    kit.m.px(10.0),
                ),
                border: UiRect::bottom(px_fixed(1.0)),
                ..default()
            },
            BackgroundColor(Color::NONE),
            crate::social::rows::head_glass(),
            BorderColor::all(tokens::GOLD.with_alpha(0.24)),
            ZIndex(1),
            ChildOf(parent),
            Pickable::IGNORE,
        ))
        .id();
    let left = commands
        .spawn((
            Node {
                align_items: AlignItems::Center,
                column_gap: kit.m.px(8.0),
                ..default()
            },
            ChildOf(row),
            Pickable::IGNORE,
        ))
        .id();
    commands.spawn((
        Text::new(super::super::glyph::DIAMOND.to_string()),
        super::super::icon_tf(kit.fonts, kit.m.small),
        TextColor(tokens::GOLD),
        ChildOf(left),
        Pickable::IGNORE,
    ));
    let heading_text =
        shellkit::surfaces::prose(commands, kit, Phrase::EchoesRunTableItems.text(lang), false);
    commands.entity(heading_text).insert((
        crate::hud::tf_bold(kit.fonts, kit.m.text),
        TextColor(tokens::INK),
    ));
    commands.entity(left).add_child(heading_text);
    let info = commands
        .spawn((
            Text::new(super::super::glyph::INFO.to_string()),
            super::super::icon_tf(kit.fonts, kit.m.small),
            TextColor(tokens::MUTED),
            ChildOf(left),
            Pickable::default(),
        ))
        .id();
    controls::attach_tooltip(commands, info, kit, Phrase::EchoesRunTableHint.text(lang));
    close(commands, row, kit, lang);
}

fn section_head(
    commands: &mut Commands,
    parent: Entity,
    kit: Kit,
    icon: char,
    accent: Color,
    name: &str,
    badge_text: Option<String>,
) {
    let row = commands
        .spawn((
            Node {
                width: percent(100),
                align_items: AlignItems::Center,
                justify_content: JustifyContent::SpaceBetween,
                column_gap: kit.m.px(8.0),
                ..default()
            },
            Pickable::IGNORE,
            ChildOf(parent),
        ))
        .id();
    let left = commands
        .spawn((
            Node {
                align_items: AlignItems::Center,
                column_gap: kit.m.px(7.0),
                min_width: px_fixed(0.0),
                ..default()
            },
            Pickable::IGNORE,
            ChildOf(row),
        ))
        .id();
    commands.spawn((
        Text::new(icon.to_string()),
        super::super::icon_tf(kit.fonts, kit.m.small * 0.92),
        TextColor(accent),
        Pickable::IGNORE,
        ChildOf(left),
    ));
    let label = shellkit::surfaces::prose(commands, kit, name, false);
    commands.entity(label).insert((
        crate::hud::tf_bold(kit.fonts, kit.m.small),
        TextColor(tokens::INK),
    ));
    commands.entity(left).add_child(label);
    if let Some(count) = badge_text {
        commands.spawn((
            Node {
                padding: UiRect::axes(kit.m.px(7.0), px_fixed(1.5)),
                border: UiRect::all(px_fixed(1.0)),
                border_radius: BorderRadius::all(px_fixed(tokens::RADIUS_PILL)),
                ..default()
            },
            BackgroundColor(Color::srgba(1.0, 1.0, 1.0, 0.045)),
            BorderColor::all(tokens::BORDER.with_alpha(0.65)),
            Pickable::IGNORE,
            ChildOf(row),
            children![(
                Text::new(count),
                crate::hud::tf_bold(kit.fonts, kit.m.small * 0.88),
                TextColor(tokens::MUTED),
                Pickable::IGNORE,
            )],
        ));
    }
}

fn items(commands: &mut Commands, parent: Entity, kit: Kit) -> Entity {
    commands
        .spawn((
            Node {
                flex_wrap: FlexWrap::Wrap,
                column_gap: kit.m.px(7.0),
                row_gap: kit.m.px(7.0),
                ..default()
            },
            Pickable::IGNORE,
            ChildOf(parent),
        ))
        .id()
}

fn emphasis_for(goods: Goods) -> crate::itemart::Emphasis {
    match goods {
        Goods::Relic(def) => match def.rarity {
            Rarity::Common => crate::itemart::Emphasis::Owned,
            Rarity::Uncommon => crate::itemart::Emphasis::Uncommon,
            Rarity::Rare => crate::itemart::Emphasis::Rare,
            Rarity::MythicRare => crate::itemart::Emphasis::Mythic,
        },
        Goods::Mythic => crate::itemart::Emphasis::Mythic,
        Goods::Potion(_) | Goods::Card(_) => crate::itemart::Emphasis::None,
    }
}

fn item(
    commands: &mut Commands,
    parent: Entity,
    kit: Kit,
    duel: &Duel,
    object: &baylee_view::PublicObject,
    goods: Goods,
    lang: Lang,
) -> Entity {
    let name = towerui::words::name(goods, lang);
    let description = towerui::words::description(goods, lang);
    let count = object.counter_count(baylee_view::CounterKind::Charge);
    let caption = (count > 0 || matches!(goods, Goods::Potion(_))).then(|| format!("×{count}"));
    let usable = can_use(duel, object.id);
    let active = duel.ability_menu == Some(object.id);
    let button = towerui::button(
        commands,
        kit,
        towerui::Item {
            goods,
            name: &name,
            description: &description,
            caption: caption.as_deref(),
            hint: None,
            size: 64.0,
            emphasis: emphasis_for(goods),
            selected: active,
        },
        if usable { Live::Yes } else { Live::No("") },
        (),
    );
    if usable {
        commands.entity(button).insert((
            UiButton,
            MenuButton {
                action: MenuAction::UseTowerItem(object.id),
            },
        ));
        commands.spawn((
            Node {
                position_type: PositionType::Absolute,
                top: kit.m.px(2.0),
                right: kit.m.px(2.0),
                width: kit.m.px(16.0),
                height: kit.m.px(16.0),
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                border_radius: BorderRadius::all(px_fixed(8.0)),
                ..default()
            },
            BackgroundColor(Color::srgba(0.08, 0.07, 0.04, 0.85)),
            Pickable::IGNORE,
            ChildOf(button),
            children![(
                Text::new(super::super::glyph::ENERGY.to_string()),
                crate::hud::icon_tf(kit.fonts, kit.m.small * 0.82),
                TextColor(tokens::GOLD),
                Pickable::IGNORE,
            )],
        ));
    }
    commands.entity(parent).add_child(button);
    button
}

fn seat_name(duel: &Duel, player: baylee_core::ids::PlayerId, lang: Lang) -> &str {
    duel.statics
        .as_ref()
        .and_then(|game| game.seats.iter().find(|entry| entry.player == player))
        .map_or_else(
            || Phrase::EchoesRunOpponent.text(lang),
            |entry| {
                if entry.is_ai && entry.display_name == "House AI" {
                    Phrase::SeatHouse.text(lang)
                } else {
                    entry.display_name.as_str()
                }
            },
        )
}

#[allow(clippy::too_many_lines)]
fn seats(commands: &mut Commands, panel: Entity, kit: Kit, duel: &Duel, lang: Lang) {
    let Some(view) = &duel.view else {
        return;
    };
    let reader = view.seat;
    // Order local seat first when playing so own relics + potions sit at the top,
    // followed by opponents who actually carry visible equipment.
    let mut ordered: Vec<usize> = (0..view.command.len()).collect();
    if !duel.watching {
        ordered.sort_by_key(|&seat| usize::from(seat != usize::from(reader.get())));
    }
    for seat in ordered {
        let objects = &view.command[seat];
        let player = baylee_core::ids::PlayerId::new(u8::try_from(seat).unwrap_or(u8::MAX));
        let is_own = !duel.watching && player == reader;
        let relics: Vec<(&baylee_view::PublicObject, Goods)> = objects
            .iter()
            .filter_map(|object| {
                let goods @ Goods::Relic(_) = visible(object, reader, duel.watching)? else {
                    return None;
                };
                Some((object, goods))
            })
            .collect();
        if !is_own && relics.is_empty() {
            continue;
        }
        let card = towerui::frame(commands, panel, kit);
        let count = u8::try_from(relics.len()).unwrap_or(6);
        if is_own {
            section_head(
                commands,
                card,
                kit,
                super::super::glyph::SHIELD,
                tokens::GOLD,
                Phrase::EchoesEquipped.text(lang),
                Some(format!("{count}/6")),
            );
        } else {
            let accent = crate::hud::seat_colour(reader, duel.statics.as_ref(), player);
            section_head(
                commands,
                card,
                kit,
                super::super::glyph::SHIELD,
                accent,
                seat_name(duel, player, lang),
                Some(format!("×{count}")),
            );
        }
        let row = items(commands, card, kit);
        for (idx, (object, goods)) in relics.into_iter().enumerate() {
            let button = item(commands, row, kit, duel, object, goods, lang);
            commands.entity(button).insert(Stop::item(
                keys::ORDER.name,
                if is_own { "own" } else { "enemy" },
                u8::try_from(idx).unwrap_or(u8::MAX),
            ));
        }
        if is_own {
            for _ in count..6 {
                towerui::empty(commands, row, kit, 64.0, crate::hud::glyph::SHIELD);
            }
            let potions: Vec<(&baylee_view::PublicObject, Goods)> = objects
                .iter()
                .filter_map(|object| {
                    let goods @ Goods::Potion(_) = visible(object, reader, false)? else {
                        return None;
                    };
                    Some((object, goods))
                })
                .collect();
            if !potions.is_empty() {
                let pot_card = towerui::frame(commands, panel, kit);
                section_head(
                    commands,
                    pot_card,
                    kit,
                    super::super::glyph::HEART,
                    Color::srgb(0.92, 0.42, 0.46),
                    Phrase::EchoesRunPotions.text(lang),
                    None,
                );
                let pot_row = items(commands, pot_card, kit);
                for (object, goods) in potions {
                    let button = item(commands, pot_row, kit, duel, object, goods, lang);
                    commands.entity(button).insert(Stop::item(
                        keys::ORDER.name,
                        "potions",
                        u8::from(goods == Goods::Potion(Potion::Mana)),
                    ));
                }
            }
        }
    }
}
