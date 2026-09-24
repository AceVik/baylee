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
        if let Some(line) = leader(commands, fonts, metrics, deck, slot) {
            commands.entity(section).add_child(line);
        }
    }
    if deck.commanders().is_empty() {
        let hint = note(commands, fonts, metrics, Phrase::CommanderHint.text(lang));
        commands.entity(section).add_child(hint);
    }
    section
}

/// One commander, drawn the way a deck row draws its card (#255): the
/// picture of the printing the deck holds, at the row's full height, which
/// opens the printing picker on that row; the name, cost and type line
/// beside it; hovering previews that printing, and a click on the rest of
/// the row reads the card. The one control is the one a commander has and a
/// deck row does not, taking the role away (the card stays in the deck).
///
/// A commander that has left the list (moved to the sideboard) keeps its
/// line, since it is still named, but shows the pool's printing and offers
/// no picker: there is no row for a printing to be chosen for.
fn leader(
    commands: &mut Commands,
    fonts: &UiFonts,
    metrics: Metrics,
    deck: &DeckBuilder,
    slot: usize,
) -> Option<Entity> {
    let card = deck.card(slot)?;
    let held = deck
        .commander_row(slot)
        .and_then(|at| deck.entries(Zone::Main).get(at));
    let hover = held.map_or_else(
        || hover_of_card(card),
        |entry| hover_of_entry(card, &entry.print),
    );
    let line = commands
        .spawn((
            Node {
                width: percent(100),
                min_height: px(70),
                flex_shrink: 0.0,
                align_items: AlignItems::Center,
                column_gap: px(metrics.gap * 0.6),
                padding: UiRect::axes(px(metrics.pad * 0.5), px(metrics.pad * 0.25)),
                border_radius: btn_radius(),
                ..default()
            },
            BackgroundColor(palette::PANEL_LIT),
            crate::ambience::Feel::tinting_to(palette::PANEL_LIT, palette::PANEL_LIT.lighter(0.06)),
            Press::Inspect(slot),
            hover.clone(),
        ))
        .id();
    let thumb = crate::lobby::thumbnails::spawn(commands, &hover);
    if held.is_some() {
        commands
            .entity(thumb)
            .insert((Press::PickCommanderPrint(slot), Pickable::default()));
    }
    fill_thumbnail(commands, thumb);

    let details = commands
        .spawn((
            Node {
                flex_direction: FlexDirection::Column,
                flex_grow: 1.0,
                min_width: px(0),
                flex_basis: px(0),
                row_gap: px(4),
                ..default()
            },
            Pickable::IGNORE,
        ))
        .id();
    let title_row = row(commands, metrics, false);
    let title = commands
        .spawn((
            Text::new(card.name.clone()),
            tf(fonts, metrics.text),
            TextColor(if card.coverage.trustworthy() {
                palette::INK
            } else {
                palette::MUTED
            }),
            Node {
                flex_grow: 1.0,
                flex_basis: px(0),
                min_width: px(0),
                ..default()
            },
            Pickable::IGNORE,
        ))
        .id();
    let gap = commands.spawn((spacer(), Pickable::IGNORE)).id();
    let cost =
        crate::manaui::spawn_cost_or_text(commands, fonts, &card.mana_cost, metrics.small * 1.3);
    for child in [Some(title), Some(gap), cost].into_iter().flatten() {
        commands.entity(title_row).add_child(child);
    }
    let chosen = held
        .map(|entry| print_mark(&entry.print))
        .unwrap_or_default();
    if !chosen.is_empty() {
        let mark = commands
            .spawn((
                Text::new(chosen),
                tf(fonts, metrics.small * 0.9),
                TextColor(palette::ACCENT),
                Pickable::IGNORE,
            ))
            .id();
        commands.entity(title_row).add_child(mark);
    }
    let kind = note(commands, fonts, metrics, &card.type_line);
    let actions = card_actions(commands, metrics);
    let remove = card_action(commands, fonts, metrics, "×", Press::RemoveCommander(slot));
    commands.entity(actions).add_child(remove);
    commands
        .entity(details)
        .add_children(&[title_row, kind, actions]);
    commands.entity(line).add_children(&[thumb, details]);
    Some(line)
}
