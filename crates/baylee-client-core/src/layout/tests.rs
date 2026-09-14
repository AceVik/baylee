use super::*;

fn seats(n: u8) -> Vec<PlayerId> {
    (0..n).map(PlayerId::new).collect()
}

#[test]
fn a_pile_stands_beside_the_ground_and_never_on_it() {
    for n in [2, 3, 4, 6, 8] {
        let layout = TableLayout::new(&seats(n), 2.0, None);
        for slot in &layout.slots {
            let side = Vec2::new(slot.facing.cos(), -slot.facing.sin());
            for pile in PileKind::ALL {
                let across = (slot.pile_center(pile) - slot.center).dot(side);
                let near_edge = across.abs() - CARD_WIDTH * 0.5;
                assert!(
                    near_edge > slot.half_extent.x,
                    "{n} seats: the near edge of the {} is {near_edge} out from \
                     the middle of a mat {} wide — it is lying on the board",
                    pile.label(),
                    slot.half_extent.x
                );
                assert!(
                    (across.signum() - pile.side()).abs() < 1e-6,
                    "{n} seats: the {} came out on the seat's other hand",
                    pile.label()
                );
            }
        }
    }
}

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

#[test]
fn the_four_piles_are_four_places() {
    let layout = TableLayout::new(&seats(2), 2.0, None);
    let slot = layout.local().expect("a local seat");
    for (i, a) in PileKind::ALL.iter().enumerate() {
        for b in &PileKind::ALL[i + 1..] {
            let gap = slot.pile_center(*a).distance(slot.pile_center(*b));
            assert!(
                gap > CARD_HEIGHT,
                "the {} and the {} are {gap} apart, and a card is {CARD_HEIGHT} \
                 long — they would be stacked on each other",
                a.label(),
                b.label()
            );
        }
    }
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

#[test]
fn no_two_seats_piles_stand_on_each_other() {
    for n in [3, 4, 5, 6, 7, 8] {
        let layout = TableLayout::new(&seats(n), 2.0, None);
        for (i, a) in layout.slots.iter().enumerate() {
            for b in &layout.slots[i + 1..] {
                for pa in PileKind::ALL {
                    for pb in PileKind::ALL {
                        assert!(
                            !quads_overlap(pile_corners(a, pa), pile_corners(b, pb)),
                            "{n} seats: seat {:?}'s {} lies on top of seat {:?}'s {}",
                            a.player,
                            pa.label(),
                            b.player,
                            pb.label()
                        );
                    }
                }
            }
        }
    }
}

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
fn a_comfortable_lane_does_not_fan() {
    let packing = pack_lane(4, 20.0);
    assert!(!packing.fanned);
    assert!(!packing.overflowing);
    assert_eq!(packing.offsets.len(), 4);
    assert!((packing.pitch - (CARD_SPAN + CARD_GAP)).abs() < 1e-5);
}

/// The overlaps the owner saw, and the reason they made no sense: the
/// lane had room to spare.
///
/// A card taps by turning a quarter of the way round, so it claims
/// [`CARD_SPAN`] of the row and not [`CARD_WIDTH`]. The lane packed to
/// the narrower of the two, so a tapped land or an attacking creature sat
/// 0.14 units inside each of its neighbours — on a duel's lane nearly
/// twenty units wide holding six cards. Tokens only made it louder: more
/// cards, tighter pitch, the same fault.
#[test]
fn a_row_of_tapped_cards_does_not_overlap_itself() {
    // A duel's own lane is about twenty units across; a four-player pod's
    // is about six. Both, and a deliberately crowded one below them.
    for width in [19.7f32, 6.1, 3.0] {
        for count in 2..=12usize {
            let packing = pack_lane(count, width);
            let step = packing.offsets[1] - packing.offsets[0];
            assert!(
                (step - packing.pitch).abs() < 1e-4,
                "the reported pitch is not the step taken"
            );
            if packing.fanned {
                // A fan is overlap on purpose, and the pitch is already
                // held above the legibility floor by the case below.
                continue;
            }
            // `CARD_HEIGHT`, not `CARD_SPAN`: the width a tapped card
            // really occupies is the card's long side, and a test that
            // measured against the constant the packing is written in
            // would agree with it however wrong both were. The first
            // draft of this did exactly that and passed against the code
            // it was written to fail.
            assert!(
                step >= CARD_HEIGHT,
                "{count} cards in {width} units: a lane with room to \
                 spare still overlapped when they tapped ({step} apart, \
                 a tapped card being {CARD_HEIGHT} wide)"
            );
        }
    }
}

#[test]
fn a_lane_is_always_centred_on_zero() {
    for count in [1usize, 2, 5, 12, 40] {
        let packing = pack_lane(count, 12.0);
        let sum: f32 = packing.offsets.iter().sum();
        assert!(sum.abs() < 1e-3, "lane of {count} is off-centre by {sum}");
    }
}

#[test]
fn offsets_are_strictly_increasing() {
    let packing = pack_lane(15, 10.0);
    for w in packing.offsets.windows(2) {
        assert!(w[1] > w[0]);
    }
}

#[test]
fn a_crowded_lane_fans_instead_of_shrinking_cards() {
    let packing = pack_lane(15, 10.0);
    assert!(packing.fanned);
    assert!(packing.pitch < CARD_WIDTH, "cards must overlap");
    // Still legible: the fan never hides more than the policy allows.
    assert!(packing.pitch >= CARD_WIDTH * MIN_VISIBLE_FRACTION);
}

#[test]
fn an_unfittable_lane_reports_overflow_so_the_caller_can_group() {
    // Sixty tokens in a narrow opponent pod cannot be fanned legibly.
    let packing = pack_lane(60, 6.0);
    assert!(packing.overflowing);
    assert!(packing.fanned);
    // The pitch is clamped, so the row deliberately runs wider than the
    // pod: the board model is expected to collapse the row instead.
    assert!(packing.pitch >= CARD_WIDTH * MIN_VISIBLE_FRACTION);
}

#[test]
fn empty_and_single_lanes_are_degenerate_but_valid() {
    assert!(pack_lane(0, 10.0).offsets.is_empty());
    assert_eq!(pack_lane(1, 10.0).offsets, vec![0.0]);
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

#[test]
fn a_duel_comes_out_the_shape_of_its_canvas() {
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
            (got - aspect).abs() < 0.05,
            "canvas {aspect}: the table came out {got} ({span:?})"
        );
    }
}

