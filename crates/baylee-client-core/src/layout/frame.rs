//! The ring packed as a **frame** (owner, 08.10.2026: *"use the table space
//! better … at 8 seats free-for-all there is a lot of wasted space between
//! the battlefields; it can be shrunk and the table zoomed nearer"*).
//!
//! An ellipse seats every side at an equal distance along a curve, and a
//! board is a straight thing: eight duel-wide boards on an ellipse leave the
//! four corners of the box round them bare and stand five to eight units
//! apart on its flat runs, and the camera frames the box. Measured at eight
//! seats on a laptop that table was 131.8 × 65.6 units for boards 31.3 long.
//!
//! A frame lays the sides on the four edges of a rectangle instead: mine on
//! the near edge, then clockwise in turn order up the left edge, along the
//! far edge and down the right one, back to mine. On each edge the boards
//! stand [`POD_GAP`] apart; where two edges meet, the boards of one stand
//! either beside the other edge's boards or between its two runs, whichever
//! the camera can frame closer. A row faces square across its edge; a
//! column may turn its far end outwards by up to an eighth of a turn
//! ([`TILTS`]), into the corner the far side of a perspective view has room
//! for, because a board standing square on a flank is drawn with its cards
//! foreshortened ([`FORESHORTEN`]). Every board faces the middle, so a seat's
//! cards face its owner, its shelf is on the hearth side and the camera's
//! visit stands behind it as before.
//!
//! What a frame keeps clear besides the boards: [`HEARTH_BAND`] on each
//! board's hearth side, where its plate and steps hang, and a circle of
//! [`DIAL_CLEAR`] round the middle for the dial. It is one candidate among
//! the shapes [`super::TableLayout::seated`] compares, and it is taken where
//! the smallest board at the table is drawn larger on it.
//!
//! All of it is arithmetic over at most eight boxes, a few hundred splits and
//! tilts: it runs when a table is laid out (a view, a seat, a resize), never
//! per frame.

use super::{PILE_STRIP, Seat, SeatSlot};
use glam::Vec2;

/// The felt between two neighbouring boards' whole places (their grounds
/// and pile strips), in table units: one card's width. Small enough that
/// the boards read as one table, and a card's width so a pile's card and
/// the next board's never read as touching.
pub const POD_GAP: f32 = 1.0;

/// The band along a board's hearth edge that its plate and its steps hang
/// in (`seatbar::attached`, flush with the mat's drawn edge), in table
/// units: no other board stands in it. The panels scale with the board's
/// shelf — the steps are 0.85 of the shelf's projected depth, and the shelf
/// is [`crate::tabletop::MAT_LEDGE`] deep — so a unit and a quarter holds a
/// three-line plate and the hairline under it at every zoom.
pub const HEARTH_BAND: f32 = 1.25;

/// The least free circle round the middle of a table of three or more seats,
/// in table units, measured to the nearest board as it is drawn — its whole
/// place plus the printed border [`crate::tabletop::MAT_MARGIN`] — and to
/// every [`HEARTH_BAND`]. The dial's rim at its smallest size and the air
/// round it (the dial lane's `MIN_FREE_RADIUS`, 1.40 + 0.35): a layout
/// packing the boards inwards stops here, and the dial may grow into
/// whatever the middle has beyond it.
pub const DIAL_CLEAR: f32 = 1.75;

/// How far a column's far end may turn outwards, in radians: square, a
/// twelfth of a turn, or an eighth, where a flank board lies across the
/// corner the way the ring's diagonals did. Nothing between square and a
/// twelfth: a board turned less than that is one whose camera side the
/// flanks' tolerance would decide ([`super::SIDE_SEAT_TILT`], four times
/// over, `nothing_at_any_table_sits_in_the_tolerance_the_flanks_need`).
const TILTS: [f32; 3] = [
    0.0,
    core::f32::consts::PI / 6.0,
    core::f32::consts::FRAC_PI_4,
];

/// How much shorter a unit of felt running up the screen is drawn than one
/// running across it, at the ring's default lean (0.62, D20): the cosine of
/// the camera's tilt, `1/√(1 + 0.62²)`. A board on a flank lays its cards'
/// width up the screen, so its cards are drawn this much narrower than the
/// same board's along a row at the same distance.
const FORESHORTEN: f32 = 0.85;

