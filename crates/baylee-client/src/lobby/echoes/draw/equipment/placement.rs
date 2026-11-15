#[allow(clippy::wildcard_imports)]
use super::*;

pub(in crate::lobby::echoes::draw) fn slots(
    commands: &mut Commands,
    root: Entity,
    kit: Kit,
    ui: &Ui,
) {
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

pub(in crate::lobby::echoes::draw) fn ingredients(
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
