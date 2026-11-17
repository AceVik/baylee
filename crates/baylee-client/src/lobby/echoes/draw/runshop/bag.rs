#[allow(clippy::wildcard_imports)]
use super::*;

const BAG_SLOT_SIZE: f32 = 54.0;

pub(super) fn draw(commands: &mut Commands, root: Entity, kit: Kit, ui: &Ui, run: &api::Run) {
    let bag = towerui::frame(commands, root, kit);
    commands.entity(bag).insert(Place::new(run, Anchor::Bag));
    commands
        .entity(bag)
        .entry::<Node>()
        .and_modify(move |mut node| {
            node.padding = UiRect::all(kit.m.px(18.0));
            node.row_gap = kit.m.px(10.0);
        });
    commands.entity(bag).insert((
        super::super::super::gestures::Destination::new(run, Slot::Inventory),
        Pickable::default(),
    ));
    toolbar(commands, bag, kit, ui, run);
    contents(commands, bag, kit, ui, run);
    stats(commands, bag, kit, run);
}

fn toolbar(commands: &mut Commands, bag: Entity, kit: Kit, ui: &Ui, run: &api::Run) {
    let lang = lang(kit);
    let top = row(commands, bag, kit);
    commands
        .entity(top)
        .entry::<Node>()
        .and_modify(move |mut node| {
            node.column_gap = kit.m.px(4.0);
        });

    for (order, tip_phrase) in [
        (workshop::Order::Newest, Phrase::EchoesRunSortNewestTip),
        (workshop::Order::Name, Phrase::EchoesRunSortNameTip),
        (workshop::Order::Value, Phrase::EchoesRunSortValueTip),
        (workshop::Order::Family, Phrase::EchoesRunSortFamilyTip),
        (workshop::Order::Rarity, Phrase::EchoesRunSortRarityTip),
    ] {
        let active = ui.bag.order == order;
        let action = Action::InventoryOrder(order);
        icon_control(
            commands,
            top,
            kit,
            ui,
            words::icon(action),
            None,
            if active {
                IconTone::Active
            } else {
                IconTone::Neutral
            },
            tip_phrase.text(lang),
            action,
            true,
            None,
        );
    }

    commands.spawn((
        Node {
            width: px_fixed(1.0),
            height: kit.m.px(18.0),
            margin: UiRect::horizontal(kit.m.px(4.0)),
            ..default()
        },
        BackgroundColor(tokens::BORDER),
        Pickable::IGNORE,
        ChildOf(top),
    ));

    icon_control(
        commands,
        top,
        kit,
        ui,
        words::GLYPH_FOLDER,
        None,
        if ui.bag.grouped {
            IconTone::Active
        } else {
            IconTone::Neutral
        },
        Phrase::EchoesRunGroupsTip.text(lang),
        Action::InventoryGroups(!ui.bag.grouped),
        true,
        None,
    );

    let spacer = crate::lobby::parts::grow(commands);
    commands.entity(top).add_child(spacer);
    selected_actions(commands, top, kit, ui, run);
}

fn spawn_bag_grid(commands: &mut Commands, bag: Entity, kit: Kit) -> Entity {
    let cells = row(commands, bag, kit);
    commands
        .entity(cells)
        .entry::<Node>()
        .and_modify(move |mut node| {
            node.display = Display::Grid;
            node.grid_template_columns = vec![RepeatedGridTrack::flex(9, 1.0)];
            node.justify_items = JustifyItems::Center;
            node.column_gap = kit.m.px(6.0);
            node.row_gap = kit.m.px(8.0);
        });
    cells
}

fn group_header(
    order: workshop::Order,
    item: &domain::v2::inventory::Relic,
    lang: Lang,
) -> (u32, String, Color) {
    let goods = Goods::Relic(item.definition);
    match order {
        workshop::Order::Rarity => (
            u32::from(item.definition.rarity.tier()),
            words::rarity(item.definition.rarity).to_owned(),
            towerui::color(goods),
        ),
        workshop::Order::Value => {
            let refund = workshop::relic_refund(item);
            (
                u32::from(refund),
                Phrase::EchoesPrice.fill(lang, &[&refund.to_string()]),
                tokens::GOLD,
            )
        }
        workshop::Order::Name => {
            let item_name = words::name(goods, lang);
            let initial = item_name
                .chars()
                .next()
                .and_then(|ch| ch.to_uppercase().next())
                .unwrap_or('—');
            (initial as u32, initial.to_string(), tokens::ACCENT)
        }
        workshop::Order::Family | workshop::Order::Newest => (
            u32::from((item.definition.id() - 1) / 4),
            words::name(goods, lang),
            towerui::color(goods),
        ),
    }
}

