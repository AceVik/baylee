#[allow(clippy::wildcard_imports)]
use super::*;

pub(super) fn draw(commands: &mut Commands, root: Entity, kit: Kit, ui: &Ui, run: &api::Run) {
    let dock = panel(commands, root, kit);
    commands
        .entity(dock)
        .entry::<Node>()
        .and_modify(move |mut node| {
            node.padding = UiRect::all(kit.m.px(10.0));
            node.row_gap = kit.m.px(6.0);
        });
    let groups = row(commands, dock, kit);
    commands
        .entity(groups)
        .entry::<Node>()
        .and_modify(|mut node| {
            node.flex_wrap = FlexWrap::NoWrap;
            node.align_items = AlignItems::Start;
        });
    for (active, count, title) in [
        (true, 3, Phrase::EchoesEquipped),
        (false, 6, Phrase::EchoesReserve),
    ] {
        let group = commands
            .spawn((
                Node {
                    flex_direction: FlexDirection::Column,
                    flex_basis: px_fixed(0.0),
                    flex_grow: if active { 1.0 } else { 2.0 },
                    min_width: px_fixed(0.0),
                    row_gap: kit.m.px(6.0),
                    ..default()
                },
                Pickable::IGNORE,
            ))
            .id();
        commands.entity(groups).add_child(group);
        let title =
            merchandise::caption(commands, group, kit, title.text(lang(kit)), tokens::MUTED);
        commands
            .entity(title)
            .insert(TextLayout::justify(Justify::Left));
        let grid = merchandise::grid(commands, group, kit, count);
        for index in 0..u8::try_from(count).unwrap_or(0) {
            let slot = if active {
                Slot::Active(index)
            } else {
                Slot::Reserve(index)
            };
            let relic = run
                .economy
                .inventory
                .relics
                .iter()
                .find(|item| item.slot == slot);
            let face = if let Some(item) = relic {
                merchandise::surface(commands, grid, kit, ui, Action::Select(item.id))
            } else {
                empty(commands, grid, kit, ui.choices.slot == slot)
            };
            if let Some(relic) = relic {
                merchandise::image(commands, face, kit, Goods::Relic(relic.definition), 32.0);
            } else {
                let empty = commands
                    .spawn((
                        Node {
                            width: percent(100),
                            height: kit.m.px(32.0),
                            align_items: AlignItems::Center,
                            ..default()
                        },
                        Pickable::IGNORE,
                    ))
                    .id();
                commands.entity(face).add_child(empty);
                merchandise::caption(commands, empty, kit, "—", tokens::MUTED);
            }
            merchandise::caption(commands, face, kit, &(index + 1).to_string(), tokens::MUTED);
        }
    }
    charges(commands, dock, kit, run);
}

fn charges(commands: &mut Commands, root: Entity, kit: Kit, run: &api::Run) {
    let charges = row(commands, root, kit);
    for (goods, count) in [
        (
            Goods::Potion(Potion::Healing),
            usize::from(run.economy.potions[0]),
        ),
        (
            Goods::Potion(Potion::Mana),
            usize::from(run.economy.potions[1]),
        ),
        (Goods::Mythic, run.economy.inventory.materials.len()),
    ] {
        merchandise::image(commands, charges, kit, goods, 24.0);
        label(
            commands,
            charges,
            kit,
            &format!("{} × {count}", words::goods(goods, lang(kit))),
            tokens::MUTED,
        );
    }
}

fn empty(commands: &mut Commands, root: Entity, kit: Kit, selected: bool) -> Entity {
    let face = commands
        .spawn((
            Node {
                width: percent(100),
                min_width: px_fixed(0.0),
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::Center,
                row_gap: kit.m.px(4.0),
                padding: UiRect::all(kit.m.px(6.0)),
                border: UiRect::all(px_fixed(1.0)),
                border_radius: BorderRadius::all(kit.m.px(8.0)),
                ..default()
            },
            BackgroundColor(tokens::CONTROL),
            BorderColor::all(if selected {
                tokens::ACCENT
            } else {
                tokens::BORDER
            }),
            surfaces::elevation_control_disabled(),
            Pickable::IGNORE,
        ))
        .id();
    commands.entity(root).add_child(face);
    face
}
