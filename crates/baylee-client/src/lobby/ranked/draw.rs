//! Retained board body. The scroll region and its scrollbar survive network refreshes.
use super::board::{board_row, filters};
use super::{
    Action, Format, Kit, Lang, League, List, Live, LobbyState, Metrics, ORDER, Phrase, Press,
    Scrolled, Track, Ui, Weight, api, board, controls, format_name, league_name, orders, parts,
    tokens,
};
use crate::hud::glyph;
use crate::lobby::{Scrollable, scrollbars};
use crate::shellkit::{ShellMetrics, surfaces};
use bevy::prelude::*;

#[derive(Component)]
pub(super) struct Page {
    metrics: ShellMetrics,
    formats: Entity,
    board: Board,
}

struct Board {
    filters: Entity,
    mark: Entity,
    list: Entity,
    pager: Entity,
}

#[derive(Default)]
pub(super) struct Painted {
    key: Option<(Entity, u64, u64)>,
    scroll: Option<Viewport>,
    format: Option<Format>,
    filters: Option<(Track, League, Option<u8>, bool)>,
    rows: Option<(Vec<api::Row>, usize, Option<Phrase>)>,
    seasons: Option<(Vec<api::Season>, bool, bool)>,
    pager: Option<Pager>,
}

#[derive(PartialEq, Eq)]
struct Viewport {
    format: Format,
    track: Track,
    league: League,
    division: Option<u8>,
    season: Option<api::Id>,
    page: usize,
    season_page: Option<usize>,
}

#[derive(PartialEq, Eq)]
struct Pager {
    season: String,
    page: usize,
    previous: bool,
    next: bool,
    busy: bool,
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
    metrics: Metrics,
    kit: Kit,
    scrolled: &Scrolled,
    kept: Option<&crate::shellkit::retain::Kept>,
) {
    if let Some(kept) = kept {
        kept.attach(commands, root);
        return;
    }
    let compact = !matches!(
        kit.m.frame,
        crate::shellkit::Frame::Wide | crate::shellkit::Frame::Vast
    );
    let page = page_root(commands, root, kit, compact);
    let formats = masthead(commands, page, kit);
    let board = board_panel(commands, page, kit, metrics, compact, scrolled);
    commands.entity(page).insert(Page {
        metrics: kit.m,
        formats,
        board,
    });
}

fn page_root(commands: &mut Commands, root: Entity, kit: Kit, compact: bool) -> Entity {
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
    let desk = if compact {
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
        band
    } else {
        commands.entity(root).add_child(page);
        page
    };
    commands
        .entity(desk)
        .insert(crate::lobby::shell::RankedDesk(kit.m, kit.lang()));
    page
}

fn masthead(commands: &mut Commands, page: Entity, kit: Kit) -> Entity {
    let top = surfaces::panel(commands, kit, percent(100));
    commands
        .entity(top)
        .entry::<Node>()
        .and_modify(move |mut node| {
            node.flex_direction = FlexDirection::Row;
            node.flex_wrap = FlexWrap::Wrap;
            node.align_items = AlignItems::Center;
            node.column_gap = kit.m.space(1);
            node.padding = UiRect::axes(kit.m.space(2), kit.m.space(1));
        });
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
        Phrase::RankedBoard.text(kit.lang()),
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
    formats
}

