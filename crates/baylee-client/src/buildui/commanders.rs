//! Visible command-zone roles, independent of the deck's card rows.
#[allow(clippy::wildcard_imports)]
use super::*;

pub(super) fn draw(
    commands: &mut Commands,
    fonts: &UiFonts,
    metrics: Metrics,
    lang: Lang,
    deck: &DeckBuilder,
) -> Entity {
    let section = commands
        .spawn((
            Node {
                flex_direction: FlexDirection::Column,
                flex_shrink: 0.0,
                row_gap: px(5),
                ..default()
            },
            Pickable::IGNORE,
        ))
        .id();
    let actions = row(commands, metrics, true);
    let title = note(
        commands,
        fonts,
        metrics,
        Phrase::CommanderSection.text(lang),
    );
    let choose = chip(
        commands,
        fonts,
        metrics,
        Phrase::ChooseCommander.text(lang),
        Press::ChooseCommander(false),
        false,
    );
    commands.entity(actions).add_children(&[title, choose]);
    if deck.commanders().len() == 1 {
        let partner = chip(
            commands,
            fonts,
            metrics,
            Phrase::ChoosePartner.text(lang),
            Press::ChooseCommander(true),
            false,
        );
        commands.entity(actions).add_child(partner);
    }
    commands.entity(section).add_child(actions);
    for &slot in deck.commanders() {
        let Some(card) = deck.card(slot) else {
            continue;
        };
        let line = row(commands, metrics, false);
        commands
            .entity(line)
            .remove::<Pickable>()
            .insert((Press::Inspect(slot), hover_of_card(card)));
        let art = crate::lobby::thumbnails::spawn(commands, &hover_of_card(card));
        commands.entity(art).entry::<Node>().and_modify(|mut n| {
            n.width = px(27);
            n.height = px(38);
        });
        let title = note(commands, fonts, metrics, &card.name);
        let remove = chip(
            commands,
            fonts,
            metrics,
            "×",
            Press::RemoveCommander(slot),
            false,
        );
        commands.entity(line).add_children(&[art, title, remove]);
        commands.entity(section).add_child(line);
    }
    if deck.commanders().is_empty() {
        let hint = note(commands, fonts, metrics, Phrase::CommanderHint.text(lang));
        commands.entity(section).add_child(hint);
    }
    section
}
