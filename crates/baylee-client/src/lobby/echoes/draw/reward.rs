//! Saved post-duel choices. Only the five revealed identities become UI nodes.
#[allow(clippy::wildcard_imports)]
use super::*;

pub(super) fn draw(commands: &mut Commands, root: Entity, kit: Kit, ui: &Ui, run: &api::Run) {
    let Some(reward) = &run.reward else {
        return;
    };
    let lang = lang(kit);
    let surface = panel(commands, root, kit);
    presentation::title(
        commands,
        surface,
        kit,
        glyph::CROWN,
        Phrase::EchoesRewardReady.text(lang),
    );
    let actions = row(commands, surface, kit);
    label(
        commands,
        actions,
        kit,
        &Phrase::EchoesRewardCount.fill(
            lang,
            &[
                &ui.choices.rewards.len().to_string(),
                &reward.take.to_string(),
            ],
        ),
        tokens::GOLD,
    );
    control(
        commands,
        actions,
        kit,
        ui,
        Phrase::EchoesRewardClaim.text(lang),
        Action::ClaimReward,
        can(
            ui,
            &Command::Claim {
                cards: ui.choices.rewards.clone(),
            },
        ),
    );
    widgets::paragraph(
        commands,
        surface,
        kit,
        Phrase::EchoesRewardLead.text(lang),
        tokens::MUTED,
    );
    let shelf = row(commands, surface, kit);
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
            surface,
            kit,
            Phrase::EchoesRewardEmpty.text(lang),
            tokens::MUTED,
        );
    }
    if let Some(card) = ui.preview.filter(|card| reward.cards.contains(card)) {
        card::preview(commands, surface, kit, card);
    }
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
    let tile = panel(commands, shelf, kit);
    commands
        .entity(tile)
        .entry::<Node>()
        .and_modify(move |mut node| {
            node.flex_basis = if kit.m.frame == Frame::Phone {
                percent(100)
            } else {
                percent(18)
            };
            node.min_width = kit.m.px(160.0);
            node.flex_grow = 1.0;
            node.max_width = kit.m.px(240.0);
            node.align_items = AlignItems::Center;
            node.padding = UiRect::all(kit.m.px(12.0));
            node.row_gap = kit.m.px(8.0);
        });
    if selected {
        commands.entity(tile).insert(BorderColor::all(tokens::GOLD));
    }
    let hover = crate::lobby::preview::hover_of_index(id.get());
    let image = crate::lobby::thumbnails::spawn_sized(
        commands,
        &hover,
        kit.m.px(112.0),
        kit.m.px(156.0),
        kit.m.px(7.0),
    );
    commands.entity(tile).insert(hover).add_child(image);
    label(commands, tile, kit, &words::card(id, lang), tokens::INK);
    if run.cards.copies(id) > 0 {
        label(
            commands,
            tile,
            kit,
            Phrase::EchoesRewardDuplicate.text(lang),
            tokens::GOLD,
        );
    }
    let spacer = crate::lobby::parts::grow(commands);
    commands.entity(tile).add_child(spacer);
    control(
        commands,
        tile,
        kit,
        ui,
        Phrase::EchoesRewardInspect.text(lang),
        Action::InspectCard(id),
        true,
    );
    control(
        commands,
        tile,
        kit,
        ui,
        if selected {
            Phrase::EchoesRewardSelected
        } else {
            Phrase::EchoesRewardSelect
        }
        .text(lang),
        Action::Reward(id),
        !ui.model.uncertain()
            && (selected
                || run
                    .reward
                    .as_ref()
                    .is_some_and(|reward| ui.choices.rewards.len() < usize::from(reward.take))),
    );
}
