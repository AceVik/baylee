//! The keyword strip on the table (#274): where its quad lies on the card,
//! and that the next card of a fanned row covers it.

use super::flying_tests::{creature, duel};
use super::*;

/// The strip's quad is where `cardrail` says, in the card's own space: its
/// bottom edge on the seam between the art and the type line, its left edge
/// where the shadow's margin begins.
///
/// Read back out of the transform and the mesh's size rather than restated,
/// so a flipped axis — a card's `+y` is its top, and its UV's is its bottom —
/// puts the strip on the ledge and this goes red.
#[test]
fn the_strip_stands_on_the_seam() {
    let at = strip_transform(0.0).translation;
    let size = crate::marksmat::quad_size();
    let (w, h) = (size.x * CARD_WIDTH, size.y * DOWN_THE_CARD);
    let bottom = (CARD_HEIGHT * 0.5 - (at.y - h * 0.5)) / DOWN_THE_CARD;
    let left = (at.x - w * 0.5) / CARD_WIDTH + 0.5;
    assert!(
        (bottom - cardrail::strip_bottom()).abs() < 1e-5,
        "the strip ends {bottom} card widths down the card, not on the seam at {}",
        cardrail::strip_bottom()
    );
    assert!(
        (left - cardrail::quad_rect()[0]).abs() < 1e-5,
        "the strip's quad starts {left} across the card, not at {}",
        cardrail::quad_rect()[0]
    );
}

/// Every strip in a fanned row lies above its own card's face and below the
/// face of the card laid over it.
///
/// The first half is what makes it visible at all; the second is what keeps
/// it off its neighbour. A strip lifted a card's thickness, which is what it
/// was first planned at, is fourteen times a whole row's rise, and was drawn
/// over the art of every card covering it.
#[test]
fn a_strip_lies_on_its_card_and_under_the_next_one() {
    for n in [1usize, 2, 5, 17, 40] {
        let groups = (0..n)
            .map(|i| {
                let slot = u32::try_from(i + 1).expect("a small row");
                creature(slot, vec![KeywordBadge::Trample, KeywordBadge::Lifelink])
            })
            .collect();
        let mut placed = placements(&duel(groups));
        assert_eq!(placed.len(), n);
        placed.sort_by(|a, b| a.lift.total_cmp(&b.lift));
        let word = cardrail::badge_bits(&[KeywordBadge::Trample, KeywordBadge::Lifelink]);
        for (i, card) in placed.iter().enumerate() {
            assert_eq!(card.marks, word, "the placement carries the strip's word");
            let face = card.lift + CARD_THICKNESS;
            let strip = card.lift + strip_transform(card.rung).translation.z;
            assert!(
                strip > face,
                "in a row of {n}, strip {i} is not above its card"
            );
            if let Some(next) = placed.get(i + 1) {
                let over = next.lift + CARD_THICKNESS;
                assert!(
                    strip < over,
                    "in a row of {n}, strip {i} at {strip} is over the next card's face at {over}"
                );
            }
        }
    }
}
