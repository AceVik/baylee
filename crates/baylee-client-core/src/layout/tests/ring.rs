//! The solve: how far out the ring stands and how wide a board it hands each seat. One board for everybody unless an opponent is under inspection, in which case what it gains comes from the other opponents; a mat the same depth at every table, because depth off the ring gave a duel the shallowest board of the lot; a middle left clear for the hearth and no wider than it has to be; and a pod that gets what `MIN_POD_WIDTH` promises or a ring at the ceiling the camera can still frame. Every claim that a ring could have grown further carries the same three escapes — the centre channel, a crowding solve that has already paid the minimum, and `MAX_RING_X`/`MAX_RING_Y` — because a ring satisfying none of them is empty felt every card is drawn smaller for. What shape the ring takes is `sides`, and what it costs the camera is `extent`.

#[allow(clippy::wildcard_imports)] // this module's own vocabulary
use super::*;

#[test]
fn every_seat_gets_the_same_board() {
    // One board, handed out unchanged to everybody. It was briefly
    // per-side — each side taking what its own neighbours allowed, so a
    // table of three left the near seat half the ring to itself — and a
    // table where one player's board is wider than another's is a table
    // where the wider board is the one being played on.
    for n in 2..=8u8 {
        for aspect in [1.78_f32, 1.0, 0.6] {
            let layout = TableLayout::seated(
                &seats(n).into_iter().map(Seat::alone).collect::<Vec<_>>(),
                aspect,
                None,
            );
            let at_ceiling =
                layout.radius.x >= MAX_RING_X - 1e-3 || layout.radius.y >= MAX_RING_Y - 1e-3;
            // A duel's ring never grows: two seats have nothing to be
            // crowded by, so their mats sit against the middle and take
            // whatever the canvas leaves. On a square one that is under
            // the minimum, and pushing them apart to reach it would buy
            // width with empty table.
            let at_channel = layout
                .slots
                .iter()
                .map(|slot| slot.center.length() - slot.half_extent.y)
                .fold(f32::INFINITY, f32::min)
                <= CENTRE_GAP * 0.5 + 1e-3;
            for slot in &layout.slots {
                assert!(
                    (slot.half_extent - layout.slots[0].half_extent).length() < 1e-3,
                    "{n} seats at {aspect}: seat {} has {:?} against the local {:?}",
                    slot.ring_index,
                    slot.half_extent,
                    layout.slots[0].half_extent
                );
                assert!(
                    at_ceiling || at_channel || slot.lane_width() >= MIN_POD_WIDTH - 1e-2,
                    "{n} seats at {aspect}: a seat plays on {} on a ring that \
                     could still have grown",
                    slot.lane_width()
                );
            }
        }
    }
}

#[test]
fn focusing_an_opponent_enlarges_it_at_everyone_elses_expense() {
    let players = seats(4);
    let plain = TableLayout::new(&players, 1.78, None);
    let focused = TableLayout::new(&players, 1.78, Some(PlayerId::new(2)));

    let target = focused.slot(PlayerId::new(2)).expect("focused slot");
    let before = plain.slot(PlayerId::new(2)).expect("plain slot");
    assert!(target.half_extent.x > before.half_extent.x);

    let bystander = focused.slot(PlayerId::new(1)).expect("bystander");
    let bystander_before = plain.slot(PlayerId::new(1)).expect("bystander");
    assert!(bystander.half_extent.x < bystander_before.half_extent.x);
}

#[test]
fn a_mat_is_the_same_depth_at_every_table() {
    // The bug this replaces: depth came off the ring, so a table laid out
    // for eight seats gave each of them a deeper mat than a duel did — and
    // a duel, which is what almost every game is, got the shallowest board
    // of the lot. A card is the same size at every table.
    for n in 1..=8 {
        for aspect in [0.6_f32, 1.0, 1.78, 2.0, 2.8] {
            for slot in &TableLayout::new(&seats(n), aspect, None).slots {
                assert!(
                    (slot.half_extent.y * 2.0 - POD_DEPTH).abs() < 1e-3,
                    "{n} seats at {aspect}: mat is {} deep, not {POD_DEPTH}",
                    slot.half_extent.y * 2.0
                );
            }
        }
    }
}

