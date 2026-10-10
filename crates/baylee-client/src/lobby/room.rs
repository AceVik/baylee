//! A dedicated waiting room: host rules beside each player's own deck choice.
use super::menus::ShellMenu;
use super::press::Cx;
#[allow(clippy::wildcard_imports)]
use super::*;
use super::{FieldLook, Masked, button, heading, note, row, text_field};
use crate::shellkit::controls::{self, Kit, Live, Weight};
use crate::shellkit::surfaces::MenuItem;
use crate::shellkit::{Frame as ShellFrame, Role, px_fixed, tokens};
use crate::tour::TourAnchor;
use baylee_client_core::lobby::room::Adjustment;
use baylee_client_core::tour::Anchor;
mod cards;
mod llm;

/// The room (the shell design, §5; WP2): the title row with Copy invite,
/// Leave and Start; the rules rail with Edit rules and Set teams; one card
/// per seat. `Esc` never leaves it (§9.6); the header's nav steps away and
/// keeps the seat (M-7).
#[allow(clippy::too_many_lines, clippy::too_many_arguments)] // the page is read in visual order
pub(super) fn draw(
    commands: &mut Commands,
    root: Entity,
    state: &LobbyState,
    fonts: &UiFonts,
    m: Metrics,
    kit: Kit,
    scroll: &Scrolled,
    index: usize,
) {
    let lobby = &state.lobby;
    let lang = lobby.lang();
    let game = &lobby.games()[index];
    let phone = kit.m.frame == ShellFrame::Phone;
    let side = phone || matches!(kit.m.frame, ShellFrame::Wide | ShellFrame::Vast);
    let draft = lobby.room_draft();
    let setup = if game.yours {
        draft.map_or(&game.setup, |d| &d.setup)
    } else {
        &game.setup
    };

    // ---- the title row, on a plate of its own
    let title = commands
        .spawn((
            Role::Panel,
            Node {
                align_items: AlignItems::Center,
                column_gap: px_fixed(kit.m.gap),
                row_gap: kit.m.px(6.0),
                flex_wrap: if phone {
                    FlexWrap::NoWrap
                } else {
                    FlexWrap::Wrap
                },
                flex_shrink: 0.0,
                padding: UiRect::axes(px_fixed(kit.m.pad), kit.m.px(8.0)),
                margin: UiRect::axes(px_fixed(kit.m.body), kit.m.px(8.0)),
                border: UiRect::all(px_fixed(1.0)),
                border_radius: BorderRadius::all(px_fixed(tokens::RADIUS_PANEL)),
                ..default()
            },
            BackgroundColor(tokens::PANEL),
            BorderColor::all(tokens::BORDER),
        ))
        .id();
    commands.entity(root).add_child(title);
    let back = controls::button(
        commands,
        kit,
        &format!("\u{2039} {}", Phrase::ShellPlay.text(lang)),
        Weight::Ghost,
        Live::Yes,
        None,
        Press::Room(RoomPress::StepAway),
    );
    let name = if game.name.is_empty() {
        Phrase::RoomTitle.text(lang)
    } else {
        &game.name
    };
    let heading = commands
        .spawn((
            Text::new(name),
            crate::hud::tf(
                kit.fonts,
                if phone {
                    20.0_f32.max(kit.m.text)
                } else {
                    kit.m.h1
                },
            ),
            TextColor(tokens::INK),
            TextLayout::no_wrap(),
            Node {
                flex_shrink: 1.0,
                min_width: px(0),
                overflow: Overflow::clip_x(),
                ..default()
            },
            Pickable::IGNORE,
        ))
        .id();
    super::orders::stop(commands, back, &super::orders::ROOM, "back");
    commands.entity(title).add_children(&[back, heading]);
    if !phone {
        if let Some(format) = client_core::lobby::play::host_format(game) {
            let b = super::parts::badge(
                commands,
                kit,
                &client_core::lobby::shelf::format_label(lang, format),
                tokens::ACCENT,
            );
            commands.entity(title).add_child(b);
        }
        if game.locked {
            let b = super::parts::badge_with(
                commands,
                kit,
                super::parts::LOCK,
                Phrase::RoomPasswordSet.text(lang),
                tokens::GOLD,
            );
            commands.entity(title).add_child(b);
        }
    }
    let gap = super::parts::grow(commands);
    commands.entity(title).add_child(gap);
    let reason = start_reason(state, game, lang);
    if !phone && let Some(reason) = &reason {
        let why = commands
            .spawn((
                Text::new(reason.clone()),
                crate::hud::tf_italic(kit.fonts, kit.m.small),
                TextColor(tokens::MUTED),
                Pickable::IGNORE,
            ))
            .id();
        commands.entity(title).add_child(why);
    }
    let leave = controls::button(
        commands,
        kit,
        Phrase::Leave.text(lang),
        if game.yours {
            Weight::Danger
        } else {
            Weight::Secondary
        },
        if lobby.busy() {
            Live::No(Phrase::VeilTalking.text(lang))
        } else {
            Live::Yes
        },
        None,
        Press::Room(RoomPress::LeaveTable(index)),
    );
    super::orders::stop(commands, leave, &super::orders::ROOM, "leave");
    commands.entity(title).add_child(leave);
    if !lobby.offline() && !phone {
        let invite = controls::button(
            commands,
            kit,
            if state.invite_copied {
                Phrase::RoomInviteCopied
            } else {
                Phrase::RoomCopyInvite
            }
            .text(lang),
            Weight::Secondary,
            Live::Yes,
            None,
            Press::Room(RoomPress::CopyInvite(index)),
        );
        super::orders::stop(commands, invite, &super::orders::ROOM, "copy-invite");
        commands
            .entity(invite)
            .insert(TourAnchor(Anchor::RoomInvite));
        commands.entity(title).add_child(invite);
    }
    if game.yours {
        let start = controls::button(
            commands,
            kit,
            Phrase::Start.text(lang),
            Weight::Primary,
            match &reason {
                Some(why) => Live::No(why),
                None => Live::Yes,
            },
            Some(if crate::shellkit::keys::mac() {
                "Cmd+Enter"
            } else {
                "Ctrl+Enter"
            }),
            Press::Room(RoomPress::StartRoom(index)),
        );
        super::orders::stop(commands, start, &super::orders::ROOM, "start");
        commands.entity(start).insert(TourAnchor(Anchor::RoomStart));
        commands.entity(title).add_child(start);
    }

    // ---- the rail and the seats
    let columns = commands
        .spawn((
            Node {
                width: percent(100),
                flex_grow: 1.0,
                min_height: px(0),
                flex_direction: if side {
                    FlexDirection::Row
                } else {
                    FlexDirection::Column
                },
                column_gap: px_fixed(kit.m.body),
                row_gap: px_fixed(kit.m.gap),
                align_items: AlignItems::Start,
                padding: UiRect::axes(px_fixed(kit.m.body), px(0)),
                overflow: Overflow::scroll_y(),
                ..default()
            },
            Scrollable(List::Table),
            ScrollPosition(Vec2::new(0.0, scroll.get(List::Table))),
        ))
        .id();
    commands.entity(root).add_child(columns);
    let rail = commands
        .spawn((
            Role::Panel,
            Node {
                width: if phone {
                    px(220)
                } else if side {
                    percent(26)
                } else {
                    percent(100)
                },
                max_width: if side && !phone {
                    kit.m.px(380.0)
                } else {
                    Val::Auto
                },
                flex_shrink: 0.0,
                flex_direction: FlexDirection::Column,
                row_gap: kit.m.px(8.0),
                padding: UiRect::all(px_fixed(kit.m.pad)),
                border: UiRect::all(px_fixed(1.0)),
                border_radius: BorderRadius::all(px_fixed(tokens::RADIUS_PANEL)),
                ..default()
            },
            BackgroundColor(tokens::PANEL),
            BorderColor::all(tokens::BORDER),
            super::dock::Dock(3),
            TourAnchor(Anchor::RoomRules),
        ))
        .id();
    commands.entity(columns).add_child(rail);
    let caption = super::parts::caption(commands, kit, Phrase::RoomRules.text(lang));
    commands.entity(rail).add_child(caption);
    let chairs = draft.map_or(game.seats.len(), |d| d.chairs);
    let format = client_core::lobby::play::host_format(game).map_or_else(
        || Phrase::FormatFreeform.text(lang).to_string(),
        |f| client_core::lobby::shelf::format_label(lang, f),
    );
    let lines = [
        Phrase::RoomRulesPlayers.fill(lang, &[&format, &chairs.to_string()]),
        format!(
            "{} · {}",
            Phrase::RulesLife.fill(lang, &[&setup.starting_life.to_string()]),
            Phrase::counted(
                usize::from(setup.free_mulligans),
                Phrase::RulesMulliganOne,
                Phrase::RulesMulliganMany,
            )
            .fill(lang, &[&setup.free_mulligans.to_string()]),
        ),
        {
            let clock = game.clock.map_or_else(
                || Phrase::ClockCasual.text(lang).to_string(),
                |c| client_core::lobby::play::table_clock_label(lang, &lobby.clocks(), c),
            );
            if game.locked {
                format!("{clock} · {}", Phrase::RoomPasswordSet.text(lang))
            } else {
                clock
            }
        },
    ];
    for line in lines {
        let l = super::parts::line(commands, kit, &line, kit.m.text, tokens::INK);
        commands.entity(rail).add_child(l);
    }
    if game.yours {
        let edit = controls::button(
            commands,
            kit,
            Phrase::SheetEditRules.text(lang),
            Weight::Secondary,
            Live::Yes,
            None,
            Press::Room(RoomPress::EditRules),
        );
        super::orders::stop(commands, edit, &super::orders::ROOM, "edit-rules");
        commands.entity(rail).add_child(edit);
    }
    let rule = commands
        .spawn((
            Node {
                height: px(1),
                width: percent(100),
                ..default()
            },
            BackgroundColor(tokens::BORDER),
            Pickable::IGNORE,
        ))
        .id();
    commands.entity(rail).add_child(rule);
    let teams_on = game.seats.iter().any(|s| s.team.is_some());
    let teams = super::parts::row(commands, kit, true);
    let said = super::parts::line(
        commands,
        kit,
        if teams_on {
            Phrase::RoomTeamsOn.text(lang)
        } else {
            Phrase::RoomTeamsOff.text(lang)
        },
        kit.m.small,
        tokens::MUTED,
    );
    commands.entity(teams).add_child(said);
    if game.yours {
        let set = controls::button(
            commands,
            kit,
            Phrase::RoomSetTeams.text(lang),
            Weight::Ghost,
            Live::Yes,
            None,
            Press::Room(RoomPress::SetTeams),
        );
        super::orders::stop(commands, set, &super::orders::ROOM, "set-teams");
        commands.entity(teams).add_child(set);
    }
    commands.entity(rail).add_child(teams);
    if !phone {
        for note in [
            Some(Phrase::RoomPlanechase),
            (!lobby.offline()).then_some(Phrase::RoomSuccession),
        ]
        .into_iter()
        .flatten()
        {
            let l = super::parts::line(commands, kit, note.text(lang), kit.m.small, tokens::MUTED);
            commands.entity(rail).add_child(l);
        }
    }
    if phone && let Some(reason) = &reason {
        let why = super::parts::line(commands, kit, reason, kit.m.small, tokens::GOLD);
        commands.entity(rail).add_child(why);
    }

    let seats = commands
        .spawn((
            Node {
                flex_direction: FlexDirection::Column,
                flex_grow: 1.0,
                flex_basis: px(0),
                min_width: px(0),
                width: if side { Val::Auto } else { percent(100) },
                row_gap: kit.m.px(8.0),
                ..default()
            },
            Pickable::IGNORE,
            TourAnchor(Anchor::RoomSeats),
        ))
        .id();
    commands.entity(columns).add_child(seats);
    let mut anchors = Vec::new();
    let mut walked = 0;
    for seat in &game.seats {
        if let Some(anchor) = seat_card(
            commands,
            seats,
            state,
            fonts,
            m,
            kit,
            index,
            seat,
            &mut walked,
        ) {
            anchors.push(anchor);
        }
    }
    // The open seat menu, hung from its `⋯` or caret.
    for (seat, anchor, ai) in anchors {
        if ai && state.menu == Some(ShellMenu::SeatAi(seat)) {
            let items: Vec<_> = baylee_core::preset::AIProfile::NAMED
                .iter()
                .map(|(name, _)| {
                    super::menus::item(
                        super::ai_name(lang, name),
                        Press::Room(RoomPress::SeatAiOpen(index, u32::from(seat), name)),
                    )
                })
                .collect();
            super::menus::draw(commands, root, kit, anchor, items);
        }
        if !ai && state.menu == Some(ShellMenu::Seat(seat)) {
            let items = seat_menu(state, index, seat);
            super::menus::draw(commands, root, kit, anchor, items);
        }
    }
    if let Some(chair) = state.chair_sheet.filter(|_| state.hosted_sheet) {
        hosted_sheet(commands, root, state, kit, index, chair);
    } else if let Some(chair) = state.chair_sheet {
        chair_sheet(commands, root, state, fonts, m, kit, index, chair);
    }
    super::play::sheet_over(commands, root, state, m, kit);
}

