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
    let lang = lang(kit);
    let grid = merchandise::grid(commands, root, kit, 4);
    for offer in &run.shop.offers {
        let cell = commands
            .spawn((
                Node {
                    flex_direction: FlexDirection::Column,
                    min_width: px_fixed(0.0),
                    ..default()
                },
                Pickable::IGNORE,
            ))
            .id();
        commands.entity(grid).add_child(cell);
        let face = merchandise::surface(commands, cell, kit, ui, Action::Offer(offer.id));
        merchandise::image(commands, face, kit, offer.goods, 56.0);
        let name = merchandise::caption(
            commands,
            face,
            kit,
            &words::name(offer.goods, lang),
            if offer.sold {
                tokens::MUTED
            } else {
                tokens::INK
            },
        );
        commands
            .entity(name)
            .entry::<Node>()
            .and_modify(move |mut node| {
                node.min_height = kit.m.px(40.0);
                node.flex_grow = 1.0;
            });
        price(commands, face, kit, offer, cohort);
        markers(commands, face, kit, offer, run);
    }
}

fn price(
    commands: &mut Commands,
    root: Entity,
    kit: Kit,
    offer: &domain::v2::shop::Offer,
    cohort: &api::Cohort,
) {
    let band = row(commands, root, kit);
    commands
        .entity(band)
        .entry::<Node>()
        .and_modify(move |mut node| {
            node.justify_content = JustifyContent::SpaceBetween;
            node.column_gap = kit.m.px(4.0);
        });
    if let Goods::Relic(definition) = offer.goods
        && !offer.sold
    {
        label(
            commands,
            band,
            kit,
            words::rarity(definition.rarity),
            tokens::MUTED,
        );
    }
    if offer.sold {
        let badge = crate::lobby::parts::badge_with(
            commands,
            kit,
            glyph::CHECK,
            Phrase::EchoesSold.text(lang(kit)),
            tokens::MUTED,
        );
        commands.entity(band).add_child(badge);
    } else {
        let price = offer
            .goods
            .price(&cohort.catalog)
            .map_or_else(|_| "—".into(), |price| price.to_string());
        let badge =
            crate::lobby::parts::badge_with(commands, kit, glyph::SUN, &price, tokens::GOLD);
        commands.entity(band).add_child(badge);
    }
}

fn markers(
    commands: &mut Commands,
    root: Entity,
    kit: Kit,
    offer: &domain::v2::shop::Offer,
    run: &api::Run,
) {
    let marks = row(commands, root, kit);
    commands
        .entity(marks)
        .entry::<Node>()
        .and_modify(move |mut node| {
            node.position_type = PositionType::Absolute;
            node.top = kit.m.px(4.0);
            node.left = kit.m.px(4.0);
            node.right = kit.m.px(4.0);
            node.width = Val::Auto;
            node.justify_content = JustifyContent::SpaceBetween;
        });
    if let Goods::Relic(definition) = offer.goods {
        let copies = run
            .economy
            .inventory
            .relics
            .iter()
            .filter(|item| item.definition == definition)
            .count();
        if copies > 0 {
            let badge =
                crate::lobby::parts::badge(commands, kit, &format!("×{copies}"), tokens::ACCENT);
            commands.entity(marks).add_child(badge);
        }
    }
    if offer.pinned {
        let badge = commands
            .spawn((
                Text::new(glyph::CLOCK.to_string()),
                crate::hud::icon_tf(kit.fonts, kit.m.small),
                TextColor(tokens::ACCENT),
                Pickable::IGNORE,
            ))
            .id();
        commands.entity(marks).add_child(badge);
    }
}