#[test]
fn the_middle_stays_clear_for_the_table() {
    // The open middle is the negative form of this layout: it is whatever
    // the mats leave. If the mats close in, the medallion has nowhere to
    // sit and a table stops reading as a table; if they drift apart,
    // every card is drawn smaller for the empty felt between them. Both
    // bounds, because the second is the mistake that was actually made.
    for n in 2..=8 {
        let layout = TableLayout::new(&seats(n), 2.0, None);
        let inner = layout
            .slots
            .iter()
            .map(|slot| slot.center.length() - slot.half_extent.y)
            .fold(f32::INFINITY, f32::min);
        assert!(
            inner >= CENTRE_GAP * 0.5 - 1e-3,
            "{n} seats: a mat reaches to {inner} of the middle, inside the {} channel",
            CENTRE_GAP * 0.5
        );

        // And no further out than it has to be. There are exactly three
        // reasons the ring may stand where it does and the mats be as
        // wide as they are, so one of them has to be tight:
        //
        // - the mats are as close to the middle as the open gap allows;
        // - the crowding solve stopped there, a pod being exactly the
        //   board's worth `MIN_POD_WIDTH` promises and one step further
        //   out therefore more than a seat needs;
        // - or the ring is at the ceiling the camera can still frame, and
        //   the pods are narrower than the minimum only because there is
        //   nowhere left to grow.
        //
        // A ring that satisfies none of the three is empty table, and
        // empty table is what every card on it is drawn smaller for.
        //
        // The ellipse used to stand here as a fourth reason — its flanks
        // bring two seats closer together than any circle of the same
        // mean radius would. It is not a separate reason any more, and
        // twice over: `sides_on` spaces sides by distance rather than by
        // angle, so no part of the ring is tighter than another, and what
        // is left of it lives inside `side_half_widths`, which is the
        // function the solve asks. So it comes out as the second bullet
        // like every other way a pod can be limited.
        //
        // It is the *narrowest* pod that has to be tight, because that is
        // what the solve grows the ring for. Some seats then get more
        // than the minimum — a table of three leaves the near seat half
        // the ring to itself — and a wider board than promised is not a
        // reason to push everyone further out.
        let narrowest = layout
            .slots
            .iter()
            .map(|slot| slot.half_extent.x * 2.0)
            .fold(f32::INFINITY, f32::min);
        let at_ceiling =
            layout.radius.x >= MAX_RING_X - 1e-3 || layout.radius.y >= MAX_RING_Y - 1e-3;
        assert!(
            (inner - CENTRE_GAP * 0.5).abs() < 1e-3
                || (narrowest - MIN_POD_WIDTH).abs() < 1e-2
                || at_ceiling,
            "{n} seats: mats stop {inner} out and are {narrowest} wide on a ring \
             {:?} that could still have grown — none of the middle, the \
             crowding or the ceiling put them there",
            layout.radius
        );
    }
}

// `MIN_POD_WIDTH` is the width a seat is *handed*, and for a long time it
// was the width of an arc a seat was then charged two pile strips out of.
// Measured at the aspect above: four seats were solved for an arc of 10.0
// and given 6.10 — four cards on a row the name promises seven to.
#[test]
fn a_pod_gets_the_width_its_minimum_promises_or_the_ring_is_at_its_ceiling() {
    for n in [3u8, 4, 5, 6, 8] {
        for aspect in [HUD_ASPECT, 16.0 / 9.0, 1.0] {
            let layout = TableLayout::new(&seats(n), aspect, None);
            let width = layout.slots[0].lane_width();
            let at_ceiling =
                layout.radius.x >= MAX_RING_X - 0.01 || layout.radius.y >= MAX_RING_Y - 0.01;
            assert!(
                width >= MIN_POD_WIDTH - 0.01 || at_ceiling,
                "{n} seats at aspect {aspect:.2}: {width:.2} wide on a ring \
                 ({:.2}, {:.2}) that could still have grown",
                layout.radius.x,
                layout.radius.y,
            );
        }
    }
}

// The complaint this closes, in the terms it was made in: at four players
// a seat's row was too narrow. Six cards is an ordinary mid-game board of
// lands, and they used to overlap on it.
#[test]
fn six_cards_lie_side_by_side_at_a_four_seat_table() {
    let layout = TableLayout::new(&seats(4), HUD_ASPECT, None);
    for slot in &layout.slots {
        let packing = pack_lane(6, slot.lane_width());
        assert!(
            !packing.fanned,
            "six cards fan on a {:.2}-wide row",
            slot.lane_width()
        );
    }
}

// Growing the ring is only free while the camera can still frame it, so
// the ceiling exists — and a ceiling that no seat count ever reaches is a
// ceiling nobody has checked. Six seats and up sit on it.
#[test]
fn a_crowded_table_stops_growing_at_the_ceiling() {
    let layout = TableLayout::new(&seats(8), HUD_ASPECT, None);
    assert!(
        layout.radius.x <= MAX_RING_X + 0.01 && layout.radius.y <= MAX_RING_Y + 0.01,
        "the ring outgrew what the camera can frame: {:?}",
        layout.radius
    );
    assert!(
        layout.slots[0].lane_width() < MIN_POD_WIDTH,
        "eight seats reaching the minimum would mean the ceiling is never tested"
    );
}

// Seats sharing a ring evenly should get less room as more of them
// arrive, and the old solve did not: five seats came out *narrower* than
// six, because the ring was sized against one bound and the width read
// off another.
#[test]
fn more_seats_never_means_a_wider_pod() {
    let mut last = f32::INFINITY;
    for n in [3u8, 4, 5, 6, 8] {
        let width = TableLayout::new(&seats(n), HUD_ASPECT, None).slots[0].lane_width();
        assert!(
            width <= last + 0.01,
            "{n} seats got {width:.2}, wider than the {last:.2} of fewer seats"
        );
        last = width;
    }
}