/// Why Start does nothing yet, said where the host looks: the chairs not
/// ready by name, the rules not applied, the gateway without a game host.
fn start_reason(state: &LobbyState, game: &GameSummary, lang: Lang) -> Option<String> {
    let lobby = &state.lobby;
    if !game.yours {
        return None;
    }
    if !lobby.games_can_start() {
        return Some(Phrase::PlayNoHost.text(lang).to_string());
    }
    if lobby.room_dirty() {
        return Some(Phrase::RoomDraft.text(lang).to_string());
    }
    let open = game.seats.iter().filter(|s| s.open()).count();
    if open > 0 {
        return Some(
            Phrase::counted(open, Phrase::RoomOpenSeatOne, Phrase::RoomOpenSeatMany)
                .fill(lang, &[&open.to_string()]),
        );
    }
    let waiting: Vec<String> = game
        .seats
        .iter()
        .filter(|s| !s.host && s.kind == SeatKind::Human && s.taken && !s.ready)
        .filter_map(|s| s.player.clone())
        .collect();
    if !waiting.is_empty() {
        return Some(Phrase::RoomWaitingFor.fill(lang, &[&waiting.join(", ")]));
    }
    if !game.startable {
        return Some(Phrase::RoomNotStartable.text(lang).to_string());
    }
    if lobby.busy() {
        return Some(Phrase::VeilTalking.text(lang).to_string());
    }
    None
}