/// How much dearer a unit of a table's width is to the home shot than the
/// canvas's aspect says, and how much each unit of its depth adds to that.
///
/// The camera leans (0.62 by default, D20), so the near edge is nearer the
/// eye than the middle and the width is fitted there, where a unit of felt
/// is drawn widest; and the deeper the table, the nearer that edge comes.
/// Read off `camera::fit` at the default lean on a laptop's canvas (1708 ×
/// 1028, the hand zone off the bottom): a table `W` wide and `H` deep needs
/// an eye at `2.0·H` where its depth binds and at `1.15·W + 0.22·H` where its
/// width does, against a canvas aspect of 2.10 — `1.21·W/aspect + 0.11·H` in
/// units of the depth's own price. The client's camera tests hold the
/// layout's choice to the camera's
/// (`the_frame_is_taken_only_where_the_camera_draws_it_larger`).
const WIDTH_PRICE: f32 = 1.21;
/// See [`WIDTH_PRICE`].
const DEPTH_RISE: f32 = 0.11;

/// What the home shot pays to frame a table whose whole span is `span`, on a
/// canvas of `aspect`, in units of canvas height: the larger of what its
/// depth and its width ask ([`WIDTH_PRICE`]).
pub(super) fn cost(span: Vec2, aspect: f32) -> f32 {
    span.y
        .max(WIDTH_PRICE * span.x / aspect.max(1e-3) + DEPTH_RISE * span.y)
}

/// How wide a card on a board turned to `facing` is drawn, against the same
/// card on a row at the same distance ([`FORESHORTEN`]).
pub(super) fn drawn_width(facing: f32) -> f32 {
    let (sin, cos) = facing.sin_cos();
    (cos * cos + FORESHORTEN * FORESHORTEN * sin * sin).sqrt()
}

/// What a table costs the smallest board on it: the home shot's price over
/// how wide the least favoured board's cards are drawn — the number the
/// shapes are compared by, lower being better.
pub(super) fn price(span: Vec2, aspect: f32, slots: &[SeatSlot]) -> f32 {
    let narrowest = slots
        .iter()
        .filter(|s| !s.parked)
        .map(|s| drawn_width(s.facing))
        .fold(1.0, f32::min);
    cost(span, aspect) / narrowest.max(1e-3)
}

/// The four edges of the frame, in the order a clockwise walk from my seat
/// meets them (the near edge holds my side and its neighbours either way).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Edge {
    Near,
    Left,
    Far,
    Right,
}

impl Edge {
    /// The facing of a board on this edge, its far end turned `tilt`
    /// outwards: a row square across it, towards the middle.
    fn facing(self, tilt: f32) -> f32 {
        use core::f32::consts::{FRAC_PI_2, PI};
        match self {
            Self::Near => 0.0,
            Self::Left => FRAC_PI_2 - tilt,
            Self::Far => PI,
            Self::Right => PI + FRAC_PI_2 + tilt,
        }
    }

    /// Whether boards on this edge run along `x` (a row) rather than `y`.
    fn is_row(self) -> bool {
        matches!(self, Self::Near | Self::Far)
    }
}

/// One board on the frame, before the frame's size is known.
#[derive(Clone, Copy, Debug)]
struct Placed {
    seat: usize,
    side: usize,
    edge: Edge,
    facing: f32,
    /// Where its middle stands along its edge (`x` on a row, `y` on a
    /// column).
    along: f32,
    /// Its whole place's box in table axes, half of it.
    foot: Vec2,
    /// The box it keeps clear — its place and its [`HEARTH_BAND`] — in
    /// table axes, about its middle.
    keep_lo: Vec2,
    keep_hi: Vec2,
}

