use super::*;
use baylee_core::ids::PlayerId;

/// A laptop's window, in logical pixels.
const WINDOW: Vec2 = Vec2::new(1728.0, 1052.0);

fn seats(n: u8) -> Vec<PlayerId> {
    (0..n).map(PlayerId::new).collect()
}

/// Where a point on the felt lands, in normalised device coordinates.
///
/// Written out forwards on purpose: [`CameraRig::home`] inverts the same
/// projection, and a test that reused the inverse would agree with it
/// however wrong both were.
fn project(rig: CameraRig, canvas: Canvas, table: Vec2) -> Vec2 {
    let lean = CAMERA_LEAN;
    let eye = rig.distance * (1.0 + lean * lean).sqrt();
    let cos = 1.0 / (1.0 + lean * lean).sqrt();
    let t = (FOV * 0.5).tan();
    let aspect = canvas.window.x / canvas.window.y;
    // The rig stores world x/z; `+y` away from the local seat is `-z`.
    let s = table.y - -rig.target.y;
    let depth = lean.mul_add(cos * s, eye);
    Vec2::new(
        (table.x - rig.target.x) / (depth * t * aspect),
        cos * s / (depth * t),
    )
}

/// Every seat's bar fits the shelf it is written on, at every table.
///
/// Two claims, and the second is the one that is easy to lose. The
/// density chosen for a shelf must not overhang it by more than that
/// density is allowed — which is what makes the choice a choice rather
/// than a label. And the shelf has to project **deeper** than the bar is
/// tall, or the ink is standing on the creature lane behind it rather
/// than on the ledge; a few pixels of shelf above and below is what makes
/// the bar sit on the felt instead of floating over it.
///
/// The local seat is in the loop and is not the easy case: its ledge is
/// the *far* edge of its own mat, so it is the more foreshortened end of
/// the nearest board.
///
/// Measured through [`Shelf`](crate::hud::Shelf), which is what the
/// renderer places from, and therefore along each shelf's **own** axis. A
/// side seat's ledge runs up and down the screen; its bounding box is
/// sixty pixels wide and the ledge is four hundred long, so a test
/// measuring the box would report a pip strip on a shelf with room for
/// every label.
#[test]
fn the_bar_fits_its_ledge_at_every_seat_of_an_eight_ring() {
    use crate::hud::Shelf;
    let canvas = Canvas::hud(WINDOW);
    for n in 2..=8 {
        let layout = TableLayout::new(&seats(n), canvas.aspect(), None);
        let rig = CameraRig::home(&layout, canvas);
        let lens = Lens::new(rig, canvas.window);
        for slot in &layout.slots {
            let corners = lens
                .corners(slot.ledge_corners())
                .expect("every ledge is in front of the camera");
            let shelf = Shelf::of(corners, false);
            let density = shelf.density;
            let over = density.width(false) - shelf.along;
            assert!(
                over <= density.width(false) * density.overhang() + 1e-3,
                "at {n} seats, seat {} takes the {density:?} bar ({} wide) \
                 on a {} shelf — {over} of overhang",
                slot.ring_index,
                density.width(false),
                shelf.along
            );
            assert!(
                shelf.depth >= density.ink_height(),
                "at {n} seats, seat {}'s shelf projects {} deep and the \
                 {density:?} bar draws {} of ink — it would stand on the \
                 creature lane",
                slot.ring_index,
                shelf.depth,
                density.ink_height()
            );
            // And no bar is drawn upside-down, whatever its mat is doing:
            // the ink reads in the viewer's order and the ground belongs
            // to the seat.
            assert!(
                shelf.tilt.abs() <= std::f32::consts::FRAC_PI_2 + 1e-3,
                "at {n} seats, seat {}'s bar is turned {} radians",
                slot.ring_index,
                shelf.tilt
            );
        }
    }
}

/// A duel gets the bar its window can hold.
///
/// The claim `baylee-client-core` cannot make on its own: which density
/// a *real* table's shelf projects to. It used to be made there anyway,
/// against a shelf modelled as `window.x * 0.635`, and a constant cannot
/// be wrong about the projection it stands in for — so it passed while
/// promising the compact bar at a width that actually gets the full one.
/// Measured here through the same `Lens` the renderer places from.
#[test]
fn a_duel_gets_the_bar_its_window_can_hold() {
    use crate::hud::Shelf;
    for (width, wanted) in DUEL_BARS {
        let window = Vec2::new(width, width * WINDOW.y / WINDOW.x);
        let canvas = Canvas::hud(window);
        let layout = TableLayout::new(&seats(2), canvas.aspect(), None);
        let rig = CameraRig::home(&layout, canvas);
        let lens = Lens::new(rig, canvas.window);
        let local = &layout.slots[0];
        let corners = lens
            .corners(local.ledge_corners())
            .expect("the local ledge is in front of the camera");
        let shelf = Shelf::of(corners, false);
        assert_eq!(
            shelf.density, wanted,
            "a {width}-wide duel projects a {:.0} px shelf, which is the \
             {:?} bar and not the {wanted:?} one",
            shelf.along, shelf.density
        );
    }
}

