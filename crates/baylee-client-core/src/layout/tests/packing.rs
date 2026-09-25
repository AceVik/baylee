//! `pack_lane` on its own, with no table around it: the pitch and offsets it returns and the two thresholds it reports. A comfortable row keeps `CARD_SPAN + CARD_GAP`, a crowded one fans rather than shrinking its cards and stays above `MIN_VISIBLE_FRACTION`, an unfittable one clamps the pitch and sets `overflowing` so the caller groups instead, and a row of any count is centred on zero with strictly increasing offsets. The overlap measurement is taken against `CARD_HEIGHT` and never against the constant the packing is written in — a draft that did agreed with the code however wrong both were. A lane width taken off a real seat belongs with the part that solves the ring.

#[allow(clippy::wildcard_imports)] // this module's own vocabulary
use super::*;

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
    // The pitch is clamped, so the row runs wider than the pod, and it
    // scrolls: only the window's run of it is drawn.
    assert!(packing.pitch >= CARD_WIDTH * MIN_VISIBLE_FRACTION);
}

/// Beside a merged card its count badge hangs off its right edge, so the gap
/// after it is a full cell (the owner, 25.09): the badge lies on no
/// neighbour and under none, tapped or not, while the gap before it fans
/// with the rest of the row as tightly as it needs to.
#[test]
fn the_gap_after_a_merged_card_is_held_and_the_rest_fan() {
    let mut held = vec![false; 14];
    held[5] = true;
    let packing = pack_row(&held, 10.0);
    assert!(packing.fanned && !packing.overflowing);
    let steps: Vec<f32> = packing.offsets.windows(2).map(|w| w[1] - w[0]).collect();
    for (i, step) in steps.iter().enumerate() {
        if i == 5 {
            assert!(
                (step - HELD_PITCH).abs() < 1e-5,
                "the gap after the merged card is {step}"
            );
        } else {
            assert!(*step < CARD_WIDTH, "gap {i} does not fan: {step}");
        }
    }
    // The room is what a tapped card after it and the badge need: a
    // constant assertion under `HELD_PITCH` says so.
}

/// A duel's rows leave the felt above their cards for the badge to stand
/// over its card, clear of the row behind; a ring's rows do not, so there
/// it stands beside the card (the owner, 25.09).
#[test]
fn a_duel_stands_its_badges_over_the_cards_and_a_ring_beside_them() {
    use crate::cardplate::{BADGE_RISE, BadgePlace};
    for seats in 2u8..=8 {
        for aspect in [0.45_f32, 1.0, 16.0 / 9.0, 21.0 / 9.0] {
            let players: Vec<PlayerId> = (0..seats).map(PlayerId::new).collect();
            let layout = TableLayout::new(&players, aspect, None);
            for slot in &layout.slots {
                let place = slot.badge_place();
                if place == BadgePlace::Above {
                    assert!(
                        BADGE_RISE <= (slot.lane_height() - CARD_HEIGHT) * 0.5,
                        "{seats} seats at {aspect}: the badge reaches the next row"
                    );
                }
                if seats == 2 && aspect > 1.0 {
                    assert_eq!(place, BadgePlace::Above, "a duel at {aspect}");
                }
                if seats > 2 {
                    assert_eq!(place, BadgePlace::Beside, "{seats} seats at {aspect}");
                }
            }
        }
    }
}

/// A row that cannot hold its cells and fan legibly scrolls: from every
/// first card, the window shows a run of whole cards that stands inside the
/// lane, the run reaches the row's end from `last_first`, and every card is
/// shown by some window. A row that fits shows every card where it packed.
#[test]
fn a_scrolled_row_shows_whole_cards_inside_its_lane_and_every_card_in_some_window() {
    let mut scrolled = 0;
    for width in [3.0f32, 6.1, 12.0, 19.7] {
        for count in [2usize, 5, 13, 40, 70] {
            for pattern in 0..4usize {
                let held: Vec<bool> = (0..count)
                    .map(|i| match pattern {
                        0 => false,
                        1 => true,
                        2 => i % 3 == 1,
                        _ => i % 7 == 0,
                    })
                    .collect();
                let packing = pack_row(&held, width);
                let usable = width.max(CARD_SPAN);
                let mut seen = vec![false; count];
                for first in 0..count + 2 {
                    let window = packing.window(first);
                    assert!(
                        !window.shown.is_empty(),
                        "{count} in {width}: nothing shown"
                    );
                    for i in window.shown.clone() {
                        seen[i] = true;
                        let at = packing.offsets[i] + window.shift;
                        assert!(
                            at - CARD_SPAN * 0.5 >= -usable * 0.5 - 1e-3
                                && at + CARD_SPAN * 0.5 <= usable * 0.5 + 1e-3,
                            "{count} in {width}, pattern {pattern}, from {first}: card {i} at {at} leaves the lane"
                        );
                    }
                }
                assert!(
                    seen.iter().all(|&s| s),
                    "{count} in {width}: a card no window shows"
                );
                if packing.overflowing {
                    scrolled += 1;
                    assert_eq!(packing.window(packing.last_first()).shown.end, count);
                    assert!(packing.window(0).shown.len() < count);
                } else {
                    assert_eq!(packing.window(3).shown, 0..count);
                }
            }
        }
    }
    assert!(scrolled >= 10, "only {scrolled} rows scrolled");
}

/// Bringing a card into view moves the window as little as it can: not at
/// all for a card that is shown, back to it for one before the window, and
/// just far enough for one after.
#[test]
fn a_card_is_revealed_by_the_smallest_move() {
    let packing = pack_row(&[true; 30], 8.0);
    assert!(packing.overflowing);
    let window = packing.window(4);
    let inside = window.shown.start + 1;
    assert_eq!(packing.reveal(4, inside), window.shown.start);
    assert_eq!(packing.reveal(4, 1), 1);
    let after = window.shown.end + 2;
    let moved = packing.reveal(4, after);
    assert!(packing.window(moved).shown.contains(&after));
    assert!(
        !packing.window(moved - 1).shown.contains(&after),
        "moved further than needed"
    );
}