impl Placed {
    /// A board turned to `facing`, `half` its whole place's half extent in
    /// its own frame (along its lanes, across them).
    fn new(seat: usize, side: usize, edge: Edge, facing: f32, half: Vec2) -> Self {
        let (sin, cos) = facing.sin_cos();
        let (along, away) = (Vec2::new(cos, -sin), Vec2::new(sin, cos));
        let boxed = |h: Vec2| {
            Vec2::new(
                along.x.abs().mul_add(h.x, away.x.abs() * h.y),
                along.y.abs().mul_add(h.x, away.y.abs() * h.y),
            )
        };
        let keep = Vec2::new(half.x, half.y + HEARTH_BAND * 0.5);
        let shift = away * (HEARTH_BAND * 0.5);
        Self {
            seat,
            side,
            edge,
            facing,
            along: 0.0,
            foot: boxed(half),
            keep_lo: shift - boxed(keep),
            keep_hi: shift + boxed(keep),
        }
    }

    /// Its middle on a frame whose column edges stand `a` and row edges `b`
    /// out: a place's outer side on its edge.
    fn centre(&self, a: f32, b: f32) -> Vec2 {
        match self.edge {
            Edge::Near => Vec2::new(self.along, self.foot.y - b),
            Edge::Far => Vec2::new(self.along, b - self.foot.y),
            Edge::Left => Vec2::new(self.foot.x - a, self.along),
            Edge::Right => Vec2::new(a - self.foot.x, self.along),
        }
    }

    /// From its edge to the inner side of what it keeps clear.
    fn inward(&self) -> f32 {
        match self.edge {
            Edge::Near => self.foot.y + self.keep_hi.y,
            Edge::Far => self.foot.y - self.keep_lo.y,
            Edge::Left => self.foot.x + self.keep_hi.x,
            Edge::Right => self.foot.x - self.keep_lo.x,
        }
    }

    /// What it keeps clear along its edge, from its middle.
    fn keeps_along(&self) -> (f32, f32) {
        if self.edge.is_row() {
            (self.along + self.keep_lo.x, self.along + self.keep_hi.x)
        } else {
            (self.along + self.keep_lo.y, self.along + self.keep_hi.y)
        }
    }
}

/// A frame that seats the table: its slots and what the smallest board on
/// it pays ([`price`]).
pub(super) struct Framed {
    pub(super) slots: Vec<SeatSlot>,
    pub(super) radius: Vec2,
    pub(super) price: f32,
}

/// How many sides each edge holds, in the order the walk meets them: beside
/// mine on the near edge to its left, the left edge, the far edge, the right
/// edge, beside mine on the near edge to its right.
type Split = [usize; 5];

/// The best frame for these sides on a canvas of `aspect`, every seat handed
/// a board `half_width` either side of its middle (piles not included) and
/// `half_depth` deep. `None` for a table of one side.
pub(super) fn frame(
    seats: &[Seat],
    parties: &[Vec<usize>],
    half_width: f32,
    half_depth: f32,
    aspect: f32,
) -> Option<Framed> {
    let t = parties.len();
    if t < 2 {
        return None;
    }
    let others = t - 1;
    let mut best: Option<(f32, Framed)> = None;
    for left_of_me in 0..=others {
        for right_of_me in 0..=others - left_of_me {
            // My side stands in the middle of the near edge, or one side
            // off it: never further.
            if left_of_me.abs_diff(right_of_me) > 1 {
                continue;
            }
            let rest = others - left_of_me - right_of_me;
            for left in 0..=rest {
                for right in 0..=rest - left {
                    if left.abs_diff(right) > 1 {
                        continue;
                    }
                    let split = [left_of_me, left, rest - left - right, right, right_of_me];
                    // Two sides on one edge, facing the same way a board's
                    // gap apart, read as a team (the gable `ROUND_COST`
                    // was written against). Where the table has an edge
                    // for every side, each side takes its own.
                    if t <= 4
                        && (left_of_me + right_of_me > 0 || split[1..4].iter().any(|&k| k > 1))
                    {
                        continue;
                    }
                    // A frame lopsided about the middle is worth taking only
                    // when it pays: a twentieth for each pair of edges that
                    // is not its own mirror.
                    let lopsided =
                        usize::from(left_of_me != right_of_me) + usize::from(left != right);
                    #[allow(clippy::cast_precision_loss)] // two at most
                    let penalty = 1.0 + 0.05 * lopsided as f32;
                    // A column turns only where it is one board: two or
                    // more turned on one edge stand like the slats of a
                    // blind, the far ones facing out past the middle.
                    let left_side = left_of_me + 1;
                    let right_side = t - right_of_me - right;
                    let single =
                        |count: usize, first: usize| count == 1 && parties[first].len() == 1;
                    let turns = left + right > 0
                        && (left == 0 || single(left, left_side))
                        && (right == 0 || single(right, right_side));
                    let tilts: &[f32] = if turns { &TILTS } else { &TILTS[..1] };
                    for &tilt in tilts {
                        let Some(framed) = solve(
                            seats,
                            parties,
                            split,
                            tilt,
                            Vec2::new(half_width, half_depth),
                            aspect,
                        ) else {
                            continue;
                        };
                        let score = framed.price * penalty;
                        if best.as_ref().is_none_or(|(b, _)| score < *b - 1e-4) {
                            best = Some((score, framed));
                        }
                    }
                }
            }
        }
    }
    best.map(|(_, framed)| framed)
}

