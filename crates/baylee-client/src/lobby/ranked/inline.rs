//! The ranked game's inline controls share the Play activity rail and its scroll position.
use super::{
    Action, Cx, Format, Kit, Lang, League, Live, LobbyRequest, LobbyState, OwnRank, Phrase,
    Press, Weight, api, controls, draw, format_name, join, native, orders, parts, queue,
};
use crate::{hud::glyph, lobby::ranked_profile::Book, shellkit::ShellMetrics};
use bevy::prelude::*;

#[derive(Component)]
pub(super) struct Desk {
    metrics: ShellMetrics,
    arrow: Entity,
    caption: Entity,
    formats: Entity,
    queue: Entity,
    board: Entity,
}

pub(in crate::lobby) fn entry(
    commands: &mut Commands,
    card: Entity,
    caption: Entity,
    arrow: Entity,
    kit: Kit,
) {
    let root = draw::column(commands, card, kit);
    commands
        .entity(root)
        .entry::<Node>()
        .and_modify(|mut n| n.display = Display::None);
    let formats = parts::row(commands, kit, true);
    commands.entity(root).add_child(formats);
    let queue = draw::column(commands, root, kit);
    let board = controls::button_compact(
        commands,
        kit,
        Some(glyph::TROPHY),
        Phrase::RankedBoard.text(kit.lang()),
        Weight::Ghost,
        Live::Yes,
        Press::Ranked(Action::Open),
    );
    commands.entity(root).add_child(board);
    commands.entity(root).insert(Desk {
        metrics: kit.m,
        arrow,
        caption,
        formats,
        queue,
        board,
    });
}

fn showing(state: &LobbyState) -> bool {
    state.showing_play()
        && matches!(state.lobby.screen(), crate::lobby::Screen::Table)
        && !state.lobby.offline()
}

fn expanded(state: &LobbyState) -> bool {
    state
        .ranked
        .expanded
        .unwrap_or(!matches!(state.ranked_net.queue, api::QueueState::Idle))
}

fn locked(state: &LobbyState) -> bool {
    state.ranked_net.busy
        || matches!(
            state.ranked_net.queue,
            api::QueueState::Waiting { .. } | api::QueueState::Assigned { .. }
        )
}

pub(in crate::lobby) fn preview_visible(state: &LobbyState) -> bool {
    showing(state) && expanded(state)
}

pub(in crate::lobby) fn escape_queue(state: &mut LobbyState) -> &'static str {
    if state.ranked.show_decks {
        state.ranked.show_decks = false;
        state.ranked.revision = state.ranked.revision.wrapping_add(1);
        "deck"
    } else {
        collapse(state);
        "ranked"
    }
}

fn collapse(state: &mut LobbyState) {
    state.ranked.expanded = Some(false);
    state.ranked.show_decks = false;
    native::clear_preview(state);
    state.ranked.revision = state.ranked.revision.wrapping_add(1);
}

pub(super) fn press(action: Action, cx: Cx<'_, '_, '_, '_, '_>) {
    if !showing(cx.state) {
        return;
    }
    let state = cx.state.bypass_change_detection();
    if action == Action::ToggleQueue {
        if expanded(state) {
            collapse(state);
        } else {
            state.ranked.expanded = Some(true);
            state.ranked.revision = state.ranked.revision.wrapping_add(1);
            if state.has_central_account() && state.central_handle().is_some_and(|h| h.is_active())
            {
                crate::lobby::dispatch(state, cx.mailbox, Some(LobbyRequest::ListDecks));
                native::refresh(state);
            }
        }
        return;
    }
    if !expanded(state) || !state.has_central_account() {
        return;
    }
    #[cfg(feature = "dev-control")]
    if super::visual_fixture::reply(state, action) {
        return;
    }
    match action {
        Action::QueueFormat(format) if !locked(state) && format != state.ranked.queue_format => {
            native::clear_preview(state);
            state.ranked.queue_format = format;
            state.ranked.deck_id = None;
            state.ranked.deck_page = 0;
            state.ranked.show_decks = false;
        }
        Action::ToggleDecks => state.ranked.show_decks = !state.ranked.show_decks,
        Action::DeckPage(page) => state.ranked.deck_page = page,
        Action::Deck(id) => {
            if Some(id) != state.ranked.deck_id {
                native::clear_preview(state);
            }
            state.ranked.deck_id = Some(id);
            state.ranked.show_decks = false;
        }
        Action::Preview if !locked(state) => {
            if let Some((id, _)) = queue::selected(state) {
                native::preview(state, queue::effective_format(state), &id.to_string());
            }
        }
        Action::Join if !locked(state) || state.ranked_net.needs_deck => join(state),
        Action::Cancel => native::cancel(state),
        Action::Resume => native::resume(state),
        Action::Refresh => native::refresh(state),
        _ => return,
    }
    state.ranked.revision = state.ranked.revision.wrapping_add(1);
}

