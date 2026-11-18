#[allow(clippy::wildcard_imports)]
use super::*;

pub(super) fn detail(
    commands: &mut Commands,
    root: Entity,
    kit: Kit,
    ui: &Ui,
    run: &api::Run,
    cohort: &api::Cohort,
) {
    let lang = kit.lang();
    if let Some(offer) = run
        .shop
        .offers
        .iter()
        .find(|offer| Some(offer.id) == ui.choices.offer && !offer.sold)
    {
        let detail = panel(commands, root, kit);
        commands
            .entity(detail)
            .entry::<Node>()
            .and_modify(move |mut node| {
                node.padding = UiRect::all(kit.m.px(12.0));
            });
        commands
            .entity(detail)
            .insert(BorderColor::all(tokens::ACCENT));
        featured_portrait(commands, detail, kit, offer.goods);
        control(
            commands,
            detail,
            kit,
            ui,
            if offer.pinned {
                Phrase::EchoesUnpin
            } else {
                Phrase::EchoesPin
            }
            .text(lang),
            Action::Pin(offer.id),
            can(
                ui,
                &Command::Pin {
                    offer: offer.id,
                    pinned: !offer.pinned,
                },
            ),
        );
        widgets::effect(commands, detail, kit, offer.goods);
        if let Some(command) = ui.choices.purchase(run) {
            let actions = row(commands, detail, kit);
            if let Command::Buy { delivery, .. } = &command {
                label(
                    commands,
                    actions,
                    kit,
                    &words::destination(delivery, lang),
                    tokens::ACCENT,
                );
                let spacer = crate::lobby::parts::grow(commands);
                commands.entity(actions).add_child(spacer);
            }
            control(
                commands,
                actions,
                kit,
                ui,
                &offer.goods.price(&cohort.catalog).map_or_else(
                    |_| Phrase::EchoesBuy.text(lang).to_owned(),
                    |price| Phrase::EchoesBuyPrice.fill(lang, &[&price.to_string()]),
                ),
                Action::Buy,
                can(ui, &command),
            );
        }
        if let Goods::Card(id) = offer.goods {
            card::rules(commands, detail, kit, id);
        }
        if let Goods::Relic(definition) = offer.goods {
            tiers::draw(commands, detail, kit, ui, definition);
            equipment::slots(commands, detail, kit, ui);
            equipment::ingredients(commands, detail, kit, ui, run);
        }
    }
}
