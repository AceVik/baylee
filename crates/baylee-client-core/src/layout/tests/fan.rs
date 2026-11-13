//! The hover fan — where `SeatSlot::fan_pose` puts each card of a pile the pointer has opened: a hand of cards held over the pile, opened in the seat's own frame.

#[allow(clippy::wildcard_imports)] // this module's own vocabulary
use super::*;

/// One seat's frame: its depth axis (towards the table's middle) and its
/// sideways one (its own right hand).
fn frame(slot: &SeatSlot) -> (Vec2, Vec2) {
    (
        Vec2::new(slot.facing.sin(), slot.facing.cos()),
        Vec2::new(slot.facing.cos(), -slot.facing.sin()),
    )
}

/// A fan in its seat's own frame: each pose's offset from the pile, along
/// the seat's right hand and along its depth, with its lift and its yaw.
/// The tip is left out: it is the viewer's (`a_fan_leans_towards_the_one_looking`).
fn in_the_seats_frame(slot: &SeatSlot, pile: PileKind, len: usize) -> Vec<[f32; 4]> {
    let (away, side) = frame(slot);
    (0..len)
        .map(|i| {
            let pose = slot.fan_pose(pile, i, len, false);
            let from = (pose.at - slot.pile_center(pile)) / slot.scale;
            [from.dot(side), from.dot(away), pose.lift, pose.yaw]
        })
        .collect()
}

/// Every seat's fan is the same hand, turned as its board is turned: the
/// seat across the table shows mine point-mirrored and a flank shows it a
/// quarter round, because the fan is built in the seat's frame and in no
/// one else's (the owner, 12.11.2026: "bei rotierten Spielfeldern auch
/// spiegelverkehrt"). The fan it replaces chose its direction by where the
/// camera lies and was a different shape at every seat.
#[test]
fn every_seats_fan_is_the_same_hand_in_its_own_frame() {
    for n in [2, 3, 4, 6, 8] {
        let layout = TableLayout::new(&seats(n), 2.0, None);
        let mine = layout.local().expect("a local seat");
        for pile in [PileKind::Graveyard, PileKind::Exile] {
            let hand = in_the_seats_frame(mine, pile, FAN_MAX);
            for slot in &layout.slots {
                let theirs = in_the_seats_frame(slot, pile, FAN_MAX);
                for (a, b) in hand.iter().zip(&theirs) {
                    for (x, y) in a.iter().zip(b) {
                        assert!(
                            (x - y).abs() < 1e-4,
                            "{n} seats, seat {}: the {} fan is not the local \
                             seat's turned round: {theirs:?} against {hand:?}",
                            slot.ring_index,
                            pile.label()
                        );
                    }
                }
            }
        }
    }
}

/// The top of the pile stays over the pile and lies on top, and the older
/// cards open away from it towards the seat's own ground.
///
/// The pointer opens a pile by resting on its top card, so a fan that moved
/// that card would slide the one thing the player is looking at out from
/// under them. And the fan opens over the strip between the pile and the
/// mat: never outwards, where the table ends or another seat begins, and
/// never along the column, where the pile's neighbours stand.
#[test]
fn the_top_of_the_pile_stays_over_the_pile_and_the_rest_open_inwards() {
    for n in [2, 3, 4, 6, 8] {
        let layout = TableLayout::new(&seats(n), 2.0, None);
        for slot in &layout.slots {
            let (away, side) = frame(slot);
            for pile in [PileKind::Graveyard, PileKind::Exile] {
                let top = slot.fan_pose(pile, 0, FAN_MAX, false);
                assert!(
                    top.at.distance(slot.pile_center(pile)) < 1e-4,
                    "{n} seats, seat {}: opening the fan moved the card the \
                     pointer opened it on",
                    slot.ring_index
                );
                assert!(top.yaw.abs() < 1e-6, "the card over the pile is turned");
                let mut before = top;
                for rung in 1..FAN_MAX {
                    let pose = slot.fan_pose(pile, rung, FAN_MAX, false);
                    let step = (pose.at - before.at) / slot.scale;
                    // Inwards is against the side the pile stands out on.
                    let inwards = -step.dot(side) * pile.side();
                    assert!(
                        (0.45..=0.55).contains(&inwards),
                        "{n} seats, seat {}: card {rung} shows {inwards} of a \
                         card's width of its neighbour",
                        slot.ring_index
                    );
                    assert!(
                        step.dot(away) < 0.0,
                        "the hand opens about a point on its owner's side"
                    );
                    assert!(
                        (before.lift - pose.lift - FAN_RISE).abs() < 1e-6,
                        "an older card lies one rise under its newer neighbour"
                    );
                    assert!(
                        (pose.yaw.abs() - FAN_SPREAD * rung as f32).abs() < 1e-6,
                        "card {rung} is not turned with the hand"
                    );
                    before = pose;
                }
            }
        }
    }
}

