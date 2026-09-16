//! Which chair a seat gets and where it stands: the local seat at the near edge of every table, two players opposite each other, the ring running clockwise in turn order so the player on your left acts after you, and no two seats' grounds sharing any table at any seat count, canvas shape or focus. The bound the layout itself applies is on the distance between two centres, which is a different question — a mat is a rectangle turned to face its own seat — so the overlap here asks the real one of every pair over the footprint, piles included. How much room a seat is handed is `ring`, which way it is turned is `facing`, and who shares a side of the ring is `sides`.

#[allow(clippy::wildcard_imports)] // this module's own vocabulary
use super::*;

#[test]
fn the_local_seat_is_always_at_the_near_edge() {
    for n in 1..=8u8 {
        let layout = TableLayout::new(&seats(n), 1.78, None);
        let local = layout.local().expect("a local slot");
        assert!(local.is_local);
        assert_eq!(local.ring_index, 0);
        assert!(
            local.center.x.abs() < 1e-4,
            "local seat is centred horizontally for {n} seats"
        );
        assert!(
            local.center.y < 0.0,
            "local seat is on the near side for {n} seats"
        );
    }
}

#[test]
fn two_players_sit_opposite_each_other() {
    let layout = TableLayout::new(&seats(2), 1.78, None);
    let a = layout.slots[0].center;
    let b = layout.slots[1].center;
    assert!(a.y < 0.0 && b.y > 0.0);
    assert!((a.x - b.x).abs() < 1e-4);
}

#[test]
fn every_seat_count_produces_distinct_pod_centres() {
    for n in 2..=8u8 {
        let layout = TableLayout::new(&seats(n), 1.78, None);
        for i in 0..layout.slots.len() {
            for j in (i + 1)..layout.slots.len() {
                let d = layout.slots[i].center.distance(layout.slots[j].center);
                assert!(d > 1.0, "seats {i} and {j} of {n} overlap (distance {d})");
            }
        }
    }
}

#[test]
fn seats_are_ordered_clockwise_in_turn_order() {
    let layout = TableLayout::new(&seats(4), 1.78, None);
    // Ring index 1 is the next player in turn order and sits to the left.
    // Clockwise from the near edge of a table *is* the left hand — six
    // o'clock to seven — and it is where Magic's turn order goes, which
    // is the association a player brings with them. This used to read
    // `> 0.0`: seats were laid out anticlockwise while every frame built
    // from `facing` assumed the other way round, so a flank seat's lands
    // stood between it and the middle and its creatures behind its back.
    assert!(layout.slots[1].center.x < 0.0);
    // Ring index 3 is the previous player and sits to the right.
    assert!(layout.slots[3].center.x > 0.0);
    // Angles increase monotonically.
    for w in layout.slots.windows(2) {
        assert!(w[1].angle > w[0].angle);
    }
}

#[test]
fn no_two_seats_play_on_the_same_table() {
    for n in 2..=8u8 {
        for aspect in [2.01_f32, 16.0 / 9.0, 1.0, 0.6] {
            // With an opponent under inspection as well: the focus is the
            // one thing that makes two boards different sizes, and the
            // bound it borrows against was measured for boards that are
            // all the same.
            for focus in [None, Some(PlayerId::new(1))] {
                let layout = TableLayout::new(&seats(n), aspect, focus);
                for i in 0..layout.slots.len() {
                    for j in (i + 1)..layout.slots.len() {
                        let (a, b) = (&layout.slots[i], &layout.slots[j]);
                        // A mat is never narrower than one card, whatever the
                        // geometry says — a board that cannot hold a single
                        // permanent is not a board. Eight seats on a portrait
                        // canvas reach that floor, and there the mats do meet;
                        // the table has already stopped working by then, and
                        // the honest answer is to seat fewer players, not to
                        // draw a board a card does not fit on.
                        let floored = |s: &SeatSlot| s.lane_width() <= CARD_WIDTH * 2.0 + 1e-3;
                        if floored(a) || floored(b) {
                            continue;
                        }
                        assert!(
                            !grounds_overlap(a, b),
                            "{n} seats at aspect {aspect:.2}, focus {focus:?}: \
                         seats {i} and {j} overlap — {:.2} wide at {:?} \
                         facing {:.2} against {:.2} wide at {:?} facing {:.2}",
                            a.lane_width(),
                            a.center,
                            a.facing,
                            b.lane_width(),
                            b.center,
                            b.facing,
                        );
                    }
                }
            }
        }
    }
}
