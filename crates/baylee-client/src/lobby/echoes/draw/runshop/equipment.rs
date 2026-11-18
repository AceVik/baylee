#[allow(clippy::wildcard_imports)]
use super::*;

pub(super) fn draw(commands: &mut Commands, root: Entity, kit: Kit, ui: &Ui, run: &api::Run) {
    let lang = kit.lang();
    let top = row(commands, root, kit);
    help_heading(
        commands,
        top,
        kit,
        Phrase::EchoesEquipped,
        Phrase::EchoesRunEquipmentHint,
    );
    let space = crate::lobby::parts::grow(commands);
    commands.entity(top).add_child(space);

    let deck_total: u16 = run
        .cards
        .deck
        .iter()
        .map(|entry| u16::from(entry.copies))
        .sum::<u16>()
        + u16::from(run.cards.formation.is_some());
    icon_control(
        commands,
        top,
        kit,
        ui,
        glyph::LIBRARY,
        Some(&deck_total.to_string()),
        IconTone::Active,
        &Phrase::EchoesDeckButtonTip.fill(lang, &[&deck_total.to_string()]),
        Action::Page(Page::Deck),
        run.phase == api::Phase::Shop,
        None,
    );

    let equipment = grid(commands, root, kit, 3);
    for slot in 0..6 {
        let target = Slot::Active(slot);
        let item = run
            .economy
            .inventory
            .relics
            .iter()
            .find(|item| item.slot == target);
        let cell = if let Some(item) = item {
            let cell = object(
                commands,
                equipment,
                kit,
                ui,
                Goods::Relic(item.definition),
                None,
                Some(Phrase::EchoesRunEquipmentHint.text(lang)),
                62.0,
                Emphasis::None,
                ui.choices.item == Some(item.id),
                Action::Select(item.id),
                true,
                None,
            );
            commands.entity(cell).insert((
                super::super::super::gestures::Owned::new(run, item.id),
                Place::new(run, Anchor::Item(item.id)),
            ));
            cell
        } else {
            sleek_empty(commands, equipment, kit, 62.0, glyph::SHIELD)
        };
        commands
            .entity(cell)
            .insert(super::super::super::gestures::Destination::new(run, target));
    }

    supplies(commands, root, kit, ui, run);

    let space = crate::lobby::parts::grow(commands);
    commands.entity(root).add_child(space);
    let footer = row(commands, root, kit);
    let lead_space = crate::lobby::parts::grow(commands);
    commands.entity(footer).add_child(lead_space);
    icon_control(
        commands,
        footer,
        kit,
        ui,
        words::GLYPH_FLAG,
        None,
        IconTone::Danger,
        Phrase::EchoesRetireTip.text(lang),
        Action::Abandon,
        can(ui, &Command::Abandon {}),
        None,
    );
}

fn supplies(commands: &mut Commands, root: Entity, kit: Kit, ui: &Ui, run: &api::Run) {
    let lang = kit.lang();
    help_heading(
        commands,
        root,
        kit,
        Phrase::EchoesRunPotions,
        Phrase::EchoesRunPotionsHint,
    );
    let potions = grid(commands, root, kit, 3);
    commands
        .entity(potions)
        .insert(Place::new(run, Anchor::Supplies));
    for (goods, count, action) in [
        (
            Goods::Potion(Potion::Healing),
            usize::from(run.economy.potions[0]),
            Action::Place(Slot::Inventory),
        ),
        (
            Goods::Potion(Potion::Mana),
            usize::from(run.economy.potions[1]),
            Action::Place(Slot::Inventory),
        ),
        (
            Goods::Mythic,
            run.economy.inventory.materials.len(),
            Action::Material(
                run.economy
                    .inventory
                    .materials
                    .first()
                    .map_or(0, |material| material.id),
            ),
        ),
    ] {
        let is_mythic = matches!(goods, Goods::Mythic);
        object(
            commands,
            potions,
            kit,
            ui,
            goods,
            Some(&format!("×{count}")),
            None,
            50.0,
            Emphasis::None,
            is_mythic && count > 0 && ui.choices.material.is_some(),
            action,
            is_mythic && count > 0,
            if is_mythic {
                None
            } else {
                Some(Phrase::EchoesRunPotionsHint.text(lang))
            },
        );
    }
}
