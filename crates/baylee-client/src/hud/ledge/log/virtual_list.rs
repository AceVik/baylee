//! The game log as a bounded window of rows, rather than every row at once.
//!
//! The model keeps every line, but at most [`MAX_MOUNTED`] rows plus two
//! spacers are mounted. Heights and scroll positions are logical pixels.
//! Mounted rows are measured from `ComputedNode`; unseen rows use an estimate
//! until they first enter the window. Prefix offsets make finding that window
//! logarithmic rather than walking the whole book on every frame.
//!
//! A list may fold its turns ([`VirtualLog::fold_with`], [`LogFolds`]): every
//! turn but the newest stands as its heading alone, with the count of its
//! lines, until the player opens it (feedback of 24.10.2026: "the log is
//! huge; fold what can be grouped, folded by default"). The list keeps the
//! whole book beside what it shows, and a refold keeps every measured height
//! and the row the player was reading.

#[allow(clippy::wildcard_imports)] // the log's own vocabulary
use super::*;
use std::collections::{BTreeMap, BTreeSet};

/// One display row: a phase caption or the log line immediately after it.
#[derive(Clone, Debug, PartialEq)]
pub(crate) enum Entry {
    Phase(Phase),
    Line {
        order: usize,
        line: LogLine,
    },
    /// A turn's heading in a list that folds ([`fold`]): open, its lines
    /// follow it; folded, they are left out and it says how many there are.
    Turn {
        order: usize,
        line: LogLine,
        open: bool,
        lines: usize,
    },
}

impl Entry {
    /// Margins are outside `ComputedNode::size`; these are the only margins
    /// carried by the rows spawned here.
    fn margin(&self) -> f32 {
        match self {
            Self::Phase(_) => PHASE_GAP,
            Self::Line { line, .. } if line.header => LOG_TURN_GAP,
            Self::Turn { .. } => LOG_TURN_GAP,
            Self::Line { .. } => 0.0,
        }
    }

    /// The book order of the line this row is, or of the line a caption
    /// stands over, which is the next row.
    fn order(&self) -> Option<usize> {
        match self {
            Self::Line { order, .. } | Self::Turn { order, .. } => Some(*order),
            Self::Phase(_) => None,
        }
    }

    /// Whether it opens a turn.
    fn heading(&self) -> bool {
        match self {
            Self::Line { line, .. } => line.header,
            Self::Turn { .. } => true,
            Self::Phase(_) => false,
        }
    }
}

/// What `book` shows under `folds`: each turn's heading, and its lines only
/// where the turn is open. The newest turn is open unless the player folded
/// it; lines before the first heading (the opening hands) always show.
pub(crate) fn fold(book: &[Entry], folds: &LogFolds) -> Vec<Entry> {
    let newest = book.iter().rev().find_map(|entry| match entry {
        Entry::Line { line, .. } | Entry::Turn { line, .. } if line.header => Some(line.turn),
        _ => None,
    });
    let mut out = Vec::with_capacity(book.len());
    let mut at = 0;
    while at < book.len() {
        let (Entry::Line { order, line } | Entry::Turn { order, line, .. }) = &book[at] else {
            out.push(book[at].clone());
            at += 1;
            continue;
        };
        if !line.header {
            out.push(book[at].clone());
            at += 1;
            continue;
        }
        let end = book[at + 1..]
            .iter()
            .position(Entry::heading)
            .map_or(book.len(), |next| at + 1 + next);
        let body = &book[at + 1..end];
        let open = folds.open(line.turn, newest == Some(line.turn));
        out.push(Entry::Turn {
            order: *order,
            line: line.clone(),
            open,
            lines: body
                .iter()
                .filter(|entry| matches!(entry, Entry::Line { .. }))
                .count(),
        });
        if open {
            out.extend_from_slice(body);
        }
        at = end;
    }
    out
}

/// A row's identity across a refold: the order of its line, and whether it
/// is the caption over that line.
fn row_key(entries: &[Entry], index: usize) -> (usize, bool) {
    match entries[index].order() {
        Some(order) => (order, false),
        None => (
            entries[index + 1..]
                .iter()
                .find_map(Entry::order)
                .unwrap_or(usize::MAX),
            true,
        ),
    }
}

