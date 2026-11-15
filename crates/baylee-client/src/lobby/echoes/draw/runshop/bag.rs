#[allow(clippy::wildcard_imports)]
use super::*;

pub(super) fn draw(commands: &mut Commands, root: Entity, kit: Kit, ui: &Ui, run: &api::Run) {
    let bag = panel(commands, root, kit);
    commands.entity(bag).insert((
        super::super::super::gestures::Destination::new(run, Slot::Inventory),
        Pickable::default(),
    ));
    sorting(commands, bag, kit, ui);
    contents(commands, bag, kit, ui, run);
    selected(commands, bag, kit, ui, run);
}

fn sorting(commands: &mut Commands, bag: Entity, kit: Kit, ui: &Ui) {
    let top = row(commands, bag, kit);
    heading(commands, top, kit, Phrase::EchoesInventory.text(lang(kit)));
    for (order, title) in [
        (workshop::Order::Newest, Phrase::EchoesRunNewest),
        (workshop::Order::Family, Phrase::EchoesRunFamily),
        (workshop::Order::Rarity, Phrase::EchoesRunRarity),
    ] {
        control(
            commands,
            top,
            kit,
            ui,
            title.text(lang(kit)),
            Action::InventoryOrder(order),
            true,
        );
    }
    control(
        commands,
        top,
        kit,
        ui,
        Phrase::EchoesRunGroups.text(lang(kit)),
        Action::InventoryGroups(!ui.bag.grouped),
        true,
    );
}

fn contents(commands: &mut Commands, bag: Entity, kit: Kit, ui: &Ui, run: &api::Run) {
    let order = if ui.bag.grouped && ui.bag.order == workshop::Order::Newest {
        workshop::Order::Family
    } else {
        ui.bag.order
    };
    let entries = workshop::inventory(&run.economy.inventory, order);
    let (page, pages) = workshop::page(entries.len(), ui.bag.page);
    if entries.is_empty() {
        label(
            commands,
            bag,
            kit,
            Phrase::EchoesRunBagEmpty.text(lang(kit)),
            tokens::MUTED,
        );
    }
    let mut previous = None;
    let mut cells = row(commands, bag, kit);
    commands
        .entity(cells)
        .entry::<Node>()
        .and_modify(|mut node| node.flex_wrap = FlexWrap::Wrap);
    for item in entries
        .into_iter()
        .skip(page * workshop::PAGE_SIZE)
        .take(workshop::PAGE_SIZE)
    {
        let key = if order == workshop::Order::Rarity {
            u16::from(item.definition.rarity.tier())
        } else {
            (item.definition.id() - 1) / 4
        };
        if ui.bag.grouped && previous != Some(key) {
            let title = if order == workshop::Order::Rarity {
                words::rarity(item.definition.rarity).to_owned()
            } else {
                words::name(Goods::Relic(item.definition), lang(kit))
            };
            label(
                commands,
                bag,
                kit,
                &title,
                towerui::color(Goods::Relic(item.definition)),
            );
            cells = row(commands, bag, kit);
            commands
                .entity(cells)
                .entry::<Node>()
                .and_modify(|mut node| node.flex_wrap = FlexWrap::Wrap);
            previous = Some(key);
        }
        let cell = object(
            commands,
            cells,
            kit,
            ui,
            Goods::Relic(item.definition),
            None,
            76.0,
            Emphasis::None,
            Action::Select(item.id),
            true,
        );
        commands
            .entity(cell)
            .insert(super::super::super::gestures::Owned::new(run, item.id));
    }
    if pages > 1 {
        let paging = row(commands, bag, kit);
        control(
            commands,
            paging,
            kit,
            ui,
            Phrase::EchoesRunPrevious.text(lang(kit)),
            Action::InventoryPage(page.saturating_sub(1)),
            page > 0,
        );
        label(
            commands,
            paging,
            kit,
            &Phrase::EchoesRunPage.fill(lang(kit), &[&(page + 1).to_string(), &pages.to_string()]),
            tokens::MUTED,
        );
        control(
            commands,
            paging,
            kit,
            ui,
            Phrase::EchoesRunNext.text(lang(kit)),
            Action::InventoryPage(page + 1),
            page + 1 < pages,
        );
    }
}

fn selected(commands: &mut Commands, root: Entity, kit: Kit, ui: &Ui, run: &api::Run) {
    let Some(item) = run
        .economy
        .inventory
        .relics
        .iter()
        .find(|item| Some(item.id) == ui.choices.item)
    else {
        return;
    };
    let band = row(commands, root, kit);
    label(
        commands,
        band,
        kit,
        &words::goods(Goods::Relic(item.definition), lang(kit)),
        towerui::color(Goods::Relic(item.definition)),
    );
    let moving = workshop::quick_move(&run.economy.inventory, item.id);
    control(
        commands,
        band,
        kit,
        ui,
        Phrase::EchoesRunEquip.text(lang(kit)),
        Action::QuickEquip(item.id),
        moving.as_ref().is_some_and(|command| can(ui, command)),
    );
    if moving.is_none() {
        label(
            commands,
            root,
            kit,
            Phrase::EchoesRunFull.text(lang(kit)),
            tokens::MUTED,
        );
    }
    if item.definition.rarity == Rarity::Rare
        && let Some(potion) = run.economy.inventory.materials.first()
    {
        let command = workshop::mythic(&run.economy.inventory, item.id, potion.id);
        control(
            commands,
            band,
            kit,
            ui,
            Phrase::EchoesRunUpgrade.text(lang(kit)),
            Action::Upgrade(item.id, potion.id),
            command.as_ref().is_some_and(|command| can(ui, command)),
        );
    }
    control(
        commands,
        band,
        kit,
        ui,
        &Phrase::EchoesSellPrice.fill(
            lang(kit),
            &[&baylee_client_core::echoes::draft::refund(run, item.id)
                .unwrap_or(0)
                .to_string()],
        ),
        Action::Sell,
        can(ui, &Command::Sell { item: item.id }),
    );
}
