//! A language model in a chair of a room this client hosts
//! (`crate::tableseats`, `docs/llm-seat.md` §"A language model at your
//! table"): which profile of the settings file, which model (its label and
//! its exact id), which effort (only those the model takes), and which
//! deck. Each change before the game starts the chair's bridge again.
#[allow(clippy::wildcard_imports)] // the room's UI vocabulary
use super::*;
use crate::tableseats::LlmPress;
use baylee_client_core::llmseat::models::Resolved;
use baylee_client_core::llmseat::seating::DECKS;

/// A caption over a wrapping row of chips, under `parent`.
fn chips(
    commands: &mut Commands,
    parent: Entity,
    fonts: &UiFonts,
    m: Metrics,
    caption: &str,
    items: &[(String, Press, bool)],
) {
    let label = note(commands, fonts, m, caption);
    commands.entity(label).insert(TextColor(palette::MUTED));
    let line = row(commands, m, true);
    commands
        .entity(line)
        .entry::<Node>()
        .and_modify(|mut node| node.flex_wrap = FlexWrap::Wrap);
    for (words, press, on) in items {
        let id = chip(commands, fonts, m, words, *press, *on);
        commands.entity(line).add_child(id);
    }
    commands.entity(parent).add_children(&[label, line]);
}

/// The editor of a chair a language model is planned for.
#[allow(clippy::too_many_lines)] // one editor, read top to bottom
pub(super) fn editor(
    commands: &mut Commands,
    card: Entity,
    state: &LobbyState,
    fonts: &UiFonts,
    m: Metrics,
    index: usize,
    chair: u32,
) {
    let lang = state.lobby.lang();
    let llm = &state.llm;
    let Some(planned) = llm.planned(chair) else {
        return;
    };
    let surface = commands
        .spawn((
            Node {
                width: percent(100),
                flex_direction: FlexDirection::Column,
                row_gap: px(m.gap * 0.6),
                padding: UiRect::all(px(m.gap)),
                border: UiRect::all(px(1)),
                border_radius: BorderRadius::all(px(8)),
                ..default()
            },
            BackgroundColor(palette::PANEL),
            BorderColor::all(palette::PANEL_LIT),
        ))
        .id();
    commands.entity(card).add_child(surface);
    let current = llm.current(chair);
    let caption = current
        .as_ref()
        .map_or_else(|| planned.model.clone(), Resolved::caption);
    let effort = planned
        .effort
        .clone()
        .unwrap_or_else(|| Phrase::RoomLlmEffortOwn.text(lang).to_string());
    add_heading(
        commands,
        surface,
        fonts,
        m,
        &Phrase::RoomLlmPlays.fill(lang, &[&caption, &effort]),
    );

    let profiles: Vec<(String, Press, bool)> = llm
        .profiles()
        .iter()
        .enumerate()
        .map(|(at, (name, profile))| {
            (
                format!(
                    "{name} · {}",
                    baylee_client_core::llmseat::seating::protocol_label(profile.provider)
                ),
                Press::Room(RoomPress::RoomLlm(index, chair, LlmPress::Profile(at))),
                *name == planned.profile,
            )
        })
        .collect();
    chips(
        commands,
        surface,
        fonts,
        m,
        Phrase::RoomLlmProfile.text(lang),
        &profiles,
    );

    let models: Vec<(String, Press, bool)> = llm
        .models(chair)
        .iter()
        .enumerate()
        .map(|(at, model)| {
            (
                model.caption(),
                Press::Room(RoomPress::RoomLlm(index, chair, LlmPress::Model(at))),
                model.id == planned.model,
            )
        })
        .collect();
    chips(
        commands,
        surface,
        fonts,
        m,
        Phrase::SeatModel.text(lang),
        &models,
    );

    if let Some(current) = &current {
        let mut efforts = vec![(
            Phrase::RoomLlmEffortOwn.text(lang).to_string(),
            Press::Room(RoomPress::RoomLlm(index, chair, LlmPress::Effort(None))),
            planned.effort.is_none(),
        )];
        efforts.extend(current.efforts.iter().enumerate().map(|(at, e)| {
            (
                (*e).to_string(),
                Press::Room(RoomPress::RoomLlm(index, chair, LlmPress::Effort(Some(at)))),
                planned.effort.as_deref() == Some(*e),
            )
        }));
        chips(
            commands,
            surface,
            fonts,
            m,
            Phrase::SeatEffort.text(lang),
            &efforts,
        );
        if current.efforts.is_empty() {
            add_note(
                commands,
                surface,
                fonts,
                m,
                Phrase::RoomLlmNoEfforts.text(lang),
            );
        }
    }

    let deck = llm.deck(chair).unwrap_or(DECKS[0]);
    let decks: Vec<(String, Press, bool)> = DECKS
        .iter()
        .enumerate()
        .map(|(at, name)| {
            (
                (*name).to_string(),
                Press::Room(RoomPress::RoomLlm(index, chair, LlmPress::Deck(at))),
                *name == deck,
            )
        })
        .collect();
    chips(
        commands,
        surface,
        fonts,
        m,
        Phrase::RoomLlmDeck.text(lang),
        &decks,
    );

    let doors = row(commands, m, true);
    commands
        .entity(doors)
        .entry::<Node>()
        .and_modify(|mut node| node.flex_wrap = FlexWrap::Wrap);
    let save = button(
        commands,
        fonts,
        m,
        Phrase::RoomLlmSave.text(lang),
        Press::Room(RoomPress::RoomLlm(index, chair, LlmPress::Save)),
        palette::PANEL_LIT,
        true,
    );
    let remove = button(
        commands,
        fonts,
        m,
        Phrase::RoomLlmRemove.text(lang),
        Press::Room(RoomPress::RoomLlm(index, chair, LlmPress::Remove)),
        palette::DANGER,
        true,
    );
    commands.entity(doors).add_children(&[save, remove]);
    commands.entity(surface).add_child(doors);
    let status = state
        .llm
        .said(chair)
        .unwrap_or_else(|| Phrase::RoomLlmStarting.text(lang).to_string());
    add_note(commands, surface, fonts, m, &status);
}
