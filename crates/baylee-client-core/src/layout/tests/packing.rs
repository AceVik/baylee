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
    // The pitch is clamped, so the row deliberately runs wider than the
    // pod: the board model is expected to collapse the row instead.
    assert!(packing.pitch >= CARD_WIDTH * MIN_VISIBLE_FRACTION);
}

#[test]
fn empty_and_single_lanes_are_degenerate_but_valid() {
    assert!(pack_lane(0, 10.0).offsets.is_empty());
    assert_eq!(pack_lane(1, 10.0).offsets, vec![0.0]);
}
