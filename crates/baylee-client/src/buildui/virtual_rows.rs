#![allow(clippy::cast_precision_loss)] // Bounded UI pixel coordinates and row indices.
//! The builder's lists as virtual lists (§10 #6): one node as tall as the
//! whole list, and rows mounted only within a viewport of what is shown —
//! one viewport above and one below, so scrolling a row a frame never shows
//! a row that is not there yet (`ListProbe::placeholders`).
//!
//! Every item's top and height are known before anything is laid out: a row
//! is the kit's pitch, a section heading its own, and a row clips what does
//! not fit, so the offsets never depend on the words.

use super::rows::{self, InDeck};
use super::{Env, Kit, LobbyState, PoolView};
use crate::hud::UiFonts;
use crate::lobby::List;
use crate::shellkit::{Frame, ShellMetrics};
use baylee_client_core::deckbuilder::{Grouping, Section, SectionKey, Zone};
use bevy::prelude::*;
use bevy::ui::CalculatedClip;
use std::collections::BTreeMap;

/// One item of a list.
#[derive(Clone, Copy, PartialEq, Debug)]
enum Item {
    /// A detailed pool row: its card's slot and its order among pool cards.
    Pool(usize, usize),
    /// A compact pool row: its card's slot and its order among pool cards.
    PoolCompact(usize, usize),
    /// A grid row of up to 4 `(slot, order)` pool cards, with the column count.
    PoolGrid([Option<(usize, usize)>; 4], usize),
    /// A pool section's heading when sectioned by card type.
    PoolHead(SectionKey, u32, bool),
    /// A deck section's heading: its key, its cards, whether it is folded.
    Head(SectionKey, u32, bool),
    /// A deck row: its index in `entries(zone)`, and its place among the
    /// drawn rows.
    Deck(usize, usize),
}

/// A virtual list: what is in it, where each item stands, and which are
/// mounted.
#[derive(Component)]
pub(crate) struct VirtualList {
    items: Vec<Item>,
    tops: Vec<f32>,
    heights: Vec<f32>,
    m: ShellMetrics,
    german: bool,
    zone: Zone,
    mounted: BTreeMap<usize, Entity>,
    last_from: f32,
    last_to: f32,
}

/// What the lists have done, for `/state` (dev-control) and the tests.
#[derive(Resource, Default, Clone, Copy, Debug)]
pub(crate) struct ListProbe {
    /// Items found inside the visible rectangle before they were mounted:
    /// a row that would have been drawn empty (§17 WP4: "0 placeholders").
    pub(crate) placeholders: u64,
    /// Rows mounted now, over every list.
    pub(crate) mounted: usize,
}

/// How far below/above the current scroll offset the first draw mounts rows,
/// before any layout has said how tall the list's window is.
const FIRST_MOUNT: f32 = 1600.0;

fn spawn_list(commands: &mut Commands, env: &Env, list: VirtualList, scroll_y: f32) -> Entity {
    let height = list
        .tops
        .last()
        .zip(list.heights.last())
        .map_or(0.0, |(t, h)| t + h + 8.0);
    let entity = commands
        .spawn((
            Node {
                width: Val::Percent(100.0),
                height: crate::shellkit::px_fixed(height),
                flex_shrink: 0.0,
                ..default()
            },
            Pickable::IGNORE,
        ))
        .id();
    let mut list = list;
    let kit = env.kit;
    let from = (scroll_y - FIRST_MOUNT).max(0.0);
    let to = scroll_y.max(0.0) + FIRST_MOUNT;
    for i in within(&list, from, to) {
        mount(
            commands,
            kit,
            env.state,
            Some(env.prefs),
            entity,
            &mut list,
            i,
        );
    }
    commands.entity(entity).insert(list);
    entity
}