fn contents(commands: &mut Commands, bag: Entity, kit: Kit, ui: &Ui, run: &api::Run) {
    let lang = lang(kit);
    let order = if ui.bag.grouped && ui.bag.order == workshop::Order::Newest {
        workshop::Order::Family
    } else {
        ui.bag.order
    };
    let entries = workshop::inventory_by(&run.economy.inventory, order, |definition| {
        words::name(Goods::Relic(definition), lang)
    });
    let (page, pages) = workshop::page(entries.len(), ui.bag.page);
    let shown = entries
        .len()
        .saturating_sub(page * workshop::PAGE_SIZE)
        .min(workshop::PAGE_SIZE);
    let fill_empty = !ui.bag.grouped || entries.is_empty();
    let mut previous = None;
    let mut cells = fill_empty.then(|| spawn_bag_grid(commands, bag, kit));
    for item in entries
        .into_iter()
        .skip(page * workshop::PAGE_SIZE)
        .take(workshop::PAGE_SIZE)
    {
        if ui.bag.grouped {
            let (key, title, tint) = group_header(order, item, lang);
            if previous != Some(key) {
                label(commands, bag, kit, &title, tint);
                cells = Some(spawn_bag_grid(commands, bag, kit));
                previous = Some(key);
            }
        }
        let grid_entity = *cells.get_or_insert_with(|| spawn_bag_grid(commands, bag, kit));
        let cell = object(
            commands,
            grid_entity,
            kit,
            ui,
            Goods::Relic(item.definition),
            None,
            Some(Phrase::EchoesRunEquipmentHint.text(lang)),
            BAG_SLOT_SIZE,
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
    }
    if let Some(grid_entity) = cells.filter(|_| fill_empty) {
        for _ in shown..workshop::PAGE_SIZE {
            sleek_empty(commands, grid_entity, kit, BAG_SLOT_SIZE, glyph::DIAMOND);
        }
    }
    if pages > 1 {
        paging_bar(commands, bag, kit, ui, page, pages);
    }
}

fn paging_bar(commands: &mut Commands, bag: Entity, kit: Kit, ui: &Ui, page: usize, pages: usize) {
    let lang = lang(kit);
    let paging = row(commands, bag, kit);
    commands
        .entity(paging)
        .entry::<Node>()
        .and_modify(|mut node| {
            node.justify_content = JustifyContent::Center;
        });
    icon_control(
        commands,
        paging,
        kit,
        ui,
        glyph::ARROW_LEFT,
        None,
        IconTone::Neutral,
        Phrase::EchoesRunPrevious.text(lang),
        Action::InventoryPage(page.saturating_sub(1)),
        page > 0,
        None,
    );
    label(
        commands,
        paging,
        kit,
        &Phrase::EchoesRunPage.fill(lang, &[&(page + 1).to_string(), &pages.to_string()]),
        tokens::MUTED,
    );
    icon_control(
        commands,
        paging,
        kit,
        ui,
        glyph::ARROW_RIGHT,
        None,
        IconTone::Neutral,
        Phrase::EchoesRunNext.text(lang),
        Action::InventoryPage(page + 1),
        page + 1 < pages,
        None,
    );
}

fn selected_actions(commands: &mut Commands, band: Entity, kit: Kit, ui: &Ui, run: &api::Run) {
    let Some(item) = run
        .economy
        .inventory
        .relics
        .iter()
        .find(|item| Some(item.id) == ui.choices.item)
    else {
        return;
    };
    let lang = lang(kit);
    let chosen_name = words::name(Goods::Relic(item.definition), lang);
    let name_label = controls::label(
        commands,
        kit,
        &chosen_name,
        kit.m.small,
        towerui::color(Goods::Relic(item.definition)),
    );
    commands
        .entity(name_label)
        .insert(crate::hud::tf_bold(kit.fonts, kit.m.small));
    commands.entity(band).add_child(name_label);

    let moving = workshop::quick_move(&run.economy.inventory, item.id);
    let can_move = moving.as_ref().is_some_and(|command| can(ui, command));
    icon_control(
        commands,
        band,
        kit,
        ui,
        words::GLYPH_SWAP,
        None,
        IconTone::Active,
        Phrase::EchoesRunEquip.text(lang),
        Action::QuickEquip(item.id),
        can_move,
        moving.is_none().then(|| Phrase::EchoesRunFull.text(lang)),
    );

    if item.definition.rarity == Rarity::Rare
        && let Some(potion) = run.economy.inventory.materials.first()
    {
        let command = workshop::mythic(&run.economy.inventory, item.id, potion.id);
        icon_control(
            commands,
            band,
            kit,
            ui,
            glyph::CROWN,
            None,
            IconTone::Mythic,
            Phrase::EchoesRunUpgrade.text(lang),
            Action::Upgrade(item.id, potion.id),
            command.as_ref().is_some_and(|command| can(ui, command)),
            None,
        );
    }

    let refund = baylee_client_core::echoes::draft::refund(run, item.id).unwrap_or(0);
    icon_control(
        commands,
        band,
        kit,
        ui,
        glyph::COINS,
        Some(&format!("+{refund}")),
        IconTone::Gold,
        &Phrase::EchoesSellPrice.fill(lang, &[&refund.to_string()]),
        Action::Sell,
        can(ui, &Command::Sell { item: item.id }),
        None,
    );
}
