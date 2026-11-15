#[allow(clippy::wildcard_imports)]
use super::*;

pub(super) fn options(commands: &mut Commands, root: Entity, kit: Kit, ui: &Ui, run: &api::Run) {
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
