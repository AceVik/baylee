#[allow(clippy::wildcard_imports)]
use super::*;

pub(super) fn draw(
    commands: &mut Commands,
    root: Entity,
    kit: Kit,
    ui: &Ui,
    run: &api::Run,
    cohort: &api::Cohort,
) {
    let title = row(commands, root, kit);
    heading(
        commands,
        title,
        kit,
        Phrase::EchoesRunWorkshop.text(lang(kit)),
    );
    shop::toolbar(commands, title, kit, ui, run);
    let market = commands
        .spawn((
            Node {
                width: percent(100),
                flex_direction: FlexDirection::Column,
                padding: UiRect::axes(kit.m.px(34.0), kit.m.px(30.0)),
                row_gap: kit.m.px(6.0),
                ..default()
            },
            BackgroundColor(Color::srgba(0.025, 0.016, 0.010, 0.82)),
            ChildOf(root),
            Pickable::IGNORE,
        ))
        .id();
    commands.spawn((
        towerui::Decor::Frame,
        Node {
            position_type: PositionType::Absolute,
            width: percent(100),
            height: percent(100),
            left: px(0),
            top: px(0),
            ..default()
        },
        ChildOf(market),
        Pickable::IGNORE,
    ));
    for row_offers in run.shop.offers.chunks(4) {
        let grid = grid(commands, market, kit, 4);
        for offer in row_offers {
            if offer.sold {
                let space = column(commands, grid, kit, 0.0, 1.0);
                commands
                    .entity(space)
                    .entry::<Node>()
                    .and_modify(move |mut node| {
                        node.height = kit.m.px(132.0);
                        node.justify_content = JustifyContent::Center;
                        node.align_items = AlignItems::Center;
                    });
                label(
                    commands,
                    space,
                    kit,
                    Phrase::EchoesRunSold.text(lang(kit)),
                    tokens::MUTED,
                );
                continue;
            }
            let emphasis = affinity(run, offer.goods);
            let price = offer.goods.price(&cohort.catalog).unwrap_or(0).to_string();
            let command = Command::Buy {
                offer: offer.id,
                delivery: domain::v2::shop::Delivery::Inventory,
            };
            object(
                commands,
                grid,
                kit,
                ui,
                offer.goods,
                Some(&price),
                106.0,
                emphasis,
                Action::Purchase(offer.id),
                can(ui, &command),
            );
        }
        commands.spawn((
            towerui::Decor::Shelf,
            Node {
                width: percent(100),
                height: kit.m.px(32.0),
                ..default()
            },
            ChildOf(market),
            Pickable::IGNORE,
        ));
    }
    label(
        commands,
        root,
        kit,
        Phrase::EchoesRunShelfHint.text(lang(kit)),
        tokens::MUTED,
    );
    footer(commands, root, kit, ui, run);
}

fn affinity(run: &api::Run, goods: Goods) -> Emphasis {
    match goods {
        Goods::Relic(definition) => match workshop::affinity(&run.economy.inventory, definition) {
            (_, Some(Rarity::Uncommon)) => Emphasis::Uncommon,
            (_, Some(Rarity::Rare)) => Emphasis::Rare,
            (_, Some(Rarity::MythicRare)) => Emphasis::Mythic,
            (0, _) => Emphasis::None,
            _ => Emphasis::Owned,
        },
        _ => Emphasis::None,
    }
}

fn footer(commands: &mut Commands, root: Entity, kit: Kit, ui: &Ui, run: &api::Run) {
    let footer = row(commands, root, kit);
    control(
        commands,
        footer,
        kit,
        ui,
        Phrase::EchoesDeck.text(lang(kit)),
        Action::Page(Page::Deck),
        true,
    );
    if run.phase == api::Phase::Shop {
        if ui.model.hosts().is_empty() {
            label(
                commands,
                footer,
                kit,
                Phrase::EchoesNoHost.text(lang(kit)),
                tokens::MUTED,
            );
        }
        for host in ui.model.hosts() {
            let title = if ui.model.hosts().len() == 1 {
                Phrase::EchoesPlay.text(lang(kit)).to_owned()
            } else {
                Phrase::EchoesPlayAt.fill(lang(kit), &[&host.name])
            };
            control(
                commands,
                footer,
                kit,
                ui,
                &title,
                Action::Play(host.id),
                ui.choices.deck_unchanged(Some(run)),
            );
        }
    } else if matches!(run.phase, api::Phase::Reserved | api::Phase::Playing) {
        control(
            commands,
            footer,
            kit,
            ui,
            Phrase::EchoesPlay.text(lang(kit)),
            Action::Resume,
            true,
        );
    }
}