/// The pool's rows, for `slots` in order (list, compact, or grid; optionally grouped by type).
#[allow(clippy::too_many_lines)]
pub(super) fn pool(commands: &mut Commands, env: &Env, slots: Vec<usize>) -> Entity {
    let view = env.ui().pool_view;
    let grouped = env.ui().pool_grouped;
    let head_h = env.head_pitch();
    let pitch = match view {
        PoolView::List => env.pool_pitch(),
        PoolView::Compact => (env.kit.m.hit + env.kit.m.scaled(3.0)).round(),
        PoolView::Grid => env.kit.m.scaled(264.0).round(),
    };
    let cols = match env.kit.m.frame {
        Frame::Vast | Frame::Wide => 4,
        Frame::Phone => 2,
        _ => 3,
    };
    let mut items = Vec::new();
    let mut tops = Vec::new();
    let mut heights = Vec::new();
    let mut y = 0.0;
    let mut order = 0;

    let append_cards = |section_slots: &[usize],
                        items: &mut Vec<Item>,
                        tops: &mut Vec<f32>,
                        heights: &mut Vec<f32>,
                        y: &mut f32,
                        order: &mut usize| match view {
        PoolView::List => {
            for &slot in section_slots {
                items.push(Item::Pool(slot, *order));
                *order += 1;
                tops.push(*y);
                heights.push(pitch);
                *y += pitch;
            }
        }
        PoolView::Compact => {
            for &slot in section_slots {
                items.push(Item::PoolCompact(slot, *order));
                *order += 1;
                tops.push(*y);
                heights.push(pitch);
                *y += pitch;
            }
        }
        PoolView::Grid => {
            for chunk in section_slots.chunks(cols) {
                let mut row_cells = [None; 4];
                for (c, &slot) in chunk.iter().enumerate() {
                    row_cells[c] = Some((slot, *order));
                    *order += 1;
                }
                items.push(Item::PoolGrid(row_cells, cols));
                tops.push(*y);
                heights.push(pitch);
                *y += pitch;
            }
        }
    };

    if grouped {
        let allowed: std::collections::HashSet<usize> = slots.iter().copied().collect();
        for section in env.deck().pool_sections(Grouping::Type) {
            let group_slots: Vec<usize> = section
                .rows
                .into_iter()
                .filter(|slot| allowed.contains(slot))
                .collect();
            if group_slots.is_empty() {
                continue;
            }
            let key = section.key;
            let folded = env.ui().pool_collapsed.contains(&key);
            items.push(Item::PoolHead(
                key,
                u32::try_from(group_slots.len()).unwrap_or(section.cards),
                folded,
            ));
            tops.push(y);
            heights.push(head_h);
            y += head_h;
            if !folded {
                append_cards(
                    &group_slots,
                    &mut items,
                    &mut tops,
                    &mut heights,
                    &mut y,
                    &mut order,
                );
            }
        }
    } else {
        append_cards(
            &slots,
            &mut items,
            &mut tops,
            &mut heights,
            &mut y,
            &mut order,
        );
    }

    let list = VirtualList {
        items,
        tops,
        heights,
        m: env.kit.m,
        german: env.kit.german,
        zone: Zone::Main,
        mounted: BTreeMap::new(),
        last_from: -1.0,
        last_to: -1.0,
    };
    spawn_list(commands, env, list, env.scrolled.get(List::Pool))
}

/// The deck list of `zone`, in `sections`, the folded ones as headings
/// alone.
pub(super) fn deck(commands: &mut Commands, env: &Env, zone: Zone, sections: &[Section]) -> Entity {
    let (row, head) = (env.deck_pitch(), env.head_pitch());
    let mut items = Vec::new();
    let mut tops = Vec::new();
    let mut heights = Vec::new();
    let mut y = 0.0;
    let mut order = 0;
    for section in sections {
        let folded = env.ui().collapsed.contains(&section.key);
        items.push(Item::Head(section.key, section.cards, folded));
        tops.push(y);
        heights.push(head);
        y += head;
        if folded {
            continue;
        }
        for &at in &section.rows {
            items.push(Item::Deck(at, order));
            order += 1;
            tops.push(y);
            heights.push(row);
            y += row;
        }
    }
    let list = VirtualList {
        items,
        tops,
        heights,
        m: env.kit.m,
        german: env.kit.german,
        zone,
        mounted: BTreeMap::new(),
        last_from: -1.0,
        last_to: -1.0,
    };
    spawn_list(commands, env, list, env.scrolled.get(List::Deck))
}

