#[allow(clippy::wildcard_imports)]
use super::*;
use shellkit::{controls, px_fixed};

pub(super) fn draw(commands: &mut Commands, kit: Kit, duel: &Duel, canvas: Vec2, lang: Lang) {
    let root = commands
        .spawn((
            Root,
            Node {
                position_type: PositionType::Absolute,
                right: px_fixed(16.0),
                bottom: px_fixed(16.0),
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
                    width: kit.m.px(480.0),
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
        heading(commands, panel, kit, Phrase::EchoesRunTableItems.text(lang));
        seats(commands, panel, kit, duel, lang);
        let hint =
            shellkit::surfaces::prose(commands, kit, Phrase::EchoesRunTableHint.text(lang), true);
        commands.entity(panel).add_child(hint);
    }
    let toggle = controls::button_with_icon(
        commands,
        kit,
        super::super::glyph::GEAR,
        if duel.tower_inventory {
            Phrase::ShellClose
        } else {
            Phrase::EchoesRunTableItems
        }
        .text(lang),
        Weight::Secondary,
        Live::Yes,
        None,
        (
            UiButton,
            MenuButton {
                action: MenuAction::ToggleTowerItems,
            },
        ),
    );
    commands.entity(root).add_child(toggle);
}

fn heading(commands: &mut Commands, parent: Entity, kit: Kit, name: &str) {
    let title = shellkit::surfaces::prose(commands, kit, name, false);
    commands
        .entity(title)
        .insert(crate::hud::tf_bold(kit.fonts, kit.m.text));
    commands.entity(parent).add_child(title);
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
) {
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
                    |entry| entry.display_name.as_str(),
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
            for object in objects {
                if let Some(goods @ Goods::Relic(_)) = visible(object, view.seat, duel.watching) {
                    item(commands, row, kit, duel, object, goods, lang);
                }
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
                    item(commands, row, kit, duel, object, goods, lang);
                }
            }
        }
    }
}