fn board_panel(
    commands: &mut Commands,
    content: Entity,
    kit: Kit,
    metrics: Metrics,
    compact: bool,
    scrolled: &Scrolled,
) -> Board {
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
    let mark = commands
        .spawn((
            Node {
                width: kit.m.px(56.0),
                height: kit.m.px(56.0),
                flex_shrink: 0.0,
                ..default()
            },
            Pickable::IGNORE,
        ))
        .id();
    let text = parts::line(
        commands,
        kit,
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
    commands
        .entity(head)
        .add_children(&[mark, text, grow, refresh]);
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
    Board {
        filters,
        mark,
        list,
        pager,
    }
}

pub(super) fn btn(
    commands: &mut Commands,
    parent: Entity,
    kit: Kit,
    words: &str,
    action: Action,
    stop: (
        &'static crate::shellkit::focus::TabOrder,
        &'static str,
        usize,
    ),
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
    orders::item(commands, b, stop.0, stop.1, stop.2);
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
    mut positions: Query<&mut ScrollPosition>,
    mut painted: Local<Painted>,
) {
    let (Ok((page, slots)), Some(fonts)) = (pages.single(), fonts.as_deref()) else {
        return;
    };
    let net = &state.ranked_net;
    let ui = &state.ranked;
    let key = (page, ui.revision, net.revision);
    if painted.key == Some(key) && !state.is_changed() {
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
    let viewport = Viewport {
        format: ui.format,
        track: ui.track,
        league: ui.league,
        division: ui.division,
        season: ui.selected_season,
        page: ui.pages.len(),
        season_page: ui.browsing_seasons.then_some(ui.season_pages.len()),
    };
    if painted.scroll.as_ref() != Some(&viewport) {
        if painted.scroll.is_some()
            && let Ok(mut scroller) = positions.get_mut(slots.board.list)
        {
            scroller.y = 0.0;
        }
        painted.scroll = Some(viewport);
    }
    paint_controls(&mut commands, slots, kit, ui, &mut painted);
    paint_board(&mut commands, &slots.board, kit, &state, &mut painted);
}

fn paint_controls(commands: &mut Commands, slots: &Page, kit: Kit, ui: &Ui, painted: &mut Painted) {
    let lang = kit.lang();
    if painted.format != Some(ui.format) {
        painted.format = Some(ui.format);
        commands.entity(slots.formats).despawn_children();
        for (i, format) in Format::ALL.into_iter().enumerate() {
            btn(
                commands,
                slots.formats,
                kit,
                format_name(format, lang),
                Action::Format(format),
                (&ORDER, "format", i),
                format == ui.format,
            );
        }
    }
    let filter = (ui.track, ui.league, ui.division, ui.show_leagues);
    if painted.filters.is_none_or(|(track, league, division, _)| {
        (track, league, division) != (ui.track, ui.league, ui.division)
    }) && let Some(mark) = crate::rank_art::RankArt::badge(
        league_name(ui.league, Lang::En),
        ui.division,
        ui.track == Track::Deck,
        None,
    ) {
        commands.entity(slots.board.mark).insert(mark);
    }
    if painted.filters != Some(filter) {
        painted.filters = Some(filter);
        commands.entity(slots.board.filters).despawn_children();
        filters(commands, slots.board.filters, kit, ui);
    }
}

fn paint_board(
    commands: &mut Commands,
    board: &Board,
    kit: Kit,
    state: &LobbyState,
    painted: &mut Painted,
) {
    let net = &state.ranked_net;
    let ui = &state.ranked;
    let lang = kit.lang();
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
    if ui.browsing_seasons {
        let items = match &net.board {
            Some(api::Response::Seasons { items, .. }) => items.as_slice(),
            _ => &[],
        };
        let failed = net.board_error.is_some();
        if painted
            .seasons
            .as_ref()
            .is_none_or(|(old, b, e)| old != items || *b != net.board_busy || *e != failed)
        {
            commands.entity(board.list).despawn_children();
            board::seasons(commands, board.list, kit, items, net.board_busy, failed);
            painted.seasons = Some((items.to_vec(), net.board_busy, failed));
        }
        painted.rows = None;
    } else if !painted
        .rows
        .as_ref()
        .is_some_and(|(r, o, n)| r.as_slice() == entries && *o == offset && *n == notice)
    {
        commands.entity(board.list).despawn_children();
        if let Some(msg) = notice {
            label(
                commands,
                board.list,
                kit,
                msg.text(lang),
                kit.m.text,
                tokens::MUTED,
            );
        }
        for (i, row) in entries.iter().enumerate() {
            board_row(commands, board.list, kit, row, offset + i + 1, i);
        }
        painted.rows = Some((entries.to_vec(), offset, notice));
        painted.seasons = None;
    }
    let pager = if ui.browsing_seasons {
        Pager {
            season: Phrase::RankedSeasons.text(lang).to_owned(),
            page: ui.season_pages.len() + 1,
            previous: !ui.season_pages.is_empty(),
            next: matches!(
                net.board,
                Some(api::Response::Seasons { next: Some(_), .. })
            ),
            busy: net.board_busy,
        }
    } else {
        Pager {
            season: season.to_owned(),
            page: ui.pages.len() + 1,
            previous: !ui.pages.is_empty(),
            next,
            busy: net.board_busy,
        }
    };
    if painted.pager.as_ref() != Some(&pager) {
        paint_pager(commands, board.pager, kit, &pager);
        painted.pager = Some(pager);
    }
}

fn paint_pager(commands: &mut Commands, container: Entity, kit: Kit, pager: &Pager) {
    let lang = kit.lang();
    commands.entity(container).despawn_children();
    let words = Phrase::RankedPage.fill(lang, &[&pager.page.to_string()]);
    label(commands, container, kit, &words, kit.m.small, tokens::MUTED);
    let season_button = parts::menu_button(
        commands,
        kit,
        if pager.season.is_empty() {
            Phrase::RankedSeasons.text(lang)
        } else {
            &pager.season
        },
        Weight::Ghost,
        Press::Ranked(Action::Seasons),
    );
    orders::stop(commands, season_button, &ORDER, "season");
    commands.entity(container).add_child(season_button);
    let grow = parts::grow(commands);
    commands.entity(container).add_child(grow);
    for (i, (on, forward, icon, txt)) in [
        (
            pager.previous,
            false,
            glyph::ARROW_LEFT,
            Phrase::ChallengePrevious,
        ),
        (pager.next, true, glyph::ARROW_RIGHT, Phrase::ChallengeNext),
    ]
    .into_iter()
    .enumerate()
    {
        let b = controls::button_compact(
            commands,
            kit,
            Some(icon),
            txt.text(lang),
            Weight::Ghost,
            if on && !pager.busy {
                Live::Yes
            } else {
                Live::No(Phrase::RankedNoPage.text(lang))
            },
            Press::Ranked(Action::Page(forward)),
        );
        if on && !pager.busy {
            orders::item(commands, b, &ORDER, "pager", i);
        }
        commands.entity(container).add_child(b);
    }
}