/// Follows keyboard navigation inside the activity rail. Wheel and thumb input
/// retain their own offset until focus actually moves to another control.
pub(super) fn reveal_focus(
    focus: Res<bevy::input_focus::InputFocus>,
    visible: Res<bevy::input_focus::InputFocusVisible>,
    controls: Query<(
        &crate::shellkit::focus::Stop,
        &ComputedNode,
        &UiGlobalTransform,
    )>,
    parents: Query<&ChildOf>,
    mut rails: Query<(
        &crate::lobby::Scrollable,
        &ComputedNode,
        &UiGlobalTransform,
        &mut ScrollPosition,
    )>,
    mut memory: ResMut<crate::lobby::Scrolled>,
    mut last: Local<Option<crate::shellkit::focus::Stop>>,
) {
    let Some((button, (stop, control, at))) = focus
        .get()
        .filter(|_| visible.0)
        .and_then(|e| controls.get(e).ok().map(|value| (e, value)))
        .filter(|(_, (stop, _, _))| stop.table == orders::PLAY.name)
    else {
        *last = None;
        return;
    };
    if *last == Some(*stop) || control.size().y <= 0.0 {
        return;
    }
    *last = Some(*stop);
    for ancestor in parents.iter_ancestors(button) {
        let Ok((list, view, origin, mut scroll)) = rails.get_mut(ancestor) else {
            continue;
        };
        if list.0 != crate::lobby::List::Discovery || view.size().y <= 0.0 {
            continue;
        }
        let top = at.translation.y - control.size().y * 0.5;
        let bottom = at.translation.y + control.size().y * 0.5;
        let view_top = origin.translation.y - view.size().y * 0.5;
        let view_bottom = origin.translation.y + view.size().y * 0.5;
        let delta = (top - view_top).min(0.0) + (bottom - view_bottom).max(0.0);
        if delta.abs() > 0.5 {
            scroll.y = (scroll.y + delta * view.inverse_scale_factor()).max(0.0);
            memory.set(list.0, scroll.y);
        }
        return;
    }
}

#[derive(Default)]
pub(super) struct Painted {
    key: Option<(Entity, u64, u64, Option<OwnRank>)>,
    open: bool,
    formats: Option<Format>,
    queue: Option<queue::Look>,
    caption: String,
}

fn caption(state: &LobbyState, open: bool) -> String {
    let lang = state.lobby.lang();
    if !open {
        match state.ranked_net.queue {
            api::QueueState::Waiting { format, .. } => {
                return Phrase::RankedSearching.fill(lang, &[format_name(format, lang)]);
            }
            api::QueueState::Assigned { ref assignment, .. } => {
                if state.ranked_net.waiting_consent() {
                    return Phrase::RankedHostsWaiting.text(lang).to_owned();
                }
                return Phrase::RankedAssigned.fill(lang, &[format_name(assignment.format, lang)]);
            }
            api::QueueState::Finished {
                format,
                outcome,
                seat,
                elo,
                delta,
                ..
            } => return queue::receipt(lang, format, outcome, seat, elo, delta),
            api::QueueState::Idle => {}
        }
    }
    Phrase::RankedModeEntry.text(lang).to_owned()
}

fn own_rank(book: &Book, format: Format) -> Option<OwnRank> {
    if let Some(s) = book.own_player(format) {
        return Some((s.rank, s.placement_games, Some(s.elo)));
    }
    book.own_loaded().then_some((
        api::Rank {
            league: League::Unranked,
            division: None,
            champion_place: None,
        },
        0,
        None,
    ))
}

pub(super) fn patch(
    mut commands: Commands,
    state: Res<LobbyState>,
    book: Option<Res<Book>>,
    fonts: Option<Res<crate::hud::UiFonts>>,
    desks: Query<(Entity, &Desk)>,
    mut painted: Local<Painted>,
) {
    let (Ok((root, desk)), Some(fonts)) = (desks.single(), fonts.as_deref()) else {
        return;
    };
    let format = queue::effective_format(&state);
    let own = book
        .as_deref()
        .filter(|_| state.has_central_account())
        .and_then(|book| own_rank(book, format));
    let key = (root, state.ranked.revision, state.ranked_net.revision, own);
    if painted.key == Some(key) && !state.is_changed() {
        return;
    }
    if painted.key.is_none_or(|old| old.0 != root) {
        *painted = Painted::default();
    }
    painted.key = Some(key);
    let open = expanded(&state);
    let words = caption(&state, open);
    if painted.caption != words {
        commands
            .entity(desk.caption)
            .insert(Text::new(words.clone()));
        painted.caption = words;
    }
    if open != painted.open {
        painted.open = open;
        commands
            .entity(root)
            .entry::<Node>()
            .and_modify(move |mut node| {
                node.display = if open { Display::Flex } else { Display::None };
            });
        let icon = if open {
            glyph::CHEVRON_UP
        } else {
            glyph::CHEVRON_DOWN
        };
        commands
            .entity(desk.arrow)
            .insert(Text::new(icon.to_string()));
        if open {
            orders::stop(&mut commands, desk.board, &orders::PLAY, "ranked-board");
        } else {
            commands
                .entity(desk.board)
                .remove::<crate::shellkit::focus::Stop>();
            commands.entity(desk.formats).despawn_children();
            commands.entity(desk.queue).despawn_children();
            painted.queue = None;
            painted.formats = None;
        }
    }
    if !open {
        return;
    }
    let kit = Kit {
        fonts,
        m: desk.metrics,
        german: state.lobby.lang() == Lang::De,
    };
    let selectable = (state.has_central_account() && !locked(&state)).then_some(format);
    if painted.formats != selectable {
        painted.formats = selectable;
        commands.entity(desk.formats).despawn_children();
        if let Some(current) = selectable {
            for (i, format) in Format::ALL.into_iter().enumerate() {
                draw::btn(
                    &mut commands,
                    desk.formats,
                    kit,
                    format_name(format, kit.lang()),
                    Action::QueueFormat(format),
                    (&orders::PLAY, "rank-format", i),
                    format == current,
                );
            }
        }
    }
    let view = queue::look(&state, own);
    if painted.queue.as_ref() != Some(&view) {
        commands.entity(desk.queue).despawn_children();
        queue::draw(&mut commands, desk.queue, kit, &state, &view);
        painted.queue = Some(view);
    }
}
