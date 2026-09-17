//! The hover fan — where `SeatSlot::fan_pose` puts each card of a pile the pointer has opened. The top card stays exactly on the pile so it does not slide out from under the player, the older cards climb backwards and upwards out of it, the step runs along the seat's depth axis and never across it onto the mat, the two ends are turned opposite ways, `FAN_POP` slides the card under the pointer outwards, and an index past the end clamps onto the last rung. A side seat has no answer to which end is nearer the camera, so the exemptions are written against `facing.cos()` and `SIDE_SEAT_TILT` rather than against a list of seat numbers. Where the pile itself stands is `piles`.

#[allow(clippy::wildcard_imports)] // this module's own vocabulary
use super::*;

/// A hover fan opens straight up out of the pile's own column, at every
/// seat of every table.
///
/// The claim that matters is the one [`FAN_STEP`]'s doc makes: the fan
/// steps *along* the seat's depth axis and never across it, because
/// across is where the mat is and a card on the mat reads as a permanent
/// in play. So this is the same measurement
/// `a_pile_stands_beside_the_ground_and_never_on_it` takes, repeated for
/// every rung of the fan, and it has to come back with the pile's own
/// number to the last bit.
///
/// [`FAN_POP`] is the one thing that does move a card across the column,
/// and it moves it the *other* way — out onto bare table, never in. It
/// has a test of its own below.
#[test]
fn a_fan_opens_up_the_column_and_never_across_it() {
    for n in [2, 3, 4, 6, 8] {
        let layout = TableLayout::new(&seats(n), 2.0, None);
        for slot in &layout.slots {
            let side = Vec2::new(slot.facing.cos(), -slot.facing.sin());
            for pile in PileKind::ALL {
                let column = (slot.pile_center(pile) - slot.center).dot(side);
                for rung in 0..FAN_MAX {
                    let pose = slot.fan_pose(pile, rung, FAN_MAX, false);
                    let across = (pose.at - slot.center).dot(side);
                    assert!(
                        (across - column).abs() < 1e-4,
                        "{n} seats: card {rung} of the {} fan is {across} out \
                         where the pile is {column} — it has stepped across the \
                         column and onto the mat",
                        pile.label()
                    );
                }
            }
        }
    }
}

/// The top of the pile stays on the pile, and the fan opens behind it.
///
/// Which is the whole shape of it, and the half that would be easy to
/// undo: the pointer opens a pile by resting on its top card, so a fan
/// that moved that card would slide the one thing the player is looking
/// at out from under them. Index 0 therefore *is* the rung, and the rung
/// is measured backwards — away from the camera and up.
///
/// Measured in table space, where `+y` runs away from the camera. The
/// side seats are exempt from the two halves that are about the camera at
/// all, because their depth axis runs across the screen: no end of it is
/// nearer, and tipping a card about it barely turns the face towards the
/// camera. `cos(facing)` is that projection, and it is what the exemption
/// is written against rather than a list of seat numbers.
#[test]
fn the_top_of_the_pile_stays_on_the_pile() {
    for n in [2, 3, 4, 6, 8] {
        let layout = TableLayout::new(&seats(n), 2.0, None);
        for slot in &layout.slots {
            let top = slot.fan_pose(PileKind::Graveyard, 0, FAN_MAX, false);
            let oldest = slot.fan_pose(PileKind::Graveyard, FAN_MAX - 1, FAN_MAX, false);

            assert_eq!(
                top.at,
                slot.pile_center(PileKind::Graveyard),
                "{n} seats, seat {}: opening the fan moved the card the \
                 pointer opened it on",
                slot.ring_index
            );
            assert!(
                (top.lift - FAN_FLOAT).abs() < 1e-6,
                "the near end of the fan does not start at the float"
            );
            assert!(
                oldest.lift > top.lift,
                "{n} seats, seat {}: the fan does not climb",
                slot.ring_index
            );
            if slot.facing.cos().abs() > SIDE_SEAT_TILT {
                assert!(
                    oldest.at.y > top.at.y + 1e-4,
                    "{n} seats, seat {}: the fan opened towards the camera, \
                     over the card it is supposed to come out from behind",
                    slot.ring_index
                );
                // The card's normal once it has been laid flat and
                // tipped: `cos(facing) · sin(tilt)` is how much of it
                // points at the camera, and it has to be positive.
                assert!(
                    slot.facing.cos() * top.tilt.sin() > 0.0,
                    "{n} seats, seat {}: the fan leans away from the camera",
                    slot.ring_index
                );
            } else {
                // A side seat has no answer to "which end is nearer the
                // camera" — its depth axis runs across the screen — and
                // takes the one that is never wrong instead. `camera_lies`
                // calls that inwards, so the fan opens *outwards* from it,
                // towards the edge of the seat's own strip and never over
                // the middle of the table where everyone else is playing.
                let away = Vec2::new(slot.facing.sin(), slot.facing.cos());
                let outwards = (oldest.at - top.at).dot(away);
                assert!(
                    outwards < 0.0,
                    "{n} seats, seat {}: the side seat's fan opened over the \
                     middle of the table",
                    slot.ring_index
                );
            }
        }
    }
}