/// A duel is written on two rows, at **both** seats.
///
/// The reason [`crate::tabletop::MAT_LEDGE`] is as deep as it is, and
/// therefore the test that says what the depth is for. `Density::Split`
/// is chosen per seat, like every other form, so a ledge deep enough for
/// only one of a duel's two shelves would draw the near seat a phase line
/// and the far seat a single crowded row — one table, two designs, and no
/// test would have noticed.
///
/// Two seats and not more, and what stops it is the shelf's **length**
/// rather than its depth. Measured at [`WINDOW`]: a duel's shelves
/// project 1127×61 and 1069×55, and three seats project 372×46, 337×44,
/// 337×44 — deep enough for the 34 px of ink two rows draw, and nowhere
/// near the 585 px the two-row bar is wide. It used to be the depth that
/// ran out first; that was the shelf being measured a printed border
/// short of the one the mat draws, and the number that moved when they
/// were reconciled was the depth.
/// [`the_bar_fits_its_ledge_at_every_seat_of_an_eight_ring`] is what
/// holds the ladder that takes over there.
#[test]
fn a_duel_is_written_on_two_rows() {
    use crate::hud::Shelf;
    use baylee_client_core::seatbar::Density;
    let canvas = Canvas::hud(WINDOW);
    let layout = TableLayout::new(&seats(2), canvas.aspect(), None);
    let rig = CameraRig::home(&layout, canvas);
    let lens = Lens::new(rig, canvas.window);
    let shelves: Vec<Shelf> = layout
        .slots
        .iter()
        .map(|slot| {
            let corners = lens
                .corners(slot.ledge_corners())
                .expect("both ledges of a duel are in front of the camera");
            Shelf::of(corners, false)
        })
        .collect();
    // Reported together rather than one at a time, because the number
    // that decides the ledge is the *shallower* of the two and a test
    // that stopped at the near seat would never print it.
    let measured: Vec<String> = shelves
        .iter()
        .map(|s| format!("{:.1}×{:.1} ({:?})", s.along, s.depth, s.density))
        .collect();
    assert!(
        shelves.iter().all(|s| s.density == Density::Split),
        "a duel's shelves project {} and the two-row bar asks for \
         {:.1}×{:.1}",
        measured.join(", "),
        Density::Split.min_length(false),
        Density::Split.ink_height()
    );
}

/// What [`a_duel_gets_the_bar_its_window_can_hold`] promises, in one
/// place, because these are the numbers a reader wants and not the loop
/// around them.
///
/// Measured, at a window kept the shape of [`WINDOW`]: the local ledge
/// projects 548, 688, 768 and 1247 pixels long and 21.1, 30.7, 36.2 and
/// 69.4 deep. The length runs 0.69, 0.67, 0.67 and 0.65 of the window's
/// width — a band and not a constant, and it narrows as the window grows
/// because `Canvas::hud` takes a *fixed* hand zone off the bottom, so a
/// small window is a squarer canvas.
///
/// Every hand-over above the smallest window is a **depth** one, and that
/// is the shape of the whole ladder now: the two-row bar wants 585 px of
/// length, which a 1024-wide window already gives it, so from there up
/// what decides the form is whether the shelf is deep enough to write two
/// rows on. The 800-wide window is the one that fails on both counts,
/// 548 long and 21.1 deep.
///
/// Measured across the band, near seat then far, when the identity row
/// grew from 14 px to 18 and took the split bar's ink from 34 to 40:
///
/// | window | near      | far       |
/// |--------|-----------|-----------|
/// | 1024   | 30.7 deep | 28.1 deep |
/// | 1152   | 36.2      | 33.0      |
/// | 1280   | 41.8      | 37.9      |
/// | 1366   | 45.5      | 41.2      |
/// | 1728   | 61.1      | 55.0      |
///
/// So a window of this shape gains the phase line at about **1280** on
/// the near seat and **1366** on both, where it used to be 1150 and 1250.
/// A row eleven pixels taller costs four hundred pixels of window, which
/// is the price of the identity row being readable at the size it is
/// written in rather than shrunk to fit under the tiles.
///
/// This list used to start at the compact bar at 1280 and reach the
/// two-row one at 1728, and every number in it moved when the shelf
/// stopped being measured a border short of the one that is drawn: the
/// same 1728 window that projected a 40.3 px shelf projects 61.1. A duel
/// is now written on two rows on any laptop, which is what the shelf
/// could always hold and not a change of mind about what it should.
///
/// The far seat's shelf stays about a tenth shallower than the near one
/// (55.0 against 61.1 at 1728), so there is still a band — now around
/// 1280 to 1366 — where a duel draws its local bar on two rows and its
/// opponent's on one. That is the same per-seat answer the ladder gives a
/// four-seat table, and the list deliberately does not try to pin its
/// edges: they move with every constant here.
const DUEL_BARS: [(f32, baylee_client_core::seatbar::Density); 5] = [
    (800.0, baylee_client_core::seatbar::Density::Pip),
    (1024.0, baylee_client_core::seatbar::Density::Compact),
    (1152.0, baylee_client_core::seatbar::Density::Compact),
    (1366.0, baylee_client_core::seatbar::Density::Split),
    (1920.0, baylee_client_core::seatbar::Density::Split),
];

