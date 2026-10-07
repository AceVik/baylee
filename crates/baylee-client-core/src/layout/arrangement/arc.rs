//! The arc rail (DESIGN-v8 §1 row 4): me near, and every other seat on one
//! arc across the top of the table, each board turned to face the arc's
//! middle; the camera is a rail along it, one stop per seat, three boards
//! in view at a time. Nothing moves when a seat is visited: the rail
//! slides to the stop that centres it.

use super::super::{PILE_STRIP, Seat, SeatSlot, TableLayout, sides_of};
use super::pair::{across_side, pair};
use glam::Vec2;

/// How many boards the rail's window holds abreast.
pub const IN_VIEW: f32 = 3.0;

/// The air between two boards on the arc, and between the arc and my board.
pub const GUTTER: f32 = PILE_STRIP + 1.0;

/// How far round the arc's circle the window of [`IN_VIEW`] boards reaches
/// each side of its middle, radians: a gentle curve, the outer boards of a
/// window turned a little toward the middle.
const WINDOW_TURN: f32 = 0.30;

/// The arc rail at a table of `seats` on a canvas of `aspect`.
///
/// The boards are cut at the widest aspect whose window of three boards is
/// no wider than the canvas (bisected, as the Turntable narrows its pair):
/// a duel's board stretches to its canvas's width, and the rail shows three.
/// The arc's circle is sized so neighbours stand a board and a gutter
/// apart along it; its middle board stands where a duel's across board
/// would, the rest curve down and away to either side in turn order (the
/// seat after mine at the far left, as the ring has it), off the window
/// from the fourth on.
#[must_use]
pub fn arc(seats: &[Seat], aspect: f32) -> TableLayout {
    let shape = |layout: &TableLayout| {
        window(layout, 0.0, IN_VIEW).map_or(0.0, |(lo, hi)| (hi.x - lo.x) / (hi.y - lo.y).max(1e-3))
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

/// The arc with every board cut at `aspect`.
fn cut_at(seats: &[Seat], aspect: f32) -> TableLayout {
    // My side and the board across as a duel's: mine (and a partner's
    // beside it) stay where the duel has them, the across board is the
    // template every board on the arc is cut from.
    let ring = pair(seats, aspect, &across_side(seats, None));
    let mine = ring.slots[0];
    let others: Vec<usize> = sides_of(seats).into_iter().skip(1).flatten().collect();
    let Some(template) = others.iter().map(|&i| ring.slots[i]).find(|s| !s.parked) else {
        return ring;
    };
    let foot = template.footprint();
    let cell = foot.x * 2.0 + GUTTER;
    // The middle board's centre, above the dial at a duel's distance from
    // mine.
    let top = -mine.center.y;
    #[allow(clippy::cast_precision_loss)] // a handful of seats
    let middle = (others.len() as f32 - 1.0) * 0.5;
    // The circle on which one cell is the chord of a step: three cells span
    // the window's turn, or less where a long arc's ends would curve down
    // past my board (the drop at the ends, `cell · middle² · step / 2` for a
    // small turn, kept above my board's far edge).
    let room = (top - mine.center.y - foot.y * 2.0 - GUTTER).max(0.0);
    let flattest = 2.0 * room / (cell * middle.max(1.0).powi(2));
    let step = (WINDOW_TURN / (IN_VIEW * 0.5)).min(flattest).max(0.01);
    let radius = cell / (2.0 * (step * 0.5).sin());
    let centre = Vec2::new(0.0, top - radius);
    let mut slots = ring.slots.clone();
    for (k, &i) in others.iter().enumerate() {
        // Turn order runs left to right along the arc: a positive turn
        // (counter-clockwise) is to the left.
        #[allow(clippy::cast_precision_loss)]
        let turn = (middle - k as f32) * step;
        let at = centre + Vec2::from_angle(turn).rotate(Vec2::new(0.0, radius));
        slots[i] = SeatSlot {
            center: at,
            // Facing the circle's middle: the across board's π, turned with
            // the arc.
            facing: (core::f32::consts::PI - turn).rem_euclid(core::f32::consts::TAU),
            half_extent: template.half_extent,
            reclaimed: template.reclaimed,
            parked: false,
            ..ring.slots[i]
        };
    }
    TableLayout {
        slots,
        radius: ring.radius,
    }
}

/// The rail's window around stop `x`, `boards` boards wide: the band of the
/// table the camera frames, as `(lo, hi)` in table space — every board on
/// the felt wholly inside it, and mine at home. Home is [`IN_VIEW`] boards round
/// the arc's middle; a visit is the one board it centres (measured: three
/// boards abreast drew a card 22 px at 1708).
#[must_use]
pub fn window(layout: &TableLayout, x: f32, boards: f32) -> Option<(Vec2, Vec2)> {
    let mine = layout.local()?;
    let half = (mine.footprint().x * 2.0 + GUTTER) * boards * 0.5;
    let mut lo = Vec2::new(x - half, f32::MAX);
    let mut hi = Vec2::new(x + half, f32::MIN);
    for slot in layout.on_felt() {
        // The boards wholly inside the band, and mine at home (a visit is
        // the one board: mine is home's, one key away).
        let home = boards >= IN_VIEW;
        let inside = (slot.center.x - x).abs() + slot.footprint().x <= half + 1e-3;
        if if slot.is_local { home } else { inside } {
            // The board's box turned with it: how far it reaches up and down.
            let foot = slot.footprint();
            let reach = slot.facing.sin().abs() * foot.x + slot.facing.cos().abs() * foot.y;
            lo.y = lo.y.min(slot.center.y - reach);
            hi.y = hi.y.max(slot.center.y + reach);
        }
    }
    Some((lo, hi))
}
