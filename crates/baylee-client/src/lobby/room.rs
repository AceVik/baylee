//! A dedicated waiting room: host rules beside each player's own deck choice.
use super::ui::{FieldLook, Masked, button, chip, heading, note, panel, row, text_field};
#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_client_core::lobby::room::Adjustment;
mod cards;

/// Draw a room in the same bounded sanctuary frame as the lobby.
#[allow(clippy::too_many_lines)] // the page is read in visual order
pub(super) fn draw(
    commands: &mut Commands,
    root: Entity,
    state: &LobbyState,
    fonts: &UiFonts,
    m: Metrics,
    scroll: &Scrolled,
    index: usize,
) {
    let lobby = &state.lobby;
    let lang = lobby.lang();
    let game = &lobby.games()[index];
    let draft = lobby.room_draft();
    let setup = if game.yours {
        draft.map_or(&game.setup, |d| &d.setup)
    } else {
        &game.setup
    };
    let chairs = draft.map_or(game.seats.len(), |d| d.chairs);
    let page = commands
        .spawn(Node {
            width: percent(100),
            min_height: px(0),
            flex_grow: 1.0,
            flex_direction: FlexDirection::Column,
            row_gap: px(m.pad),
            padding: UiRect::all(px(m.pad)),
            overflow: if m.frame == Frame::Phone {
                Overflow::visible()
            } else {
                Overflow::scroll_y()
            },
            ..default()
        })
        .id();
    if m.frame != Frame::Phone {
        commands.entity(page).insert((
            Scrollable(List::Table),
            ScrollPosition(Vec2::new(0.0, scroll.get(List::Table))),
        ));
    }
    commands.entity(root).add_child(page);
    let title = row(commands, m, true);
    let name = if game.name.is_empty() {
        Phrase::RoomTitle.text(lang)
    } else {
        &game.name
    };
    let h = heading(commands, fonts, m, name);
    commands.entity(title).add_child(h);
    let leave = button(
        commands,
        fonts,
        m,
        Phrase::Leave.text(lang),
        Press::LeaveTable(index),
        palette::PANEL_LIT,
        !lobby.busy(),
    );
    commands.entity(title).add_child(leave);
    commands.entity(page).add_child(title);
    let help = note(
        commands,
        fonts,
        m,
        if lobby.room_dirty() {
            Phrase::RoomDraft
        } else if game.yours {
            Phrase::RoomHostHelp
        } else {
            Phrase::RoomGuestHelp
        }
        .text(lang),
    );
    commands.entity(page).add_child(help);
    let columns = commands
        .spawn((
            Node {
                width: percent(100),
                flex_shrink: 0.0,
                flex_direction: if m.stacked() {
                    FlexDirection::Column
                } else {
                    FlexDirection::Row
                },
                column_gap: px(m.pad),
                row_gap: px(m.pad),
                align_items: AlignItems::Start,
                ..default()
            },
            Pickable::IGNORE,
        ))
        .id();
    let labels = row(commands, m, false);
    commands
        .entity(labels)
        .entry::<Node>()
        .and_modify(move |mut n| {
            n.column_gap = px(m.pad);
            n.height = px(m.head * 1.5);
            n.flex_shrink = 0.0;
            n.align_items = AlignItems::Start;
        });
    let label = heading(commands, fonts, m, Phrase::RoomRules.text(lang));
    commands.entity(label).entry::<Node>().and_modify(|mut n| {
        n.width = px(365);
        n.flex_shrink = 0.0;
    });
    commands.entity(labels).add_child(label);
    add_heading(commands, labels, fonts, m, Phrase::RoomPlayers.text(lang));
    if !m.stacked() {
        commands.entity(page).add_child(labels);
    }
    commands.entity(page).add_child(columns);
    let rules = panel(
        commands,
        m,
        if m.stacked() { percent(100) } else { px(365) },
        0.0,
    );
    commands.entity(rules).entry::<Node>().and_modify(|mut n| {
        n.min_height = px(0);
        n.flex_shrink = 0.0;
    });
    commands.entity(rules).insert(super::dock::Dock(3));
    commands.entity(columns).add_child(rules);

    if game.yours {
        field(
            commands,
            rules,
            state,
            fonts,
            m,
            Phrase::RoomName.text(lang),
            Field::RoomName,
            false,
        );
        if !lobby.offline() {
            field(
                commands,
                rules,
                state,
                fonts,
                m,
                Phrase::RoomPassword.text(lang),
                Field::RoomPassword,
                true,
            );
            add_note(commands, rules, fonts, m, Phrase::RoomLockHelp.text(lang));
        }
        add_note(commands, rules, fonts, m, Phrase::RoomTemplate.text(lang));
        let presets = row(commands, m, true);
        for (label, value) in [
            ("Commander", 0),
            ("Duel · 20", 1),
            (Phrase::RoomFast.text(lang), 2),
        ] {
            let b = chip(
                commands,
                fonts,
                m,
                label,
                Press::RoomAdjust(Adjustment::Template(value)),
                false,
            );
            commands.entity(presets).add_child(b);
        }
        commands.entity(rules).add_child(presets);
        stepper(
            commands,
            rules,
            fonts,
            m,
            &Phrase::PlayerCount.fill(lang, &[&chairs.to_string()]),
            Adjustment::Chairs(false),
            Adjustment::Chairs(true),
            chairs > 2,
            chairs < 8,
        );
        stepper(
            commands,
            rules,
            fonts,
            m,
            &format!("{} · {}", Phrase::RoomLife.text(lang), setup.starting_life),
            Adjustment::Life(-1),
            Adjustment::Life(1),
            setup.starting_life > 1,
            setup.starting_life < 999,
        );
        stepper(
            commands,
            rules,
            fonts,
            m,
            &format!(
                "{} · {}",
                Phrase::RoomMulligans.text(lang),
                setup.free_mulligans
            ),
            Adjustment::Mulligans(false),
            Adjustment::Mulligans(true),
            setup.free_mulligans > 0,
            setup.free_mulligans < 7,
        );
        let apply = button(
            commands,
            fonts,
            m,
            Phrase::RoomApply.text(lang),
            Press::SaveRoom(false),
            palette::ACCENT,
            !lobby.busy(),
        );
        commands.entity(rules).add_child(apply);
        if game.locked {
            let unlock = button(
                commands,
                fonts,
                m,
                Phrase::RoomUnlock.text(lang),
                Press::SaveRoom(true),
                palette::PANEL_LIT,
                !lobby.busy(),
            );
            commands.entity(rules).add_child(unlock);
        }
    } else {
        add_note(
            commands,
            rules,
            fonts,
            m,
            &format!(
                "{} · {}\n{} · {}",
                Phrase::RoomLife.text(lang),
                setup.starting_life,
                Phrase::RoomMulligans.text(lang),
                setup.free_mulligans
            ),
        );
    }
    add_note(commands, rules, fonts, m, Phrase::RoomPlanechase.text(lang));
    add_note(commands, rules, fonts, m, Phrase::RoomSuccession.text(lang));
    let seats = commands
        .spawn((
            Node {
                flex_direction: FlexDirection::Column,
                width: if m.stacked() { percent(100) } else { px(0) },
                min_width: px(0),
                flex_grow: 1.0,
                row_gap: px(m.gap),
                ..default()
            },
            Pickable::IGNORE,
        ))
        .id();
    commands.entity(columns).add_child(seats);

    let actions = row(commands, m, true);
    commands
        .entity(actions)
        .entry::<Node>()
        .and_modify(|mut n| {
            n.width = Val::Auto;
            n.margin.left = Val::Auto;
        });
    if !game.yours {
        let ready = game.i_am_ready();
        let own_deck = game.seats.iter().any(|s| s.you && !s.deck.is_empty());
        let b = button(
            commands,
            fonts,
            m,
            if ready {
                Phrase::NotReady
            } else {
                Phrase::Ready
            }
            .text(lang),
            Press::Ready(index, !ready),
            palette::ACCENT,
            !lobby.busy() && own_deck,
        );
        commands.entity(actions).add_child(b);
    }
    if game.yours {
        let b = button(
            commands,
            fonts,
            m,
            Phrase::Start.text(lang),
            Press::StartRoom(index),
            palette::ACCENT,
            game.startable && !lobby.busy() && !lobby.room_dirty(),
        );
        commands.entity(actions).add_child(b);
    }
    commands.entity(title).add_child(actions);

    for seat in &game.seats {
        seat_card(commands, seats, state, fonts, m, index, seat);
    }
}