/// A free-for-all of three is on the circle it is supposed to be on.
///
/// `layout::ROUND_COST` has always said a table with no allies is
/// *offered* a circle and takes it when the camera can afford it, and at
/// three seats a circle is the whole point: an ellipse shaped to a wide
/// canvas puts the two opponents at 150° and 210°, side by side across
/// the top, which is the silhouette a 2v1 draws.
///
/// It was not afforded until the rail and the tab strip came off the top
/// of the window. Measured at 1728×1052 on either canvas: with `top` at
/// 110 the three-seat ring settled at 12.95 × 4.99 — an ellipse — and
/// with `top` at nothing it settles at 8.28 × 8.28. That is the one seat
/// count where the taller canvas made a board *smaller*, 34.9 → 32.0
/// pixels a table unit, and it is the circle being bought rather than
/// anything going wrong: 9.3% of reach, against the 30% `ROUND_COST`
/// allows.
///
/// Written as a test because it is a *silhouette*, and the arithmetic
/// that produces it turns on a filter that a slightly different window
/// can flip. Nothing else at the table notices when it does.
#[test]
fn three_seats_playing_for_themselves_sit_on_a_circle() {
    let canvas = Canvas::hud(WINDOW);
    let layout = TableLayout::new(&seats(3), canvas.aspect(), None);
    assert!(
        (layout.radius.x - layout.radius.y).abs() < 1e-3,
        "a three-seat free-for-all is on a {:?} ring, not a circle",
        layout.radius
    );
    // And the two opponents are a third of the way round from the local
    // seat and from each other, which is what a circle is for here.
    for slot in &layout.slots {
        let want = std::f32::consts::TAU * slot.ring_index as f32 / 3.0;
        let off = (slot.angle - want).abs();
        assert!(
            off < 0.02,
            "seat {} sits at {} radians, not {want}",
            slot.ring_index,
            slot.angle
        );
    }
}

/// [`Lens`] and the projection written out above agree.
///
/// The one is a matrix built from the rig's own eye transform and the
/// other is the closed form this file has always tested with, derived by
/// hand from the lean and the lens. They are two independent derivations
/// of one camera, which is the only reason either is evidence about the
/// other — and it is what makes `Lens` safe to place the seat bars with,
/// since a bar pinned to a shelf by a projection nobody has checked is a
/// bar that lands wherever the arithmetic happens to put it.
///
/// At yaw zero, because the closed form assumes it: it reads `table.y`
/// straight down the view axis. The general case is `Lens`'s alone, which
/// is the whole reason it exists — a player may orbit the table.
#[test]
fn the_lens_and_the_written_out_projection_agree() {
    let canvas = Canvas::hud(WINDOW);
    let layout = TableLayout::new(&seats(4), canvas.aspect(), None);
    let rig = CameraRig::home(&layout, canvas);
    let lens = Lens::new(rig, canvas.window);
    for slot in &layout.slots {
        for corner in slot.ledge_corners() {
            let theirs = project(rig, canvas, corner) * canvas.window * 0.5;
            // The closed form answers in pixels from the middle of the
            // window with `+y` up; the lens answers from the top-left
            // corner with `+y` down, which is where a `Node` lives.
            let theirs = Vec2::new(
                theirs.x + canvas.window.x * 0.5,
                canvas.window.y.mul_add(0.5, -theirs.y),
            );
            let ours = lens.project(corner).expect("the table is in front");
            assert!(
                ours.distance(theirs) < 0.5,
                "the lens puts {corner:?} at {ours:?} and the written-out \
                 projection at {theirs:?}"
            );
        }
    }
}