/// The sides on each edge for one split, in the order each edge is laid out
/// from its own low end (left on a row, bottom on a column).
fn runs(t: usize, split: Split) -> Option<[(Edge, Vec<usize>); 4]> {
    let [left_of_me, left, far, right, right_of_me] = split;
    // The walk: mine, then clockwise. Sides `1..=left_of_me` stand to my
    // left on the near edge, the next `left` up the left edge, `far` along
    // the far edge, `right` down the right edge, and the last `right_of_me`
    // to my right on the near edge, the one before mine beside me.
    let mut near: Vec<usize> = (1..=left_of_me).rev().collect();
    near.push(0);
    near.extend((t - right_of_me..t).rev());
    let mut cursor = left_of_me + 1;
    let mut take = |count: usize| {
        let run: Vec<usize> = (cursor..cursor + count).collect();
        cursor += count;
        run
    };
    let up_left = take(left);
    let along_far = take(far);
    // Walked top to bottom; laid out from the bottom.
    let mut down_right = take(right);
    down_right.reverse();
    (cursor + right_of_me == t).then_some([
        (Edge::Near, near),
        (Edge::Left, up_left),
        (Edge::Far, along_far),
        (Edge::Right, down_right),
    ])
}

/// A frame for one split of the sides over the edges, its columns turned
/// `tilt`, at the smallest size that keeps every board, band and the dial's
/// circle apart. `half` is a board's half extent, piles not included.
fn solve(
    seats: &[Seat],
    parties: &[Vec<usize>],
    split: Split,
    tilt: f32,
    half: Vec2,
    aspect: f32,
) -> Option<Framed> {
    let t = parties.len();
    // A whole place's half extent in its own frame.
    let whole = Vec2::new(half.x + PILE_STRIP, half.y);
    let mut placed: Vec<Placed> = Vec::with_capacity(seats.len());
    for (edge, run) in runs(t, split)? {
        // Each run centred on its edge, its sides in order from the edge's
        // low end, a side's allies shoulder to shoulder as the ring lays
        // them: the ring walks them along the lane axis, which on a far or
        // a left edge points down the edge.
        let first = placed.len();
        let mut at = 0.0_f32;
        for &side in &run {
            let members: Vec<usize> = if matches!(edge, Edge::Far | Edge::Left) {
                parties[side].iter().rev().copied().collect()
            } else {
                parties[side].clone()
            };
            for seat in members {
                let mut p = Placed::new(seat, side, edge, edge.facing(tilt), whole);
                let (lo, hi) = p.keeps_along();
                if placed.len() > first {
                    at += POD_GAP;
                }
                p.along = at - lo;
                at += hi - lo;
                placed.push(p);
            }
        }
        for p in &mut placed[first..] {
            p.along -= at * 0.5;
        }
    }
    let rows = placed.iter().any(|p| p.edge.is_row());
    let columns = placed.iter().any(|p| !p.edge.is_row());

    // The least `A` (a column's outer side from the middle) and `B` (a
    // row's) each board asks for on its own: what it keeps clear outside the
    // dial's circle, and apart from the board opposite on its own axis.
    let mut a0: f32 = 0.0;
    let mut b0: f32 = 0.0;
    for p in &placed {
        let (lo, hi) = p.keeps_along();
        let off = (lo.max(0.0) - hi.min(0.0)).max(0.0);
        let clear = (DIAL_CLEAR * DIAL_CLEAR - off * off).max(0.0).sqrt();
        let floor = p.inward() + clear.max(POD_GAP * 0.5);
        if p.edge.is_row() {
            b0 = b0.max(floor);
        } else {
            a0 = a0.max(floor);
        }
    }
    // A column board against a row board: apart across the table (`A` large
    // enough that the column stands beside the row) or along it (`B` large
    // enough that the row stands beyond the column's end). Each pair is one
    // `(A ≥ a) or (B ≥ b)`, read off the boxes each keeps clear.
    let mut pairs: Vec<(f32, f32)> = Vec::new();
    for c in placed.iter().filter(|p| !p.edge.is_row()) {
        let (c_lo, c_hi) = c.keeps_along();
        for r in placed.iter().filter(|p| p.edge.is_row()) {
            let (r_lo, r_hi) = r.keeps_along();
            let a = POD_GAP
                + c.inward()
                + match c.edge {
                    Edge::Left => -r_lo,
                    _ => r_hi,
                };
            let b = POD_GAP
                + r.inward()
                + match r.edge {
                    Edge::Near => -c_lo,
                    _ => c_hi,
                };
            pairs.push((a, b));
        }
    }

    // The frame's extent at a given `A` and `B`.
    let span = |a: f32, b: f32| -> Vec2 {
        let (mut lo, mut hi) = (Vec2::splat(f32::INFINITY), Vec2::splat(f32::NEG_INFINITY));
        for p in &placed {
            let centre = p.centre(a, b);
            lo = lo.min(centre - p.foot);
            hi = hi.max(centre + p.foot);
        }
        hi - lo
    };

    // `A` only matters at the values where a pair changes its mind; at each,
    // `B` is the least every pair not yet apart across the table asks.
    let mut tries: Vec<f32> = vec![a0];
    tries.extend(pairs.iter().map(|&(a, _)| a).filter(|&a| a > a0));
    let mut best: Option<(f32, f32, f32)> = None;
    for &a in &tries {
        let b = pairs
            .iter()
            .filter(|&&(need, _)| need > a + 1e-5)
            .map(|&(_, b)| b)
            .fold(b0, f32::max);
        let c = cost(span(a, b), aspect);
        if best.is_none_or(|(k, _, _)| c < k - 1e-4) {
            best = Some((c, a, b));
        }
    }
    let (_, mut a, mut b) = best?;
    // Every edge out to the frame's own extent: a column whose rows reach
    // further stands flush with their ends, a row whose columns reach
    // further at theirs. Nothing comes closer to anything, and the box the
    // camera frames is the same one.
    let extent = span(a, b);
    if columns {
        a = a.max(extent.x * 0.5);
    }
    if rows {
        b = b.max(extent.y * 0.5);
    }

    // The sides' bearings, for the dial and the ring order: the middle of
    // each side's run, measured from the near edge clockwise.
    let mut mid = vec![Vec2::ZERO; t];
    let mut count = vec![0.0_f32; t];
    for p in &placed {
        mid[p.side] += p.centre(a, b);
        count[p.side] += 1.0;
    }
    let mut slots: Vec<Option<SeatSlot>> = vec![None; seats.len()];
    for p in &placed {
        slots[p.seat] = Some(SeatSlot {
            player: seats[p.seat].player,
            ring_index: p.seat,
            angle: bearing(mid[p.side] / count[p.side].max(1.0)),
            center: p.centre(a, b),
            facing: p.facing,
            half_extent: half,
            reclaimed: 0.0,
            is_local: p.seat == 0,
            scale: 1.0,
            parked: false,
        });
    }
    let slots: Vec<SeatSlot> = slots.into_iter().flatten().collect();
    let price = price(extent, aspect, &slots);
    // The ring's radii, read as the frame's: where its columns' and its
    // rows' middles stand.
    let radius = Vec2::new(a - half.y, b - half.y);
    Some(Framed {
        slots,
        radius,
        price,
    })
}

