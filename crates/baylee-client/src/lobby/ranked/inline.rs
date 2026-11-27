//! The ranked desk: Central's matchmaking for the signed-in account (its
//! standing, the format, the deck, the search and its result). It stands on
//! the Ranked page beside the public ladder. The Play screen's tile opens the
//! page and says in one line how the queue stands, so nothing in the lobby
//! unfolds and pushes the tiles under it down.
use super::{
    Action, Cx, Format, Hub, Kit, Lang, League, LobbyRequest, LobbyState, OwnRank, Phrase, api,
    draw, format_name, join, native, orders, queue,
};
use crate::{
    lobby::{Mailbox, ranked_profile::Book},
    shellkit::ShellMetrics,
};
use bevy::prelude::*;

/// The desk's slot on the Ranked page, filled by [`patch`].
#[derive(Component)]
pub(super) struct Desk {
    metrics: ShellMetrics,
    queue: Entity,
}

/// The Play tile's line under its title. [`patch`] rewrites it as the queue
/// moves; the lobby is never rebuilt for it.
#[derive(Component)]
pub(in crate::lobby) struct TileCaption;

/// Stands the desk's slot in `parent`. The page's one format choice, in its
/// head, serves the ladder and the queue alike ([`follow_format`]).
pub(super) fn desk(commands: &mut Commands, parent: Entity, kit: Kit) {
    let queue = draw::column(commands, parent, kit);
    commands.entity(parent).insert(Desk {
        metrics: kit.m,
        queue,
    });
}

/// The page's format chose `format`: the queue follows it, unless an entry
/// is already waiting or seated in another (the ladder follows regardless).
pub(super) fn follow_format(state: &mut LobbyState, format: Format) {
    if locked(state) || format == state.ranked.queue_format {
        return;
    }
    native::clear_preview(state);
    state.ranked.queue_format = format;
    state.ranked.deck_id = None;
    state.ranked.deck_page = 0;
    state.ranked.show_decks = false;
}

fn locked(state: &LobbyState) -> bool {
    state.ranked_net.busy
        || matches!(
            state.ranked_net.queue,
            api::QueueState::Waiting { .. } | api::QueueState::Assigned { .. }
        )
}

/// The desk's own actions, which only the Ranked page offers.
pub(super) fn owns(action: Action) -> bool {
    matches!(
        action,
        Action::ToggleDecks
            | Action::DeckPage(_)
            | Action::Deck(_)
            | Action::Preview
            | Action::Join
            | Action::Cancel
            | Action::Resume
    )
}

/// The page opened: the account's decks and its queue entry are read again,
/// as the Play tile's desk read them when it unfolded.
pub(super) fn opened(state: &mut LobbyState, mailbox: &Mailbox) {
    if state.has_central_account() && state.central_handle().is_some_and(|h| h.is_active()) {
        crate::lobby::dispatch(state, mailbox, Some(LobbyRequest::ListDecks));
        native::refresh(state);
    }
}

/// The page's Refresh, for the desk: the queue entry read again.
pub(super) fn refresh(state: &mut LobbyState) {
    if state.has_central_account() {
        native::refresh(state);
    }
}

/// Esc on the Ranked page, first rung: an open deck list closes. `false`
/// when there was none, and the page's own ladder goes on.
pub(super) fn escape(state: &mut LobbyState) -> bool {
    if !state.ranked.show_decks {
        return false;
    }
    state.ranked.show_decks = false;
    state.ranked.revision = state.ranked.revision.wrapping_add(1);
    true
}