/// The press `Ctrl/Cmd+Enter` makes in a room: Start, for its host.
pub(super) fn start_press(state: &LobbyState) -> Option<Press> {
    let handover = state.lobby.awaiting()?;
    let index = state
        .lobby
        .games()
        .iter()
        .position(|g| g.id == handover.game_id && g.state == "waiting")?;
    let game = &state.lobby.games()[index];
    (game.yours && start_reason(state, game, state.lobby.lang()).is_none())
        .then_some(Press::Room(RoomPress::StartRoom(index)))
}

/// A seat `⋯`'s items, per chair kind (§5): a person — Make host, Team,
/// Life and starting position; the house or a language model — the same
/// two, and Make it an open seat.
fn seat_menu(state: &LobbyState, index: usize, seat: u8) -> Vec<MenuItem<'static, Press>> {
    let lobby = &state.lobby;
    let lang = lobby.lang();
    let Some(game) = lobby.games().get(index) else {
        return Vec::new();
    };
    let Some(chair) = game.seats.iter().find(|s| s.seat == u32::from(seat)) else {
        return Vec::new();
    };
    let mut items = Vec::new();
    // A hosted model on its way is not a person to hand the room to.
    let hosted = chair
        .hosted
        .as_ref()
        .is_some_and(client_core::lobby::hosted::HostedChair::standing);
    let person =
        chair.kind == SeatKind::Human && chair.taken && chair.delegated_by.is_none() && !hosted;
    if person && !chair.you {
        items.push(super::menus::item(
            Phrase::RoomMakeHost.text(lang),
            Press::Room(RoomPress::HandOver(index, chair.seat)),
        ));
    }
    if game.seats.iter().any(|s| s.team.is_some()) {
        let next = chair.team.map_or(1, |t| {
            if usize::from(t) >= game.seats.len() {
                0
            } else {
                t + 1
            }
        });
        items.push(super::menus::item(
            Phrase::RoomNextTeam.text(lang),
            Press::Room(RoomPress::SeatTeam(index, chair.seat, next)),
        ));
    }
    items.push(super::menus::item(
        Phrase::RoomLifeOverride.text(lang),
        Press::Room(RoomPress::LifeOverride(seat)),
    ));
    items.push(super::menus::item(
        Phrase::RoomStartingPosition.text(lang),
        Press::Room(RoomPress::RoomSetup(seat)),
    ));
    if !person && !lobby.offline() {
        items.push(super::menus::danger(
            Phrase::RoomMakeOpen.text(lang),
            Press::Room(RoomPress::SeatKind(index, chair.seat, SeatKind::Human)),
        ));
    }
    items
}

