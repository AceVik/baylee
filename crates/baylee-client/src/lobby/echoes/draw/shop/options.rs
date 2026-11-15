#[allow(clippy::wildcard_imports)]
use super::*;

pub(in crate::lobby::echoes::draw) fn toolbar(
    commands: &mut Commands,
    root: Entity,
    kit: Kit,
    ui: &Ui,
    run: &api::Run,
) {
    let lang = lang(kit);
    let footer = root;
    let spacer = crate::lobby::parts::grow(commands);
    commands.entity(footer).add_child(spacer);
    if run.shop.generator == domain::v2::generation::RUN {
        control(
            commands,
            footer,
            kit,
            ui,
            if run.shop.frozen {
                Phrase::EchoesShelfFrozen
            } else {
                Phrase::EchoesFreezeShelf
            }
            .text(lang),
            Action::Freeze,
            can(
                ui,
                &Command::Freeze {
                    frozen: !run.shop.frozen,
                },
            ),
        );
    }
    let text = run.shop.reroll_price().map_or_else(
        || {
            if run.shop.frozen {
                Phrase::EchoesShelfFrozen
            } else {
                Phrase::EchoesRefreshSpent
            }
            .text(lang)
            .to_string()
        },
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
}

pub(super) fn options(commands: &mut Commands, root: Entity, kit: Kit, ui: &Ui, run: &api::Run) {
    let lang = lang(kit);
    let footer = row(commands, root, kit);
    control(
        commands,
        footer,
        kit,
        ui,
        Phrase::EchoesRuns.text(lang),
        Action::Overview,
        ui.choices.deck_unchanged(ui.model.run()),
    );
    control(
        commands,
        footer,
        kit,
        ui,
        Phrase::Refresh.text(lang),
        Action::Refresh,
        true,
    );
    let spacer = crate::lobby::parts::grow(commands);
    commands.entity(footer).add_child(spacer);
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
