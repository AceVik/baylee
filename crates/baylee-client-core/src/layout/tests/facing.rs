//! What a seat's `facing` decides once it has a place: that forward is the way the creatures are, that the three lanes stack from the middle of the table back towards the seat, that a creature staged by `STAGE_STEP` never reaches the ground in front of it, and which of the mat's two long edges carries the bar — the near one at the local seat, the outer one everywhere else. Written as comparisons between two lanes or two seats rather than against a hand-derived angle, because a sign error at a flank moves a card sideways instead of backwards and looks almost right. The flanks are also what the `SIDE_SEAT_TILT` tolerance is measured against: both have to answer alike, with room to spare before a seat genuinely across the table, or the tolerance starts deciding real seats.

#[allow(clippy::wildcard_imports)] // this module's own vocabulary
use super::*;

/// The owner's rule, measured rather than restated: in a duel both bars
/// are at the ends of the screen and the boards are between them. The
/// local seat's shelf is the *nearest* ink it has and its opponent's is
/// the furthest, which is what "immer unten" means once there are two
/// seats facing each other.
///
/// Table `+y` runs away from the camera, so "nearer" is smaller `y`.
#[test]
fn the_local_bar_is_the_nearest_ink_at_its_own_seat() {
    let layout = TableLayout::new(&seats(2), 1.78, None);
    let local = layout.local().expect("a local seat");
    let across = &layout.slots[1];

    let shelf_y = |slot: &SeatSlot| slot.ledge_corners()[0].y;
    for lane in LaneKind::ALL {
        assert!(
            shelf_y(local) < local.lane_center(lane).y,
            "the local bar is nearer the camera than its own {lane:?} lane"
        );
        assert!(
            shelf_y(across) > across.lane_center(lane).y,
            "the opponent's bar stays above its own {lane:?} lane"
        );
    }
    // And the boards are between the two bars, not stacked against one
    // of them: the whole point of moving the local shelf is that it took
    // the near strip with it.
    assert!(
        shelf_y(local) < across.lane_center(LaneKind::ALL[0]).y,
        "the two shelves are at opposite ends of the table"
    );
}

/// The two flanks of a table have to answer alike, and a seat across it
/// has to answer differently — with room to spare between the two, or
/// the tolerance that settles the flanks would start deciding real
/// seats.
#[test]
fn the_two_flanks_of_a_table_put_their_bars_on_the_same_edge() {
    let table = TableLayout::new(&seats(4), 1.78, None);
    let local = table.local().expect("a local seat");
    assert!(
        local.ledge_is_outer(),
        "the seat the camera sits behind reads its own bar at the bottom \
         of the screen, on the near edge of its own mat"
    );

    for slot in &TableLayout::new(&seats(3), 1.78, None).slots[1..] {
        assert!(
            slot.ledge_is_outer(),
            "an opponent in a three-way is across the table and its board \
             is drawn upside-down from here, so its bar belongs on the \
             outer edge; cos is {}",
            slot.facing.cos()
        );
    }

    // Every case below is also run with a board being inspected, because
    // that is the live call — `TableLayout::new(…, duel.focus)`, and `F`
    // is a key a player presses. A focus reweights the compartments and
    // with them the size of the ring, and a flank that drifted off the
    // side of a ring while somebody looked at an opponent would move its
    // bar to the other edge of its mat for as long as they looked.
    let lookers = [None, Some(PlayerId::new(1)), Some(PlayerId::new(2))];
    for n in [4, 8] {
        for aspect in [1.4_f32, 1.78, 2.25] {
            for focus in lookers {
                let layout = TableLayout::new(&seats(n), aspect, focus);
                let flanks: Vec<&SeatSlot> = layout
                    .slots
                    .iter()
                    .filter(|slot| slot.facing.cos().abs() < 0.5)
                    .collect();
                assert_eq!(
                    flanks.len(),
                    2,
                    "{n} seats at {aspect} with {focus:?} inspected: a table \
                     has two flanks"
                );
                for slot in &flanks {
                    assert!(
                        !slot.ledge_is_outer(),
                        "{n} seats at {aspect} with {focus:?} inspected: a \
                         flank has no 'above' and keeps the edge nearer the \
                         hearth; cos is {}",
                        slot.facing.cos()
                    );
                }
            }
        }
    }

    // And the margin the tolerance is chosen against: nothing at any
    // table sits in the gap between "a flank" and "across from here".
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
