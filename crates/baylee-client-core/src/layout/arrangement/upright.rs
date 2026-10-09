//! The upright ring (DESIGN-v8 §1 row 2, §1.2; MULTIPLAYER §D): the ring's
//! places, every board turned to face me.

use super::super::{PILE_STRIP, Seat, SeatSlot, TableLayout};
use super::facing_for;
use glam::Vec2;

/// Whether two upright pods stand apart: their whole places (ground and
/// pile strips) separated by at least `gap` on one of the table's axes.
/// Upright, every footprint is a box square to the table, so the two axes
/// are the whole separating-axis test. [`shape`]'s search does the same
/// sums over [`Place`]s; the layout's tests ask this one.
#[cfg(test)]
pub(crate) fn apart(a: &SeatSlot, b: &SeatSlot, gap: f32) -> bool {
    let d = (b.footprint_center() - a.footprint_center()).abs();
    let reach = a.footprint() + b.footprint() + Vec2::splat(gap);
    d.x >= reach.x || d.y >= reach.y
}

/// How many steps each axis's scale is tried at, between [`LEAST`] and
/// [`MOST`], before the best is refined.
const STEPS: usize = 48;
/// The narrowest the ring may be drawn in, as a share of `seated`'s.
const LEAST: f32 = 0.35;
/// The widest.
const MOST: f32 = 3.0;

/// `seated`'s places, every board square to my chair — a teammate's as mine
/// (`facing = 0`), an opponent's as a duel opponent's (`facing = π`, the
/// owner's rule of 07.10.2026: [`facing_for`]) — and
/// the ring re-shaped — each axis scaled on its own, every seat staying on
/// its side of the middle — to the shape the camera can frame closest
/// while no two upright places meet (§1.2's rule: disjoint by
/// [`PILE_STRIP`]).
///
/// The design asked only for the ring to grow until the boxes part, and
/// measured that is the wrong half of the question: a flank pod stood
/// upright is a duel's board **across** the ring rather than along it, so
/// at four seats the ring's own width put the two flanks a board's width
/// further out than they need be and the camera stood back for it (my card
/// 23 px at 1708 against the ring's 31). The flanks may come in as far as
/// the seats above and below them allow, and that is what scaling the two
/// radii apart buys; at three seats, where the round ring's two opponents
/// meet once upright, the same search pushes them apart. A grid over the
/// two scales and a refinement round the best: a few thousand box tests,
/// once per rebuild, never per frame.
///
/// `angle` keeps the ring's bearing, which is where the dial's jewel for
/// the seat points.
///
/// Stood up from the table `seated` lays — a frame where one is taken — or
/// from the ellipse, whichever the camera frames closer upright: a frame's
/// flanks stand upright as boards across the table, and with a long side
/// beside them (one, two and three) that cost the whole table a fifth.
#[must_use]
pub fn upright(seats: &[Seat], aspect: f32) -> TableLayout {
    let ring = TableLayout::on_ring(seats, aspect, None);
    let laid = TableLayout::framed_or(ring.clone(), seats, aspect);
    if laid == ring {
        return upright_of(ring, seats, aspect);
    }
    let (framed, ellipse) = (
        upright_of(laid, seats, aspect),
        upright_of(ring, seats, aspect),
    );
    let price = |table: &TableLayout| {
        table.extent().map_or(f32::INFINITY, |(lo, hi)| {
            super::super::frame::price(hi - lo, aspect.clamp(0.45, 2.8), &table.slots)
        })
    };
    if price(&framed) <= price(&ellipse) {
        framed
    } else {
        ellipse
    }
}

/// [`upright`] over a ring already laid: the ellipse's, where a phone keeps
/// it (`TableLayout::arranged_in`).
#[must_use]
pub fn upright_of(mut layout: TableLayout, seats: &[Seat], aspect: f32) -> TableLayout {
    for slot in &mut layout.slots {
        slot.facing = facing_for(seats, slot.ring_index);
    }
    let scale = shape(&layout, aspect);
    for slot in &mut layout.slots {
        slot.center *= scale;
    }
    layout.radius *= scale;
    layout
}

