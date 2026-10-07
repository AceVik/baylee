//! The Turntable (DESIGN-v8 §1 row 3): the pair — me near, the seat of
//! interest's side across, as a duel — and every other seat a **side mat**
//! on a flank, drawn small (`SeatSlot::scale`) and squared to my chair as
//! the duel's boards are (`facing_for`: an opponent's turned to me as the
//! duel's opponent's is, a teammate's as mine).
//!
//! The flanks keep the ring's order: the sides between mine and the one
//! across, going clockwise, stand on the left from the bottom up; the rest
//! on the right from the top down — so reading round the table from my
//! chair, clockwise, still reads the turn order.

use super::super::{PILE_STRIP, Seat, SeatSlot, TableLayout, sides_of};
use super::facing_for;
use super::pair::{across_side, pair};
use baylee_core::ids::PlayerId;
use glam::Vec2;

/// How large a side mat is drawn against the pair's boards, by the table's
/// seat count (DESIGN-v8 §1 row 3): 0.55 at three and four, 0.40 at five
/// and six, 0.33 at seven and eight.
#[must_use]
pub fn side_scale(seats: usize) -> f32 {
    match seats {
        0..=4 => 0.55,
        5..=6 => 0.40,
        _ => 0.33,
    }
}

/// The air between the pair's footprint and a flank, and between two side
/// mats on one flank, table units.
pub const FLANK_GAP: f32 = 1.0;

/// The Turntable at a table of `seats` on a canvas of `aspect`, the seat
/// of interest `interest` (its side across; at home the side across the
/// ring).
///
/// The pair is seated at the widest aspect that leaves the whole table —
/// pair and flanks — no wider than the canvas: a duel's boards stretch to
/// fill the canvas's width (a pod is 27 units wide at a laptop's 2.0, 18 at
/// 1.44), so a pair seated at the canvas's own aspect leaves no room beside
/// it, and the flanks would make the camera stand back and draw the pair
/// smaller than the duel's (measured: the across card 30 px for the duel's
/// 45 at 1708). Narrowed, the pair keeps the duel's depth, so its cards are
/// the duel's size; its rows fan sooner.
#[must_use]
pub fn turntable(seats: &[Seat], aspect: f32, interest: Option<PlayerId>) -> TableLayout {
    let across = across_side(seats, interest);
    let shape = |layout: &TableLayout| {
        layout
            .extent()
            .map_or(0.0, |(lo, hi)| (hi.x - lo.x) / (hi.y - lo.y).max(1e-3))
    };
    let at = |a: f32| flanked(seats, a, &across);
    let whole = at(aspect);
    if shape(&whole) <= aspect + 1e-3 {
        return whole;
    }
    // The widest pair that still fits: bisected, as the ring's radii are.
    let (mut fits, mut wide) = (0.5_f32.min(aspect), aspect);
    for _ in 0..24 {
        let mid = f32::midpoint(fits, wide);
        if shape(&at(mid)) <= aspect {
            fits = mid;
        } else {
            wide = mid;
        }
    }
    at(fits)
}

/// The pair seated at `aspect` with every other seat on its flank.
fn flanked(seats: &[Seat], aspect: f32, across: &[usize]) -> TableLayout {
    let mut layout = pair(seats, aspect, across);
    let sides = sides_of(seats);
    let Some(across_at) = sides.iter().position(|side| side.as_slice() == across) else {
        return layout;
    };
    let (left, right): (Vec<usize>, Vec<usize>) = {
        let between: Vec<usize> = sides[1..across_at].iter().flatten().copied().collect();
        // The rest, from just past the side across round to just before
        // mine: top to bottom on the right.
        let rest: Vec<usize> = sides[across_at + 1..].iter().flatten().copied().collect();
        (between, rest)
    };
    let scale = side_scale(seats.len());
    let (lo, hi) = layout
        .extent()
        .unwrap_or((Vec2::splat(-10.0), Vec2::splat(10.0)));
    // A side mat's drawn footprint, from a pair seat's own (every seat's
    // board is the standard one).
    let model = layout
        .slots
        .iter()
        .find(|s| !s.parked)
        .copied()
        .unwrap_or(layout.slots[0]);
    let half = model.reach() * scale;
    let foot = Vec2::new(half.x + PILE_STRIP * scale, half.y);
    let middle = f32::midpoint(lo.y, hi.y);
    // My seat stays the nearest (DESIGN-v8 §3, invariant 1): a column too
    // tall to stand about the middle stands up from just behind my seat.
    let mine = layout.slots[0].center.y;
    let mut stand = |column: &[usize], x: f32, upward: bool| {
        let count = column.len();
        #[allow(clippy::cast_precision_loss)] // a handful of seats
        let span = count as f32 * foot.y * 2.0 + count.saturating_sub(1) as f32 * FLANK_GAP;
        let bottom = (middle - span * 0.5 + foot.y).max(mine + 0.01);
        for (k, &i) in column.iter().enumerate() {
            #[allow(clippy::cast_precision_loss)]
            let step = (foot.y * 2.0 + FLANK_GAP) * k as f32;
            // From the bottom up on the left, from the top down on the right.
            let y = if upward {
                bottom + step
            } else {
                bottom + span - foot.y * 2.0 - step
            };
            let ring = layout.slots[i];
            layout.slots[i] = SeatSlot {
                center: Vec2::new(x, y),
                facing: facing_for(seats, i),
                half_extent: half,
                reclaimed: 0.0,
                scale,
                parked: false,
                ..ring
            };
        }
    };
    stand(&left, lo.x - FLANK_GAP - foot.x, true);
    stand(&right, hi.x + FLANK_GAP + foot.x, false);
    layout
}