fn mount(
    commands: &mut Commands,
    kit: Kit,
    state: &LobbyState,
    prefs: Option<&baylee_client_core::prefs::Preferences>,
    parent: Entity,
    list: &mut VirtualList,
    i: usize,
) {
    let (top, height) = (list.tops[i], list.heights[i]);
    let row = match list.items[i] {
        Item::Pool(slot, order) => {
            rows::pool_row_at(commands, kit, state, prefs, slot, order, top, height)
        }
        Item::PoolCompact(slot, order) => {
            rows::pool_compact_row(commands, kit, state, prefs, slot, order, top, height)
        }
        Item::PoolGrid(ref cells, cols) => Some(rows::pool_grid_row(
            commands, kit, state, prefs, cells, cols, top, height,
        )),
        Item::PoolHead(key, cards, folded) => Some(rows::pool_section_head(
            commands,
            kit,
            state.lobby.lang(),
            key,
            cards,
            folded,
            top,
            height,
        )),
        Item::Head(key, cards, folded) => Some(rows::section_head(
            commands,
            kit,
            state.lobby.lang(),
            key,
            cards,
            folded,
            top,
            height,
        )),
        Item::Deck(at, order) => rows::deck_row(
            commands, kit, state, prefs, list.zone, at, order, top, height,
        ),
    };
    if let Some(row) = row {
        commands.entity(parent).add_child(row);
        list.mounted.insert(i, row);
    }
}

/// The items that intersect `from..to` (logical pixels from the list's top),
/// found in `O(log N)` via binary search (`partition_point`).
fn within(list: &VirtualList, from: f32, to: f32) -> std::ops::Range<usize> {
    let end = list.tops.partition_point(|&top| top < to);
    let mut start = list.tops[..end]
        .partition_point(|&top| top <= from)
        .saturating_sub(1);
    while start < end && list.tops[start] + list.heights[start] <= from {
        start += 1;
    }
    start..end.max(start)
}

/// Mounts the rows within a viewport of what is shown, drops the ones two
/// viewports away, and counts any row found visible and not yet there.
#[allow(clippy::type_complexity)] // geometry of the list and its scroller
pub(crate) fn update(
    mut commands: Commands,
    state: Res<LobbyState>,
    prefs: Option<Res<crate::prefs::Prefs>>,
    fonts: Option<Res<UiFonts>>,
    mut lists: Query<(
        Entity,
        &mut VirtualList,
        &ComputedNode,
        &UiGlobalTransform,
        &CalculatedClip,
        &ChildOf,
    )>,
    scrollers: Query<(&ScrollPosition, &ComputedNode)>,
    mut probe: ResMut<ListProbe>,
) {
    let Some(fonts) = fonts else {
        return;
    };
    let prefs_ref = prefs.as_deref().map(crate::prefs::Prefs::all);
    let mut mounted = 0;
    for (entity, mut list, node, transform, clip, parent) in &mut lists {
        if node.size.min_element() <= 0.0 {
            mounted += list.mounted.len();
            continue;
        }
        let delta = scroll_delta(parent, &scrollers);
        let top = transform.translation.y - node.size.y * 0.5 - delta;
        let Some(visible) = crate::shellkit::clip_bounds(clip) else {
            mounted += list.mounted.len();
            continue;
        };
        let scale = node.inverse_scale_factor();
        let from = ((visible.min.y - top) * scale).max(0.0);
        let to = ((visible.max.y - top) * scale).max(from);
        let fast_jump = (delta.abs() * scale) > (to - from).max(1.0) * 0.65;
        if !fast_jump
            && list.last_from >= 0.0
            && (from - list.last_from).abs() < 0.5
            && (to - list.last_to).abs() < 0.5
        {
            mounted += list.mounted.len();
            continue;
        }
        list.last_from = from;
        list.last_to = to;
        let kit = Kit {
            fonts: &fonts,
            m: list.m,
            german: list.german,
        };
        let view = (to - from).max(1.0);
        let seen = within(&list, from, to);
        let missing = seen.filter(|i| !list.mounted.contains_key(i)).count();
        if missing > 0 && probe.placeholders < u64::MAX {
            probe.placeholders += missing as u64;
        }
        // During high-velocity scrollbar drags, mount only the visible viewport
        // so we don't spawn/layout 3 full viewports per frame for intermediate
        // scroll positions; expand to the overscan once the drag slows.
        let overscan = if fast_jump { 0.0 } else { 0.45 * view };
        let keep = within(&list, (from - 1.25 * view).max(0.0), to + 1.25 * view);
        let gone: Vec<usize> = list
            .mounted
            .keys()
            .copied()
            .filter(|i| !keep.contains(i))
            .collect();
        for i in gone {
            if let Some(row) = list.mounted.remove(&i) {
                commands.entity(row).despawn();
            }
        }
        for i in within(&list, (from - overscan).max(0.0), to + overscan) {
            if !list.mounted.contains_key(&i) {
                mount(&mut commands, kit, &state, prefs_ref, entity, &mut list, i);
            }
        }
        mounted += list.mounted.len();
    }
    if probe.mounted != mounted {
        probe.mounted = mounted;
    }
}

