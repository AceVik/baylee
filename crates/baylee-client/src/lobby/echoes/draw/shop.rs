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
    detail(commands, root, kit, ui, run, cohort);
    let offers = row(commands, root, kit);
    for offer in &run.shop.offers {
        let card = tile(commands, offers, kit);
        portrait(commands, card, kit, offer.goods);
        let price = offer.goods.price(&cohort.catalog).ok();
        let price_text = price.map_or_else(
            || "—".into(),
            |price| Phrase::EchoesPrice.fill(lang, &[&price.to_string()]),
        );
        label(
            commands,
            card,
            kit,
            if offer.sold {
                Phrase::EchoesSold.text(lang)
            } else {
                &price_text
            },
            tokens::GOLD,
        );
        if ui.choices.offer == Some(offer.id) {
            commands
                .entity(card)
                .insert(BorderColor::all(tokens::ACCENT));
        }
        let band = row(commands, card, kit);
        control(
            commands,
            band,
            kit,
            ui,
            Phrase::EchoesInspect.text(lang),
            Action::Offer(offer.id),
            !offer.sold && run.phase == api::Phase::Shop,
        );
        control(
            commands,
            band,
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
    }
    options(commands, root, kit, ui, run);
}

fn detail(
    commands: &mut Commands,
    root: Entity,
    kit: Kit,
    ui: &Ui,
    run: &api::Run,
    cohort: &api::Cohort,
) {
    let lang = lang(kit);
    if let Some(offer) = run
        .shop
        .offers
        .iter()
        .find(|offer| Some(offer.id) == ui.choices.offer && !offer.sold)
    {
        let detail = panel(commands, root, kit);
        commands
            .entity(detail)
            .insert(BorderColor::all(tokens::ACCENT));
        portrait(commands, detail, kit, offer.goods);
        label(
            commands,
            detail,
            kit,
            &words::effect(offer.goods, lang),
            tokens::INK,
        );
        if matches!(offer.goods, Goods::Relic(_)) {
            equipment::slots(commands, detail, kit, ui);
            equipment::ingredients(commands, detail, kit, ui, run);
        }
        if let Some(command) = ui.choices.purchase(run) {
            control(
                commands,
                detail,
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
    }
}

fn options(commands: &mut Commands, root: Entity, kit: Kit, ui: &Ui, run: &api::Run) {
    let lang = lang(kit);
    let footer = panel(commands, root, kit);
    label(
        commands,
        footer,
        kit,
        Phrase::EchoesShopRules.text(lang),
        tokens::MUTED,
    );
    let text = run.shop.reroll_price().map_or_else(
        || Phrase::EchoesRefreshSpent.text(lang).to_string(),
        |price| Phrase::EchoesRefreshPrice.fill(lang, &[&price.to_string()]),
    );
    control(
        commands,
        footer,
        kit,
        ui,
        &text,
        Action::Reroll,
        can(ui, &Command::Refresh {}),
    );
    if run.shop.mythic_choice {
        label(
            commands,
            footer,
            kit,
            Phrase::EchoesMythicOffer.text(lang),
            tokens::GOLD,
        );
        for slot in [6, 7] {
            control(
                commands,
                footer,
                kit,
                ui,
                &Phrase::EchoesReplaceOffer.fill(lang, &[&(slot + 1).to_string()]),
                Action::Mythic(slot),
                can(
                    ui,
                    &Command::ChooseMythic {
                        slot,
                        replace_pinned: true,
                    },
                ),
            );
        }
    }
    control(
        commands,
        footer,
        kit,
        ui,
        Phrase::TowerRetire.text(lang),
        Action::Abandon,
        can(ui, &Command::Abandon {}),
    );
}