pub(super) fn press(action: Action, cx: Cx<'_, '_, '_, '_, '_>) {
    if cx.state.hub != Hub::Ranked || !cx.state.has_central_account() {
        return;
    }
    let state = cx.state.bypass_change_detection();
    #[cfg(feature = "dev-control")]
    if super::visual_fixture::reply(state, action) {
        return;
    }
    match action {
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

/// What [`patch`] last painted for: the desk, the tile's line, both
/// revisions and the account's standing.
type PaintKey = (Option<Entity>, Option<Entity>, u64, u64, Option<OwnRank>);

#[derive(Default)]
pub(super) struct Painted {
    key: Option<PaintKey>,
    queue: Option<queue::Look>,
    caption: String,
    /// Whether the desk stands: only for a Central account. A guest reading
    /// the ladder from the front door sees the ladder alone, full width.
    shown: Option<bool>,
}

/// The tile's line: the queue's state while there is one, else the
/// account's standing in the queue's format, else what the mode is.
fn caption(state: &LobbyState, own: Option<OwnRank>) -> String {
    let lang = state.lobby.lang();
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
    match own {
        Some(OwnRank {
            rank,
            elo: Some(elo),
            ..
        }) if rank.league != League::Unranked => format!(
            "{} · {}",
            super::rank_name(rank, lang),
            Phrase::RankedElo.fill(lang, &[&elo.to_string()])
        ),
        _ => Phrase::RankedModeEntry.text(lang).to_owned(),
    }
}

fn own_rank(book: &Book, format: Format) -> Option<OwnRank> {
    if let Some(s) = book.own_player(format) {
        return Some(OwnRank {
            rank: s.rank,
            placements: s.placement_games,
            elo: Some(s.elo),
            record: Some((s.wins, s.losses, s.draws)),
            career: Some(s.career),
        });
    }
    book.own_loaded().then_some(OwnRank {
        rank: api::Rank {
            league: League::Unranked,
            division: None,
            champion_place: None,
        },
        placements: 0,
        elo: None,
        record: None,
        career: None,
    })
}

pub(super) fn patch(
    mut commands: Commands,
    state: Res<LobbyState>,
    book: Option<Res<Book>>,
    fonts: Option<Res<crate::hud::UiFonts>>,
    desks: Query<(Entity, &Desk)>,
    captions: Query<Entity, With<TileCaption>>,
    mut painted: Local<Painted>,
) {
    let desk = desks.single().ok();
    let caption_at = captions.single().ok();
    if desk.is_none() && caption_at.is_none() {
        *painted = Painted::default();
        return;
    }
    let format = queue::effective_format(&state);
    let own = book
        .as_deref()
        .filter(|_| state.has_central_account())
        .and_then(|book| own_rank(book, format));
    let key = (
        desk.map(|(root, _)| root),
        caption_at,
        state.ranked.revision,
        state.ranked_net.revision,
        own,
    );
    if painted.key == Some(key) && !state.is_changed() {
        return;
    }
    // A desk or a tile drawn afresh holds nothing of what was painted.
    let old = painted.key;
    if old.is_none_or(|old| old.0 != key.0) {
        painted.queue = None;
        painted.shown = None;
    }
    if old.is_none_or(|old| old.1 != key.1) {
        painted.caption.clear();
    }
    painted.key = Some(key);
    let words = caption(&state, own);
    if painted.caption != words {
        if let Some(line) = caption_at {
            commands.entity(line).insert(Text::new(words.clone()));
        }
        painted.caption = words;
    }
    let (Some((root, desk)), Some(fonts)) = (desk, fonts.as_deref()) else {
        return;
    };
    let eligible = state.has_central_account();
    if painted.shown != Some(eligible) {
        painted.shown = Some(eligible);
        commands
            .entity(root)
            .entry::<Node>()
            .and_modify(move |mut n| {
                n.display = if eligible {
                    Display::Flex
                } else {
                    Display::None
                };
            });
    }
    if !eligible {
        return;
    }
    let kit = Kit {
        fonts,
        m: desk.metrics,
        german: state.lobby.lang() == Lang::De,
    };
    let view = queue::look(&state, own);
    if painted.queue.as_ref() != Some(&view) {
        commands.entity(desk.queue).despawn_children();
        queue::draw(&mut commands, desk.queue, kit, &state, &view);
        painted.queue = Some(view);
    }
}
