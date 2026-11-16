#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_client_core::echoes::domain::v2::catalog::Potion;
use shellkit::focus::Stop;
use shellkit::{controls, px_fixed};

pub(super) fn draw(commands: &mut Commands, kit: Kit, duel: &Duel, canvas: Vec2, lang: Lang) {
    if !duel.tower_inventory {
        // The source is run equipment, not a card on the felt. Keep a real,
        // visible anchor after closing the inventory so the shared ability
        // chooser can place itself and show what is about to be consumed.
        if let Some(object) = duel.ability_menu
            && can_use(duel, object)
            && let Some(source) = duel.view.as_ref().and_then(|view| view.object(object))
            && let Some(goods) = crate::itemart::identity::run_source(source)
        {
            let anchor = towerui::inspectable(commands, goods, kit.fonts, lang, 64.0);
            commands.entity(anchor).insert((
                Root,
                super::super::sheet::AbilityAnchor(object),
                Node {
                    position_type: PositionType::Absolute,
                    right: px_fixed(24.0),
                    bottom: px_fixed(super::super::hand::HAND_ZONE_H + 64.0),
                    width: px_fixed(64.0),
                    height: px_fixed(64.0),
                    flex_shrink: 0.0,
                    ..default()
                },
                GlobalZIndex(super::super::Z_MENU),
            ));
        }
        return;
    }
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
    if duel.tower_inventory {
        let panel = commands
            .spawn((
                shellkit::Role::Opaque,
                ScrollPosition::default(),
                Node {
                    width: kit.m.px(448.0),
                    max_width: px_fixed(canvas.x - 32.0),
                    max_height: px_fixed(canvas.y - 100.0),
                    overflow: Overflow::scroll_y(),
                    flex_direction: FlexDirection::Column,
                    row_gap: kit.m.px(10.0),
                    padding: UiRect::all(kit.m.px(14.0)),
                    border: UiRect::all(px_fixed(1.0)),
                    border_radius: BorderRadius::all(kit.m.px(12.0)),
                    ..default()
                },
                BackgroundColor(tokens::OPAQUE),
                BorderColor::all(tokens::GOLD),
                Pickable::default(),
                ChildOf(root),
            ))
            .id();
        title(commands, panel, kit, lang);
        seats(commands, panel, kit, duel, lang);
    }
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

fn heading(commands: &mut Commands, parent: Entity, kit: Kit, name: &str) {
    let title = shellkit::surfaces::prose(commands, kit, name, false);
    commands
        .entity(title)
        .insert(crate::hud::tf_bold(kit.fonts, kit.m.text));
    commands.entity(parent).add_child(title);
}

fn title(commands: &mut Commands, parent: Entity, kit: Kit, lang: Lang) {
    let row = commands
        .spawn((
            Node {
                justify_content: JustifyContent::SpaceBetween,
                align_items: AlignItems::Center,
                ..default()
            },
            ChildOf(parent),
            Pickable::IGNORE,
        ))
        .id();
    heading(commands, row, kit, Phrase::EchoesRunTableItems.text(lang));
    let info = commands
        .spawn((
            Text::new(super::super::glyph::INFO.to_string()),
            super::super::icon_tf(kit.fonts, kit.m.small),
            TextColor(tokens::MUTED),
            ChildOf(row),
            Pickable::default(),
        ))
        .id();
    controls::attach_tooltip(commands, info, kit, Phrase::EchoesRunTableHint.text(lang));
    close(commands, row, kit, lang);
}

fn items(commands: &mut Commands, parent: Entity, kit: Kit) -> Entity {
    commands
        .spawn((
            Node {
                flex_wrap: FlexWrap::Wrap,
                column_gap: kit.m.px(5.0),
                row_gap: kit.m.px(5.0),
                ..default()
            },
            Pickable::IGNORE,
            ChildOf(parent),
        ))
        .id()
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
    let name = towerui::words::goods(goods, lang);
    let description = towerui::words::description(goods, lang);
    let count = object.counter_count(baylee_view::CounterKind::Charge);
    let caption = (count > 0 || matches!(goods, Goods::Potion(_))).then(|| format!("×{count}"));
    let button = towerui::button(
        commands,
        kit,
        towerui::Item {
            goods,
            name: &name,
            description: &description,
            caption: caption.as_deref(),
            size: 64.0,
            emphasis: crate::itemart::Emphasis::None,
        },
        Live::Yes,
        (),
    );
    if can_use(duel, object.id) {
        commands.entity(button).insert((
            UiButton,
            MenuButton {
                action: MenuAction::UseTowerItem(object.id),
            },
        ));
        commands.spawn((
            Text::new(super::super::glyph::ENERGY.to_string()),
            crate::hud::icon_tf(kit.fonts, kit.m.small),
            TextColor(tokens::GOLD),
            Node {
                position_type: PositionType::Absolute,
                top: kit.m.px(2.0),
                right: kit.m.px(2.0),
                ..default()
            },
            Pickable::IGNORE,
            ChildOf(button),
        ));
    }
    commands.entity(parent).add_child(button);
    button
}

fn seats(commands: &mut Commands, panel: Entity, kit: Kit, duel: &Duel, lang: Lang) {
    if let Some(view) = &duel.view {
        for (seat, objects) in view.command.iter().enumerate() {
            let player = baylee_core::ids::PlayerId::new(u8::try_from(seat).unwrap_or(u8::MAX));
            let name = duel
                .statics
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
                );
            heading(
                commands,
                panel,
                kit,
                if player == view.seat && !duel.watching {
                    Phrase::EchoesEquipped.text(lang)
                } else {
                    name
                },
            );
            let row = items(commands, panel, kit);
            let mut count = 0;
            for object in objects {
                if let Some(goods @ Goods::Relic(_)) = visible(object, view.seat, duel.watching) {
                    let button = item(commands, row, kit, duel, object, goods, lang);
                    commands.entity(button).insert(Stop::item(
                        keys::ORDER.name,
                        if player == view.seat { "own" } else { "enemy" },
                        count,
                    ));
                    count = count.saturating_add(1);
                }
            }
            for _ in count..6 {
                towerui::empty(commands, row, kit, 64.0, crate::hud::glyph::SHIELD);
            }
        }
        if !duel.watching {
            heading(commands, panel, kit, Phrase::EchoesRunPotions.text(lang));
            let row = items(commands, panel, kit);
            for object in view
                .command
                .get(usize::from(view.seat.get()))
                .into_iter()
                .flatten()
            {
                if let Some(goods @ Goods::Potion(_)) = visible(object, view.seat, false) {
                    let button = item(commands, row, kit, duel, object, goods, lang);
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