/// Every corner of every seat's mat, in table space.
fn corners(layout: &TableLayout) -> Vec<Vec2> {
    box_corners(layout, |slot| slot.half_extent)
}

/// The corners of every seat's whole *place* — the ground and the piles
/// standing beside it, which is the box the camera actually frames.
fn places(layout: &TableLayout) -> Vec<Vec2> {
    box_corners(layout, SeatSlot::footprint)
}

fn box_corners(layout: &TableLayout, half: fn(&SeatSlot) -> Vec2) -> Vec<Vec2> {
    let mut out = Vec::new();
    for slot in &layout.slots {
        let (sin, cos) = slot.facing.sin_cos();
        for sx in [-1.0_f32, 1.0] {
            for sy in [-1.0_f32, 1.0] {
                let local = half(slot) * Vec2::new(sx, sy);
                out.push(
                    slot.center
                        + Vec2::new(
                            cos.mul_add(local.x, sin * local.y),
                            (-sin).mul_add(local.x, cos * local.y),
                        ),
                );
            }
        }
    }
    out
}

/// How wide one seat's board is drawn, in pixels.
fn drawn_width(rig: CameraRig, canvas: Canvas, slot: &SeatSlot) -> f32 {
    let (sin, cos) = slot.facing.sin_cos();
    let axis = Vec2::new(cos, -sin) * slot.half_extent.x;
    let ends = [slot.center + axis, slot.center - axis]
        .map(|end| project(rig, canvas, end) * canvas.window * 0.5);
    ends[0].distance(ends[1])
}

/// Every seat's board is laid out the same width, and is *drawn* nearly
/// the same width too.
///
/// The layout half of that has its own test in `layout.rs`; this is the
/// camera half, and the two are different claims. A lean spends the
/// furthest seat's size on the nearest one, and the nearest one is always
/// the player's own — so the seat whose board the player compares every
/// other against was the one drawn wrong. At the lean and the lens this
/// shipped with it was 18.9% wider than its opponents' at a three-player
/// free-for-all, which is a difference a player reads as a different
/// format rather than as a camera.
///
/// Bounded rather than equalised: the remaining few per cent is the
/// foreshortening of a board turned away from the camera, and squeezing
/// that out means a lean of zero, which is a table of decals.
///
/// The bound was 1.08, then 1.12, and is 1.13 — a promise being given
/// back in pieces, so each piece says what bought it. [`CAMERA_LEAN`]
/// went 0.27 → 0.36 because the owner asked a third time for more angle
/// after being told what it trades against, and the widest board on an
/// eight-seat ring went 6.3% → 10.6% with it. Then the tab strip and the
/// phase rail came off the top of the window, [`Canvas::hud`]'s `top`
/// dropped from 110 to nothing, and the same ring went 10.3% → 12.1%:
/// a canvas that is taller is also *squarer*, the ring is laid out
/// rounder against it, and a rounder ring turns its side seats further
/// away from the camera. The bound is that measurement plus a hair and
/// not a round number chosen to be safe: it still fails the shot this
/// test was written for, which drew one board 18.9% wider than its
/// neighbours.
///
/// The phone keeps 1.18 and did not move — it was already 16.5% at three
/// seats for the reason below, and the taller canvas took it to 17.5%.
#[test]
fn every_seat_is_drawn_a_board_of_the_same_width() {
    // A phone is allowed a little more. Its ring is nearly a column —
    // 2.5 × 11.2 at three seats — so the near seat stands a far larger
    // fraction of the eye distance closer than it does on a ring that had
    // room to be round, and no lens shortens that.
    for (window, bound) in [
        (WINDOW, 1.13),
        (Vec2::new(1280.0, 800.0), 1.13),
        (Vec2::new(430.0, 932.0), 1.18),
    ] {
        let canvas = Canvas::hud(window);
        for n in 2..=8u8 {
            let layout = TableLayout::new(&seats(n), canvas.aspect(), None);
            let rig = CameraRig::home(&layout, canvas);
            let drawn: Vec<f32> = layout
                .slots
                .iter()
                .map(|slot| drawn_width(rig, canvas, slot))
                .collect();
            let widest = drawn.iter().copied().fold(0.0_f32, f32::max);
            let narrowest = drawn.iter().copied().fold(f32::INFINITY, f32::min);
            assert!(
                widest <= narrowest * bound,
                "{n} seats in {window}: boards laid out {:.2} wide are drawn \
                 {drawn:?} — {:.1}% apart, and the widest is seat {}",
                layout.slots[0].lane_width(),
                (widest / narrowest - 1.0) * 100.0,
                drawn
                    .iter()
                    .position(|w| (w - widest).abs() < 1e-3)
                    .unwrap_or_default()
            );
        }
    }
}

