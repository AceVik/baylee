//! Per-copy starting cards, using the builder's preview and printing catalog.
#[allow(clippy::wildcard_imports)] // the room's UI vocabulary
use super::*;
use baylee_core::preset::RoomSeatSetup;

#[allow(clippy::too_many_arguments, clippy::too_many_lines)] // one inline editor, in visual order
pub(super) fn draw(
    commands: &mut Commands,
    parent: Entity,
    state: &LobbyState,
    fonts: &UiFonts,
    m: Metrics,
    seat: u8,
    personal: Option<&RoomSeatSetup>,
    host: bool,
) {
    let lobby = &state.lobby;
    let lang = lobby.lang();
    let surface = commands
        .spawn((
            Node {
                width: percent(100),
                flex_direction: FlexDirection::Column,
                row_gap: px(m.gap),
                padding: UiRect::all(px(m.gap)),
                border: UiRect::all(px(1)),
                border_radius: BorderRadius::all(px(8)),
                ..default()
            },
            BackgroundColor(palette::PANEL),
            BorderColor::all(palette::DOCK_EDGE.with_alpha(0.45)),
        ))
        .id();
    commands.entity(parent).add_child(surface);
    let badges = row(commands, m, true);
    commands.entity(surface).add_child(badges);
    if let Some(personal) = personal {
        for (at, line) in personal.permanents.iter().enumerate() {
            let Ok(entry) = baylee_core::deckrow::parse(&format!("1 {line}")) else {
                continue;
            };
            let badge = commands
                .spawn((
                    Node {
                        max_width: percent(100),
                        align_items: AlignItems::Center,
                        column_gap: px(m.gap),
                        padding: UiRect::all(px(6)),
                        border_radius: BorderRadius::all(px(8)),
                        border: UiRect::all(px(1)),
                        ..default()
                    },
                    BackgroundColor(palette::PANEL_LIT),
                    BorderColor::all(palette::DOCK_EDGE),
                    Press::Room(RoomPress::RoomCardEdit(seat, at)),
                ))
                .id();
            let slot = lobby.builder().slot_of(&entry.name);
            if let Some(card) = slot.and_then(|slot| lobby.builder().card(slot)) {
                let hover = super::super::preview::hover_of_entry(card, &entry.print);
                let thumb = super::super::thumbnails::spawn(commands, &hover);
                commands.entity(badge).insert(hover).add_child(thumb);
            }
            let mark = super::super::print_mark(&entry.print);
            let caption = if mark.is_empty() {
                entry.name.clone()
            } else {
                format!("{}\n{mark}", entry.name)
            };
            let name = note(commands, fonts, m, &caption);
            commands.entity(badge).add_child(name);
            if let Some(counters) = personal.counters.get(at).filter(|c| !c.is_empty()) {
                add_note(
                    commands,
                    badge,
                    fonts,
                    m,
                    &counters
                        .iter()
                        .map(|c| format!("{} ×{}", c.kind, c.amount))
                        .collect::<Vec<_>>()
                        .join(" · "),
                );
            }
            if host {
                let remove = chip(
                    commands,
                    fonts,
                    m,
                    "×",
                    Press::Room(RoomPress::RoomCardRemove(seat, at)),
                    false,
                );
                commands
                    .entity(remove)
                    .insert(super::super::hint::HoverHint(
                        Phrase::RoomRemoveCard.text(lang).into(),
                    ));
                commands.entity(badge).add_child(remove);
            }
            commands.entity(badges).add_child(badge);
        }
    }
    if host {
        field(
            commands,
            surface,
            state,
            fonts,
            m,
            Phrase::RoomBoard.text(lang),
            Field::RoomBoard(seat),
            false,
        );
        if lobby.focus() == Field::RoomBoard(seat)
            && !lobby.field(Field::RoomBoard(seat)).trim().is_empty()
        {
            let matches = lobby.room_matches(seat);
            if matches.is_empty() {
                add_note(
                    commands,
                    surface,
                    fonts,
                    m,
                    Phrase::RoomNoCardMatches.text(lang),
                );
            }
            let dropdown = commands
                .spawn((
                    Node {
                        width: percent(100),
                        max_height: px(280),
                        min_height: px(0),
                        flex_direction: FlexDirection::Column,
                        overflow: Overflow::scroll_y(),
                        row_gap: px(4),
                        ..default()
                    },
                    Scrollable(List::RoomCards),
                    ScrollPosition::default(),
                    BackgroundColor(palette::PANEL_HOT),
                ))
                .id();
            commands.entity(surface).add_child(dropdown);
            for slot in matches {
                let Some(card) = lobby.builder().card(slot) else {
                    continue;
                };
                let hover = super::super::preview::hover_of_card(card);
                let result = commands
                    .spawn((
                        Node {
                            width: percent(100),
                            flex_shrink: 0.0,
                            align_items: AlignItems::Center,
                            column_gap: px(m.gap),
                            padding: UiRect::all(px(6)),
                            ..default()
                        },
                        BackgroundColor(palette::PANEL_LIT),
                        Press::Room(RoomPress::RoomCardAdd(seat, slot, false)),
                        hover.clone(),
                    ))
                    .id();
                let thumb = super::super::thumbnails::spawn(commands, &hover);
                let name = note(
                    commands,
                    fonts,
                    m,
                    &format!("{}\n{}", card.name, card.type_line),
                );
                commands.entity(name).entry::<Node>().and_modify(|mut n| {
                    n.flex_grow = 1.0;
                    n.min_width = px(0);
                });
                let printing = chip(
                    commands,
                    fonts,
                    m,
                    Phrase::RoomCardAppearance.text(lang),
                    Press::Room(RoomPress::RoomCardAdd(seat, slot, true)),
                    false,
                );
                commands
                    .entity(result)
                    .add_children(&[thumb, name, printing]);
                commands.entity(dropdown).add_child(result);
            }
        }
        if let Some((which, at)) = state.room_card_edit.filter(|(which, _)| *which == seat)
            && let Some(personal) = personal
            && at < personal.permanents.len()
        {
            let tools = row(commands, m, true);
            let appearance = chip(
                commands,
                fonts,
                m,
                Phrase::RoomCardAppearance.text(lang),
                Press::Room(RoomPress::RoomCardPrint(which, at)),
                false,
            );
            commands.entity(tools).add_child(appearance);
            commands.entity(surface).add_child(tools);
            add_heading(
                commands,
                surface,
                fonts,
                m,
                Phrase::RoomCardCounters.text(lang),
            );
            if let Some(counters) = personal.counters.get(at) {
                for (index, counter) in counters.iter().enumerate() {
                    let line = row(commands, m, false);
                    add_note(
                        commands,
                        line,
                        fonts,
                        m,
                        &format!("{} · {}", counter.kind, counter.amount),
                    );
                    for (label, delta) in [("−", -1), ("+", 1), ("×", -999)] {
                        let b = chip(
                            commands,
                            fonts,
                            m,
                            label,
                            Press::Room(RoomPress::RoomCounterStep(seat, at, index, delta)),
                            false,
                        );
                        commands.entity(line).add_child(b);
                    }
                    commands.entity(surface).add_child(line);
                }
            }
            field(
                commands,
                surface,
                state,
                fonts,
                m,
                Phrase::RoomCounterKind.text(lang),
                Field::RoomCounter,
                false,
            );
            add_note(
                commands,
                surface,
                fonts,
                m,
                Phrase::RoomCounterHint.text(lang),
            );
            let add = button(
                commands,
                fonts,
                m,
                Phrase::RoomAddCounter.text(lang),
                Press::Room(RoomPress::RoomCounterAdd(seat, at)),
                palette::PANEL_LIT,
                baylee_cards_dsl::CounterKind::from_setup_name(lobby.field(Field::RoomCounter))
                    .is_some(),
            );
            commands.entity(surface).add_child(add);
        }
    }
}
