#[allow(clippy::wildcard_imports)]
use super::*;

pub(in crate::lobby::echoes::draw) fn slots(
    commands: &mut Commands,
    root: Entity,
    kit: Kit,
    ui: &Ui,
) {
    if ui.page == Page::Shop {
        compact(commands, root, kit, ui);
        return;
    }
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

fn compact(commands: &mut Commands, root: Entity, kit: Kit, ui: &Ui) {
    for (active, count, title) in [
        (true, ACTIVE_SLOTS, Phrase::EchoesActiveSlots),
        (false, RESERVE_SLOTS, Phrase::EchoesReserveSlots),
    ] {
        label(commands, root, kit, title.text(lang(kit)), tokens::MUTED);
        let grid = merchandise::grid(commands, root, kit, 3);
        for index in 0..count {
            let slot = if active {
                Slot::Active(index)
            } else {
                Slot::Reserve(index)
            };
            let face = merchandise::surface(commands, grid, kit, ui, Action::Place(slot));
            let occupant = ui.model.run().and_then(|run| {
                run.economy
                    .inventory
                    .relics
                    .iter()
                    .find(|relic| relic.slot == slot)
            });
            if let Some(relic) = occupant {
                merchandise::image(commands, face, kit, Goods::Relic(relic.definition), 24.0);
            }
            let number =
                merchandise::caption(commands, face, kit, &(index + 1).to_string(), tokens::INK);
            commands
                .entity(number)
                .entry::<Node>()
                .and_modify(|mut node| node.width = Val::Auto);
            if ui.choices.slot == slot {
                let mark = commands
                    .spawn((
                        Text::new(glyph::CHECK.to_string()),
                        crate::hud::icon_tf(kit.fonts, kit.m.small),
                        TextColor(tokens::INK),
                        Pickable::IGNORE,
                    ))
                    .id();
                commands.entity(face).add_child(mark);
            }
            commands
                .entity(face)
                .entry::<Node>()
                .and_modify(move |mut node| {
                    node.min_height = px_fixed(kit.m.control);
                    node.justify_content = JustifyContent::Center;
                    node.flex_direction = FlexDirection::Row;
                    node.column_gap = kit.m.px(4.0);
                });
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