/// The ring's angle for a point: from the near edge, clockwise (to the
/// left first), as `sides_on` measures it.
fn bearing(p: Vec2) -> f32 {
    (-p.x).atan2(-p.y).rem_euclid(core::f32::consts::TAU)
}

/// Whether a table laid as `slots` keeps what a frame keeps by
/// construction: every two boards [`POD_GAP`] apart, no board on another's
/// [`HEARTH_BAND`], and nothing drawn — a board with its printed border, a
/// band — within [`DIAL_CLEAR`] of the middle. The ellipse keeps it at the
/// canvases it was shaped for and not everywhere (three seats on a window
/// held upright stand a third of a unit apart): one that does not is no
/// candidate where a frame is.
pub(super) fn holds(slots: &[SeatSlot]) -> bool {
    let frame_of = |s: &SeatSlot| {
        let (sin, cos) = s.facing.sin_cos();
        (Vec2::new(cos, -sin), Vec2::new(sin, cos))
    };
    // A box about `centre`, `half` along and across the seat's own axes.
    let quad = |s: &SeatSlot, centre: Vec2, half: Vec2| -> [Vec2; 4] {
        let (along, away) = frame_of(s);
        [(-1.0, -1.0), (1.0, -1.0), (1.0, 1.0), (-1.0, 1.0)]
            .map(|(x, y)| centre + along * (x * half.x) + away * (y * half.y))
    };
    let place = |s: &SeatSlot| quad(s, s.footprint_center(), s.footprint());
    let band = |s: &SeatSlot| {
        let (_, away) = frame_of(s);
        let foot = s.footprint();
        let edge = s.footprint_center() + away * (foot.y + HEARTH_BAND * 0.5);
        quad(s, edge, Vec2::new(foot.x, HEARTH_BAND * 0.5))
    };
    let drawn = |s: &SeatSlot| {
        let margin = crate::tabletop::MAT_MARGIN * s.scale;
        quad(s, s.footprint_center(), s.footprint() + Vec2::splat(margin))
    };
    let shown: Vec<&SeatSlot> = slots.iter().filter(|s| !s.parked).collect();
    for (i, a) in shown.iter().enumerate() {
        if from_middle(&drawn(a)) < DIAL_CLEAR - 1e-3 || from_middle(&band(a)) < DIAL_CLEAR - 1e-3 {
            return false;
        }
        for b in &shown[i + 1..] {
            if apart(&place(a), &place(b)) < POD_GAP - 1e-3
                || apart(&band(a), &place(b)) <= 0.0
                || apart(&band(b), &place(a)) <= 0.0
            {
                return false;
            }
        }
    }
    true
}

