//! A dedicated waiting room: host rules beside each player's own deck choice.
use super::press::Cx;
#[allow(clippy::wildcard_imports)]
use super::*;
use super::{FieldLook, Masked, button, chip, heading, note, panel, row, text_field};
use baylee_client_core::lobby::room::Adjustment;
mod cards;
mod llm;

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
            row_gap: px(m.gap),
            padding: UiRect::all(px(m.pad)),
            overflow: if m.frame == Frame::Compact {
                Overflow::visible()
            } else {
                Overflow::scroll_y()
            },
            ..default()
        })
        .id();
    if m.frame != Frame::Compact {
        commands.entity(page).insert((
            Scrollable(List::Table),
            ScrollPosition(Vec2::new(0.0, scroll.get(List::Table))),
        ));
    }
    let title = row(commands, m, true);
    commands
        .entity(title)
        .entry::<Node>()
        .and_modify(move |mut n| {
            n.padding = UiRect::all(px(m.pad));
            n.flex_shrink = 0.0;
        });
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
        Press::Room(RoomPress::LeaveTable(index)),
        palette::PANEL_LIT,
        !lobby.busy(),
    );
    commands.entity(title).add_child(leave);
    // Keep the room's primary action reachable while scrolling many seats
    // or an expanded starting-position editor.
    commands.entity(root).add_child(title);
    if m.frame == Frame::Compact {
        commands.entity(root).add_child(page);
    } else {
        super::scrollbars::attach(commands, root, page, m);
    }
    let help = note(
        commands,
        fonts,
        m,
        if lobby.room_dirty() {
            Phrase::RoomDraft
        } else if lobby.offline() {
            Phrase::RoomOfflineHelp
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
                Press::Room(RoomPress::RoomAdjust(Adjustment::Template(value))),
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
            Press::Room(RoomPress::SaveRoom(false)),
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
                Press::Room(RoomPress::SaveRoom(true)),
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
    if !lobby.offline() {
        add_note(commands, rules, fonts, m, Phrase::RoomSuccession.text(lang));
    }
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
            Press::Room(RoomPress::Ready(index, !ready)),
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
            Press::Room(RoomPress::StartRoom(index)),
            palette::ACCENT,
            game.startable && !lobby.busy() && !lobby.room_dirty() && lobby.games_can_start(),
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
        super::ai_name(lang, seat.ai.as_deref().unwrap_or("steady")).to_string()
    } else {
        let player = seat
            .player
            .as_deref()
            .unwrap_or(Phrase::StateWaiting.text(lang));
        // A host's language model says whose it is: it sits on that
        // account's word, with no account of its own.
        match &seat.delegated_by {
            Some(host) => format!("{player} ({host})"),
            None => player.to_string(),
        }
    };
    let identity = row(commands, m, true);
    commands.entity(card).add_child(identity);
    add_heading(
        commands,
        identity,
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
    let readiness = note(commands, fonts, m, status.text(lang));
    commands.entity(readiness).insert((
        TextColor(palette::DOCK_INK),
        BackgroundColor(if seat.ready {
            palette::ACTIVE.with_alpha(0.22)
        } else {
            palette::PANEL_LIT
        }),
    ));
    commands.entity(identity).add_child(readiness);
    commands
        .entity(readiness)
        .entry::<Node>()
        .and_modify(|mut n| {
            n.padding = UiRect::axes(px(10), px(4));
            n.border_radius = BorderRadius::all(px(6));
        });
    let deck_line = row(commands, m, true);
    commands.entity(card).add_child(deck_line);
    add_note(
        commands,
        deck_line,
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
            Press::Room(RoomPress::SeatTeam(index, seat.seat, next)),
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
                    Press::Room(RoomPress::SeatKind(index, seat.seat, kind)),
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
                Press::Room(RoomPress::HandOver(index, seat.seat)),
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
        for (name, _) in baylee_core::preset::AIProfile::NAMED {
            let b = chip(
                commands,
                fonts,
                m,
                super::ai_name(lang, name),
                Press::Room(RoomPress::SeatAi(index, seat.seat, name)),
                seat.ai.as_deref() == Some(name),
            );
            commands.entity(tools).add_child(b);
        }
    }
    // A language model of this client's in this chair, or the chip that
    // seats one (`crate::tableseats`): the host's, on a desktop, at a
    // gateway.
    if game.yours && !lobby.offline() && crate::tableseats::available() {
        if state.llm.planned(seat.seat).is_some() {
            llm::editor(commands, card, state, fonts, m, index, seat.seat);
        } else if !seat.taken {
            llm::offer(commands, (tools, card), state, fonts, m, index, seat.seat);
        }
    }
    if seat.you || (game.yours && seat.kind == SeatKind::Ai) {
        let b = button(
            commands,
            fonts,
            m,
            Phrase::RoomPickDeck.text(lang),
            Press::Room(RoomPress::RoomDeckPicker(seat.seat)),
            palette::PANEL_LIT,
            !lobby.busy(),
        );
        commands.entity(deck_line).add_child(b);
        if state.room_deck_seat == Some(seat.seat) {
            let picks = row(commands, m, true);
            for (at, deck) in lobby.decks().iter().enumerate() {
                let b = button(
                    commands,
                    fonts,
                    m,
                    &format!("{} · {}", deck.name, deck.format),
                    Press::Room(RoomPress::RoomDeck(index, seat.seat, at)),
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
                    Press::Library(LibraryPress::BrowseHouse),
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
    let count = personal.map_or(0, |s| s.permanents.len());
    let expanded = state.room_setup_seat == Some(seat.seat as u8);
    if game.yours || count > 0 {
        let toggle = chip(
            commands,
            fonts,
            m,
            &format!(
                "{} {}",
                if expanded { "−" } else { "+" },
                Phrase::RoomStartingCards.fill(lang, &[&count.to_string()])
            ),
            Press::Room(RoomPress::RoomSetup(seat.seat as u8)),
            expanded,
        );
        commands.entity(toggle).entry::<Node>().and_modify(|mut n| {
            n.align_self = AlignSelf::Start;
        });
        commands.entity(card).add_child(toggle);
    }
    if expanded {
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
            Press::Room(RoomPress::RoomAdjust(press)),
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
                field: Some(field),
                shown: state.lobby.showing(field),
            }),
            press: Press::Shared(SharedPress::Focus(field)),
            lead: None,
            hint: None,
            tail: None,
        },
    );
    commands.entity(parent).add_child(e);
}

/// A control of a waiting room: its rules, its chairs and the board a
/// chair starts with.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum RoomPress {
    /// A language-model control of a chair: the room's index, the chair.
    RoomLlm(usize, u32, crate::tableseats::LlmPress),
    /// Edit the local room draft.
    RoomAdjust(baylee_client_core::lobby::room::Adjustment),
    /// Apply the host draft; true explicitly removes the password.
    SaveRoom(bool),
    /// Apply a chosen deck directly to this seat.
    RoomDeck(usize, u32, usize),
    /// Toggle the deck choices for a seat.
    RoomDeckPicker(u32),
    /// Expand or collapse a seat's optional starting-position editor.
    RoomSetup(u8),
    RoomCardAdd(u8, usize, bool),
    RoomCardRemove(u8, usize),
    RoomCardEdit(u8, usize),
    RoomCardPrint(u8, usize),
    RoomCounterAdd(u8, usize),
    RoomCounterStep(u8, usize, usize, i16),
    /// Give up a chair. The room outlives it.
    LeaveTable(usize),
    /// Say whether this player is ready at a listed table.
    Ready(usize, bool),
    /// Start a room this account hosts.
    StartRoom(usize),
    /// Hand the room to the player in a chair.
    HandOver(usize, u32),
    /// Make a chair a person's or the AI's.
    SeatKind(usize, u32, SeatKind),
    /// Set an AI chair's difficulty.
    SeatAi(usize, u32, &'static str),
    /// Move a chair onto a side. `0` puts it back on its own.
    SeatTeam(usize, u32, u8),
}

impl RoomPress {
    /// What a click on this control does.
    #[allow(clippy::too_many_lines)] // one flat match, read top to bottom
    pub(super) fn handle(self, cx: Cx<'_, '_, '_, '_, '_>) {
        let Cx { state, mailbox, .. } = cx;
        match self {
            RoomPress::RoomCardAdd(seat, slot, printing) => {
                let count = state
                    .lobby
                    .room_draft()
                    .and_then(|d| d.setup.seats.get(usize::from(seat)))
                    .map_or(0, |s| s.permanents.len());
                state.lobby.room_add_card(seat, slot);
                if printing {
                    let request = state.lobby.room_pick_print(seat, count);
                    dispatch(state, mailbox, request);
                }
            }
            RoomPress::RoomCardRemove(seat, at) => {
                state.room_card_edit = None;
                state.lobby.room_remove_card(seat, at);
            }
            RoomPress::RoomCardEdit(seat, at) => {
                state.room_card_edit = if state.room_card_edit == Some((seat, at)) {
                    None
                } else {
                    Some((seat, at))
                };
            }
            RoomPress::RoomCardPrint(seat, at) => {
                let request = state.lobby.room_pick_print(seat, at);
                dispatch(state, mailbox, request);
            }
            RoomPress::RoomCounterAdd(seat, at) => state.lobby.room_add_counter(seat, at),
            RoomPress::RoomCounterStep(seat, at, counter, delta) => {
                state.lobby.room_counter_step(seat, at, counter, delta);
            }
            RoomPress::RoomDeckPicker(seat) => {
                state.room_deck_seat = (state.room_deck_seat != Some(seat)).then_some(seat);
            }
            RoomPress::RoomSetup(seat) => {
                state.room_setup_seat = (state.room_setup_seat != Some(seat)).then_some(seat);
                state.room_card_edit = None;
                // Collapsing or switching editors must not leave a hidden
                // card search (or counter field) receiving keyboard input.
                if matches!(
                    state.lobby.focus(),
                    Field::RoomBoard(_) | Field::RoomCounter
                ) {
                    state.lobby.focus_on(Field::RoomName);
                }
            }
            RoomPress::RoomAdjust(change) => state.lobby.adjust_room(change),
            RoomPress::SaveRoom(remove_password) => {
                let request = state.lobby.save_room(remove_password);
                dispatch(state, mailbox, request);
            }
            RoomPress::RoomDeck(index, seat, deck) => {
                state.lobby.select_deck(deck);
                state.room_deck_seat = None;
                if let Some(game) = state.lobby.games().get(index).map(|g| g.id.clone()) {
                    let request = state.lobby.seat_deck(&game, seat);
                    dispatch(state, mailbox, request);
                }
            }
            RoomPress::LeaveTable(index) => {
                let game = state.lobby.games().get(index).map(|g| g.id.clone());
                if let Some(game) = game {
                    let request = state.lobby.leave_table(&game);
                    dispatch(state, mailbox, request);
                }
            }
            RoomPress::Ready(index, ready) => {
                let game = state.lobby.games().get(index).map(|g| g.id.clone());
                if let Some(game) = game {
                    let request = state.lobby.set_ready(&game, ready);
                    dispatch(state, mailbox, request);
                }
            }
            RoomPress::StartRoom(index) => {
                let game = state.lobby.games().get(index).map(|g| g.id.clone());
                if let Some(game) = game {
                    let request = state.lobby.start_room(&game);
                    dispatch(state, mailbox, request);
                }
            }
            RoomPress::HandOver(index, seat) => {
                let game = state.lobby.games().get(index).map(|g| g.id.clone());
                if let Some(game) = game {
                    let request = state.lobby.hand_over(&game, seat);
                    dispatch(state, mailbox, request);
                }
            }
            RoomPress::RoomLlm(index, seat, press) => {
                let found = state.lobby.games().get(index).map(|g| {
                    let phase = if g.state == "waiting" {
                        crate::tableseats::Phase::Waiting
                    } else {
                        crate::tableseats::Phase::Playing
                    };
                    let house = g
                        .seats
                        .iter()
                        .any(|s| s.seat == seat && s.kind == SeatKind::Ai);
                    (g.id.clone(), phase, house)
                });
                if let Some((game, phase, house)) = found {
                    let open_it = state.llm.press(seat, press, phase, "steady");
                    // The house's chair is the gateway's: open it, and the
                    // bridge takes it once the room lists it open.
                    if open_it && house {
                        let request =
                            state
                                .lobby
                                .set_seat(&game, seat, Some(SeatKind::Human), None);
                        dispatch(state, mailbox, request);
                    }
                }
            }
            RoomPress::SeatKind(index, seat, kind) => {
                let game = state.lobby.games().get(index).map(|g| g.id.clone());
                if let Some(game) = game {
                    // Open or the house's: no language model of ours here.
                    state.llm.unplan(seat);
                    let request = state.lobby.set_seat(&game, seat, Some(kind), None);
                    dispatch(state, mailbox, request);
                }
            }
            RoomPress::SeatAi(index, seat, profile) => {
                let game = state.lobby.games().get(index).map(|g| g.id.clone());
                if let Some(game) = game {
                    let request =
                        state
                            .lobby
                            .set_seat(&game, seat, None, Some(profile.to_string()));
                    dispatch(state, mailbox, request);
                }
            }
            RoomPress::SeatTeam(index, seat, team) => {
                let game = state.lobby.games().get(index).map(|g| g.id.clone());
                if let Some(game) = game {
                    let request = state.lobby.seat_team(&game, seat, team);
                    dispatch(state, mailbox, request);
                }
            }
        }
    }
}