/// A card's head points away from the hinge of the hand, so the cards open
/// like a hand held at the owner's edge and not like a row of dominoes.
///
/// Read off the pose alone: a card turned by `yaw` in its own plane has its
/// head along the seat's depth axis turned by `yaw` towards the seat's left,
/// and that has to be the line from the hinge through the card.
#[test]
fn every_card_of_a_fan_points_away_from_its_hinge() {
    let layout = TableLayout::new(&seats(4), 2.0, None);
    for slot in &layout.slots {
        let (away, side) = frame(slot);
        for pile in [PileKind::Graveyard, PileKind::Exile] {
            let hinge = slot.pile_center(pile) - away * (FAN_RADIUS * slot.scale);
            for rung in 0..FAN_MAX {
                let pose = slot.fan_pose(pile, rung, FAN_MAX, false);
                let head = away * pose.yaw.cos() - side * pose.yaw.sin();
                let out = (pose.at - hinge).normalize();
                assert!(
                    head.distance(out) < 1e-4,
                    "seat {}: card {rung} of the {} fan points {head:?}, and \
                     the hinge lies the other way from {out:?}",
                    slot.ring_index,
                    pile.label()
                );
            }
        }
    }
}

/// The card under the pointer lies on top of the whole hand and has moved
/// nowhere: the card being read is the card nothing covers, and one that
/// stepped aside would step out from under the pointer that chose it.
#[test]
fn the_card_under_the_pointer_lies_on_top_and_stays_where_it_was() {
    for n in [2, 4, 8] {
        let layout = TableLayout::new(&seats(n), 2.0, None);
        for slot in &layout.slots {
            for pile in PileKind::ALL {
                let highest = slot.fan_pose(pile, 0, FAN_MAX, false).lift;
                for rung in 0..FAN_MAX {
                    let still = slot.fan_pose(pile, rung, FAN_MAX, false);
                    let read = slot.fan_pose(pile, rung, FAN_MAX, true);
                    assert_eq!(
                        (still.at, still.tilt, still.yaw),
                        (read.at, read.tilt, read.yaw)
                    );
                    assert!(
                        read.lift >= highest + FAN_RISE - 1e-6,
                        "{n} seats: card {rung} under the pointer does not \
                         clear the top of its fan"
                    );
                }
            }
        }
    }
}

/// The tip is the viewer's: every seat's fan leans its faces towards the
/// camera, and a flank's, whose depth axis runs across the screen, leans
/// over the table and never out past its edge.
#[test]
fn a_fan_leans_towards_the_one_looking() {
    for n in [2, 3, 4, 6, 8] {
        let layout = TableLayout::new(&seats(n), 2.0, None);
        for slot in &layout.slots {
            let pose = slot.fan_pose(PileKind::Graveyard, 1, FAN_MAX, false);
            assert!((pose.tilt.abs() - FAN_TILT).abs() < 1e-6);
            if slot.facing.cos().abs() > SIDE_SEAT_TILT {
                // The card's normal once it has been laid flat and tipped:
                // `cos(facing) · sin(tilt)` is how much of it points at the
                // camera, and it has to be positive.
                assert!(
                    slot.facing.cos() * pose.tilt.sin() > 0.0,
                    "{n} seats, seat {}: the fan leans away from the camera",
                    slot.ring_index
                );
            }
        }
    }
}

