//! Five authoritative offers over the shop; selection never spends or grants locally.
#[allow(clippy::wildcard_imports)]
use super::*;

pub(super) fn draw(commands: &mut Commands, root: Entity, kit: Kit, ui: &Ui, run: &api::Run) {
    let Some(reward) = &run.reward else {
        return;
    };
    let lang = lang(kit);
    let body = commands
        .spawn((
            Node {
                width: percent(100),
                flex_direction: FlexDirection::Column,
                row_gap: kit.m.px(16.0),
                ..default()
            },
            Pickable::IGNORE,
        ))
        .id();
    widgets::paragraph(
        commands,
        body,
        kit,
        Phrase::EchoesRewardLead.text(lang),
        tokens::MUTED,
    );
    let shelf = row(commands, body, kit);
    commands
        .entity(shelf)
        .entry::<Node>()
        .and_modify(|mut node| {
            node.justify_content = JustifyContent::Center;
            node.align_items = AlignItems::Stretch;
        });
    for &id in &reward.cards {
        candidate(commands, shelf, kit, ui, run, id);
    }
    if reward.cards.is_empty() {
        label(
            commands,
            body,
            kit,
            Phrase::EchoesRewardEmpty.text(lang),
            tokens::MUTED,
        );
    }
    selection(commands, body, kit, ui, reward.take);
    let surface = surfaces::sheet_box_listed(
        commands,
        kit,
        surfaces::SheetWidth::Large,
        Phrase::EchoesRewardReady.text(lang),
        &[body],
        &[],
        (Scrollable(List::EchoesInspector), ScrollPosition::default()),
    );
    commands
        .entity(surface)
        .insert(Press::Shared(crate::lobby::SharedPress::PickerNothing));
    let scrim = surfaces::sheet(commands, surface);
    commands.entity(root).add_child(scrim);
}

fn selection(commands: &mut Commands, body: Entity, kit: Kit, ui: &Ui, take: u8) {
    let lang = lang(kit);
    let actions = row(commands, body, kit);
    commands
        .entity(actions)
        .entry::<Node>()
        .and_modify(move |mut node| {
            node.min_height = kit.m.px(44.0);
            node.justify_content = JustifyContent::Center;
        });
    label(
        commands,
        actions,
        kit,
        &Phrase::EchoesRewardCount.fill(
            lang,
            &[&ui.choices.rewards.len().to_string(), &take.to_string()],
        ),
        tokens::GOLD,
    );
    let selected = ui.choices.rewards.len() == usize::from(take);
    let confirm = controls::button_compact(
        commands,
        kit,
        Some(glyph::CHECK),
        Phrase::EchoesRewardClaim.text(lang),
        Weight::Gold,
        if selected
            && !ui.model.busy()
            && can(
                ui,
                &Command::Claim {
                    cards: ui.choices.rewards.clone(),
                },
            )
        {
            Live::Yes
        } else {
            Live::No(Phrase::EchoesUnavailable.text(lang))
        },
        Press::Echoes(Action::ClaimReward),
    );
    commands
        .entity(confirm)
        .insert(super::super::keys::stop(Action::ClaimReward, ui));
    if !selected {
        commands.entity(confirm).insert(Visibility::Hidden);
    }
    commands.entity(actions).add_child(confirm);
}

fn candidate(
    commands: &mut Commands,
    shelf: Entity,
    kit: Kit,
    ui: &Ui,
    run: &api::Run,
    id: baylee_core::ids::CardIndex,
) {
    let lang = lang(kit);
    let selected = ui.choices.rewards.contains(&id);
    let rarity = crate::lobby::decks::rarity_dot_color(
        baylee_cards::sides::rarity(id).unwrap_or(baylee_client_core::deckbuilder::Rarity::Common),
    );
    let tile = panel(commands, shelf, kit);
    let action = Action::Reward(id);
    commands
        .entity(tile)
        .insert((
            crate::shellkit::Role::Hit,
            Pickable::default(),
            PickingInteraction::default(),
            Press::Echoes(action),
            super::super::keys::stop(action, ui),
            BorderColor::all(rarity),
            BackgroundColor(if selected {
                tokens::SELECTED
            } else {
                tokens::CONTROL
            }),
            crate::ambience::Feel::tinting_to(
                if selected {
                    tokens::SELECTED
                } else {
                    tokens::CONTROL
                },
                tokens::HOVER,
            ),
            crate::ambience::FeelBorder {
                rest: rarity,
                hot: tokens::INK,
                press: tokens::GOLD,
            },
            crate::ambience::FeelShadow,
            crate::shellkit::sheen::Sheen,
        ))
        .entry::<Node>()
        .and_modify(move |mut node| {
            node.flex_basis = percent(18);
            node.min_width = kit.m.px(136.0);
            node.flex_grow = 1.0;
            node.max_width = kit.m.px(190.0);
            node.align_items = AlignItems::Center;
            node.padding = UiRect::all(kit.m.px(10.0));
            node.row_gap = kit.m.px(8.0);
        });
    if ui.model.busy()
        || ui.model.uncertain()
        || (!selected
            && run
                .reward
                .as_ref()
                .is_some_and(|reward| ui.choices.rewards.len() >= usize::from(reward.take)))
    {
        commands.entity(tile).insert(controls::Disabled);
    }
    let hover = crate::lobby::hover_of_index(id.get());
    let image = crate::lobby::thumbnails::spawn_sized(
        commands,
        &hover,
        kit.m.px(112.0),
        kit.m.px(156.0),
        kit.m.px(7.0),
    );
    commands.entity(image).insert(surfaces::elevation_control());
    commands.entity(tile).insert(hover).add_child(image);
    let name = surfaces::prose(commands, kit, &words::card(id, lang), false);
    commands
        .entity(name)
        .insert(TextLayout::justify(Justify::Center));
    commands.entity(tile).add_child(name);
    if run.cards.copies(id) > 0 {
        label(
            commands,
            tile,
            kit,
            Phrase::EchoesRewardDuplicate.text(lang),
            tokens::GOLD,
        );
    }
    let space = crate::lobby::parts::grow(commands);
    commands.entity(tile).add_child(space);
    let mark = commands
        .spawn((
            Text::new(glyph::CHECK.to_string()),
            crate::hud::icon_tf(kit.fonts, kit.m.text),
            TextColor(tokens::GOLD),
            Pickable::IGNORE,
            ChildOf(tile),
        ))
        .id();
    if !selected {
        commands.entity(mark).insert(Visibility::Hidden);
    }
}
