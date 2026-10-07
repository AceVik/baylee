//! Pods (DESIGN-v8 §1 row 5, MULTIPLAYER §C): me near at a duel's size and
//! every other seat an upright pod in a grid above — one row up to three,
//! two from four — squared to my chair (`facing_for`), seen from nearly
//! straight above. Nothing moves when a seat is visited: the camera zooms.

use super::super::{PILE_STRIP, Seat, SeatSlot, TableLayout, sides_of};
use super::facing_for;
use glam::Vec2;

/// The air between two pods of the grid, and between the grid and my pod:
/// a pile strip and a unit (§1 row 5).
pub const GUTTER: f32 = PILE_STRIP + 1.0;

/// The gap between my pod and the grid, where the dial stands: its compass
/// and a hair of felt round it.
pub const DIAL_GAP: f32 = 2.0 * crate::dial::COMPASS_R + 1.0;

/// How many rows the grid has for `others` seats: one up to three, two from
/// four.
#[must_use]
pub fn rows_for(others: usize) -> usize {
    if others <= 3 { 1 } else { 2 }
}

/// The pods at a table of `seats` on a canvas of `aspect`: `seated`'s pods
/// (every one a duel's board wide, #264), mine where the ring has it, the
/// others in turn order across the grid — the front row first, each row
/// left to right — so a side's seats (allies, `sides_of`, contiguous in turn
/// order) stand in adjacent cells, in one row where the row holds them.
///
/// The boards are cut at the widest aspect whose grid is no wider than the
/// canvas (bisected, as the Turntable's pair is): a duel's board stretches
/// to its canvas's width, and three of them abreast cut at the canvas's own
/// aspect made the camera stand back until a card was 16 px at four seats
/// on a laptop (measured), where narrower boards are drawn nearer.
#[must_use]
pub fn pods(seats: &[Seat], aspect: f32) -> TableLayout {
    let shape = |layout: &TableLayout| {
        layout
            .extent()
            .map_or(0.0, |(lo, hi)| (hi.x - lo.x) / (hi.y - lo.y).max(1e-3))
    };
    let whole = cut_at(seats, aspect);
    if shape(&whole) <= aspect + 1e-3 {
        return whole;
    }
    let (mut fits, mut wide) = (0.5_f32.min(aspect), aspect);
    for _ in 0..24 {
        let mid = f32::midpoint(fits, wide);
        if shape(&cut_at(seats, mid)) <= aspect {
            fits = mid;
        } else {
            wide = mid;
        }
    }
    cut_at(seats, fits)
}

/// The grid with every board cut at `aspect`.
fn cut_at(seats: &[Seat], aspect: f32) -> TableLayout {
    let ring = TableLayout::seated(seats, aspect, None);
    let mine = ring.slots[0];
    let others: Vec<usize> = sides_of(seats).into_iter().skip(1).flatten().collect();
    if others.is_empty() {
        return ring;
    }
    let template = ring.slots[others[0]];
    let foot = template.footprint();
    let cell = Vec2::new(foot.x * 2.0 + GUTTER, foot.y * 2.0 + GUTTER);
    let rows = rows_for(others.len());
    let per_row = others.len().div_ceil(rows);
    // The dial stands in the gap between my pod and the grid, at the
    // table's middle, so every jewel points at its pod: my side (my pod and
    // a partner's beside it) moved down to leave it, the grid above it.
    let drop = -(mine.center.y + mine.footprint().y + DIAL_GAP * 0.5);
    let front = DIAL_GAP * 0.5 + foot.y;
    let mut slots = ring.slots.clone();
    for slot in &mut slots {
        slot.center.y += drop;
    }
    for (k, &i) in others.iter().enumerate() {
        let (row, col) = (k / per_row, k % per_row);
        let in_row = (others.len() - row * per_row).min(per_row);
        // Two rows with a pod each straight above the dial would put two
        // jewels in one direction: the back row then stands a quarter cell
        // aside.
        let stagger = if row > 0 && in_row % 2 == 1 && per_row % 2 == 1 {
            0.25
        } else {
            0.0
        };
        #[allow(clippy::cast_precision_loss)] // a handful of seats
        let x = (col as f32 - (in_row as f32 - 1.0) * 0.5 + stagger) * cell.x;
        #[allow(clippy::cast_precision_loss)]
        let y = front + row as f32 * cell.y;
        slots[i] = SeatSlot {
            center: Vec2::new(x, y),
            facing: facing_for(seats, i),
            ..ring.slots[i]
        };
    }
    TableLayout {
        slots,
        radius: ring.radius,
    }
}
