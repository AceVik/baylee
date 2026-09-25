//! Who shares a side of the ring, and what shape that asks the ring to be. Allies sit shoulder to shoulder along one side and face the same way, because across the table a partner's board is drawn upside down; a table of pairs and lone players still hands every *seat* the same board rather than every side; and a format that gives everybody a team of their own is laid out as the free-for-all it is. The round table is offered at three and priced there: three seats alone take a circle, where an ellipse would draw them in the gable a 2v1 makes, unless the canvas is too narrow to pay `ROUND_COST`; four seats and up stay shaped to the canvas, because a five-seat circle puts a seat between a flank and across the table. The prices are re-derived here by walking a ring outwards rather than by asking `TableLayout::seated`'s own search, which would agree with it however wrong both were.

#[allow(clippy::wildcard_imports)] // this module's own vocabulary
use super::*;

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
                slot.lane_width() >= standard_board(order.len(), 1.78) - 1e-2,
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

        // This also said a table of two sides was a smaller table than one
        // of four, and it was while a seat was twelve units wide. Since
        // every seat is handed a duel's board (#264), a side of two is two
        // duel boards shoulder to shoulder, and at 1.78 it costs the camera
        // 33.3 units of reach against the free-for-all's 27.4. That is the
        // owner's trade and not a ring nobody sits on: the table is still
        // the shape of the canvas, which is what the bound above says.
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
                    at_ceiling
                        || at_channel
                        || slot.lane_width() >= standard_board(order.len(), aspect) - 1e-2,
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
                (slot.lane_width() - standard_board(3, aspect)).abs() < 1e-2,
                "aspect {aspect:.2}: seat {i} plays on {:.2} units, and the \
                 standard board is {:.2}",
                slot.lane_width(),
                standard_board(3, aspect)
            );
        }
        // Taken because it is affordable, and the bound is the one the
        // constant names.
        let standard = standard_board(3, aspect);
        let shaped = tightest(3, aspect, standard, |ry| {
            ring_for(ry, aspect, POD_DEPTH * 0.5, 1.0)
        })
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
    // Four seats and up keep the ring shaped to the canvas, as they always
    // have. It used to be the price that refused them — a circle cost more
    // than [`ROUND_COST`] of the shaped ring, or ran past the ceiling before
    // it had handed anybody a board — and when the ceiling rose to hand
    // every seat a duel's board (#264), a five-seat circle came in under the
    // price, at 1.4 and on a 1024 × 768 window. Five on a circle puts a seat
    // at 72°, and a lean of 0.31 is neither a flank nor across the table:
    // the gap `facing::nothing_at_any_table_sits_in_the_tolerance_the_flanks_need`
    // keeps clear, and the test that caught it. So the circle is offered at
    // three, the one table it was argued for, and #264 changed how wide the
    // seats are and not where they sit.
    for n in [4u8, 5, 6, 7, 8] {
        for aspect in [2.014_f32, 16.0 / 9.0, 1.4, 1.0] {
            let layout = TableLayout::new(&seats(n), aspect, None);
            assert!(
                (layout.radius.x - layout.radius.y).abs() > 1e-3,
                "{n} seats at {aspect:.2}: the ring came out round, at {:?}",
                layout.radius
            );
        }
    }
}

#[test]
fn a_narrow_canvas_cannot_afford_a_round_table() {
    // At three the rule is a price and not a seat count, so it answers
    // differently on a canvas taller than it is wide — where a circle is the one
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
        let (radius, cost) = tightest(3, aspect, standard_board(3, aspect), Vec2::splat)
            .expect("a circle seats three at any aspect");
        assert!(
            cost > shaped * ROUND_COST,
            "aspect {aspect:.2}: a circle of {:.2} would have cost {cost:.1} \
             units of reach against {shaped:.1}, which is inside \
             {ROUND_COST} — it should have been taken",
            radius.x
        );
    }
}