/// A seat's mat is wider than the box that seat reports.
///
/// The layout answers where *cards* go, so `half_extent` stops at the
/// cards; the mat under them is drawn `ZONE_MARGIN` wider on every side,
/// and that printed border is what makes it a playmat rather than a
/// rectangle ruled tight around the lanes. Nothing in the layout knows
/// it exists, so the camera has to, and [`AIR`] is where it is known.
/// This is the assertion that keeps the two in step: shrink `AIR` back
/// under `ZONE_MARGIN` and the near seat's border goes under the hand
/// bar, which is the one edge a player is looking at.
#[test]
fn a_seats_printed_border_is_inside_the_band_too() {
    for window in [WINDOW, Vec2::new(1280.0, 800.0), Vec2::new(430.0, 932.0)] {
        let canvas = Canvas::hud(window);
        let top = 1.0 - 2.0 * canvas.top / canvas.window.y;
        let bottom = -1.0 + 2.0 * canvas.bottom / canvas.window.y;
        let right = 1.0 - 2.0 * canvas.right / canvas.window.x;
        for n in 2..=8u8 {
            let layout = TableLayout::new(&seats(n), canvas.aspect(), None);
            let rig = CameraRig::home(&layout, canvas);
            for corner in box_corners(&layout, |slot| slot.half_extent + Vec2::splat(ZONE_MARGIN)) {
                let at = project(rig, canvas, corner);
                assert!(
                    at.y >= bottom - 1e-3 && at.y <= top + 1e-3,
                    "{n} seats in {window}: a mat's border lands at y {}, outside \
                     {bottom}..{top}",
                    at.y
                );
                assert!(
                    at.x >= -1.0 - 1e-3 && at.x <= right + 1e-3,
                    "{n} seats in {window}: a mat's border lands at x {}, outside -1..{right}",
                    at.x
                );
            }
        }
    }
}

/// The shot is as close as the free band allows — around the felt.
///
/// What the camera frames is the table plus [`AIR`], and *that* is what
/// has to fill the band: a fit with room to spare on both axes is a fit
/// that could have come in, and every card at the table is drawn smaller
/// for it. This used to be the case at every seat count, because the fit
/// measured the corners of the box around the table, and on a ring those
/// corners are bare felt — at three seats it filled 86% of the width it
/// was given and 81% of the height, binding on neither.
///
/// Bounded from below as well, on the bare table this time, because the
/// two failures look nothing alike and only one of them is arithmetic: a
/// camera that could have come in wastes the screen, and a camera pushed
/// out until the table is a coaster in the middle of it has answered a
/// question nobody asked.
#[test]
fn the_shot_is_as_close_as_the_band_allows() {
    let canvas = Canvas::hud(WINDOW);
    let top = 1.0 - 2.0 * canvas.top / canvas.window.y;
    let bottom = -1.0 + 2.0 * canvas.bottom / canvas.window.y;
    let right = 1.0 - 2.0 * canvas.right / canvas.window.x;
    let fill = |rig: CameraRig, points: &[Vec2]| {
        let (mut lo, mut hi) = (Vec2::splat(f32::INFINITY), Vec2::splat(f32::NEG_INFINITY));
        for &corner in points {
            let at = project(rig, canvas, corner);
            lo = lo.min(at);
            hi = hi.max(at);
        }
        Vec2::new(
            (hi.x - lo.x) / (right + 1.0),
            (hi.y - lo.y) / (top - bottom),
        )
    };
    for n in 2..=8u8 {
        let layout = TableLayout::new(&seats(n), 2.01, None);
        let rig = CameraRig::home(&layout, canvas);
        let framed = fill(rig, &layout.corners(AIR));
        assert!(
            framed.x.max(framed.y) > 0.93,
            "{n} seats fills {:.0}% of the band across and {:.0}% along it, \
             so the camera could have come in",
            framed.x * 100.0,
            framed.y * 100.0
        );
        let table = fill(rig, &places(&layout));
        // 0.7 before there was a sky. [`AIR`] now leaves a deliberate
        // band outside the slab, and the play area gives up nine per
        // cent of its width to it — which is a *decision*, so the bound
        // moves with it rather than the decision being reverted to keep
        // a number. What the bound still catches is the failure it was
        // written for: a camera pushed out until the table is a coaster.
        assert!(
            table.x.max(table.y) > 0.6,
            "{n} seats leaves the table filling {:.0}% across and {:.0}% along, \
             which is more room than a table needs around it",
            table.x * 100.0,
            table.y * 100.0
        );
    }
}