/// The two scales [`upright`] applies to the ring: the pair whose table the
/// camera frames closest (the canvas-relative reach, as `seated`'s round
/// ring is judged), among those holding every place a pile strip apart.
#[must_use]
pub fn shape(layout: &TableLayout, aspect: f32) -> Vec2 {
    // What a candidate scale cannot change, read once: where each place's
    // footprint stands off its centre, its half size, and its half extent
    // once turned (`TableLayout::extent`'s). The search tries a few
    // thousand scales, and a fresh table for each, with every pair's sines
    // worked out again, was nearly all of its cost; the arithmetic per
    // candidate is `apart`'s and `extent`'s, operand for operand.
    let places: Vec<Place> = layout.slots.iter().map(Place::of).collect();
    let mut centres: Vec<Vec2> = vec![Vec2::ZERO; places.len()];
    let mut score = |scale: Vec2| -> Option<f32> {
        for (centre, place) in centres.iter_mut().zip(&places) {
            *centre = place.laid * scale + place.off;
        }
        let holds = places.iter().enumerate().all(|(i, a)| {
            places[i + 1..].iter().enumerate().all(|(k, b)| {
                let d = (centres[i + 1 + k] - centres[i]).abs();
                let reach = a.foot + b.foot + Vec2::splat(PILE_STRIP);
                d.x >= reach.x || d.y >= reach.y
            })
        });
        if !holds {
            return None;
        }
        let mut bounds: Option<(Vec2, Vec2)> = None;
        for (centre, place) in centres.iter().zip(&places) {
            if place.parked {
                continue;
            }
            let (lo, hi) = (*centre - place.half, *centre + place.half);
            bounds = Some(match bounds {
                None => (lo, hi),
                Some((min, max)) => (min.min(lo), max.max(hi)),
            });
        }
        Some(bounds.map_or(0.0, |(lo, hi)| {
            let span = hi - lo;
            (span.x / aspect).max(span.y)
        }))
    };
    #[allow(clippy::cast_precision_loss)] // forty-eight steps
    let at = |k: usize| LEAST + (MOST - LEAST) * k as f32 / STEPS as f32;
    let mut best: Option<(f32, Vec2)> = None;
    let mut consider = |scale: Vec2, best: &mut Option<(f32, Vec2)>| {
        if let Some(r) = score(scale) {
            // Ties to the larger ring: the same frame with more felt
            // between the seats.
            let better = best.is_none_or(|(b, s)| {
                r < b - 1e-4 || ((r - b).abs() <= 1e-4 && scale.length() > s.length())
            });
            if better {
                *best = Some((r, scale));
            }
        }
    };
    for i in 0..=STEPS {
        for j in 0..=STEPS {
            consider(Vec2::new(at(i), at(j)), &mut best);
        }
    }
    let Some((_, mut centre)) = best else {
        return Vec2::splat(MOST);
    };
    #[allow(clippy::cast_precision_loss)] // forty-eight steps
    let mut step = (MOST - LEAST) / STEPS as f32;
    for _ in 0..12 {
        step *= 0.5;
        for dx in [-1.0_f32, 0.0, 1.0] {
            for dy in [-1.0_f32, 0.0, 1.0] {
                let candidate = (centre + Vec2::new(dx, dy) * step)
                    .clamp(Vec2::splat(LEAST), Vec2::splat(MOST));
                consider(candidate, &mut best);
            }
        }
        if let Some((_, s)) = best {
            centre = s;
        }
    }
    centre
}

/// A place's footprint, as far as no scale moves it: what [`shape`]'s
/// search reads instead of the slot.
struct Place {
    /// The ring's centre, before scaling.
    laid: Vec2,
    /// `footprint_center() - center`.
    off: Vec2,
    /// `footprint()`.
    foot: Vec2,
    /// The footprint's half size turned by the facing (`extent`'s).
    half: Vec2,
    /// Not drawn on the felt: outside the extent.
    parked: bool,
}

impl Place {
    fn of(slot: &SeatSlot) -> Self {
        let foot = slot.footprint();
        let (sin, cos) = slot.facing.sin_cos();
        let (sin, cos) = (sin.abs(), cos.abs());
        Self {
            laid: slot.center,
            off: Vec2::new(slot.facing.cos(), -slot.facing.sin())
                * (slot.reclaimed * 0.5 * slot.scale),
            foot,
            half: Vec2::new(
                cos.mul_add(foot.x, sin * foot.y),
                sin.mul_add(foot.x, cos * foot.y),
            ),
            parked: slot.parked,
        }
    }
}