/// An index past the end of the fan is clamped onto the last card rather
/// than refused — a fan is a drawing, and the worst a clamp does is put
/// two cards in one place.
#[test]
fn a_card_past_the_end_of_the_fan_lands_on_its_last_card() {
    let layout = TableLayout::new(&seats(2), 2.0, None);
    let slot = layout.local().expect("a local seat");
    let last = slot.fan_pose(PileKind::Graveyard, 2, 3, false);
    for beyond in [3, 4, 99] {
        assert_eq!(slot.fan_pose(PileKind::Graveyard, beyond, 3, false), last);
    }
    // And a fan of nothing is the pile itself, floated.
    let none = slot.fan_pose(PileKind::Graveyard, 0, 0, false);
    assert!(none.at.distance(slot.pile_center(PileKind::Graveyard)) < 1e-5);
    assert!((none.lift - FAN_FLOAT).abs() < 1e-6);
}

/// A whole zone's count must not turn a glance into a table-wide spread:
/// the hand stays in the strip between its pile and the mat, reaches at
/// most half a card onto the mat's own edge, and stays off the piles
/// standing beside it in their column.
#[test]
fn the_fan_has_a_bounded_silhouette_even_for_a_whole_zone() {
    for n in [2, 3, 4, 6, 8] {
        let layout = TableLayout::new(&seats(n), 2.0, None);
        for slot in &layout.slots {
            let (away, side) = frame(slot);
            for pile in PileKind::ALL {
                let last = slot.fan_pose(pile, FAN_MAX - 1, FAN_MAX, false);
                assert_eq!(slot.fan_pose(pile, usize::MAX, usize::MAX, false), last);
                for len in 1..=FAN_MAX {
                    for rung in 0..len {
                        let pose = slot.fan_pose(pile, rung, len, false);
                        let from = (pose.at - slot.pile_center(pile)) / slot.scale;
                        let inwards = -from.dot(side) * pile.side();
                        assert!((-1e-4..=1.6).contains(&inwards));
                        assert!((-0.4..=1e-4).contains(&from.dot(away)));
                        assert!((FAN_FLOAT..=FAN_FLOAT + FAN_POP).contains(&pose.lift));
                        assert!(pose.yaw.abs() <= 0.45);
                        // No further onto the mat than two thirds of a card
                        // past its edge, measured to the card's far side.
                        let over = inwards + CARD_WIDTH * 0.5 - PILE_REACH;
                        assert!(over <= 0.66, "the hand lies {over} over the mat");
                    }
                }
            }
        }
    }
}

/// Turning cards in their own planes leaves their normals parallel, so the
/// order in height is the rise's alone; the tip takes a little of it back
/// on one half of the ring and must leave the newer card on top.
#[test]
fn each_card_of_a_fan_clears_the_next_along_its_normal() {
    let layout = TableLayout::new(&seats(4), 2.0, None);
    for slot in &layout.slots {
        let (away, _) = frame(slot);
        for rung in 0..FAN_MAX - 1 {
            let newer = slot.fan_pose(PileKind::Graveyard, rung, FAN_MAX, false);
            let older = slot.fan_pose(PileKind::Graveyard, rung + 1, FAN_MAX, false);
            let back = (older.at - newer.at).dot(away).abs();
            let separation =
                (newer.lift - older.lift) * newer.tilt.cos() - back * newer.tilt.sin().abs();
            assert!(
                separation >= 0.01,
                "seat {}: cards {rung} and {} of a fan lie {separation} apart",
                slot.ring_index,
                rung + 1
            );
        }
    }
}