/// The shape of the space the duel HUD leaves on a laptop window — what
/// the layout is actually built against, and nothing like the window's.
const HUD_ASPECT: f32 = 2.01;

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

/// Nobody plays on anybody else's table.
///
/// The bound the layout actually applies is on the distance between two
/// seats' *centres*, which is not the same question: a mat is a rectangle
/// turned to face its own seat, and two rectangles at a given distance
/// may or may not meet depending on how they are turned. This asks the
/// real question of every pair, at every seat count, on three shapes of
/// canvas.
#[test]
fn a_team_sits_along_one_side_of_the_table() {
    // Two-headed giant, seated both ways round: partners next to each
    // other in turn order, and partners alternating with the opposition.
    // Either way a team is one side of the table — a partner's board is
    // read as often as one's own, and across the table it was upside
    // down.
    for order in [[1u8, 1, 2, 2], [1, 2, 1, 2]] {
        let table: Vec<Seat> = order
            .iter()
            .enumerate()
            .map(|(i, &t)| Seat::on(PlayerId::new(i as u8), Some(t)))
            .collect();
        let layout = TableLayout::seated(&table, 1.78, None);
        assert_eq!(layout.slots.len(), 4);

        for (i, slot) in layout.slots.iter().enumerate() {
            let ally = layout
                .slots
                .iter()
                .enumerate()
                .find(|(j, _)| *j != i && order[*j] == order[i])
                .expect("a partner")
                .1;
            assert!(
                (slot.facing - ally.facing).abs() < 1e-4,
                "{order:?}: seat {i} faces {} and its partner {}",
                slot.facing,
                ally.facing
            );
            // Shoulder to shoulder: one step apart along their own side,
            // and neither of them any nearer the middle than the other.
            let step = (ally.center - slot.center).length();
            let footprint = (slot.half_extent.x + PILE_STRIP) * 2.0;
            assert!(
                (footprint..footprint * 1.25).contains(&step),
                "{order:?}: partners stand {step} apart, against a {footprint} board"
            );
            assert!(
                (slot.center.length() - ally.center.length()).abs() < 1e-3,
                "{order:?}: one partner sits further out than the other"
            );
            assert!(
                slot.lane_width() >= MIN_POD_WIDTH - 1e-2,
                "{order:?}: seat {i} plays on {}",
                slot.lane_width()
            );
        }

        for i in 0..layout.slots.len() {
            for j in (i + 1)..layout.slots.len() {
                assert!(
                    !grounds_overlap(&layout.slots[i], &layout.slots[j]),
                    "{order:?}: seats {i} and {j} overlap"
                );
            }
        }

        // And it is still the shape of the canvas. A side of two reaches
        // twice as far along itself as a side of one, and the ring was
        // shaped as though it did not: the table came out exactly twice
        // as wide as it asked to be, the camera fitted it by width, and
        // all four boards sat in the top half of the window with bare
        // felt under them.
        let (min, max) = layout.extent().expect("a seated table has an extent");
        let span = max - min;
        assert!(
            (span.x / span.y - 1.78).abs() < 0.15,
            "{order:?}: the table came out {} ({span:?})",
            span.x / span.y
        );

        // And the whole point of it: a table of two sides is a smaller
        // table than one of four, so the camera comes in rather than
        // pulling back to frame a ring nobody is sitting on.
        let apart = TableLayout::new(&seats(4), 1.78, None);
        assert!(
            layout.radius.x < apart.radius.x && layout.radius.y < apart.radius.y,
            "{order:?}: two sides want a ring of {:?}, four wanted {:?}",
            layout.radius,
            apart.radius
        );
    }
}