/// One seat's card: number, who, deck, the fit or the warning, Ready on the
/// player's own, `⋯` for the host. Answers the anchor its open menu hangs
/// from: `(seat, entity, is the AI ▾)`.
#[allow(clippy::too_many_lines, clippy::too_many_arguments)] // one seat's card, in visual order
fn seat_card(
    commands: &mut Commands,
    parent: Entity,
    state: &LobbyState,
    fonts: &UiFonts,
    m: Metrics,
    kit: Kit,
    index: usize,
    seat: &client_core::lobby::GameSeat,
    walked: &mut usize,
) -> Option<(u8, Entity, bool)> {
    // Every control of every seat is one item of the seats' grid, in
    // reading order (`KEYBOARD.md` §1.4 "Grid of rows").
    let mut walk = |commands: &mut Commands, control: Entity| {
        super::orders::item(commands, control, &super::orders::ROOM, "seats", *walked);
        *walked += 1;
    };
    let lobby = &state.lobby;
    let game = &lobby.games()[index];
    let lang = lobby.lang();
    let phone = kit.m.frame == ShellFrame::Phone;
    let at = u8::try_from(seat.seat).unwrap_or(0);
    let mut anchor = None;
    let card = commands
        .spawn((
            Role::Row,
            Node {
                width: percent(100),
                flex_direction: FlexDirection::Column,
                row_gap: kit.m.px(6.0),
                padding: UiRect::axes(px_fixed(kit.m.pad), kit.m.px(8.0)),
                border: UiRect::all(px_fixed(1.0)),
                border_radius: BorderRadius::all(px_fixed(tokens::RADIUS_PANEL)),
                ..default()
            },
            BackgroundColor(tokens::PANEL),
            BorderColor::all(if seat.you {
                tokens::ACCENT
            } else {
                tokens::BORDER
            }),
            Pickable::IGNORE,
        ))
        .id();
    commands.entity(parent).add_child(card);
    let line = commands
        .spawn((
            Node {
                align_items: AlignItems::Center,
                column_gap: px_fixed(kit.m.gap),
                row_gap: kit.m.px(6.0),
                flex_wrap: if phone {
                    FlexWrap::Wrap
                } else {
                    FlexWrap::NoWrap
                },
                min_height: px_fixed(kit.m.hit),
                ..default()
            },
            Pickable::IGNORE,
        ))
        .id();
    commands.entity(card).add_child(line);
    // The chair's number in a ring.
    let number = commands
        .spawn((
            Node {
                width: kit.m.px(30.0),
                height: kit.m.px(30.0),
                flex_shrink: 0.0,
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                border: UiRect::all(px_fixed(1.0)),
                border_radius: BorderRadius::all(px_fixed(tokens::RADIUS_PILL)),
                ..default()
            },
            BorderColor::all(tokens::BORDER),
            Pickable::IGNORE,
        ))
        .id();
    let digit = super::parts::line(
        commands,
        kit,
        &(seat.seat + 1).to_string(),
        kit.m.small,
        tokens::INK,
    );
    commands.entity(number).add_child(digit);
    commands.entity(line).add_child(number);
    // Who sits here.
    let planned = state.llm.planned(seat.seat);
    let empty = seat.kind == SeatKind::Human && !seat.taken && planned.is_none();
    let who = commands
        .spawn((
            Node {
                flex_direction: FlexDirection::Column,
                width: kit.m.px(if phone { 120.0 } else { 170.0 }),
                flex_shrink: 0.0,
                ..default()
            },
            Pickable::IGNORE,
        ))
        .id();
    let (name, under): (String, Option<(String, Color)>) = if seat.kind == SeatKind::Ai {
        (
            super::ai_name(lang, seat.ai.as_deref().unwrap_or("steady")).to_string(),
            Some((Phrase::RoomHouseAi.text(lang).to_string(), tokens::MUTED)),
        )
    } else if let Some(hosted) = seat.hosted.as_ref().filter(|h| h.standing()) {
        (
            hosted.label.clone(),
            Some((
                format!("{} · {}", Phrase::HostedEntry.text(lang), hosted.vendor),
                tokens::MUTED,
            )),
        )
    } else if let Some(model) = planned.filter(|_| !seat.taken || seat.delegated_by.is_some()) {
        (
            model.model.clone(),
            Some((
                Phrase::RoomLanguageModel.text(lang).to_string(),
                tokens::MUTED,
            )),
        )
    } else if empty {
        (Phrase::RoomEmptySeat.text(lang).to_string(), None)
    } else {
        let player = seat.player.clone().unwrap_or_default();
        let under = if seat.host {
            Some((Phrase::RoomHost.text(lang).to_string(), tokens::GOLD))
        } else {
            seat.delegated_by
                .as_ref()
                .map(|by| (Phrase::RoomFor.fill(lang, &[by]), tokens::MUTED))
        };
        (player, under)
    };
    let title = commands
        .spawn((
            Text::new(name),
            crate::hud::tf_bold(kit.fonts, kit.m.text),
            TextColor(if empty { tokens::MUTED } else { tokens::INK }),
            TextLayout::no_wrap(),
            Pickable::IGNORE,
        ))
        .id();
    commands.entity(who).add_child(title);
    if let Some((said, ink)) = under {
        let l = super::parts::line(commands, kit, &said, kit.m.small, ink);
        commands.entity(who).add_child(l);
    }
    commands.entity(line).add_child(who);

    if empty {
        // An open chair: the host may give it to the house or a model.
        if game.yours && !lobby.offline() {
            let ai = super::parts::menu_button(
                commands,
                kit,
                Phrase::RoomAi.text(lang),
                Weight::Secondary,
                Press::Shared(SharedPress::OpenMenu(ShellMenu::SeatAi(at))),
            );
            walk(commands, ai);
            anchor = Some((at, ai, true));
            commands.entity(line).add_child(ai);
            // Desktop builds only (M-3): nothing to spawn elsewhere.
            if crate::tableseats::available() {
                let llm = super::parts::menu_button(
                    commands,
                    kit,
                    Phrase::RoomLanguageModel.text(lang),
                    Weight::Secondary,
                    Press::Room(RoomPress::OpenChair(index, seat.seat)),
                );
                walk(commands, llm);
                commands.entity(line).add_child(llm);
            }
            // A model the gateway runs: any platform, registered hosts only.
            if lobby.may_order_hosted() {
                let hosted = super::parts::menu_button(
                    commands,
                    kit,
                    Phrase::HostedEntry.text(lang),
                    Weight::Secondary,
                    Press::Room(RoomPress::HostedOpen(index, seat.seat)),
                );
                walk(commands, hosted);
                commands.entity(line).add_child(hosted);
            }
        }
        // An order that failed left the chair open: say why.
        if let Some(failed) = seat.hosted.as_ref().filter(|h| !h.standing()) {
            let said =
                super::parts::line(commands, kit, &failed.said(lang), kit.m.small, tokens::GOLD);
            commands.entity(card).add_child(said);
        }
        return anchor;
    }

    // The deck: my own a menu button, everyone else's words.
    let deck_line = commands
        .spawn((
            Node {
                align_items: AlignItems::Center,
                column_gap: kit.m.px(8.0),
                flex_grow: 1.0,
                flex_shrink: 1.0,
                min_width: px(0),
                ..default()
            },
            Pickable::IGNORE,
        ))
        .id();
    let mine = seat.you || (game.yours && seat.kind == SeatKind::Ai);
    if mine {
        let label = if seat.deck.is_empty() {
            Phrase::RoomPickDeck.text(lang).to_string()
        } else {
            seat.deck.clone()
        };
        let pick = super::parts::menu_button(
            commands,
            kit,
            &label,
            Weight::Secondary,
            Press::Room(RoomPress::RoomDeckPicker(seat.seat)),
        );
        walk(commands, pick);
        commands.entity(deck_line).add_child(pick);
    } else {
        let label = super::parts::line(
            commands,
            kit,
            Phrase::RoomDeckWord.text(lang),
            kit.m.small,
            tokens::MUTED,
        );
        let deck = super::parts::line(
            commands,
            kit,
            if seat.deck.is_empty() {
                Phrase::RoomNoDeck.text(lang)
            } else {
                &seat.deck
            },
            kit.m.text,
            tokens::INK,
        );
        commands.entity(deck_line).add_children(&[label, deck]);
    }
    // Fits or not: the seat's deck against the host's (S-4, heuristic 5).
    let host_format = client_core::lobby::play::host_format(game);
    if !seat.format.is_empty()
        && let Some(theirs) = host_format
    {
        let chip = if seat.format == theirs {
            super::parts::badge(commands, kit, Phrase::RoomFits.text(lang), tokens::ACCENT)
        } else {
            super::parts::badge_with(
                commands,
                kit,
                super::parts::WARNING,
                &Phrase::RoomOtherFormat.fill(
                    lang,
                    &[&client_core::lobby::shelf::format_label(lang, theirs)],
                ),
                tokens::GOLD,
            )
        };
        commands.entity(deck_line).add_child(chip);
    }
    let side = seat.team.map_or_else(
        || Phrase::SeatSideNone.text(lang).to_string(),
        |t| Phrase::SeatSide.fill(lang, &[&t.to_string()]),
    );
    if game.yours && state.teams_edit {
        let next = seat.team.map_or(1, |t| {
            if usize::from(t) >= game.seats.len() {
                0
            } else {
                t + 1
            }
        });
        let chip = controls::chip(
            commands,
            kit,
            &side,
            seat.team.is_some(),
            None,
            false,
            Press::Room(RoomPress::SeatTeam(index, seat.seat, next)),
        );
        walk(commands, chip);
        commands.entity(deck_line).add_child(chip);
    } else if seat.team.is_some() {
        let chip = super::parts::badge(commands, kit, &side, tokens::INK);
        commands.entity(deck_line).add_child(chip);
    }
    commands.entity(line).add_child(deck_line);
    // Ready: a large toggle on my own card (the host starts instead).
    if seat.you && !game.yours {
        let ready = game.i_am_ready();
        let own_deck = !seat.deck.is_empty();
        let words = super::parts::line(
            commands,
            kit,
            Phrase::Ready.text(lang),
            kit.m.text,
            tokens::INK,
        );
        let toggle = controls::toggle(
            commands,
            kit,
            ready,
            Press::Room(RoomPress::Ready(index, !ready)),
        );
        if !own_deck || lobby.busy() {
            commands
                .entity(toggle)
                .insert(crate::shellkit::controls::Disabled);
        }
        walk(commands, toggle);
        commands.entity(line).add_children(&[words, toggle]);
    }
    if game.yours {
        let more = super::parts::icon_button(
            commands,
            kit,
            super::parts::ELLIPSIS,
            Press::Shared(SharedPress::OpenMenu(ShellMenu::Seat(at))),
        );
        walk(commands, more);
        if seat.you {
            commands
                .entity(more)
                .insert(TourAnchor(Anchor::RoomMySeatMenu));
        }
        anchor = Some((at, more, false));
        commands.entity(line).add_child(more);
    }
    // The deck choices, unfolded under the card.
    if mine && state.room_deck_seat == Some(seat.seat) {
        let picks = super::parts::row(commands, kit, true);
        for (at, deck) in lobby.decks().iter().enumerate() {
            let b = controls::button(
                commands,
                kit,
                &deck.name,
                Weight::Secondary,
                Live::Yes,
                None,
                Press::Room(RoomPress::RoomDeck(index, seat.seat, at)),
            );
            commands.entity(picks).add_child(b);
        }
        commands.entity(card).add_child(picks);
    }
    // A language model of this client's in this chair: what it plays.
    if game.yours && state.llm.planned(seat.seat).is_some() {
        let said = state
            .llm
            .said(seat.seat)
            .unwrap_or_else(|| Phrase::RoomLlmStarting.text(lang).to_string());
        let status = super::parts::line(commands, kit, &said, kit.m.small, tokens::MUTED);
        let edit = controls::button(
            commands,
            kit,
            Phrase::ShellEdit.text(lang),
            Weight::Ghost,
            Live::Yes,
            None,
            Press::Room(RoomPress::OpenChair(index, seat.seat)),
        );
        let row_ = super::parts::row(commands, kit, true);
        commands.entity(row_).add_children(&[status, edit]);
        commands.entity(card).add_child(row_);
    }
    // A hosted model: its state and where the game's data goes, for
    // everyone at the table; Take back for the host.
    if let Some(hosted) = seat.hosted.as_ref().filter(|h| h.standing()) {
        let row_ = super::parts::row(commands, kit, true);
        let said = super::parts::line(commands, kit, &hosted.said(lang), kit.m.small, tokens::INK);
        let goes = super::parts::line(
            commands,
            kit,
            &hosted.data_goes(lang),
            kit.m.small,
            tokens::MUTED,
        );
        commands.entity(row_).add_children(&[said, goes]);
        if game.yours {
            let back = controls::button(
                commands,
                kit,
                Phrase::HostedTakeBack.text(lang),
                Weight::Ghost,
                Live::Yes,
                None,
                Press::Room(RoomPress::HostedCancel(index, seat.seat)),
            );
            walk(commands, back);
            commands.entity(row_).add_child(back);
        }
        commands.entity(card).add_child(row_);
    }
    // Life override and starting position: a drawer under the card, "for
    // testing and puzzles".
    let setup = if game.yours {
        lobby.room_draft().map_or(&game.setup, |d| &d.setup)
    } else {
        &game.setup
    };
    let personal = setup.seats.get(seat.seat as usize);
    if state.room_setup_seat == Some(at) {
        let drawer = commands
            .spawn((
                Node {
                    flex_direction: FlexDirection::Column,
                    row_gap: kit.m.px(6.0),
                    ..default()
                },
                Pickable::IGNORE,
            ))
            .id();
        let caption = super::parts::caption(commands, kit, Phrase::RoomForTesting.text(lang));
        commands.entity(drawer).add_child(caption);
        let life = personal.and_then(|s| s.life).unwrap_or(setup.starting_life);
        if game.yours {
            stepper(
                commands,
                drawer,
                fonts,
                m,
                &format!("{} · {life}", Phrase::RoomLife.text(lang)),
                Adjustment::SeatLife(at, -1),
                Adjustment::SeatLife(at, 1),
                life > 1,
                life < 999,
            );
        }
        cards::draw(commands, drawer, state, fonts, m, at, personal, game.yours);
        if game.yours && lobby.room_dirty() {
            let apply = controls::button(
                commands,
                kit,
                Phrase::SheetApply.text(lang),
                Weight::Primary,
                Live::Yes,
                None,
                Press::Room(RoomPress::SaveRoom(false)),
            );
            commands.entity(drawer).add_child(apply);
        }
        commands.entity(card).add_child(drawer);
    } else if personal.is_some_and(|p| !p.permanents.is_empty() || p.life.is_some()) {
        let count = personal.map_or(0, |s| s.permanents.len());
        let note = super::parts::line(
            commands,
            kit,
            &Phrase::RoomStartingCards.fill(lang, &[&count.to_string()]),
            kit.m.small,
            tokens::MUTED,
        );
        commands.entity(card).add_child(note);
    }
    anchor
}