/// Turns new book lines into display entries, advancing the phase state used
/// by the next append. `first_order` is the line's index in the whole book.
pub(crate) fn append_entries(
    lines: &[LogLine],
    first_order: usize,
    run: &mut PhaseRun,
) -> Vec<Entry> {
    let mut out = Vec::with_capacity(lines.len() * 2);
    for (offset, line) in lines.iter().enumerate() {
        if let Some(phase) = run.heading(line) {
            out.push(Entry::Phase(phase));
        }
        out.push(Entry::Line {
            order: first_order + offset,
            line: line.clone(),
        });
    }
    out
}

/// Turns a whole book into display entries and returns its final phase state.
pub(crate) fn entries(lines: &[LogLine]) -> (Vec<Entry>, PhaseRun) {
    let mut run = PhaseRun::default();
    let entries = append_entries(lines, 0, &mut run);
    (entries, run)
}

/// The greatest number of mounted rows, before and after the visible band.
pub(crate) const MAX_MOUNTED: usize = 64;
const OVERSCAN: usize = 6;
const ESTIMATED_HEIGHT: f32 = 24.0;
const FALLBACK_VIEWPORT: f32 = 400.0;

/// One of the two nodes standing in for rows outside the mounted window.
#[derive(Component)]
pub struct LogSpacer;

/// A bounded, scrollable log window.
#[derive(Default, Debug)]
pub(crate) struct VirtualLog {
    /// Every entry of the book; `entries` is what of it is shown.
    book: Vec<Entry>,
    /// The [`LogFolds`] generation last folded with, once the list folds.
    folding: Option<u64>,
    /// The book changed in a way only a refold can show: a turn began, or
    /// lines arrived under a folded newest turn, or the book was replaced.
    refold: bool,
    /// Where a refold put the row the player was reading, for the next sync.
    reanchor: Option<f32>,
    entries: Vec<Entry>,
    heights: Vec<f32>,
    /// One more item than `heights`: the logical top of every row, plus the
    /// end including one trailing gap. `total()` removes that final gap.
    offsets: Vec<f32>,
    mounted: BTreeMap<usize, Entity>,
    spacers: Option<(Entity, Entity)>,
    range: std::ops::Range<usize>,
    viewport: f32,
    dirty: bool,
    /// Book orders appended since the last sync. Only a newly appended line
    /// arrives with motion; a row remounted by scrolling does not.
    fresh: BTreeSet<usize>,
}

impl VirtualLog {
    /// Forgets a list whose root the caller is about to remove. The caller
    /// owns that removal; this does not despawn anything twice.
    pub(crate) fn reset(&mut self) {
        self.book.clear();
        self.folding = None;
        self.refold = false;
        self.reanchor = None;
        self.entries.clear();
        self.heights.clear();
        self.offsets.clear();
        self.mounted.clear();
        self.spacers = None;
        self.range = 0..0;
        self.viewport = FALLBACK_VIEWPORT;
        self.dirty = false;
        self.fresh.clear();
    }

    /// Replaces every entry after a language, text or book rewrite. Existing
    /// mounted rows are despawned by the next sync; the two spacers survive.
    pub(crate) fn replace(&mut self, entries: Vec<Entry>) {
        self.book.clone_from(&entries);
        self.refold = self.folding.is_some();
        self.entries = entries;
        self.heights = vec![ESTIMATED_HEIGHT; self.entries.len()];
        self.offsets = Vec::with_capacity(self.entries.len() + 1);
        self.rebuild_offsets_from(0);
        self.dirty = true;
        self.fresh.clear();
    }

    /// Adds display entries without rewriting any row already mounted.
    pub(crate) fn append(&mut self, entries: Vec<Entry>) {
        if entries.is_empty() {
            return;
        }
        self.book.extend(entries.iter().cloned());
        if self.folding.is_some() {
            let opens_a_turn = entries.iter().any(Entry::heading);
            let under_a_fold = matches!(
                self.entries.iter().rev().find(|entry| entry.heading()),
                Some(Entry::Turn { open: false, .. })
            );
            if opens_a_turn || under_a_fold {
                for order in entries.iter().filter_map(Entry::order) {
                    self.fresh.insert(order);
                }
                self.refold = true;
                return;
            }
        }
        if self.offsets.is_empty() {
            self.offsets.push(0.0);
        }
        self.entries.reserve(entries.len());
        self.heights.reserve(entries.len());
        self.offsets.reserve(entries.len());
        for entry in entries {
            if let Entry::Line { order, .. } = entry {
                self.fresh.insert(order);
            }
            self.entries.push(entry);
            self.heights.push(ESTIMATED_HEIGHT);
            let next = self.offsets.last().copied().unwrap_or(0.0) + ESTIMATED_HEIGHT + LOG_ROW_GAP;
            self.offsets.push(next);
        }
    }

