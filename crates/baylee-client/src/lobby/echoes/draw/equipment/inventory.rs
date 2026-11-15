#[allow(clippy::wildcard_imports)]
use super::*;

pub(super) fn inventory(commands: &mut Commands, root: Entity, kit: Kit, ui: &Ui, run: &api::Run) {
    let lang = lang(kit);
    if ui.choices.item.is_some() {
        detail(commands, root, kit, ui, run);
    } else {
        let hint = panel(commands, root, kit);
        label(
            commands,
            hint,
            kit,
            if run.economy.inventory.relics.is_empty() && run.economy.inventory.materials.is_empty()
            {
                Phrase::EchoesEmptyEquipment.text(lang)
            } else {
                Phrase::EchoesSelectEquipment.text(lang)
            },
            tokens::MUTED,
        );
        control(
            commands,
            hint,
            kit,
            ui,
            Phrase::EchoesShop.text(lang),
            Action::GoShop,
            true,
        );
    }
    let band = row(commands, root, kit);
    for relic in &run.economy.inventory.relics {
        let card = tile(commands, band, kit);
        portrait(commands, card, kit, Goods::Relic(relic.definition));
        label(
            commands,
            card,
            kit,
            &words::slot(relic.slot, lang),
            tokens::ACCENT,
        );
        label(
            commands,
            card,
            kit,
            &words::effect(Goods::Relic(relic.definition), lang),
            tokens::MUTED,
        );
        control(
            commands,
            card,
            kit,
            ui,
            Phrase::EchoesInspect.text(lang),
            Action::Select(relic.id),
            run.phase == api::Phase::Shop,
        );
    }
    for material in &run.economy.inventory.materials {
        control(
            commands,
            root,
            kit,
            ui,
            &format!("{} · {}", Phrase::EchoesMythic.text(lang), material.id),
            Action::Select(material.id),
            run.phase == api::Phase::Shop,
        );
    }
}

pub(in crate::lobby::echoes::draw) fn detail(
    commands: &mut Commands,
    root: Entity,
    kit: Kit,
    ui: &Ui,
    run: &api::Run,
) {
    let lang = lang(kit);
    let detail = panel(commands, root, kit);
    if let Some(relic) = run
        .economy
        .inventory
        .relics
        .iter()
        .find(|relic| Some(relic.id) == ui.choices.item)
    {
        featured_portrait(commands, detail, kit, Goods::Relic(relic.definition));
        label(
            commands,
            detail,
            kit,
            &words::slot(relic.slot, lang),
            tokens::MUTED,
        );
        widgets::effect(commands, detail, kit, Goods::Relic(relic.definition));
    }
    label(
        commands,
        detail,
        kit,
        &Phrase::EchoesDestination.fill(lang, &[&words::slot(ui.choices.slot, lang)]),
        tokens::ACCENT,
    );
    let actions = row(commands, detail, kit);
    if let Some(id) = ui.choices.item {
        control(
            commands,
            actions,
            kit,
            ui,
            Phrase::EchoesMove.text(lang),
            Action::Move,
            can(
                ui,
                &Command::Move {
                    item: id,
                    slot: ui.choices.slot,
                },
            ),
        );
        let refund = baylee_client_core::echoes::draft::refund(run, id).unwrap_or(0);
        control(
            commands,
            actions,
            kit,
            ui,
            &Phrase::EchoesSellPrice.fill(lang, &[&refund.to_string()]),
            Action::Sell,
            can(ui, &Command::Sell { item: id }),
        );
    }
    slots(commands, detail, kit, ui);
    label(
        commands,
        detail,
        kit,
        Phrase::EchoesSlotRules.text(lang),
        tokens::MUTED,
    );
    ingredients(commands, detail, kit, ui, run);
    control(
        commands,
        detail,
        kit,
        ui,
        Phrase::EchoesFuse.text(lang),
        Action::Fuse,
        can(
            ui,
            &Command::Fuse {
                recipe: ui.choices.recipe(),
            },
        ),
    );
}
