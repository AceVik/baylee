//! What `TableLayout::extent` reports, which is the rectangle the camera frames: every mat's four corners inside it, every pile inside it too — the reason `SeatSlot::footprint` exists — a seat across the table measured across it rather than along it, a duel coming out the shape of its canvas so neither its width nor its height is wasted, and an empty table answering `None` instead of panicking. A pod's half-extent is stated in the seat's own frame, so everything here is measured after the same rotation `extent` applies; taking it unrotated is what reported a four-seat table's side seats as deep and narrow and cut their lands off the screen. Why a pile stands where it does is `piles`, and what the ring costs in reach is `sides`.

#[allow(clippy::wildcard_imports)] // this module's own vocabulary
use super::*;

/// The reason [`SeatSlot::footprint`] exists at all: the piles stand
/// outside the playing surface, so a camera framed from `half_extent`
/// puts every one of them off the screen.
#[test]
fn every_pile_is_inside_the_rectangle_the_camera_frames() {
    for n in [2, 3, 4, 6, 8] {
        let layout = TableLayout::new(&seats(n), 2.0, None);
        let (min, max) = layout.extent().expect("a table with seats");
        for slot in &layout.slots {
            let (sin, cos) = slot.facing.sin_cos();
            // A card's own box, turned to face its seat, then measured
            // along the table's axes — the same rotation `extent` does.
            let half = Vec2::new(
                cos.abs()
                    .mul_add(CARD_WIDTH * 0.5, sin.abs() * CARD_HEIGHT * 0.5),
                sin.abs()
                    .mul_add(CARD_WIDTH * 0.5, cos.abs() * CARD_HEIGHT * 0.5),
            );
            for pile in PileKind::ALL {
                let at = slot.pile_center(pile);
                let (lo, hi) = (at - half, at + half);
                assert!(
                    lo.x >= min.x - 1e-3
                        && lo.y >= min.y - 1e-3
                        && hi.x <= max.x + 1e-3
                        && hi.y <= max.y + 1e-3,
                    "{n} seats: the {} of seat {:?} runs {lo} to {hi}, outside \
                     the framed table {min} to {max} — the camera cuts it off",
                    pile.label(),
                    slot.player
                );
            }
        }
    }
}

#[test]
fn an_empty_table_is_handled_without_panicking() {
    let layout = TableLayout::new(&[], 1.78, None);
    assert!(layout.slots.is_empty());
    assert!(layout.local().is_none());
    assert!(layout.extent().is_none());
}

#[test]
fn the_tables_extent_holds_every_seats_mat() {
    for n in 2..=8 {
        let layout = TableLayout::new(&seats(n), 1.78, None);
        let (min, max) = layout.extent().expect("a seated table has an extent");
        for slot in &layout.slots {
            // Whatever a pod's own frame is, its four corners are inside.
            let (sin, cos) = slot.facing.sin_cos();
            for sx in [-1.0_f32, 1.0] {
                for sy in [-1.0_f32, 1.0] {
                    let local = slot.half_extent * Vec2::new(sx, sy);
                    let corner = slot.center
                        + Vec2::new(
                            cos.mul_add(local.x, sin * local.y),
                            (-sin).mul_add(local.x, cos * local.y),
                        );
                    assert!(
                        corner.x >= min.x - 1e-3
                            && corner.x <= max.x + 1e-3
                            && corner.y >= min.y - 1e-3
                            && corner.y <= max.y + 1e-3,
                        "{n} seats: {corner} escapes {min}..{max}"
                    );
                }
            }
        }
    }
}

#[test]
fn a_seat_across_the_table_is_measured_across_the_table() {
    // The bug this is here for: taking `half_extent` unrotated makes a
    // four-seat table report the side seats as deep and narrow when they
    // are wide and shallow, and the camera then cuts their lands off.
    let layout = TableLayout::new(&seats(4), 1.78, None);
    let side = layout
        .slots
        .iter()
        .find(|s| s.center.x.abs() > s.center.y.abs())
        .copied()
        .expect("a four-seat table has a seat on each side");
    let (min, max) = layout.extent().expect("extent");
    assert!(
        max.x - min.x >= 2.0 * (side.center.x.abs() + side.half_extent.y) - 1e-3,
        "the side seat is laid across the table, not along it"
    );
}

#[test]
fn a_wide_duel_compensates_for_camera_foreshortening() {
    // A span taller than the canvas wastes its width, a span wider wastes
    // its height, and the camera fits whatever this reports — so only a
    // span of the canvas's own shape wastes neither. Two seats is the case
    // worth pinning: a ring of six has neighbours to clear and cannot
    // always have it.
    for aspect in [1.0_f32, 1.6, 1.78, 2.0, 2.4] {
        let layout = TableLayout::new(&seats(2), aspect, None);
        let (min, max) = layout.extent().expect("a seated table has an extent");
        let span = max - min;
        let got = span.x / span.y;
        assert!(
            (got - aspect * if aspect >= 1.6 { 0.88 } else { 1.0 }).abs() < 0.05,
            "canvas {aspect}: the table came out {got} ({span:?})"
        );
    }
}