    /// Folds the list's turns as `folds` says, from now on. Refolds only
    /// when the player opened or closed a turn since the last call, or the
    /// book changed what folding shows; `scroll` is where the list stands,
    /// so the row the player is reading stays where it is (or, folded away,
    /// its turn's heading takes its place).
    pub(crate) fn fold_with(&mut self, folds: &LogFolds, scroll: f32) {
        let generation = folds.generation();
        if self.folding == Some(generation) && !self.refold {
            return;
        }
        self.folding = Some(generation);
        self.refold = false;
        let mut known = BTreeMap::new();
        for index in 0..self.entries.len() {
            known.insert(row_key(&self.entries, index), self.heights[index]);
        }
        let anchor = (!self.entries.is_empty()).then(|| {
            let index = self.offsets[..self.entries.len()]
                .partition_point(|top| *top <= scroll)
                .saturating_sub(1);
            (row_key(&self.entries, index), scroll - self.offset(index))
        });
        let shown = fold(&self.book, folds);
        self.heights = (0..shown.len())
            .map(|index| {
                known
                    .get(&row_key(&shown, index))
                    .copied()
                    .unwrap_or(ESTIMATED_HEIGHT)
            })
            .collect();
        self.entries = shown;
        self.offsets.clear();
        self.rebuild_offsets_from(0);
        self.dirty = true;
        self.reanchor = anchor.and_then(|(key, within)| {
            if let Some(index) = (0..self.entries.len()).find(|&i| row_key(&self.entries, i) == key)
            {
                return Some(self.offset(index) + within);
            }
            // Folded away: its turn's heading, the last one at or before it.
            let heading = (0..self.entries.len()).rfind(
                |&i| matches!(&self.entries[i], Entry::Turn { order, .. } if *order <= key.0),
            )?;
            Some(self.offset(heading))
        });
    }

    fn rebuild_offsets_from(&mut self, from: usize) {
        if self.offsets.is_empty() {
            self.offsets.push(0.0);
        }
        self.offsets.resize(self.heights.len() + 1, 0.0);
        for index in from..self.heights.len() {
            self.offsets[index + 1] = self.offsets[index] + self.heights[index] + LOG_ROW_GAP;
        }
    }

    fn offset(&self, index: usize) -> f32 {
        self.offsets
            .get(index.min(self.entries.len()))
            .copied()
            .unwrap_or(0.0)
    }

    fn total(&self) -> f32 {
        if self.entries.is_empty() {
            0.0
        } else {
            (self.offset(self.entries.len()) - LOG_ROW_GAP).max(0.0)
        }
    }

    fn range(&self, scroll: f32, viewport: f32) -> std::ops::Range<usize> {
        let len = self.entries.len();
        if len == 0 {
            return 0..0;
        }
        let scroll = scroll.clamp(0.0, self.total());
        let tops = &self.offsets[..len];
        let visible = tops.partition_point(|top| *top <= scroll).saturating_sub(1);
        let bottom = scroll + viewport.max(1.0);
        let visible_end = tops.partition_point(|top| *top < bottom).max(visible + 1);
        let mut start = visible.saturating_sub(OVERSCAN);
        let mut end = (visible_end + OVERSCAN).min(len);
        if end - start > MAX_MOUNTED {
            if end == len && bottom >= self.total() - LOG_ROW_GAP {
                start = end - MAX_MOUNTED;
            } else {
                end = start + MAX_MOUNTED;
            }
        }
        start..end
    }