/// Where the cursor's row of a list stands, for the cursor's scroll: its
/// top and its height. The pool's cursor is its order among drawn cards; the
/// deck's is its place among the drawn rows, headings aside.
pub(crate) fn place_of(list: &VirtualList, cursor: usize) -> Option<(f32, f32)> {
    let i = if is_pool(list) {
        list.items.iter().position(|item| match item {
            Item::Pool(_, order) | Item::PoolCompact(_, order) => *order == cursor,
            Item::PoolGrid(cells, _) => cells.iter().flatten().any(|(_, order)| *order == cursor),
            _ => false,
        })?
    } else {
        list.items
            .iter()
            .position(|item| matches!(item, Item::Deck(_, order) if *order == cursor))?
    };
    Some((*list.tops.get(i)?, *list.heights.get(i)?))
}

/// Whether a list is the pool's.
pub(crate) fn is_pool(list: &VirtualList) -> bool {
    matches!(
        list.items.first(),
        Some(
            Item::Pool(_, _)
                | Item::PoolCompact(_, _)
                | Item::PoolGrid(_, _)
                | Item::PoolHead(_, _, _)
        )
    )
}

/// Keeps every pool row's count badge true as the deck changes, without
/// redrawing a row.
pub(crate) fn in_deck(
    state: Res<LobbyState>,
    mut badges: Query<(&InDeck, &mut Node, &Children)>,
    mut texts: Query<&mut Text>,
) {
    if !state.is_changed() {
        return;
    }
    let deck = state.lobby.builder();
    for (badge, mut node, children) in &mut badges {
        let held = deck.count_of(badge.0, Zone::Main) + deck.count_of(badge.0, Zone::Side);
        let display = if held == 0 {
            Display::None
        } else {
            Display::Flex
        };
        if node.display != display {
            node.display = display;
        }
        for child in children.iter() {
            if let Ok(mut text) = texts.get_mut(child) {
                let value = held.to_string();
                if text.0 != value {
                    text.0 = value;
                }
            }
        }
    }
}

// Account for this frame's scroll input before Bevy computes new transforms.
fn scroll_delta(parent: &ChildOf, scrollers: &Query<(&ScrollPosition, &ComputedNode)>) -> f32 {
    scrollers
        .get(parent.parent())
        .map_or(0.0, |(position, node)| {
            position.y / node.inverse_scale_factor().max(f32::EPSILON) - node.scroll_position.y
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn list(heights: &[f32]) -> VirtualList {
        let mut tops = Vec::new();
        let mut y = 0.0;
        for h in heights {
            tops.push(y);
            y += h;
        }
        VirtualList {
            items: heights.iter().map(|_| Item::Pool(0, 0)).collect(),
            tops,
            heights: heights.to_vec(),
            m: ShellMetrics::of(
                crate::shellkit::Viewport::desktop(1920.0, 1080.0),
                crate::shellkit::TextSize::default(),
            ),
            german: false,
            zone: Zone::Main,
            mounted: BTreeMap::new(),
            last_from: -1.0,
            last_to: -1.0,
        }
    }

    #[test]
    fn the_window_takes_exactly_the_items_it_touches() {
        let rows = list(&[72.0; 10]);
        assert_eq!(within(&rows, 0.0, 72.0), 0..1);
        assert_eq!(within(&rows, 10.0, 150.0), 0..3);
        assert_eq!(within(&rows, 144.0, 216.0), 2..3);
        assert_eq!(within(&rows, 700.0, 900.0), 9..10);
        assert_eq!(within(&rows, 900.0, 1000.0), 10..10);
        let mixed = list(&[30.0, 54.0, 54.0, 30.0, 54.0]);
        assert_eq!(within(&mixed, 0.0, 31.0), 0..2);
        assert_eq!(within(&mixed, 140.0, 170.0), 3..5);
    }

    /// No two items of a list overlap and none has a gap above it: the
    /// offsets are the pitch, whatever a row's words are (§10 #6).
    #[test]
    fn items_stand_edge_to_edge() {
        let mixed = list(&[30.0, 54.0, 54.0, 30.0, 54.0]);
        for i in 1..mixed.items.len() {
            assert!((mixed.tops[i] - (mixed.tops[i - 1] + mixed.heights[i - 1])).abs() < 1e-3);
        }
    }
}