/// And what is left over is left over on both sides of it.
///
/// The far edge used to be pinned under the tab strip and every spare
/// unit opened up in front of the local seat — on a duel a fifth of the
/// window of bare felt below the mats, with the whole table riding high.
/// Measured in table units rather than on screen, because perspective
/// makes the same span of felt a different height at each end.
#[test]
fn the_table_sits_in_the_middle_of_what_can_be_seen() {
    let canvas = Canvas::hud(WINDOW);
    let top = 1.0 - 2.0 * canvas.top / canvas.window.y;
    let bottom = -1.0 + 2.0 * canvas.bottom / canvas.window.y;
    for n in 2..=8u8 {
        let layout = TableLayout::new(&seats(n), 2.01, None);
        let rig = CameraRig::home(&layout, canvas);
        let (min, max) = layout.extent().expect("a seated table has an extent");
        let eye = rig.distance * (1.0 + CAMERA_LEAN * CAMERA_LEAN).sqrt();
        // The rig stores world x/z; `+y` away from the local seat is `-z`.
        let along = -rig.target.y;
        let behind = eye * ground(top) - (max.y - along);
        let ahead = (min.y - along) - eye * ground(bottom);
        assert!(
            (behind - ahead).abs() < 0.1,
            "{n} seats: {behind:.2} units of felt behind the far seat \
             against {ahead:.2} in front of the near one"
        );
    }
}

/// The bug this whole framing exists for: the table shipped with a
/// hard-coded 20-unit camera looking at the middle of the felt, and the
/// local seat's own mat came out *underneath the hand zone*. A player
/// could not see their own creatures.
#[test]
fn the_local_seats_own_mat_is_not_behind_the_hand_bar() {
    let canvas = Canvas::hud(WINDOW);
    let layout = TableLayout::new(&seats(2), 1.78, None);
    let local = layout.local().copied().expect("a local seat");
    let near = local.center.y - local.half_extent.y;

    let bad = project(CameraRig::default(), canvas, Vec2::new(0.0, near));
    let floor = -1.0 + 2.0 * canvas.bottom / canvas.window.y;
    assert!(
        bad.y < floor,
        "the old framing is supposed to be the broken one: {} vs {floor}",
        bad.y
    );

    let good = project(
        CameraRig::home(&layout, canvas),
        canvas,
        Vec2::new(0.0, near),
    );
    assert!(
        good.y >= floor,
        "the near edge of my own mat is still under the hand zone: {} vs {floor}",
        good.y
    );
}

#[test]
fn every_seats_place_is_inside_the_part_of_the_window_you_can_see() {
    let canvas = Canvas::hud(WINDOW);
    for n in 2..=8 {
        let layout = TableLayout::new(&seats(n), 1.78, None);
        let rig = CameraRig::home(&layout, canvas);
        let top = 1.0 - 2.0 * canvas.top / canvas.window.y;
        let bottom = -1.0 + 2.0 * canvas.bottom / canvas.window.y;
        let right = 1.0 - 2.0 * canvas.right / canvas.window.x;
        for corner in places(&layout) {
            let at = project(rig, canvas, corner);
            assert!(
                at.y >= bottom - 1e-3 && at.y <= top + 1e-3,
                "{n} seats: {corner} lands at y {} , outside {bottom}..{top}",
                at.y
            );
            assert!(
                at.x >= -1.0 - 1e-3 && at.x <= right + 1e-3,
                "{n} seats: {corner} lands at x {} , outside -1..{right}",
                at.x
            );
        }
    }
}