/// The chair sheet (desktop builds): a language model for an open chair —
/// profile, model, effort, deck — and Seat. With no profile set up, the
/// way to Settings' language models (S-6).
#[allow(clippy::too_many_arguments)] // the room's inputs
fn chair_sheet(
    commands: &mut Commands,
    root: Entity,
    state: &LobbyState,
    fonts: &UiFonts,
    m: Metrics,
    kit: Kit,
    index: usize,
    chair: u32,
) {
    let lang = state.lobby.lang();
    let column = commands
        .spawn((
            Node {
                flex_direction: FlexDirection::Column,
                row_gap: px_fixed(kit.m.gap),
                ..default()
            },
            Pickable::IGNORE,
        ))
        .id();
    let mut footer = Vec::new();
    if state.llm.planned(chair).is_some() {
        llm::editor(commands, column, state, fonts, m, index, chair);
        footer.push(controls::button(
            commands,
            kit,
            Phrase::RoomLlmRemove.text(lang),
            Weight::Danger,
            Live::Yes,
            None,
            Press::Room(RoomPress::RoomLlm(
                index,
                chair,
                crate::tableseats::LlmPress::Remove,
            )),
        ));
        footer.push(controls::button(
            commands,
            kit,
            Phrase::RoomSeat.text(lang),
            Weight::Primary,
            Live::Yes,
            None,
            Press::Room(RoomPress::CloseChair),
        ));
    } else {
        let said = state
            .llm
            .said(chair)
            .unwrap_or_else(|| Phrase::RoomNoModel.text(lang).to_string());
        let line = super::parts::line(commands, kit, &said, kit.m.text, tokens::INK);
        commands.entity(column).add_child(line);
        footer.push(controls::button(
            commands,
            kit,
            Phrase::SheetCancel.text(lang),
            Weight::Secondary,
            Live::Yes,
            None,
            Press::Room(RoomPress::CloseChair),
        ));
        footer.push(controls::button(
            commands,
            kit,
            Phrase::RoomSetUpModel.text(lang),
            Weight::Primary,
            Live::Yes,
            None,
            Press::Settings(SettingsPress::OpenSettings),
        ));
    }
    let ids: [&'static str; 2] = if state.llm.planned(chair).is_some() {
        ["remove", "seat"]
    } else {
        ["cancel", "seat"]
    };
    for (control, id) in footer.iter().zip(ids) {
        super::orders::stop(commands, *control, &super::orders::CHAIR, id);
    }
    let surface = crate::shellkit::surfaces::sheet_box(
        commands,
        kit,
        crate::shellkit::surfaces::SheetWidth::Medium,
        &Phrase::RoomChairTitle.fill(lang, &[&(chair + 1).to_string()]),
        &[column],
        &footer,
    );
    let scrim = crate::shellkit::surfaces::sheet(commands, surface);
    commands
        .entity(scrim)
        .insert(Press::Room(RoomPress::CloseChair));
    commands
        .entity(surface)
        .insert(Press::Shared(SharedPress::PickerNothing));
    commands.entity(root).add_child(scrim);
}

