#[allow(clippy::wildcard_imports)]
use super::*;

pub(super) fn draw(commands: &mut Commands, root: Entity, kit: Kit, ui: &Ui, run: &api::Run) {
    help_heading(
        commands,
        root,
        kit,
        Phrase::EchoesEquipped,
        Phrase::EchoesRunEquipmentHint,
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
                68.0,
                Emphasis::None,
                Action::Select(item.id),
                true,
            );
            commands
                .entity(cell)
                .insert(super::super::super::gestures::Owned::new(run, item.id));
            cell
        } else {
            let cell = commands
                .spawn((
                    Node {
                        width: kit.m.px(68.0),
                        height: kit.m.px(68.0),
                        align_items: AlignItems::Center,
                        justify_content: JustifyContent::Center,
                        border: UiRect::all(px_fixed(1.0)),
                        border_radius: BorderRadius::all(kit.m.px(10.0)),
                        ..default()
                    },
                    BorderColor::all(tokens::BORDER),
                    BackgroundColor(tokens::CONTROL),
                    Pickable::default(),
                    ChildOf(equipment),
                ))
                .id();
            label(commands, cell, kit, &(slot + 1).to_string(), tokens::MUTED);
            cell
        };
        commands
            .entity(cell)
            .insert(super::super::super::gestures::Destination::new(run, target));
    }
    supplies(commands, root, kit, ui, run);
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
            56.0,
            Emphasis::None,
            action,
            matches!(goods, Goods::Mythic) && count > 0,
        );
    }
}
