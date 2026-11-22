//! Retained board body. The scroll region and its scrollbar survive network refreshes.
use super::board::{board_row, filters};
use super::*;
use crate::hud::glyph;
use crate::lobby::{Scrollable, scrollbars};
use crate::shellkit::{ShellMetrics, surfaces};

#[derive(Component)]
pub(super) struct Page {
    metrics: ShellMetrics,
    formats: Entity,
    queue: Entity,
    filters: Entity,
    list: Entity,
    pager: Entity,
}

#[derive(Default)]
pub(super) struct Painted {
    key: Option<(Entity, u64, u64, Option<(api::Rank, u8, Option<i32>)>)>,
    scroll: Option<(Format, Track, League, Option<u8>, usize)>,
    format: Option<Format>,
    filters: Option<(Track, League, Option<u8>, bool)>,
    queue: Option<queue::Look>,
    rows: Option<(Vec<api::Row>, usize, Option<Phrase>)>,
    pager: Option<(String, usize, bool, bool, bool)>,
}

pub(super) fn column(commands: &mut Commands, parent: Entity, kit: Kit) -> Entity {
    let id = commands
        .spawn((
            Node {
                flex_direction: FlexDirection::Column,
                min_width: px(0),
                row_gap: kit.m.space(1),
                ..default()
            },
            Pickable::IGNORE,
        ))
        .id();
    commands.entity(parent).add_child(id);
    id
}

pub(in crate::lobby) fn draw(
    commands: &mut Commands,
    root: Entity,
    state: &LobbyState,
    metrics: Metrics,
    kit: Kit,
    scrolled: &Scrolled,
) {
    let compact = !matches!(
        kit.m.frame,
        crate::shellkit::Frame::Wide | crate::shellkit::Frame::Vast
    );
    let page = commands
        .spawn((
            Node {
                width: percent(100),
                max_width: px(kit.m.body_max),
                min_height: px(0),
                flex_grow: 1.0,
                align_self: AlignSelf::Center,
                margin: UiRect::horizontal(Val::Auto),
                flex_direction: FlexDirection::Column,
                padding: UiRect::all(px(kit.m.body)),
                row_gap: kit.m.space(2),
                overflow: if compact {
                    Overflow::scroll_y()
                } else {
                    Overflow::clip()
                },
                ..default()
            },
            Pickable::default(),
        ))
        .id();
    if compact {
        // Small windows scroll the entire desk; the leaderboard has its own bounded viewport.
        commands
            .entity(page)
            .insert((ScrollPosition::default(), surfaces::SheetScroller));
        let band = scrollbars::band_stable(commands, page, true);
        commands
            .entity(band)
            .entry::<Node>()
            .and_modify(|mut n| n.flex_grow = 1.0);
        commands.entity(root).add_child(band);
    } else {
        commands.entity(root).add_child(page);
    }
    let top = parts::row(commands, kit, true);
    commands.entity(page).add_child(top);
    let back = controls::button_compact(
        commands,
        kit,
        Some(glyph::ARROW_LEFT),
        Phrase::Back.text(kit.lang()),
        Weight::Ghost,
        Live::Yes,
        Press::Ranked(Action::Back),
    );
    orders::stop(commands, back, &ORDER, "back");
    let title = parts::line(
        commands,
        kit,
        Phrase::RankedTitle.text(kit.lang()),
        kit.m.h1,
        tokens::INK,
    );
    commands
        .entity(title)
        .insert(crate::hud::tf_bold(kit.fonts, kit.m.h1));
    let grow = parts::grow(commands);
    let formats = parts::row(commands, kit, true);
    commands
        .entity(top)
        .add_children(&[back, title, grow, formats]);
    let content = commands
        .spawn((
            Node {
                width: percent(100),
                min_height: px(0),
                flex_grow: 1.0,
                flex_direction: if compact {
                    FlexDirection::Column
                } else {
                    FlexDirection::Row
                },
                column_gap: kit.m.space(2),
                row_gap: kit.m.space(2),
                ..default()
            },
            Pickable::IGNORE,
        ))
        .id();
    commands.entity(page).add_child(content);
    let queue = surfaces::panel(
        commands,
        kit,
        if compact { percent(100) } else { percent(30) },
    );
    commands
        .entity(queue)
        .entry::<Node>()
        .and_modify(move |mut n| {
            n.align_self = AlignSelf::FlexStart;
            n.flex_shrink = 0.0;
            n.row_gap = kit.m.space(2);
            if !compact {
                n.max_height = percent(100);
                n.overflow = Overflow::scroll_y();
            }
        });
    if !compact {
        commands
            .entity(queue)
            .insert((ScrollPosition::default(), surfaces::SheetScroller));
        let queue_band = scrollbars::band_stable(commands, queue, false);
        commands
            .entity(queue_band)
            .entry::<Node>()
            .and_modify(|mut n| {
                n.width = percent(30);
                n.align_self = AlignSelf::FlexStart;
                n.max_height = percent(100);
            });
        commands.entity(content).add_child(queue_band);
    } else {
        commands.entity(content).add_child(queue);
    }
    let board = surfaces::panel(
        commands,
        kit,
        if compact { percent(100) } else { Val::Auto },
    );
    commands
        .entity(board)
        .entry::<Node>()
        .and_modify(move |mut n| {
            n.flex_grow = 1.0;
            n.min_height = px(0);
            n.min_width = px(0);
            n.row_gap = kit.m.space(2);
            if compact {
                n.flex_shrink = 0.0;
                n.height = Val::Vh(66.0);
                n.min_height = kit.m.px(320.0);
            }
        });
    commands.entity(content).add_child(board);
    let head = parts::row(commands, kit, false);
    let text = parts::marked_line(
        commands,
        kit,
        glyph::TROPHY,
        Phrase::RankedBoard.text(kit.lang()),
        kit.m.head,
        tokens::GOLD,
    );
    let grow = parts::grow(commands);
    let refresh = controls::button_compact(
        commands,
        kit,
        Some(glyph::REFRESH),
        Phrase::Refresh.text(kit.lang()),
        Weight::Ghost,
        Live::Yes,
        Press::Ranked(Action::Refresh),
    );
    orders::stop(commands, refresh, &ORDER, "refresh");
    commands.entity(head).add_children(&[text, grow, refresh]);
    commands.entity(board).add_child(head);
    let filters = column(commands, board, kit);
    let list = commands
        .spawn((
            Node {
                flex_direction: FlexDirection::Column,
                overflow: Overflow::scroll_y(),
                row_gap: kit.m.px(3.0),
                min_height: px(0),
                ..default()
            },
            Scrollable(List::Ranked),
            ScrollPosition(Vec2::new(0.0, scrolled.get(List::Ranked))),
            crate::shellkit::Role::Scroll,
            Pickable::default(),
        ))
        .id();
    scrollbars::attach(commands, board, list, metrics);
    let pager = parts::row(commands, kit, true);
    commands.entity(board).add_child(pager);
    commands.entity(page).insert(Page {
        metrics: kit.m,
        formats,
        queue,
        filters,
        list,
        pager,
    });
    let _ = state;
}