/// The felt under each row answers to that row, the combat step in front of
/// the creatures to theirs, and the felt beside the lane to none.
#[test]
fn the_felt_under_a_row_is_that_rows() {
    let layout = TableLayout::new(&[PlayerId::new(0), PlayerId::new(1)], 16.0 / 9.0, None);
    for slot in &layout.slots {
        for lane in LaneKind::ALL {
            assert_eq!(slot.lane_at(slot.lane_center(lane)), Some(lane));
        }
        let step = slot.lane_center(LaneKind::Creatures)
            + slot.forward() * (slot.lane_height() * 0.5 + STAGE_STEP * 0.5);
        assert_eq!(slot.lane_at(step), Some(LaneKind::Creatures));
        let along = Vec2::new(slot.facing.cos(), -slot.facing.sin());
        let beside = slot.lane_center(LaneKind::Lands) + along * (slot.half_extent.x + 0.2);
        assert_eq!(slot.lane_at(beside), None);
    }
}

#[test]
fn empty_and_single_lanes_are_degenerate_but_valid() {
    assert!(pack_lane(0, 10.0).offsets.is_empty());
    assert_eq!(pack_lane(1, 10.0).offsets, vec![0.0]);
}

/// Sections stand apart (#263): the last card of a section lies whole, with
/// [`SECTION_AIR`] after it on a roomy row. As the row fills, the air closes
/// in step with the fan to [`SECTION_AIR_MIN`] and no further, and it never
/// grows while the lane narrows.
#[test]
fn a_section_keeps_its_last_card_whole_and_air_after_it() {
    let mut after = vec![Gap::Free; 24];
    after[7] = Gap::Section;
    after[15] = Gap::Section;
    let reach = vec![0.0; after.len()];
    let mut last_air = f32::INFINITY;
    let mut fanned = 0;
    for tenth in (40..=400).rev() {
        #[allow(clippy::cast_precision_loss)] // a few hundred widths
        let width = tenth as f32 * 0.1;
        let packing = pack_gaps(&after, &reach, width);
        for boundary in [7, 15] {
            let air = packing.offsets[boundary + 1] - packing.offsets[boundary] - CARD_SPAN;
            assert!(
                (SECTION_AIR_MIN - 1e-4..=SECTION_AIR + 1e-4).contains(&air),
                "{width}: the air after a section is {air}"
            );
            if !packing.fanned {
                assert!(
                    (air - SECTION_AIR).abs() < 1e-4,
                    "{width}: a roomy row's air is {air}"
                );
            }
            if packing.overflowing {
                assert!(
                    (air - SECTION_AIR_MIN).abs() < 1e-4,
                    "{width}: a full row's air is {air}"
                );
            }
            let window = packing.window(0);
            assert!(
                !packing.covered(boundary, &window),
                "{width}: the last card of a section is covered"
            );
        }
        let air = packing.offsets[8] - packing.offsets[7] - CARD_SPAN;
        assert!(
            air <= last_air + 1e-4,
            "{width}: the air grew as the lane narrowed"
        );
        last_air = air;
        fanned += usize::from(packing.fanned && !packing.overflowing);
    }
    assert!(fanned > 10, "only {fanned} widths fanned");
}

/// A pile's cards step out on its left (the owner, 25.09), and the row holds
/// that room as its own: the card before a pile shows as much of itself as
/// the fan gives every card, a row that fits stays inside its lane with the
/// first pile's cards too, and so does every window of a row that scrolls.
#[test]
fn a_pile_holds_its_room_on_the_left() {
    for width in [3.0f32, 6.1, 12.0, 19.7] {
        for count in [2usize, 5, 13, 40] {
            let after = vec![Gap::Free; count];
            let reach: Vec<f32> = (0..count)
                .map(|i| if i % 3 == 0 { pile_reach(9) } else { 0.0 })
                .collect();
            let packing = pack_gaps(&after, &reach, width);
            let usable = width.max(CARD_SPAN);
            for (i, (pair, reach)) in packing.offsets.windows(2).zip(&reach[1..]).enumerate() {
                let shows = pair[1] - reach - pair[0];
                assert!(
                    shows >= CARD_WIDTH * MIN_VISIBLE_FRACTION - 1e-4,
                    "{count} in {width}: card {i} shows {shows} beside the pile after it"
                );
            }
            for first in 0..count {
                let window = packing.window(first);
                let start = window.shown.start;
                let end = window.shown.end - 1;
                let left = packing.offsets[start] + window.shift - reach[start] - CARD_SPAN * 0.5;
                let right = packing.offsets[end] + window.shift + CARD_SPAN * 0.5;
                assert!(
                    left >= -usable * 0.5 - 1e-3 && right <= usable * 0.5 + 1e-3,
                    "{count} in {width}, from {first}: the run spans {left}..{right}"
                );
            }
            if !packing.overflowing {
                let left = packing.offsets[0] - reach[0] - CARD_SPAN * 0.5;
                let right = packing.offsets[count - 1] + CARD_SPAN * 0.5;
                assert!(
                    (left + right).abs() < 1e-3,
                    "{count} in {width}: off-centre"
                );
            }
        }
    }
    assert!((pile_reach(1)).abs() < 1e-6, "a lone card has no pile");
    assert!(
        (pile_reach(100) - pile_reach(PILE_SLABS + 1)).abs() < 1e-6,
        "a pile shows at most {PILE_SLABS} cards under its top"
    );
}