/// The card under the pointer steps out of the line, and outwards — away
/// from the mat, which is the only direction that is neither the board
/// nor another card of the fan.
///
/// The fan is thin enough to be walked rather than clicked, so this is
/// the walk's only feedback: without it nothing on screen says which card
/// the preview is reading.
#[test]
fn the_card_under_the_pointer_steps_out_of_the_fan() {
    for n in [2, 4, 8] {
        let layout = TableLayout::new(&seats(n), 2.0, None);
        for slot in &layout.slots {
            for pile in PileKind::ALL {
                let side = Vec2::new(slot.facing.cos(), -slot.facing.sin());
                let still = slot.fan_pose(pile, 3, FAN_MAX, false);
                let popped = slot.fan_pose(pile, 3, FAN_MAX, true);
                let out = (popped.at - still.at).dot(side) * pile.side();
                assert!(
                    (out - FAN_POP).abs() < 1e-4,
                    "{n} seats: the {} popped {out} out, which is not {FAN_POP} \
                     away from the mat",
                    pile.label()
                );
                // And nothing else about the card changed: a pop is a
                // step aside, not a second pose.
                assert_eq!(
                    (still.lift, still.tilt, still.yaw),
                    (popped.lift, popped.tilt, popped.yaw)
                );
            }
        }
    }
}

/// A fan is a hand of cards and not a staircase: its two ends are turned
/// opposite ways by the same amount, and its middle is not turned at all.
#[test]
fn a_fan_is_turned_symmetrically_about_its_middle() {
    let layout = TableLayout::new(&seats(2), 2.0, None);
    let slot = layout.local().expect("a local seat");
    for len in 1..=FAN_MAX {
        let yaws: Vec<f32> = (0..len)
            .map(|i| slot.fan_pose(PileKind::Exile, i, len, false).yaw)
            .collect();
        let sum: f32 = yaws.iter().sum();
        assert!(
            sum.abs() < 1e-5,
            "a fan of {len} is turned {sum} out of true overall: {yaws:?}"
        );
        if len > 1 {
            assert!(
                (yaws[0] + yaws[len - 1]).abs() < 1e-6,
                "a fan of {len} turns its two ends by different amounts"
            );
            assert!(yaws[0] > 0.0, "the top card is turned the wrong way");
        }
    }
}

/// An index past the end of the fan is clamped onto the last rung rather
/// than refused — a fan is a drawing, and the worst a clamp does is put
/// two cards in one place.
#[test]
fn a_card_past_the_end_of_the_fan_lands_on_the_bottom_rung() {
    let layout = TableLayout::new(&seats(2), 2.0, None);
    let slot = layout.local().expect("a local seat");
    let bottom = slot.fan_pose(PileKind::Graveyard, 2, 3, false);
    for beyond in [3, 4, 99] {
        assert_eq!(slot.fan_pose(PileKind::Graveyard, beyond, 3, false), bottom);
    }
    // And a fan of nothing is the pile itself, floated.
    let none = slot.fan_pose(PileKind::Graveyard, 0, 0, false);
    assert_eq!(none.at, slot.pile_center(PileKind::Graveyard));
    assert!((none.lift - FAN_FLOAT).abs() < 1e-6);
}

/// A whole zone count must not turn a seven-card glance into a table-wide
/// spread. Both the travel and the rotated silhouette stay in the pile strip.
#[test]
fn the_fan_has_a_bounded_silhouette_even_for_a_whole_library() {
    for n in [2, 3, 4, 6, 8] {
        let layout = TableLayout::new(&seats(n), 2.0, None);
        for slot in &layout.slots {
            for pile in PileKind::ALL {
                let bottom = slot.fan_pose(pile, FAN_MAX - 1, FAN_MAX, false);
                assert_eq!(slot.fan_pose(pile, usize::MAX, usize::MAX, false), bottom);
                for len in 1..=FAN_MAX {
                    for rung in 0..len {
                        let pose = slot.fan_pose(pile, rung, len, false);
                        assert!(pose.at.distance(slot.pile_center(pile)) <= 1.69);
                        assert!((0.09..=0.41).contains(&pose.lift));
                        assert!(pose.yaw.abs() <= 0.14);
                        let half_width = (CARD_WIDTH * pose.yaw.cos().abs()
                            + CARD_HEIGHT * pose.yaw.sin().abs())
                            * 0.5;
                        assert!(PILE_REACH - half_width >= 0.85);
                        assert!(half_width + FAN_POP <= 0.90);
                    }
                }
            }
        }
    }
}

/// Turning cards in their own planes leaves their normals parallel. The
/// depth step must beat the rise along that normal, keeping the newest on top.
#[test]
fn each_fan_rung_clears_the_next_card_along_its_normal() {
    let layout = TableLayout::new(&seats(4), 2.0, None);
    for slot in &layout.slots {
        let top = slot.fan_pose(PileKind::Graveyard, 0, FAN_MAX, false);
        let next = slot.fan_pose(PileKind::Graveyard, 1, FAN_MAX, false);
        let separation = top.at.distance(next.at) * top.tilt.sin().abs()
            - (next.lift - top.lift) * top.tilt.cos();
        assert!((0.08..=0.11).contains(&separation));
    }
}
