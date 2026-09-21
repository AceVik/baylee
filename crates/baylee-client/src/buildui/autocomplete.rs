//! Search suggestions stay inside the field's interaction model.
#[allow(clippy::wildcard_imports)] // This screen module shares its widget vocabulary.
use super::*;

pub(crate) fn suggestions(state: &LobbyState) -> Vec<usize> {
    let deck = state.lobby.builder();
    let query = deck.text().trim().to_lowercase();
    if deck.picker().is_some()
        || state.completion_hidden
        || deck.focus() != BuildField::Search
        || deck.panel().is_some()
        || query.is_empty()
        || query.contains(':')
    {
        return Vec::new();
    }
    deck.results()
        .iter()
        .copied()
        .filter(|slot| {
            deck.card(*slot).is_some_and(|c| {
                c.name.to_lowercase().contains(&query)
                    || c.english_name.to_lowercase().contains(&query)
            })
        })
        .take(6)
        .collect()
}

pub(crate) fn choose(state: &mut LobbyState, slot: usize) {
    if let Some(card) = state.lobby.builder().card(slot) {
        let name = card.name.clone();
        state.lobby.builder_mut().set_text(&name);
        state.completion_hidden = true;
        state.completion = None;
    }
}

pub(super) fn draw(
    commands: &mut Commands,
    state: &LobbyState,
    fonts: &UiFonts,
    metrics: Metrics,
    field: Entity,
) {
    let slots = suggestions(state);
    if slots.is_empty() {
        return;
    }
    let dropdown = commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                top: percent(100),
                width: percent(100),
                flex_direction: FlexDirection::Column,
                padding: UiRect::all(px(6)),
                row_gap: px(4),
                border: UiRect::all(px(1)),
                border_radius: btn_radius(),
                ..default()
            },
            BackgroundColor(palette::PANEL.with_alpha(0.99)),
            BorderColor::all(palette::DOCK_EDGE),
            GlobalZIndex(100),
            BoxShadow::new(Color::BLACK.with_alpha(0.5), px(0), px(10), px(0), px(24)),
        ))
        .id();
    for (at, slot) in slots.into_iter().enumerate() {
        let Some(card) = state.lobby.builder().card(slot) else {
            continue;
        };
        let entry = commands
            .spawn((
                Node {
                    min_height: px(34),
                    width: percent(100),
                    align_items: AlignItems::Center,
                    column_gap: px(10),
                    padding: UiRect::axes(px(10), px(5)),
                    ..default()
                },
                BackgroundColor(if state.completion == Some(at) {
                    palette::PANEL_LIT
                } else {
                    palette::PANEL
                }),
                Press::CompleteSearch(slot),
                crate::ambience::Feel::tinting_to(
                    if state.completion == Some(at) {
                        palette::PANEL_LIT
                    } else {
                        palette::PANEL
                    },
                    palette::PANEL_LIT,
                ),
            ))
            .id();
        let icon = commands
            .spawn((
                Text::new(crate::hud::glyph::MAGNIFIER.to_string()),
                crate::hud::icon_tf(fonts, metrics.small),
                TextColor(palette::MUTED),
                Pickable::IGNORE,
            ))
            .id();
        let name = commands
            .spawn((
                Text::new(&card.name),
                tf(fonts, metrics.text),
                TextColor(palette::INK),
                Pickable::IGNORE,
            ))
            .id();
        commands.entity(entry).add_children(&[icon, name]);
        commands.entity(dropdown).add_child(entry);
    }
    commands.entity(field).add_child(dropdown);
}