#[allow(clippy::too_many_lines)] // one seat's controls, grouped by authority
fn seat_card(
    commands: &mut Commands,
    parent: Entity,
    state: &LobbyState,
    fonts: &UiFonts,
    m: Metrics,
    index: usize,
    seat: &client_core::lobby::GameSeat,
) {
    let lobby = &state.lobby;
    let game = &lobby.games()[index];
    let lang = lobby.lang();
    let card = panel(commands, m, percent(100), 0.0);
    commands
        .entity(card)
        .entry::<Node>()
        .and_modify(move |mut n| {
            n.min_height = px(0);
            n.padding = UiRect::all(px(m.pad));
        });
    commands.entity(card).insert(super::dock::Dock(4));
    commands.entity(parent).add_child(card);
    let who = if seat.kind == SeatKind::Ai {
        super::ui::ai_name(lang, seat.ai.as_deref().unwrap_or("steady"))
    } else {
        seat.player
            .as_deref()
            .unwrap_or(Phrase::StateWaiting.text(lang))
    };
    add_heading(
        commands,
        card,
        fonts,
        m,
        &format!(
            "{:02}  ·  {}{}",
            seat.seat + 1,
            who,
            if seat.host { " · Host" } else { "" }
        ),
    );
    let status = if seat.ready {
        Phrase::Ready
    } else {
        Phrase::NotReady
    };
    let readiness = chip(
        commands,
        fonts,
        m,
        status.text(lang),
        Press::PickerNothing,
        seat.ready,
    );
    commands
        .entity(readiness)
        .insert(BackgroundColor(if seat.ready {
            palette::ACTIVE.with_alpha(0.22)
        } else {
            palette::PANEL_LIT
        }));
    commands.entity(card).add_child(readiness);
    commands
        .entity(readiness)
        .entry::<Node>()
        .and_modify(|mut n| {
            n.align_self = AlignSelf::Start;
        });
    add_note(
        commands,
        card,
        fonts,
        m,
        &if seat.deck.is_empty() {
            Phrase::RoomNoDeck.text(lang).to_string()
        } else {
            format!("{} · {}", seat.deck, seat.format)
        },
    );
    let tools = row(commands, m, true);
    if game.yours {
        let next = seat.team.map_or(1, |t| {
            if usize::from(t) >= game.seats.len() {
                0
            } else {
                t + 1
            }
        });
        let label = seat.team.map_or_else(
            || Phrase::SeatSideNone.text(lang).to_string(),
            |t| Phrase::SeatSide.fill(lang, &[&t.to_string()]),
        );
        let team = chip(
            commands,
            fonts,
            m,
            &label,
            Press::SeatTeam(index, seat.seat, next),
            seat.team.is_some(),
        );
        commands.entity(tools).add_child(team);
        if !seat.taken {
            let (kind, label) = if seat.kind == SeatKind::Ai {
                (SeatKind::Human, Phrase::SeatToOpen)
            } else {
                (SeatKind::Ai, Phrase::SeatToAi)
            };
            if !lobby.offline() {
                let b = chip(
                    commands,
                    fonts,
                    m,
                    label.text(lang),
                    Press::SeatKind(index, seat.seat, kind),
                    false,
                );
                commands.entity(tools).add_child(b);
            }
        }
        if !seat.you && seat.taken {
            let b = chip(
                commands,
                fonts,
                m,
                Phrase::MakeHost.text(lang),
                Press::HandOver(index, seat.seat),
                false,
            );
            commands.entity(tools).add_child(b);
        }
    } else if let Some(team) = seat.team {
        add_note(
            commands,
            tools,
            fonts,
            m,
            &Phrase::SeatSide.fill(lang, &[&team.to_string()]),
        );
    }
    commands.entity(card).add_child(tools);
    if game.yours && seat.kind == SeatKind::Ai {
        let ai = row(commands, m, true);
        for (name, _) in baylee_core::preset::AIProfile::NAMED {
            let b = chip(
                commands,
                fonts,
                m,
                super::ui::ai_name(lang, name),
                Press::SeatAi(index, seat.seat, name),
                seat.ai.as_deref() == Some(name),
            );
            commands.entity(ai).add_child(b);
        }
        commands.entity(card).add_child(ai);
    }
    if seat.you || (game.yours && seat.kind == SeatKind::Ai) {
        let b = button(
            commands,
            fonts,
            m,
            Phrase::RoomPickDeck.text(lang),
            Press::RoomDeckPicker(seat.seat),
            palette::PANEL_LIT,
            !lobby.busy(),
        );
        commands.entity(card).add_child(b);
        if state.room_deck_seat == Some(seat.seat) {
            let picks = row(commands, m, true);
            for (at, deck) in lobby.decks().iter().enumerate() {
                let b = button(
                    commands,
                    fonts,
                    m,
                    &format!("{} · {}", deck.name, deck.format),
                    Press::RoomDeck(index, seat.seat, at),
                    palette::PANEL_LIT,
                    !lobby.busy(),
                );
                commands.entity(picks).add_child(b);
            }
            if lobby.decks().is_empty() {
                let b = button(
                    commands,
                    fonts,
                    m,
                    Phrase::HouseDecks.text(lang),
                    Press::BrowseHouse,
                    palette::ACCENT,
                    !lobby.busy(),
                );
                commands.entity(picks).add_child(b);
            }
            commands.entity(card).add_child(picks);
        }
    }
    let setup = if game.yours {
        lobby.room_draft().map_or(&game.setup, |d| &d.setup)
    } else {
        &game.setup
    };
    let personal = setup.seats.get(seat.seat as usize);
    let life = personal.and_then(|s| s.life).unwrap_or(setup.starting_life);
    if game.yours {
        stepper(
            commands,
            card,
            fonts,
            m,
            &format!("{} · {life}", Phrase::RoomLife.text(lang)),
            Adjustment::SeatLife(seat.seat as u8, -1),
            Adjustment::SeatLife(seat.seat as u8, 1),
            life > 1,
            life < 999,
        );
    } else {
        add_note(
            commands,
            card,
            fonts,
            m,
            &format!("{} · {life}", Phrase::RoomLife.text(lang)),
        );
    }
    cards::draw(
        commands,
        card,
        state,
        fonts,
        m,
        seat.seat as u8,
        personal,
        game.yours,
    );
}

