//! What a seat's `facing` decides once it has a place: that forward is the way the creatures are, that the three lanes stack from the middle of the table back towards the seat, that a creature staged by `STAGE_STEP` never reaches the ground in front of it, and that the band its bar is written on is the one edge of its mat nearest the middle of the table — the same edge at every seat, which is what `LEDGE_IS_OUTER` says and what no card may stand on. Written as comparisons between two lanes or two seats rather than against a hand-derived angle, because a sign error at a flank moves a card sideways instead of backwards and looks almost right. The flanks are also what the `SIDE_SEAT_TILT` tolerance is measured against: both have to answer alike, with room to spare before a seat genuinely across the table, or the tolerance starts deciding real seats.

#[allow(clippy::wildcard_imports)] // this module's own vocabulary
use super::*;

/// The owner's rule, measured rather than restated: every seat's bar is
/// written on the edge of its own battlefield nearest the middle of the
/// table, which is the edge *that seat* reads as above its board.
///
/// So the shelf is forward of all three lanes at every seat of every ring,
/// and in a duel the two bars face each other across the hearth instead of
/// sitting at the two ends of the screen. Table `+y` runs away from the
/// camera, so the local bar is now that seat's *furthest* ink and the
/// opponent's is theirs — which is one sentence said once, for the first
/// time since the bar had a rule of its own at the local seat.
#[test]
fn every_bar_is_written_between_its_own_board_and_the_hearth() {
    for n in 2..=8u8 {
        let layout = TableLayout::new(&seats(n), 1.78, None);
        for slot in &layout.slots {
            let forward = slot.forward();
            let shelf = slot.ledge_corners()[0].dot(forward);
            for lane in LaneKind::ALL {
                assert!(
                    shelf > slot.lane_center(lane).dot(forward),
                    "{n} seats, seat {:?}: the bar is behind its own {lane:?} lane",
                    slot.player
                );
            }
        }
    }

    // And the duel in particular, because that is the table the owner asked
    // about: the two bars are in the middle with the boards outside them,
    // the mirror image of the arrangement they replaced.
    let duel = TableLayout::new(&seats(2), 1.78, None);
    let local = duel.local().expect("a local seat");
    let across = &duel.slots[1];
    let shelf_y = |slot: &SeatSlot| slot.ledge_corners()[0].y;
    assert!(
        shelf_y(local) > local.lane_center(LaneKind::Lands).y,
        "the local bar is further from the camera than its own land row"
    );
    assert!(
        shelf_y(across) < across.lane_center(LaneKind::Lands).y,
        "the opponent's bar is nearer the camera than their own land row"
    );
    assert!(
        shelf_y(local) < shelf_y(across),
        "the two bars face each other across the hearth"
    );
}

/// Nothing is ever drawn where the bar is.
///
/// The band takes [`crate::tabletop::MAT_LEDGE`] out of the mat's depth and
/// [`SeatSlot::lane_center`] has to spend it at the same end. The two are
/// separate readings of [`LEDGE_IS_OUTER`], and they were at opposite ends of
/// the mat for as long as the local seat had a rule of its own — which cost
/// nothing only because the ink was not on either edge, but floating in the
/// gap past the rim. Measured against a whole card rather than a lane centre,
/// because a card is what would be standing on the writing.
#[test]
fn no_card_reaches_the_band_its_seat_writes_on() {
    for n in 2..=8u8 {
        let layout = TableLayout::new(&seats(n), 1.78, None);
        for slot in &layout.slots {
            let forward = slot.forward();
            let inner = (slot.center + forward * (slot.half_extent.y - crate::tabletop::MAT_LEDGE))
                .dot(forward);
            let front =
                slot.lane_center(LaneKind::Creatures).dot(forward) + STAGE_STEP + CARD_HEIGHT * 0.5;
            assert!(
                front <= inner + 1e-4,
                "{n} seats, seat {:?}: a creature reaches {front} and the band \
                 starts at {inner}",
                slot.player
            );
        }
    }
}

