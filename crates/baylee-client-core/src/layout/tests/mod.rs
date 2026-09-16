mod extent;
mod facing;
mod fan;
mod packing;
mod piles;
mod ring;
mod seating;
mod sides;

use super::*;

fn seats(n: u8) -> Vec<PlayerId> {
    (0..n).map(PlayerId::new).collect()
}

/// At a crowded table the piles are the parts that come nearest the seat
/// next door, and they are the last thing added to a ring solve that was
/// written without them.
/// The four corners of a pile's card, turned to face its seat.
fn pile_corners(slot: &SeatSlot, pile: PileKind) -> [Vec2; 4] {
    let at = slot.pile_center(pile);
    let side = Vec2::new(slot.facing.cos(), -slot.facing.sin());
    let away = Vec2::new(slot.facing.sin(), slot.facing.cos());
    [(-1.0, -1.0), (1.0, -1.0), (1.0, 1.0), (-1.0, 1.0)]
        .map(|(x, y)| at + side * (x * CARD_WIDTH * 0.5) + away * (y * CARD_HEIGHT * 0.5))
}

/// Whether two convex quads share any area — the separating-axis test,
/// written out rather than approximated by circles round each box. A
/// circle bound is *sufficient* and would have failed at seat counts
/// where nothing actually touches, which makes it useless for telling a
/// real overlap from a near miss.
fn quads_overlap(a: [Vec2; 4], b: [Vec2; 4]) -> bool {
    for poly in [a, b] {
        for i in 0..4 {
            let edge = poly[(i + 1) % 4] - poly[i];
            let axis = Vec2::new(-edge.y, edge.x);
            let (pa, pb) = (a.map(|p| axis.dot(p)), b.map(|p| axis.dot(p)));
            let hi = |v: [f32; 4]| v.into_iter().fold(f32::NEG_INFINITY, f32::max);
            let lo = |v: [f32; 4]| v.into_iter().fold(f32::INFINITY, f32::min);
            if hi(pa) < lo(pb) - 1e-6 || hi(pb) < lo(pa) - 1e-6 {
                return false;
            }
        }
    }
    true
}

/// The shape of the space the duel HUD leaves on a laptop window — what
/// the layout is actually built against, and nothing like the window's.
const HUD_ASPECT: f32 = 2.01;

/// Whether two seats' grounds — their mats *and* the pile strips beside
/// them, which is what [`SeatSlot::footprint`] is — share any table at
/// all.
///
/// The separating-axis test over the two boxes' own four axes, which is
/// exact for rectangles: two convex shapes are disjoint exactly when some
/// axis separates their projections, and for boxes the only candidates
/// are their edge normals. Written out here because the layout has never
/// had a way to *ask* — every bound it applies is a bound on the distance
/// between two centres, and a centre distance says nothing on its own
/// about two rectangles turned to face different seats.
fn grounds_overlap(a: &SeatSlot, b: &SeatSlot) -> bool {
    let axes = |s: &SeatSlot| {
        let (sin, cos) = s.facing.sin_cos();
        [Vec2::new(cos, -sin), Vec2::new(sin, cos)]
    };
    let reach = |s: &SeatSlot, u: Vec2| {
        let [along, away] = axes(s);
        let f = s.footprint();
        f.x.mul_add(u.dot(along).abs(), f.y * u.dot(away).abs())
    };
    for u in axes(a).into_iter().chain(axes(b)) {
        if (b.center - a.center).dot(u).abs() > reach(a, u) + reach(b, u) + 1e-4 {
            return false;
        }
    }
    true
}

/// The tightest ring of a given shape that hands every seat the standard
/// board, and what the camera pays to frame it — found by walking
/// outwards rather than by bisecting.
///
/// Written out a second time on purpose: a test that asked
/// [`TableLayout::seated`]'s own search would agree with it however wrong
/// both were.
fn tightest(count: usize, aspect: f32, shape: impl Fn(f32) -> Vec2) -> Option<(Vec2, f32)> {
    let half_depth = POD_DEPTH * 0.5;
    let even = vec![1.0; count];
    let floor = half_depth + CENTRE_GAP * 0.5;
    for step in 0..=800_u16 {
        let ry = floor + (MAX_RING_Y - floor) * f32::from(step) / 800.0;
        let radius = shape(ry);
        if radius.x > MAX_RING_X + 1e-3 {
            break;
        }
        let sides = sides_on(count, radius);
        let held = compartment_half(&sides, &even, radius, half_depth);
        let half = pod_half_width(held, radius.x + half_depth);
        if half * 2.0 >= MIN_POD_WIDTH {
            return Some((radius, reach_of(&sides, half, half_depth, aspect)));
        }
    }
    None
}

/// What the camera has to swallow to frame a table that was built.
fn reach_of_layout(layout: &TableLayout, aspect: f32) -> f32 {
    let (lo, hi) = layout.extent().expect("a seated table has an extent");
    let span = hi - lo;
    (span.x / aspect).max(span.y)
}
