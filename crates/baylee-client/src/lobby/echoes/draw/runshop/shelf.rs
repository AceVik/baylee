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
    let top = row(commands, root, kit);
    landing::market_progress(commands, top, kit, ui);
    let spacer = crate::lobby::parts::grow(commands);
    commands.entity(top).add_child(spacer);
    control(
        commands,
        top,
        kit,
        ui,
        if run.shop.frozen {
            Phrase::EchoesShelfFrozen
        } else {
            Phrase::EchoesFreezeShelf
        }
        .text(lang(kit)),
        Action::Freeze,
        can(
            ui,
            &Command::Freeze {
                frozen: !run.shop.frozen,
            },
        ),
    );
    actions(commands, top, kit, ui, run);

    let market = commands
        .spawn((
            Node {
                display: Display::Grid,
                grid_template_columns: vec![RepeatedGridTrack::flex(
                    if run.shop.offers.len() > 8 { 6 } else { 4 },
                    1.0,
                )],
                width: percent(100),
                flex_grow: 1.0,
                row_gap: kit.m.px(8.0),
                column_gap: kit.m.px(12.0),
                padding: UiRect::all(kit.m.px(8.0)),
                border_radius: BorderRadius::all(kit.m.px(10.0)),
                ..default()
            },
            BackgroundColor(Color::srgb(0.035, 0.025, 0.019)),
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
    for offer in &run.shop.offers {
        offer_tile(commands, market, kit, ui, run, cohort, offer);
    }
    footer(commands, root, kit, ui, run);
}

#[allow(clippy::too_many_arguments)] // Render context plus the authoritative offer.
fn offer_tile(
    commands: &mut Commands,
    market: Entity,
    kit: Kit,
    ui: &Ui,
    run: &api::Run,
    cohort: &api::Cohort,
    offer: &domain::v2::shop::Offer,
) {
    let stand = commands
        .spawn((
            Node {
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::Center,
                justify_content: JustifyContent::End,
                height: kit.m.px(100.0),
                min_width: px(0),
                ..default()
            },
            ChildOf(market),
            Pickable::IGNORE,
        ))
        .id();
    if offer.sold {
        towerui::empty(commands, stand, kit, 68.0, glyph::CHECK);
    } else {
        let item = object(
            commands,
            stand,
            kit,
            ui,
            offer.goods,
            None,
            68.0,
            affinity(run, offer.goods),
            Action::Purchase(offer.id),
            can(
                ui,
                &Command::Buy {
                    offer: offer.id,
                    delivery: domain::v2::shop::Delivery::Inventory,
                },
            ),
        );
        commands
            .entity(item)
            .insert(super::super::super::gestures::Sale::new(run, offer.id));
    }
    // Every object has its own ledge. Sold slots retain exactly this geometry.
    let ledge = commands
        .spawn((
            Node {
                width: percent(100),
                height: kit.m.px(30.0),
                flex_shrink: 0.0,
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                ..default()
            },
            ChildOf(stand),
            Pickable::IGNORE,
        ))
        .id();
    commands.spawn((
        towerui::Decor::Shelf,
        Node {
            position_type: PositionType::Absolute,
            left: px(0),
            top: px(0),
            width: percent(100),
            height: percent(100),
            ..default()
        },
        ChildOf(ledge),
        Pickable::IGNORE,
    ));
    let price = towerui::coin(
        commands,
        ledge,
        kit,
        u32::from(offer.goods.price(&cohort.catalog).unwrap_or(0)),
    );
    if offer.sold {
        commands.entity(price).insert(Visibility::Hidden);
    }
}

fn footer(commands: &mut Commands, root: Entity, kit: Kit, ui: &Ui, run: &api::Run) {
    let footer = row(commands, root, kit);
    let wallet = towerui::coin(commands, footer, kit, u32::from(run.economy.gold));
    commands.entity(wallet).insert(Pickable::default());
    controls::attach_tooltip(commands, wallet, kit, Phrase::EchoesGold.text(lang(kit)));
    let spacer = crate::lobby::parts::grow(commands);
    commands.entity(footer).add_child(spacer);
    let price = run.shop.reroll_price().map_or_else(
        || {
            if run.shop.frozen {
                Phrase::EchoesShelfFrozen
            } else {
                Phrase::EchoesRefreshSpent
            }
            .text(lang(kit))
            .to_owned()
        },
        |price| Phrase::EchoesRefreshPrice.fill(lang(kit), &[&price.to_string()]),
    );
    control(
        commands,
        footer,
        kit,
        ui,
        &price,
        Action::Reroll,
        can(ui, &Command::Refresh {}),
    );
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

fn actions(commands: &mut Commands, footer: Entity, kit: Kit, ui: &Ui, run: &api::Run) {
    if run.phase == api::Phase::Shop {
        if ui.model.hosts().is_empty() {
            control(
                commands,
                footer,
                kit,
                ui,
                Phrase::EchoesPlay.text(lang(kit)),
                Action::Resume,
                false,
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