    /// Heights that make the mounted window occupy exactly the full book's
    /// range. A hidden spacer has no flex gap around it; a visible one already
    /// includes that gap on the mounted rows' side, hence the subtraction.
    fn spacer_heights(&self) -> (f32, f32) {
        if self.entries.is_empty() {
            return (0.0, 0.0);
        }
        let before = if self.range.start == 0 {
            0.0
        } else {
            (self.offset(self.range.start) - LOG_ROW_GAP).max(0.0)
        };
        let after = if self.range.end == self.entries.len() {
            0.0
        } else {
            (self.offset(self.entries.len()) - self.offset(self.range.end) - LOG_ROW_GAP).max(0.0)
        };
        (before, after)
    }

    fn spacer(commands: &mut Commands, height: f32) -> Entity {
        commands
            .spawn((
                LogSpacer,
                Node {
                    display: if height > 0.0 {
                        Display::Flex
                    } else {
                        Display::None
                    },
                    height: px(height.max(0.0)),
                    flex_shrink: 0.0,
                    ..default()
                },
                Pickable::IGNORE,
            ))
            .id()
    }

    fn update_spacers(&self, nodes: &mut Query<&mut Node, With<LogSpacer>>) {
        let Some((before, after)) = self.spacers else {
            return;
        };
        let (before_height, after_height) = self.spacer_heights();
        for (entity, height) in [(before, before_height), (after, after_height)] {
            if let Ok(mut node) = nodes.get_mut(entity) {
                let height = px(height);
                if node.height != height {
                    node.height = height;
                }
                let display = if height == px(0.0) {
                    Display::None
                } else {
                    Display::Flex
                };
                if node.display != display {
                    node.display = display;
                }
            }
        }
    }

    /// Moves the scroll position and the follower's memory together, only
    /// when the target really differs.
    fn move_to(target: f32, follow: &mut LogFollow, scroll: &mut ScrollPosition) {
        if (scroll.y - target).abs() > 0.01 {
            scroll.y = target;
            follow.set = target;
        }
    }

    #[allow(clippy::too_many_arguments)] // the log's drawing context, passed through
    fn mount<F>(
        &mut self,
        commands: &mut Commands,
        list: Entity,
        fonts: &UiFonts,
        lang: Lang,
        wording: Option<&Wording>,
        inks: &LineInks,
        decorate: &mut F,
    ) where
        F: FnMut(usize, &LogLine, bool, &mut EntityCommands),
    {
        if self.dirty {
            for (_, entity) in std::mem::take(&mut self.mounted) {
                commands.entity(entity).despawn();
            }
        } else {
            let outside: Vec<usize> = self
                .mounted
                .keys()
                .filter(|index| !self.range.contains(index))
                .copied()
                .collect();
            for index in outside {
                if let Some(entity) = self.mounted.remove(&index) {
                    commands.entity(entity).despawn();
                }
            }
        }
        if self.entries.is_empty() {
            if let Some((before, after)) = self.spacers.take() {
                commands.entity(before).despawn();
                commands.entity(after).despawn();
            }
            commands.entity(list).replace_children(&[]);
            self.dirty = false;
            return;
        }

        let (before_height, after_height) = self.spacer_heights();
        let (before, after) = self.spacers.unwrap_or_else(|| {
            (
                Self::spacer(commands, before_height),
                Self::spacer(commands, after_height),
            )
        });
        self.spacers = Some((before, after));
        let mut children = Vec::with_capacity(self.range.len() + 2);
        children.push(before);
        for index in self.range.clone() {
            let entity = if let Some(entity) = self.mounted.get(&index) {
                *entity
            } else {
                let entity = match &self.entries[index] {
                    Entry::Phase(phase) => spawn_phase(commands, fonts, lang, *phase, inks),
                    Entry::Turn {
                        order,
                        line,
                        open,
                        lines,
                    } => {
                        let row = spawn_turn(commands, fonts, lang, line, inks, *open, *lines);
                        decorate(*order, line, false, &mut commands.entity(row));
                        row
                    }
                    Entry::Line { order, line } => {
                        let swatch = wording.map(|words| subject_ink(words, line.subject));
                        let row = spawn_line(commands, fonts, lang, line, inks, swatch);
                        decorate(
                            *order,
                            line,
                            self.fresh.contains(order),
                            &mut commands.entity(row),
                        );
                        row
                    }
                };
                self.mounted.insert(index, entity);
                entity
            };
            children.push(entity);
        }
        children.push(after);
        commands.entity(list).replace_children(&children);
        self.dirty = false;
    }