/// The hosted-model sheet: the gateway's profiles for an open chair,
/// available ones first, the rest greyed with when they are back; a press
/// orders one. Every platform: the bridge runs beside the gateway.
fn hosted_sheet(
    commands: &mut Commands,
    root: Entity,
    state: &LobbyState,
    kit: Kit,
    index: usize,
    chair: u32,
) {
    let lobby = &state.lobby;
    let lang = lobby.lang();
    let column = commands
        .spawn((
            Node {
                flex_direction: FlexDirection::Column,
                row_gap: px_fixed(kit.m.gap),
                ..default()
            },
            Pickable::IGNORE,
        ))
        .id();
    let offer = lobby.hosted_offer(super::parts::local_offset());
    let intro = if !lobby.hosted_listed() {
        Phrase::HostedAsking
    } else if offer.is_empty() {
        Phrase::HostedNone
    } else {
        Phrase::HostedIntro
    };
    let line = super::parts::line(commands, kit, intro.text(lang), kit.m.text, tokens::MUTED);
    commands.entity(column).add_child(line);
    for (at, row) in offer.iter().enumerate() {
        let item = commands
            .spawn((
                Node {
                    flex_direction: FlexDirection::Column,
                    row_gap: kit.m.px(2.0),
                    ..default()
                },
                Pickable::IGNORE,
            ))
            .id();
        let press = if row.available {
            RoomPress::HostedOrder(index, chair, at)
        } else {
            RoomPress::HostedNothing
        };
        let button = controls::button(
            commands,
            kit,
            &row.words,
            Weight::Secondary,
            Live::Yes,
            None,
            Press::Room(press),
        );
        if !row.available || lobby.busy() {
            commands
                .entity(button)
                .insert(crate::shellkit::controls::Disabled);
        }
        super::orders::item(commands, button, &super::orders::CHAIR, "controls", at);
        commands.entity(item).add_child(button);
        if let Some(why) = &row.why {
            let l = super::parts::line(commands, kit, why, kit.m.small, tokens::MUTED);
            commands.entity(item).add_child(l);
        }
        commands.entity(column).add_child(item);
    }
    let cancel = controls::button(
        commands,
        kit,
        Phrase::SheetCancel.text(lang),
        Weight::Secondary,
        Live::Yes,
        None,
        Press::Room(RoomPress::CloseChair),
    );
    super::orders::stop(commands, cancel, &super::orders::CHAIR, "cancel");
    let surface = crate::shellkit::surfaces::sheet_box(
        commands,
        kit,
        crate::shellkit::surfaces::SheetWidth::Medium,
        &Phrase::HostedTitle.fill(lang, &[&(chair + 1).to_string()]),
        &[column],
        &[cancel],
    );
    let scrim = crate::shellkit::surfaces::sheet(commands, surface);
    commands
        .entity(scrim)
        .insert(Press::Room(RoomPress::CloseChair));
    commands
        .entity(surface)
        .insert(Press::Shared(SharedPress::PickerNothing));
    commands.entity(root).add_child(scrim);
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
    // The drawer's two boxes are the room's last stops.
    let id = match field {
        Field::RoomCounter => "counter",
        _ => "board",
    };
    super::orders::field(commands, e, &super::orders::ROOM, id);
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
    /// Move a chair onto a side. `0` puts it back on its own.
    SeatTeam(usize, u32, u8),
    /// `‹ Play`: step away from the room, keeping the seat (M-7).
    StepAway,
    /// Copy "table · gateway · password set" for a friend (§4).
    CopyInvite(usize),
    /// The Create-table sheet over the room, with Apply.
    EditRules,
    /// Teams on (sides 1 and 2 by turns) or off again.
    SetTeams,
    /// An open chair to the house, at a difficulty, in one request.
    SeatAiOpen(usize, u32, &'static str),
    /// The chair sheet: a language model for this chair (desktop builds).
    OpenChair(usize, u32),
    /// Close the chair sheet.
    CloseChair,
    /// The chair sheet listing the gateway's hosted models for a chair.
    HostedOpen(usize, u32),
    /// Order the hosted model at this row of the sheet's offer.
    HostedOrder(usize, u32, usize),
    /// Take a hosted chair back before the game.
    HostedCancel(usize, u32),
    /// An unavailable row: nothing to order.
    HostedNothing,
    /// A seat's own starting life: the same drawer as its starting position.
    LifeOverride(u8),
}

/// Leaves the listed table at `index`, as the room's Leave does: the host's
/// Leave hands the table on or closes it, so it is asked first. The tour's
/// bubble asks the same (`touring`).
pub(super) fn leave_table(state: &mut LobbyState, mailbox: &Mailbox, index: usize) {
    let game = state.lobby.games().get(index);
    if let Some(game) = game.filter(|g| g.yours) {
        state.confirmation = Some(super::confirm::Destructive::LeaveHosting(game.id.clone()));
    } else if let Some(game) = game.map(|g| g.id.clone()) {
        let request = state.lobby.leave_table(&game);
        dispatch(state, mailbox, request);
    }
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
                    // No name box stands in the room (WP2): the caret goes
                    // where nothing is typed into.
                    state.lobby.focus_on(Field::Search);
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
            RoomPress::LeaveTable(index) => leave_table(state, mailbox, index),
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
            RoomPress::SeatTeam(index, seat, team) => {
                let game = state.lobby.games().get(index).map(|g| g.id.clone());
                if let Some(game) = game {
                    let request = state.lobby.seat_team(&game, seat, team);
                    dispatch(state, mailbox, request);
                }
            }
            RoomPress::StepAway => {
                state.room_away = true;
                state.hub = Hub::Play;
            }
            RoomPress::CopyInvite(index) => {
                if let Some(game) = state.lobby.games().get(index) {
                    let lang = state.lobby.lang();
                    let gateway = state
                        .gateway_name()
                        .unwrap_or_else(|| state.gateway.clone());
                    let mut text = format!(
                        "{} · {gateway}",
                        client_core::lobby::strips::table_name(game)
                    );
                    if game.locked {
                        text.push_str(" · ");
                        text.push_str(Phrase::RoomPasswordSet.text(lang));
                    }
                    state.clipboard_out = Some(text);
                    state.invite_copied = true;
                }
            }
            RoomPress::EditRules => {
                if let Some(draft) = state.lobby.room_as_draft() {
                    super::play::open_sheet(state, draft, true);
                }
            }
            // Each seat shows its side as a chip that steps through the
            // sides (the gateway takes one chair at a time).
            RoomPress::SetTeams => state.teams_edit = !state.teams_edit,
            RoomPress::SeatAiOpen(index, seat, profile) => {
                let game = state.lobby.games().get(index).map(|g| g.id.clone());
                if let Some(game) = game {
                    state.llm.unplan(seat);
                    let request = state.lobby.set_seat(
                        &game,
                        seat,
                        Some(SeatKind::Ai),
                        Some(profile.to_string()),
                    );
                    dispatch(state, mailbox, request);
                }
            }
            RoomPress::OpenChair(_, seat) => {
                state.chair_sheet = Some(seat);
                state.hosted_sheet = false;
                // An open chair: the file's default profile, planned now so
                // the sheet shows its models (as the chip used to).
                if state.llm.planned(seat).is_none() {
                    let _ = state.llm.press(
                        seat,
                        crate::tableseats::LlmPress::Plan,
                        crate::tableseats::Phase::Waiting,
                        "steady",
                    );
                }
            }
            RoomPress::CloseChair => {
                state.chair_sheet = None;
                state.hosted_sheet = false;
            }
            RoomPress::HostedOpen(_, seat) => {
                state.chair_sheet = Some(seat);
                state.hosted_sheet = true;
                let request = state.lobby.list_hosted();
                dispatch(state, mailbox, request);
            }
            RoomPress::HostedOrder(index, seat, row) => {
                let game = state.lobby.games().get(index).map(|g| g.id.clone());
                let offer = state.lobby.hosted_offer(super::parts::local_offset());
                let profile = offer.get(row).filter(|r| r.available).map(|r| r.id.clone());
                if let (Some(game), Some(profile)) = (game, profile) {
                    // Not ours to run: no bridge of this client's here.
                    state.llm.unplan(seat);
                    let request = state.lobby.order_hosted(&game, seat, &profile);
                    if request.is_some() {
                        state.chair_sheet = None;
                        state.hosted_sheet = false;
                    }
                    dispatch(state, mailbox, request);
                }
            }
            RoomPress::HostedCancel(index, seat) => {
                let game = state.lobby.games().get(index).map(|g| g.id.clone());
                if let Some(game) = game {
                    let request = state.lobby.cancel_hosted(&game, seat);
                    dispatch(state, mailbox, request);
                }
            }
            RoomPress::HostedNothing => {}
            RoomPress::LifeOverride(seat) => {
                if state.room_setup_seat != Some(seat) {
                    state.room_setup_seat = Some(seat);
                    state.room_card_edit = None;
                }
            }
        }
    }
}