/// A window is not always a laptop's. The framing has to hold for a phone
/// held upright, where the HUD covers proportionally far more of it.
#[test]
fn the_framing_holds_on_a_tall_narrow_window() {
    let canvas = Canvas::hud(Vec2::new(430.0, 932.0));
    let layout = TableLayout::new(&seats(4), 0.46, None);
    let rig = CameraRig::home(&layout, canvas);
    let top = 1.0 - 2.0 * canvas.top / canvas.window.y;
    let bottom = -1.0 + 2.0 * canvas.bottom / canvas.window.y;
    // Both bounds, because only checking the near edge is exactly the
    // hole that let the hand zone bug through in the first place: a shot
    // aimed too far off can satisfy one edge by breaking the other.
    for corner in corners(&layout) {
        let at = project(rig, canvas, corner);
        assert!(
            at.y >= bottom - 1e-3 && at.y <= top + 1e-3,
            "{corner} lands at y {}, outside {bottom}..{top}",
            at.y
        );
    }
    // Sideways this used to be a deliberate gap: a four-seat table did
    // not fit a phone at any honest distance, and the test asserted the
    // overflow so nobody would mistake it for working. It fits now, and
    // not because the camera got cleverer — the ring solve takes the pile
    // strips out of `x` before it fits the span to the canvas, so the
    // whole table is a strip narrower than it was and each seat's ground
    // a strip narrower still. The claim is now the strong one, and it is
    // made about the seat's whole *place*, piles included, because that
    // is the box the camera frames.
    let right = 1.0 - 2.0 * canvas.right / canvas.window.x;
    for corner in places(&layout) {
        let at = project(rig, canvas, corner);
        assert!(
            at.x >= -1.0 - 1e-3 && at.x <= right + 1e-3,
            "{corner} lands at x {}, outside -1..{right}",
            at.x
        );
    }
}

#[test]
fn a_table_with_nobody_at_it_frames_nothing_rather_than_dividing_by_zero() {
    let rig = CameraRig::home(&TableLayout::new(&[], 1.78, None), Canvas::hud(WINDOW));
    assert_eq!(rig, CameraRig::default());
    assert!(rig.distance.is_finite());
}

/// `docs/design.md` §1.1: the middle of the table is atmosphere, the mats
/// are the game. The bound is measurable, so it is measured — and it goes
/// both ways.
///
/// This replaces the lamplight ring's version of the same rule, which the
/// felt's own open middle has taken over from. That test is worth remembering
/// twice: it first compared the ring against `SeatSlot::lane_width`, the
/// mat's *long* edge, which a ring two and a half times the mat's depth
/// passes comfortably — the hearth dominated four straight screenshots
/// while its test agreed it was small. So the medallion is measured
/// against the gap it actually sits in, and the lower bound is here for
/// the reason `docs/client.md` gives about the felt's own brightness: a
/// one-sided assertion only stops the mistake it was written after, and
/// the opposite mistake ships next.
/// The cloth is written twice — once in Rust, where a test can block the
/// image at card size and measure that the tooth survives and that the
/// baize stays dark enough to read a card against, and once in WGSL,
/// where the GPU actually draws it. Nothing in either compiler can notice
/// that they have drifted apart, and the drawing is the one nobody can
/// assert about directly.
///
/// The two are not pixel-identical and are not meant to be: a lattice
/// hash on the CPU and a `sin`-based one on the GPU give the same kind of
/// noise and not the same noise, and the shader works in table units
/// where the generator works in texels. What has to agree is every colour
/// a test or a person reasoned about — the three the cloth is mixed from,
/// and the three the rail and its apron are.
#[test]
fn the_shader_and_the_generator_agree_about_the_cloth() {
    let src = include_str!("../shaders/felt.wgsl");
    for (name, ours) in [
        ("FELT_DEEP", tabletop::FELT_DEEP),
        ("FELT_CLOTH", tabletop::FELT_CLOTH),
        ("FELT_WORN", tabletop::FELT_WORN),
        ("RAIL_HIDE", tabletop::RAIL_HIDE),
        ("RAIL_LIP", tabletop::RAIL_LIP),
        ("APRON", tabletop::APRON),
    ] {
        let line = src
            .lines()
            .find(|line| line.trim_start().starts_with(&format!("const {name}:")))
            .unwrap_or_else(|| panic!("the shader has no {name}"));
        let Some((inside, _)) = line
            .rsplit_once("vec3<f32>(")
            .and_then(|(_, tail)| tail.split_once(')'))
        else {
            panic!("{name} is not a vec3 literal: {line}")
        };
        let theirs: Vec<f32> = inside
            .split(',')
            .map(|part| part.trim().parse().expect("a number"))
            .collect();
        assert_eq!(theirs.len(), 3, "{name} has {} channels", theirs.len());
        for (channel, (ours, theirs)) in ours.iter().zip(&theirs).enumerate() {
            assert!(
                (ours - theirs).abs() < 1e-6,
                "{name} channel {channel}: {ours} here, {theirs} in the shader"
            );
        }
    }
}