fn add_heading(commands: &mut Commands, parent: Entity, fonts: &UiFonts, m: Metrics, text: &str) {
    let e = heading(commands, fonts, m, text);
    commands.entity(parent).add_child(e);
}
fn add_note(commands: &mut Commands, parent: Entity, fonts: &UiFonts, m: Metrics, text: &str) {
    let e = note(commands, fonts, m, text);
    commands.entity(parent).add_child(e);
}
#[allow(clippy::too_many_arguments)] // paired buttons share one label
fn stepper(
    commands: &mut Commands,
    parent: Entity,
    fonts: &UiFonts,
    m: Metrics,
    label: &str,
    less: Adjustment,
    more: Adjustment,
    can_less: bool,
    can_more: bool,
) {
    let line = row(commands, m, false);
    let caption = note(commands, fonts, m, label);
    commands
        .entity(caption)
        .entry::<Node>()
        .and_modify(|mut n| {
            n.flex_grow = 1.0;
            n.min_width = px(0);
        });
    commands.entity(line).add_child(caption);
    for (text, press, enabled) in [("−", less, can_less), ("+", more, can_more)] {
        let b = button(
            commands,
            fonts,
            m,
            text,
            Press::RoomAdjust(press),
            palette::PANEL_LIT,
            enabled,
        );
        commands.entity(b).entry::<Node>().and_modify(move |mut n| {
            n.width = px(m.tap);
            n.flex_shrink = 0.0;
        });
        commands.entity(line).add_child(b);
    }
    commands.entity(parent).add_child(line);
}
#[allow(clippy::too_many_arguments)] // standard field with room ownership
fn field(
    commands: &mut Commands,
    parent: Entity,
    state: &LobbyState,
    fonts: &UiFonts,
    m: Metrics,
    label: &str,
    field: Field,
    secret: bool,
) {
    let e = text_field(
        commands,
        fonts,
        m,
        label,
        &FieldLook {
            buffer: state.lobby.buffer(field),
            focused: state.lobby.focus() == field,
            mask: secret.then_some(Masked {
                field,
                shown: state.lobby.showing(field),
            }),
            press: Press::Focus(field),
            lead: None,
            hint: None,
            tail: None,
        },
    );
    commands.entity(parent).add_child(e);
}
