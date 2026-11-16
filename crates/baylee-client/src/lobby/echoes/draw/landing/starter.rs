//! A compact identity for each published Commander, never inferred from its position.
#[allow(clippy::wildcard_imports)]
use super::*;

pub(super) fn draw(
    commands: &mut Commands,
    root: Entity,
    kit: Kit,
    ui: &Ui,
    cohort: Uuid,
    number: u8,
    commander: baylee_core::ids::CardIndex,
) {
    let lang = lang(kit);
    let choice = tile(commands, root, kit);
    commands
        .entity(choice)
        .entry::<Node>()
        .and_modify(move |mut node| {
            node.min_width = px(0);
            node.row_gap = kit.m.px(8.0);
            node.padding = UiRect::all(kit.m.px(12.0));
            node.align_items = AlignItems::Center;
        });
    let name = words::card(commander, lang);
    let title = controls::label(commands, kit, &name, kit.m.text, tokens::INK);
    commands
        .entity(title)
        .insert((
            crate::hud::tf_bold(kit.fonts, kit.m.text),
            TextLayout::justify(Justify::Center),
        ))
        .entry::<Node>()
        .and_modify(move |mut node| node.min_height = kit.m.px(48.0));
    commands.entity(choice).add_child(title);
    if let Some((family, strategy)) = identity(&name) {
        let band = row(commands, choice, kit);
        commands
            .entity(band)
            .entry::<Node>()
            .and_modify(|mut node| {
                node.flex_direction = FlexDirection::Column;
                node.align_items = AlignItems::Center;
            });
        commands.spawn((
            crate::itemart::Portrait(Goods::Relic(Definition {
                family,
                rarity: Rarity::Rare,
            })),
            Node {
                width: kit.m.px(76.0),
                height: kit.m.px(76.0),
                flex_shrink: 0.0,
                ..default()
            },
            Pickable::IGNORE,
            ChildOf(band),
        ));
        label(commands, band, kit, strategy.text(lang), tokens::GOLD);
    }
    let spacer = crate::lobby::parts::grow(commands);
    commands.entity(choice).add_child(spacer);
    let actions = row(commands, choice, kit);
    control(
        commands,
        actions,
        kit,
        ui,
        Phrase::EchoesRunCommander.text(lang),
        Action::InspectCard(commander),
        true,
    );
    control(
        commands,
        actions,
        kit,
        ui,
        Phrase::EchoesRunChoose.text(lang),
        Action::Starter(cohort, number),
        !ui.model.uncertain() && ui.choices.deck_unchanged(ui.model.run()),
    );
}

fn identity(name: &str) -> Option<(Family, Phrase)> {
    Some(match name {
        "Emmara, Soul of the Accord" => (Family::Grove, Phrase::EchoesRunEmmara),
        "Elas il-Kor, Sadistic Pilgrim" => (Family::SpringChalice, Phrase::EchoesRunElas),
        "Talrand, Sky Summoner" => (Family::StormSpindle, Phrase::EchoesRunTalrand),
        "Slimefoot, the Stowaway" => (Family::HarvestWheel, Phrase::EchoesRunSlimefoot),
        "Pia and Kiran Nalaar" => (Family::EmberLantern, Phrase::EchoesRunPia),
        _ => return None,
    })
}