/// The mat is written twice for the same reason the cloth is: once in
/// Rust, where `tabletop::seat_mat`'s tests can measure that only the rim
/// carries the seat's colour and that the seam falls between two lanes,
/// and once in WGSL, where the GPU actually draws it. Nothing in either
/// compiler can notice that they have drifted apart.
///
/// Every number here is a shading decision that was argued somewhere —
/// the lanes are a quarter of what they first were, the rim's hue and its
/// opacity ride two different exponents on purpose — so a copy of one of
/// them in the shader that no longer matched would silently undo the
/// argument. The lengths (`MAT_CORNER`, `MAT_RIM`) are not here: they
/// travel to the GPU as uniforms, so there is only ever one of each.
#[test]
fn the_shader_and_the_generator_agree_about_the_mat() {
    let src = include_str!("../shaders/mat.wgsl");
    let read = |name: &str| crate::cardmat::tests::wgsl_const(src, name);
    for (name, ours) in [
        ("LANE_NEAR", tabletop::MAT_LANES[0]),
        ("LANE_MID", tabletop::MAT_LANES[1]),
        ("LANE_FAR", tabletop::MAT_LANES[2]),
        ("LANE_LEDGE", tabletop::MAT_LEDGE_VALUE),
        ("LEDGE_FRAC", tabletop::LEDGE_FRAC),
        ("LANE_FRAC", tabletop::LANE_FRAC),
        ("MARGIN_FRAC", tabletop::MARGIN_FRAC),
        ("LEDGE_SEAM", tabletop::MAT_LEDGE_SEAM),
        ("SEAM", tabletop::MAT_SEAM),
        ("SEAM_W", tabletop::MAT_SEAM_WIDTH),
        ("RIM_LIGHT", tabletop::MAT_RIM_LIGHT),
        ("RIM_FALL", tabletop::MAT_RIM_FALL),
        ("HUE_FALL", tabletop::MAT_HUE_FALL),
    ] {
        let theirs = read(name);
        assert!(
            (ours - theirs).abs() < 1e-6,
            "{name} is {ours} here and {theirs} in the shader"
        );
    }
}

/// The mat's WGSL is parsed and validated with the front end wgpu uses.
///
/// `the_shader_and_the_generator_agree_about_the_mat` reads constants out
/// of this file as text and would go on passing over a shader that does
/// not compile — and nothing else here ever compiled it, so a typo in the
/// mat's arithmetic surfaced as a mat that simply did not draw, with the
/// reason in a browser console.
#[test]
fn the_mat_shader_compiles() {
    let prelude = "\
struct VertexOutput {
@builtin(position) position: vec4<f32>,
@location(0) world_position: vec4<f32>,
@location(1) world_normal: vec3<f32>,
@location(2) uv: vec2<f32>,
};
struct Globals { time: f32 };
@group(0) @binding(11) var<uniform> globals: Globals;
";
    crate::cardmat::tests::check_wgsl(include_str!("../shaders/mat.wgsl"), prelude);
}

/// `PILE_REACH` is chosen in the model crate, which cannot see the mat's
/// printed border — that is `ZONE_MARGIN`, and it lives here. This is the
/// two of them being made to agree.
#[test]
fn a_pile_stands_clear_of_the_mat_it_serves() {
    let spare = baylee_client_core::layout::PILE_REACH - CARD_WIDTH * 0.5 - ZONE_MARGIN;
    assert!(
        spare > 0.0,
        "a pile's near edge falls {} inside the mat's own border — it \
         would be lying on the board it stands beside, not on the table",
        -spare
    );
}

#[test]
fn the_medallion_floats_in_the_open_middle() {
    let gap = baylee_client_core::layout::CENTRE_GAP;
    assert!(
        MEDALLION_SIZE < gap,
        "the colour wheel is {MEDALLION_SIZE} across a gap of {gap} — it \
         would be lying on both players' mats"
    );
    // And with felt visible on both sides of it, or it is not inlaid in
    // anything: it is a lid.
    let bare = (gap - MEDALLION_SIZE) * 0.5;
    assert!(
        bare > 0.4,
        "only {bare} of table shows beside the medallion"
    );

    for n in [2, 4, 6] {
        let layout = TableLayout::new(&seats(n), 2.0, None);
        let local = layout.local().copied().expect("a local seat");
        assert!(
            MEDALLION_SIZE < local.mat_depth(),
            "at {n} seats the eye lands on the medallion, not on the board: \
             {MEDALLION_SIZE} vs a mat {} deep",
            local.mat_depth()
        );
        assert!(
            MEDALLION_SIZE > local.lane_height(),
            "at {n} seats the medallion has shrunk to nothing: \
             {MEDALLION_SIZE} vs a lane {} tall",
            local.lane_height()
        );
    }
}