pub(super) fn btn(
    commands: &mut Commands,
    parent: Entity,
    kit: Kit,
    words: &str,
    action: Action,
    stop: (&'static str, usize),
    chosen: bool,
) {
    let b = controls::button_compact(
        commands,
        kit,
        None,
        words,
        if chosen {
            Weight::Primary
        } else {
            Weight::Secondary
        },
        Live::Yes,
        Press::Ranked(action),
    );
    orders::item(commands, b, &ORDER, stop.0, stop.1);
    commands.entity(parent).add_child(b);
}

pub(super) fn label(
    commands: &mut Commands,
    parent: Entity,
    kit: Kit,
    text: &str,
    size: f32,
    ink: Color,
) {
    let line = parts::line(commands, kit, text, size, ink);
    commands.entity(parent).add_child(line);
}
pub(super) fn patch(
    mut commands: Commands,
    state: Res<LobbyState>,
    fonts: Option<Res<crate::hud::UiFonts>>,
    pages: Query<(Entity, &Page)>,
    book: Option<Res<crate::lobby::ranked_profile::Book>>,
    mut positions: Query<&mut ScrollPosition>,
    mut painted: Local<Painted>,
) {
    let (Ok((page, slots)), Some(fonts)) = (pages.single(), fonts.as_deref()) else {
        return;
    };
    let net = &state.ranked_net;
    let ui = &state.ranked;
    let own = book.as_ref().and_then(|book| {
        let format = queue::effective_format(&state);
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
    });
    let key = (page, ui.revision, net.revision, own);
    if painted.key == Some(key) {
        return;
    }
    if painted.key.is_none_or(|k| k.0 != page) {
        *painted = Painted::default();
    }
    painted.key = Some(key);
    let kit = Kit {
        fonts,
        m: slots.metrics,
        german: state.lobby.lang() == Lang::De,
    };
    let lang = kit.lang();
    let viewport = (ui.format, ui.track, ui.league, ui.division, ui.pages.len());
    if painted
        .scroll
        .replace(viewport)
        .is_some_and(|old| old != viewport)
        && let Ok(mut scroller) = positions.get_mut(slots.list)
    {
        scroller.y = 0.0;
    }
    if painted.format != Some(ui.format) {
        painted.format = Some(ui.format);
        commands.entity(slots.formats).despawn_children();
        for (i, format) in FORMATS.into_iter().enumerate() {
            btn(
                &mut commands,
                slots.formats,
                kit,
                format_name(format, lang),
                Action::Format(format),
                ("format", i),
                format == ui.format,
            );
        }
    }
    let filter = (ui.track, ui.league, ui.division, ui.show_leagues);
    if painted.filters != Some(filter) {
        painted.filters = Some(filter);
        commands.entity(slots.filters).despawn_children();
        filters(&mut commands, slots.filters, kit, ui);
    }
    let queue = queue::look(&state, own);
    if painted.queue.as_ref() != Some(&queue) {
        commands.entity(slots.queue).despawn_children();
        queue::draw(&mut commands, slots.queue, kit, &state, &queue);
        painted.queue = Some(queue);
    }
    let (season, entries, next) = match &net.board {
        Some(api::Response::Board {
            season,
            entries,
            next,
        }) => (season.name.as_str(), entries.as_slice(), next.is_some()),
        _ => ("", &[][..], false),
    };
    let notice = if net.board_error.is_some() {
        Some(Phrase::RankedBoardFailed)
    } else if entries.is_empty() {
        Some(if net.board_busy {
            Phrase::ChallengeLoading
        } else {
            Phrase::RankedBoardEmpty
        })
    } else {
        None
    };
    let offset = ui.pages.last().map_or(0, |(_, n)| *n);
    if !painted
        .rows
        .as_ref()
        .is_some_and(|(r, o, n)| r.as_slice() == entries && *o == offset && *n == notice)
    {
        commands.entity(slots.list).despawn_children();
        if let Some(msg) = notice {
            label(
                &mut commands,
                slots.list,
                kit,
                msg.text(lang),
                kit.m.text,
                tokens::MUTED,
            );
        }
        for (i, row) in entries.iter().enumerate() {
            board_row(&mut commands, slots.list, kit, row, offset + i + 1, i);
        }
        painted.rows = Some((entries.to_vec(), offset, notice));
    }
    let pager = (
        season.to_owned(),
        ui.pages.len() + 1,
        !ui.pages.is_empty(),
        next,
        net.board_busy,
    );
    if painted.pager.as_ref() != Some(&pager) {
        commands.entity(slots.pager).despawn_children();
        let words = Phrase::RankedPage.fill(lang, &[&pager.1.to_string()]);
        label(
            &mut commands,
            slots.pager,
            kit,
            &words,
            kit.m.small,
            tokens::MUTED,
        );
        label(
            &mut commands,
            slots.pager,
            kit,
            season,
            kit.m.small,
            tokens::ACCENT,
        );
        let grow = parts::grow(&mut commands);
        commands.entity(slots.pager).add_child(grow);
        for (i, (on, forward, icon, txt)) in [
            (pager.2, false, glyph::ARROW_LEFT, Phrase::ChallengePrevious),
            (pager.3, true, glyph::ARROW_RIGHT, Phrase::ChallengeNext),
        ]
        .into_iter()
        .enumerate()
        {
            let b = controls::button_compact(
                &mut commands,
                kit,
                Some(icon),
                txt.text(lang),
                Weight::Ghost,
                if on && !pager.4 {
                    Live::Yes
                } else {
                    Live::No(Phrase::RankedNoPage.text(lang))
                },
                Press::Ranked(Action::Page(forward)),
            );
            if on && !pager.4 {
                orders::item(&mut commands, b, &ORDER, "pager", i);
            }
            commands.entity(slots.pager).add_child(b);
        }
        painted.pager = Some(pager);
    }
}