    /// Measures mounted rows and rebuilds only the changed tail of the prefix
    /// offsets. At rest this only reads the bounded mounted window.
    fn measure(&mut self, nodes: &Query<&ComputedNode>) {
        // Replaced entries do not belong to the old mounted rows. A replay
        // seek can also shorten the book below those rows' indices.
        if self.dirty {
            return;
        }
        let mut first_changed = None;
        for (index, entity) in &self.mounted {
            let Ok(node) = nodes.get(*entity) else {
                continue;
            };
            // A newly spawned row has a required but empty ComputedNode until
            // layout first visits it. That is not a one-pixel measurement.
            if node.size().y <= 0.0 {
                continue;
            }
            let height = (node.size().y * node.inverse_scale_factor()
                + self.entries[*index].margin())
            .max(1.0);
            if (self.heights[*index] - height).abs() > 0.01 {
                self.heights[*index] = height;
                first_changed = Some(first_changed.map_or(*index, |old: usize| old.min(*index)));
            }
        }
        if let Some(from) = first_changed {
            self.rebuild_offsets_from(from);
        }
    }

    /// Synchronises one list's mounted rows with its entries and scroll
    /// position. Only missing rows are spawned; rows shared by the old and
    /// new ranges keep their entities, links and copied-line order.
    #[allow(clippy::too_many_arguments)] // a Bevy drawing context and its two queries
    pub(crate) fn sync<F>(
        &mut self,
        commands: &mut Commands,
        list: Entity,
        fonts: &UiFonts,
        lang: Lang,
        wording: Option<&Wording>,
        inks: &LineInks,
        follow: &mut LogFollow,
        scroll: &mut ScrollPosition,
        nodes: &Query<&ComputedNode>,
        spacer_nodes: &mut Query<&mut Node, With<LogSpacer>>,
        decorate: &mut F,
    ) where
        F: FnMut(usize, &LogLine, bool, &mut EntityCommands),
    {
        if self.viewport <= 0.0 {
            self.viewport = FALLBACK_VIEWPORT;
        }
        let mut actual_end = None;
        if let Ok(node) = nodes.get(list) {
            let scale = node.inverse_scale_factor();
            if node.size().y > 0.0 {
                self.viewport = (node.size().y * scale).max(1.0);
            }
            if node.content_size().y > 0.0 {
                actual_end = Some((node.content_size().y - node.size().y).max(0.0) * scale);
            }
        }
        // Input can move the list before this system runs. Recognise that
        // before following the end, or the first wheel notch would be undone.
        if (scroll.y - follow.set).abs() > f32::EPSILON {
            let end = actual_end.unwrap_or_else(|| (self.total() - self.viewport).max(0.0));
            follow.stuck = scroll.y >= end - FOLLOW_SLACK;
            follow.set = scroll.y;
        }
        if let Some(at) = self.reanchor.take()
            && !follow.stuck
        {
            Self::move_to(at, follow, scroll);
        }

        // Measured overscan can grow above the reading position. Keep the
        // visible row and its intra-row offset, not an obsolete pixel offset.
        let anchor = self
            .offsets
            .partition_point(|top| *top <= scroll.y)
            .saturating_sub(1);
        let within = scroll.y - self.offset(anchor);
        self.measure(nodes);
        if !follow.stuck {
            Self::move_to((self.offset(anchor) + within).max(0.0), follow, scroll);
        }
        let end = (self.total() - self.viewport).max(0.0);
        if follow.stuck || scroll.y > end {
            Self::move_to(end, follow, scroll);
        }
        let range = self.range(scroll.y, self.viewport);
        if self.dirty || range != self.range {
            self.range = range;
            self.mount(commands, list, fonts, lang, wording, inks, decorate);
        }
        self.measure(nodes);
        self.update_spacers(spacer_nodes);
        if follow.stuck {
            let end = (self.total() - self.viewport).max(0.0);
            Self::move_to(end, follow, scroll);
        }
        // Off-screen arrivals did not animate under the old full list either:
        // their short arrival had finished before the player scrolled to them.
        self.fresh.clear();
    }
}

#[cfg(test)]
mod tests;
