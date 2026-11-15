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

mod detail;
mod options;
use detail::detail;
use options::options;
