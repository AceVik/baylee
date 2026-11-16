#[allow(clippy::wildcard_imports)]
use super::*;

pub(super) fn draw(commands: &mut Commands, root: Entity, kit: Kit, ui: &Ui, run: &api::Run) {
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
    let deck = crate::lobby::parts::icon_button(
        commands,
        kit,
        glyph::LIBRARY,
        Press::Echoes(Action::Page(Page::Deck)),
    );
    commands
        .entity(deck)
        .insert(super::super::super::keys::stop(
            Action::Page(Page::Deck),
            ui,
        ));
    controls::attach_tooltip(commands, deck, kit, Phrase::EchoesDeck.text(lang(kit)));
    commands.entity(top).add_child(deck);
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
                60.0,
                Emphasis::None,
                Action::Select(item.id),
                true,
            );
            commands
                .entity(cell)
                .insert(super::super::super::gestures::Owned::new(run, item.id));
            cell
        } else {
            towerui::empty(commands, equipment, kit, 60.0, glyph::SHIELD)
        };
        commands
            .entity(cell)
            .insert(super::super::super::gestures::Destination::new(run, target));
    }
    supplies(commands, root, kit, ui, run);
    let space = crate::lobby::parts::grow(commands);
    commands.entity(root).add_child(space);
    let footer = row(commands, root, kit);
    control(
        commands,
        footer,
        kit,
        ui,
        Phrase::TowerRetire.text(lang(kit)),
        Action::Abandon,
        can(ui, &Command::Abandon {}),
    );
}

fn supplies(commands: &mut Commands, root: Entity, kit: Kit, ui: &Ui, run: &api::Run) {
    help_heading(
        commands,
        root,
        kit,
        Phrase::EchoesRunPotions,
        Phrase::EchoesRunPotionsHint,
    );
    let potions = grid(commands, root, kit, 3);
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
        object(
            commands,
            potions,
            kit,
            ui,
            goods,
            Some(&format!("×{count}")),
            48.0,
            Emphasis::None,
            action,
            matches!(goods, Goods::Mythic) && count > 0,
        );
    }
}