/// The two flanks of a table have to answer alike, and a seat across it
/// has to answer differently — with room to spare between the two, or the
/// tolerance that settles the flanks would start deciding real seats.
///
/// [`SIDE_SEAT_TILT`] had two readers and now has one: which edge carries
/// the bar stopped being a question the moment every seat answered it the
/// same way ([`LEDGE_IS_OUTER`]), and [`SeatSlot::camera_lies`] is what is
/// left. The margin below is measured for its sake, and it is the half of
/// this test that could not be written from the constant: a table whose
/// seats landed *in* the gap would be one where the tolerance, rather than
/// the geometry, decided where the camera was.
#[test]
fn nothing_at_any_table_sits_in_the_tolerance_the_flanks_need() {
    // Every case is run with a board being inspected too, because that is
    // the live call — `TableLayout::new(…, duel.focus)`, and `F` is a key a
    // player presses. A focus reweights the compartments and with them the
    // size of the ring, and a flank that drifted off the side of a ring
    // while somebody looked at an opponent would change its answer for as
    // long as they looked.
    let lookers = [None, Some(PlayerId::new(1)), Some(PlayerId::new(2))];
    for n in [4, 8] {
        for aspect in [1.4_f32, 1.78, 2.25] {
            for focus in lookers {
                let layout = TableLayout::new(&seats(n), aspect, focus);
                let flanks = layout
                    .slots
                    .iter()
                    .filter(|slot| slot.facing.cos().abs() < 0.5)
                    .count();
                assert_eq!(
                    flanks, 2,
                    "{n} seats at {aspect} with {focus:?} inspected: a table has \
                     two flanks"
                );
            }
        }
    }

    for n in 2..=8 {
        for aspect in [1.4_f32, 1.78, 2.25] {
            for focus in lookers {
                for slot in &TableLayout::new(&seats(n), aspect, focus).slots {
                    let lean = slot.facing.cos().abs();
                    assert!(
                        lean < SIDE_SEAT_TILT || lean > SIDE_SEAT_TILT * 4.0,
                        "{n} seats at {aspect} with {focus:?} inspected: seat \
                         {} leans {lean}, which is neither a flank nor plainly \
                         across the table",
                        slot.ring_index
                    );
                }
            }
        }
    }
}

/// Forward is the way the creatures are, at every seat of every ring.
///
/// Written as a comparison between two lanes and not against an angle,
/// because the sign of [`SeatSlot::forward`] is the thing that can be
/// wrong and a hand-derived angle would be derived the same wrong way.
/// A seat on a flank is the case that catches it: its `facing` is near a
/// quarter turn and a sign error there moves a card sideways rather than
/// backwards, which is the kind of wrong that looks almost right.
#[test]
fn the_creature_lane_is_forward_of_the_land_lane() {
    for n in 2..=8u8 {
        let layout = TableLayout::new(&seats(n), 1.78, None);
        for slot in &layout.slots {
            let creatures = slot.lane_center(LaneKind::Creatures);
            let lands = slot.lane_center(LaneKind::Lands);
            let advance = (creatures - lands).dot(slot.forward());
            assert!(
                advance > 0.0,
                "{n} seats, seat {:?}: the creature lane is {advance} forward of the \
                 land lane — forward points the wrong way",
                slot.player
            );
            // And forward is a direction, so it has unit length: the step
            // is a distance in table units at every seat or it is not a
            // distance at all.
            assert!((slot.forward().length() - 1.0).abs() < 1e-5);
        }
    }
}