/// A table where some seats are partnered and some are not.
///
/// The gateway arranges any of these — `--teams 1,1,2` is a two-on-one,
/// `1,1,2,2,0` is two pairs and a player on their own — and a side of two
/// reaches twice as far along itself as a side of one. The board is still
/// one board: the rule is that every *seat* gets the same one, not every
/// side, so a lone player's mat is exactly as wide as each half of the
/// pair across from them. Anything else and the table tells a player
/// their board is the smaller one before the game has started.
#[test]
fn a_mixed_table_still_hands_out_one_board() {
    for order in [
        vec![Some(1u8), Some(1), Some(2)],
        vec![Some(1u8), Some(1), None, None],
        vec![Some(1u8), Some(1), Some(2), Some(2), None],
        vec![Some(1u8), Some(2), Some(1), None, Some(2), None],
    ] {
        let table: Vec<Seat> = order
            .iter()
            .enumerate()
            .map(|(i, &t)| Seat::on(PlayerId::new(i as u8), t))
            .collect();
        for aspect in [1.78_f32, 1.0] {
            let layout = TableLayout::seated(&table, aspect, None);
            assert_eq!(layout.slots.len(), order.len());
            // The same two escapes as `every_seat_gets_the_same_board`:
            // a ring that has stopped growing, and mats already against
            // the centre channel with nothing to be crowded by.
            let at_ceiling =
                layout.radius.x >= MAX_RING_X - 1e-3 || layout.radius.y >= MAX_RING_Y - 1e-3;
            let at_channel = layout
                .slots
                .iter()
                .map(|slot| slot.center.length() - slot.half_extent.y)
                .fold(f32::INFINITY, f32::min)
                <= CENTRE_GAP * 0.5 + 1e-3;
            for slot in &layout.slots {
                assert!(
                    (slot.half_extent - layout.slots[0].half_extent).length() < 1e-3,
                    "{order:?} at {aspect}: seat {} plays on {:?} against the local \
                     seat's {:?}",
                    slot.ring_index,
                    slot.half_extent,
                    layout.slots[0].half_extent
                );
                assert!(
                    at_ceiling || at_channel || slot.lane_width() >= MIN_POD_WIDTH - 1e-2,
                    "{order:?} at {aspect}: a seat plays on {}",
                    slot.lane_width()
                );
            }
            // Partners still share a side, and a lone seat still has one
            // to itself.
            for (i, slot) in layout.slots.iter().enumerate() {
                for (j, other) in layout.slots.iter().enumerate().skip(i + 1) {
                    let together = order[i].is_some() && order[i] == order[j];
                    let same_side = (slot.facing - other.facing).abs() < 1e-4;
                    assert_eq!(
                        together, same_side,
                        "{order:?} at {aspect}: seats {i} and {j} face {} and {}",
                        slot.facing, other.facing
                    );
                    assert!(
                        !grounds_overlap(slot, other),
                        "{order:?} at {aspect}: seats {i} and {j} overlap"
                    );
                }
            }
        }
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

#[test]
fn three_seats_playing_for_themselves_sit_round_the_table() {
    // A ring shaped to the canvas seats two and four where anybody would
    // sit — opposite each other, or on the four points of a diamond — and
    // three in a gable: the two opponents at 150° and 210°, side by side
    // across the top with their inner corners nearly touching, which is
    // the silhouette a 2v1 draws. Three seats each playing for themselves
    // are a circle, and only a circle says so.
    for aspect in [2.014_f32, 16.0 / 9.0, 1.6, 1.0] {
        let layout = TableLayout::new(&seats(3), aspect, None);
        assert!(
            (layout.radius.x - layout.radius.y).abs() < 1e-3,
            "aspect {aspect:.2}: the ring came out {:?}, which is not round",
            layout.radius
        );
        for (i, slot) in layout.slots.iter().enumerate() {
            let want = core::f32::consts::TAU * i as f32 / 3.0;
            assert!(
                (slot.facing - want).abs() < 0.01,
                "aspect {aspect:.2}: seat {i} faces {:.1}°, not the {:.1}° \
                 that would put it a third of the way round",
                slot.facing.to_degrees(),
                want.to_degrees()
            );
            assert!(
                (slot.lane_width() - MIN_POD_WIDTH).abs() < 1e-2,
                "aspect {aspect:.2}: seat {i} plays on {:.2} units, and the \
                 standard board is {MIN_POD_WIDTH}",
                slot.lane_width()
            );
        }
        // Taken because it is affordable, and the bound is the one the
        // constant names.
        let shaped = tightest(3, aspect, |ry| ring_for(ry, aspect, POD_DEPTH * 0.5, 1.0))
            .expect("three seats fit on a ring shaped to the canvas");
        assert!(
            reach_of_layout(&layout, aspect) <= shaped.1 * ROUND_COST,
            "aspect {aspect:.2}: sitting round costs {:.1} units of reach \
             against {:.1} shaped to the canvas, which is past {ROUND_COST}",
            reach_of_layout(&layout, aspect),
            shaped.1
        );
    }
}

#[test]
fn a_table_of_teams_of_one_is_a_free_for_all() {
    // A format that hands every seat a team of its own is not a format
    // with teams in it, and must not be laid out as one.
    for aspect in [2.014_f32, 1.6] {
        let alone = TableLayout::new(&seats(3), aspect, None);
        let labelled = TableLayout::seated(
            &[
                Seat::on(PlayerId::new(0), Some(1)),
                Seat::on(PlayerId::new(1), Some(2)),
                Seat::on(PlayerId::new(2), Some(3)),
            ],
            aspect,
            None,
        );
        assert!(
            (alone.radius - labelled.radius).abs().max_element() < 1e-3,
            "aspect {aspect:.2}: three teams of one came out on {:?}, three \
             seats alone on {:?}",
            labelled.radius,
            alone.radius
        );
    }
}

#[test]
fn a_bigger_free_for_all_stays_shaped_to_the_canvas() {
    // And it is refused for a reason, not by accident: at four seats and
    // up a round table costs more than [`ROUND_COST`] of what the same
    // seats cost on a ring shaped to the canvas — or it runs past the
    // ring's own ceiling before it has handed anybody a board.
    for n in [4u8, 5, 6, 8] {
        for aspect in [2.014_f32, 16.0 / 9.0, 1.0] {
            let layout = TableLayout::new(&seats(n), aspect, None);
            assert!(
                (layout.radius.x - layout.radius.y).abs() > 1e-3,
                "{n} seats at {aspect:.2}: the ring came out round, at {:?}",
                layout.radius
            );
            let shaped = reach_of_layout(&layout, aspect);
            if let Some((radius, cost)) = tightest(n as usize, aspect, Vec2::splat) {
                assert!(
                    cost > shaped * ROUND_COST,
                    "{n} seats at {aspect:.2}: a circle of {:.2} would have \
                     cost {cost:.1} units of reach against {shaped:.1}, which \
                     is inside {ROUND_COST} — it should have been taken",
                    radius.x
                );
            }
        }
    }
}

#[test]
fn a_narrow_canvas_cannot_afford_a_round_table() {
    // The rule is a price and not a seat count, so it answers differently
    // on a canvas taller than it is wide — where a circle is the one
    // shape the camera cannot pay for. A phone held upright is 0.46, the
    // narrowest frame `Metrics::of` draws and the clamp in
    // [`TableLayout::seated`]: a circle wide enough to seat three would
    // cost 56 units of reach against the 22 the table is laid out on. It
    // stays refused all the way up to a canvas nearly square, and three
    // seats on a phone go on sitting where they fit rather than where
    // they would like to.
    for aspect in [0.46_f32, 0.6, 0.75] {
        let layout = TableLayout::new(&seats(3), aspect, None);
        assert!(
            (layout.radius.x - layout.radius.y).abs() > 1e-3,
            "aspect {aspect:.2}: the ring came out round, at {:?}",
            layout.radius
        );
        let shaped = reach_of_layout(&layout, aspect);
        let (radius, cost) =
            tightest(3, aspect, Vec2::splat).expect("a circle seats three at any aspect");
        assert!(
            cost > shaped * ROUND_COST,
            "aspect {aspect:.2}: a circle of {:.2} would have cost {cost:.1} \
             units of reach against {shaped:.1}, which is inside \
             {ROUND_COST} — it should have been taken",
            radius.x
        );
    }
}