/// How far apart two convex quads stand on the axis that separates them
/// best (their edges' normals): negative where they overlap. Never more
/// than their true distance.
fn apart(a: &[Vec2; 4], b: &[Vec2; 4]) -> f32 {
    let mut best = f32::NEG_INFINITY;
    for poly in [a, b] {
        for i in 0..4 {
            let axis = (poly[(i + 1) % 4] - poly[i]).perp().normalize_or_zero();
            let span = |q: &[Vec2; 4]| {
                q.iter()
                    .fold((f32::INFINITY, f32::NEG_INFINITY), |(lo, hi), p| {
                        (lo.min(p.dot(axis)), hi.max(p.dot(axis)))
                    })
            };
            let ((a0, a1), (b0, b1)) = (span(a), span(b));
            best = best.max(b0 - a1).max(a0 - b1);
        }
    }
    best
}

/// How far a convex quad stands from the middle of the table: zero where
/// it covers the middle.
fn from_middle(quad: &[Vec2; 4]) -> f32 {
    let side = |i: usize| (quad[(i + 1) % 4] - quad[i]).perp_dot(-quad[i]);
    if (0..4).all(|i| side(i) >= 0.0) || (0..4).all(|i| side(i) <= 0.0) {
        return 0.0;
    }
    (0..4)
        .map(|i| {
            let (p, q) = (quad[i], quad[(i + 1) % 4]);
            let t = (-p).dot(q - p) / (q - p).length_squared().max(1e-9);
            (p + (q - p) * t.clamp(0.0, 1.0)).length()
        })
        .fold(f32::INFINITY, f32::min)
}