/// A creature that steps out to fight stays on its own side of the felt.
///
/// The bound is the *other* seat's ground, and the reason it is measured
/// here rather than reasoned about is that the room in front of a
/// creature lane is different at every seat count and different again
/// with teams: a duel's two pods face each other across the shortest gap
/// there is, and a ring of eight has almost none of its neighbours in
/// front of it at all. The duel is the one that binds — 3.53 units of
/// room against a [`STAGE_STEP`] of 0.70 — and this is what would fail if
/// the step, the mat depth or the ring ever grew into each other.
#[test]
fn a_staged_creature_never_reaches_another_seats_ground() {
    for n in 2..=8u8 {
        for paired in [false, true] {
            let table: Vec<Seat> = seats(n)
                .into_iter()
                .enumerate()
                .map(|(i, player)| {
                    if paired {
                        Seat {
                            player,
                            team: Some(i as u8 / 2 + 1),
                        }
                    } else {
                        Seat::alone(player)
                    }
                })
                .collect();
            let layout = TableLayout::seated(&table, 1.78, None);
            for slot in &layout.slots {
                // The leading edge of a staged card, which is what
                // arrives first and is therefore what has to clear.
                let edge = slot.lane_center(LaneKind::Creatures)
                    + slot.forward() * (STAGE_STEP + CARD_HEIGHT * 0.5);
                for other in &layout.slots {
                    if other.player == slot.player {
                        continue;
                    }
                    let across = Vec2::new(other.facing.cos(), -other.facing.sin());
                    let depth = other.forward();
                    let d = edge - other.center;
                    let on_ground = d.dot(across).abs() <= other.half_extent.x
                        && d.dot(depth).abs() <= other.half_extent.y;
                    assert!(
                        !on_ground,
                        "{n} seats (paired {paired}): a staged creature of seat {:?} \
                         stands on seat {:?}'s ground",
                        slot.player, other.player
                    );
                }
            }
        }
    }
}

#[test]
fn lanes_stack_from_the_table_centre_towards_the_seat() {
    let layout = TableLayout::new(&seats(2), 1.78, None);
    let local = layout.local().expect("local");
    let creatures = local.lane_center(LaneKind::Creatures);
    let lands = local.lane_center(LaneKind::Lands);
    // For the near seat, "away from the seat" is +y, so creatures — drawn
    // towards the middle of the table — have the larger y.
    assert!(
        creatures.y > lands.y,
        "creatures {creatures:?} should sit closer to the table centre than lands {lands:?}"
    );

    // At every other seat too, which is the half this test used to miss.
    // Only the near seat was ever asked, and only in a duel — so a flank
    // seat kept its lands towards the middle and its creatures out behind
    // its own back, on every table of three or more, for as long as one
    // could be sat down. `facing` was right and the ring ran the other
    // way, and neither is visible from a seat that sits at angle zero.
    for n in 2..=8u8 {
        for aspect in [1.78_f32, 1.0] {
            for slot in &TableLayout::new(&seats(n), aspect, None).slots {
                let out = slot.center.length();
                let creatures = slot.lane_center(LaneKind::Creatures).length();
                let lands = slot.lane_center(LaneKind::Lands).length();
                assert!(
                    creatures < out && out < lands,
                    "{n} seats at {aspect}: seat {} has its creatures {creatures:.2} \
                     and its lands {lands:.2} from the middle, sitting at {out:.2}",
                    slot.ring_index
                );
            }
        }
    }
}

#[test]
fn reclaiming_command_space_preserves_framing_and_public_piles() {
    for n in 2..=8u8 {
        let mut layout = TableLayout::new(&seats(n), 1.78, None);
        let before = layout.clone();
        for slot in &mut layout.slots {
            let original = *slot;
            slot.reclaim_command_strip();
            assert!(slot.lane_width() > original.lane_width() + 1.0);
            for pile in [PileKind::Library, PileKind::Graveyard, PileKind::Exile] {
                assert!(slot.pile_center(pile).distance(original.pile_center(pile)) < 1e-4);
            }
            let once = *slot;
            slot.reclaim_command_strip();
            assert_eq!(*slot, once);
        }
        for (a, b) in layout.corners(0.0).iter().zip(before.corners(0.0)) {
            assert!(a.distance(b) < 1e-4);
        }
    }
}
