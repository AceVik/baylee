#[allow(clippy::wildcard_imports)]
use super::*;
use domain::v2::catalog::{ACTIVE_SLOTS, RESERVE_SLOTS};

pub(super) fn draw(
    commands: &mut Commands,
    root: Entity,
    kit: Kit,
    ui: &Ui,
    run: &api::Run,
    _cohort: &api::Cohort,
) {
    attributes(commands, root, kit, run);
    inventory(commands, root, kit, ui, run);
}

fn attributes(commands: &mut Commands, root: Entity, kit: Kit, run: &api::Run) {
    let lang = lang(kit);
    let stats = panel(commands, root, kit);
    heading(commands, stats, kit, Phrase::EchoesEquipment.text(lang));
    if let Ok(stats_value) = domain::v2::catalog::attributes(&run.economy.inventory.active()) {
        let band = row(commands, stats, kit);
        for (icon, phrase, value) in [
            (
                glyph::HEART,
                Phrase::EchoesStartingLife,
                stats_value.life.to_string(),
            ),
            (
                glyph::HAND,
                Phrase::EchoesOpeningCards,
                stats_value.cards.to_string(),
            ),
            (
                glyph::LIBRARY,
                Phrase::EchoesHandLimit,
                stats_value.maximum_hand.to_string(),
            ),
            (
                glyph::REFRESH,
                Phrase::EchoesMulligans,
                stats_value.free_mulligans.to_string(),
            ),
        ] {
            presentation::metric(commands, band, kit, icon, phrase.text(lang), &value);
        }
    }
    label(
        commands,
        stats,
        kit,
        &Phrase::EchoesCharges.fill(
            lang,
            &[
                &run.economy.potions[0].to_string(),
                &run.economy.potions[1].to_string(),
            ],
        ),
        tokens::MUTED,
    );
}

fn inventory(commands: &mut Commands, root: Entity, kit: Kit, ui: &Ui, run: &api::Run) {
    let lang = lang(kit);
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
    let detail = panel(commands, root, kit);
    label(
        commands,
        detail,
        kit,
        Phrase::EchoesSlotRules.text(lang),
        tokens::MUTED,
    );
    slots(commands, detail, kit, ui);
    if let Some(id) = ui.choices.item {
        control(
            commands,
            detail,
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
            detail,
            kit,
            ui,
            &Phrase::EchoesSellPrice.fill(lang, &[&refund.to_string()]),
            Action::Sell,
            can(ui, &Command::Sell { item: id }),
        );
    }
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

pub(super) fn slots(commands: &mut Commands, root: Entity, kit: Kit, ui: &Ui) {
    let lang = lang(kit);
    for (active, count, title) in [
        (true, ACTIVE_SLOTS, Phrase::EchoesActiveSlots),
        (false, RESERVE_SLOTS, Phrase::EchoesReserveSlots),
    ] {
        label(commands, root, kit, title.text(lang), tokens::MUTED);
        let band = row(commands, root, kit);
        for index in 0..count {
            let slot = if active {
                Slot::Active(index)
            } else {
                Slot::Reserve(index)
            };
            let cell = commands
                .spawn((
                    Node {
                        flex_direction: FlexDirection::Column,
                        flex_basis: percent(30),
                        flex_grow: 1.0,
                        min_width: px(0),
                        row_gap: kit.m.px(4.0),
                        ..default()
                    },
                    Pickable::IGNORE,
                ))
                .id();
            commands.entity(band).add_child(cell);
            control(
                commands,
                cell,
                kit,
                ui,
                &words::slot(slot, lang),
                Action::Place(slot),
                true,
            );
            let occupant = ui.model.run().and_then(|run| {
                run.economy
                    .inventory
                    .relics
                    .iter()
                    .find(|relic| relic.slot == slot)
            });
            let description = occupant.map_or_else(
                || Phrase::EchoesSlotEmpty.text(lang).to_owned(),
                |relic| words::goods(Goods::Relic(relic.definition), lang),
            );
            label(commands, cell, kit, &description, tokens::MUTED);
        }
    }
}

pub(super) fn ingredients(
    commands: &mut Commands,
    root: Entity,
    kit: Kit,
    ui: &Ui,
    run: &api::Run,
) {
    let lang = lang(kit);
    if run.economy.inventory.relics.is_empty() && run.economy.inventory.materials.is_empty() {
        return;
    }
    label(
        commands,
        root,
        kit,
        Phrase::EchoesFusionRules.text(lang),
        tokens::MUTED,
    );
    let band = row(commands, root, kit);
    for relic in &run.economy.inventory.relics {
        let selected = ui.choices.ingredients.contains(&relic.id);
        let text = format!(
            "{} · {} · {}",
            if selected { "✓" } else { "+" },
            words::goods(Goods::Relic(relic.definition), lang),
            words::slot(relic.slot, lang)
        );
        control(
            commands,
            band,
            kit,
            ui,
            &text,
            Action::Ingredient(relic.id),
            selected || ui.choices.ingredients.len() < 3,
        );
    }
    for material in &run.economy.inventory.materials {
        let text = format!(
            "{} · {}",
            if ui.choices.material == Some(material.id) {
                "✓"
            } else {
                "+"
            },
            Phrase::EchoesMythic.text(lang)
        );
        control(
            commands,
            band,
            kit,
            ui,
            &text,
            Action::Material(material.id),
            true,
        );
    }
}
