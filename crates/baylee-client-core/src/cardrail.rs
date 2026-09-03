//! Where a permanent's keyword marks sit on its card.
//!
//! The marks themselves are drawn by the card shader — eleven procedural
//! pictograms in `baylee-client/src/shaders/card_common.wgsl`, so that a
//! creature with six keywords is still one draw. *Where* they sit is
//! arithmetic, and arithmetic belongs somewhere it can be tested without a
//! GPU: these constants are the shader's, mirrored, and a test in
//! `baylee-client` reads the WGSL text and fails if the two ever drift.
//!
//! What it is *today* is that mirror and nothing more: the shader draws the
//! rail, this module says where the rail is, and the test holds them
//! together. [`badge_at`] is the half that answers "what is that little
//! green drop?" for a pointer, and nothing calls it yet — the hover tooltip
//! that will is the next step, and this note stays here until it exists,
//! because a function written and never called is how the client's combat
//! path stayed broken for weeks.

use crate::board::KeywordBadge;

/// The card's aspect, so a length measured in card widths means the same
/// thing on both axes.
pub const CARD_ASPECT: f32 = 63.0 / 88.0;

/// How far the rail sits in from the printed edge, in card widths.
pub const RAIL_INSET: f32 = 0.052;

/// A slot's size when there is room for it, in card widths.
pub const RAIL_SLOT: f32 = 0.115;

/// How much of the card's width the rail may ever take.
///
/// The last fifth of the bottom edge is reserved, on purpose and before
/// anything is in it: power/toughness and the counter dice belong in that
/// corner. A rail that had to move once they arrived would move on every card
/// in every screenshot ever taken of this client.
pub const RAIL_SPAN: f32 = 0.70;

/// The keywords that ride the rail, in the order their slots run.
///
/// [`KeywordBadge`]'s own order, minus the two the border already speaks for:
/// hexproof and indestructible are a *material* on the card's edge, and a
/// mark repeating them would be the same claim twice in two languages.
/// Shroud is not a badge at all.
pub const MARK_ORDER: [KeywordBadge; 11] = [
    KeywordBadge::Flying,
    KeywordBadge::FirstStrike,
    KeywordBadge::DoubleStrike,
    KeywordBadge::Deathtouch,
    KeywordBadge::Haste,
    KeywordBadge::Lifelink,
    KeywordBadge::Menace,
    KeywordBadge::Reach,
    KeywordBadge::Trample,
    KeywordBadge::Vigilance,
    KeywordBadge::Defender,
];

/// Which slot a badge occupies, or `None` for one the border draws.
#[must_use]
pub fn slot_of(badge: KeywordBadge) -> Option<usize> {
    MARK_ORDER.iter().position(|m| *m == badge)
}

/// The badges a card wears on the rail, in slot order.
#[must_use]
pub fn marks(badges: &[KeywordBadge]) -> Vec<KeywordBadge> {
    MARK_ORDER
        .iter()
        .copied()
        .filter(|m| badges.contains(m))
        .collect()
}

/// How big one slot is when `n` marks share the rail, in card widths.
///
/// Marks shrink rather than spill: eleven of them are eleven coloured pips
/// where six are six pictograms, which is the honest failure — a row that ran
/// off the card, or a row that hid its tail, would both be lying about what
/// the creature is.
#[must_use]
pub fn slot_size(n: usize) -> f32 {
    if n == 0 {
        return 0.0;
    }
    let even = RAIL_SPAN / n as f32;
    if even < RAIL_SLOT { even } else { RAIL_SLOT }
}

/// The badge under a point on the card, in the card's own UV.
///
/// `uv` is `(0,0)` at the card's top-left and `(1,1)` at its bottom-right,
/// which is what both the mesh and the UI node hand a shader.
#[must_use]
pub fn badge_at(uv: (f32, f32), badges: &[KeywordBadge]) -> Option<KeywordBadge> {
    let row = marks(badges);
    if row.is_empty() {
        return None;
    }
    let slot = slot_size(row.len());
    // Width-units, so a slot is square.
    let (x, y) = (uv.0, uv.1 / CARD_ASPECT);
    let top = 1.0 / CARD_ASPECT - RAIL_INSET - slot;
    if y < top || y > top + slot {
        return None;
    }
    let left = RAIL_INSET;
    if x < left || x > left + row.len() as f32 * slot {
        return None;
    }
    let k = ((x - left) / slot) as usize;
    row.get(k.min(row.len() - 1)).copied()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::board::keyword_bits as k;

    /// The rail is the badge list minus what the border says, and in the
    /// badge list's order. Both halves matter: a slot that moved would move
    /// every mark to its right, and a keyword drawn twice would be read as
    /// two.
    #[test]
    fn the_rail_is_every_badge_the_border_does_not_draw() {
        let all = KeywordBadge::from_bits(u128::MAX);
        let expected: Vec<KeywordBadge> = all
            .into_iter()
            .filter(|b| !matches!(b, KeywordBadge::Hexproof | KeywordBadge::Indestructible))
            .collect();
        assert_eq!(expected.as_slice(), MARK_ORDER.as_slice());
        assert_eq!(slot_of(KeywordBadge::Hexproof), None);
        assert_eq!(slot_of(KeywordBadge::Flying), Some(0));
    }

    /// A full rail stays on the card and out of the corner the numbers will
    /// want.
    #[test]
    fn the_rail_never_reaches_the_corner_it_is_leaving_free() {
        for n in 1..=MARK_ORDER.len() {
            let width = RAIL_INSET + n as f32 * slot_size(n);
            assert!(
                width <= RAIL_INSET + RAIL_SPAN + 1e-6,
                "{n} marks reach {width}"
            );
        }
    }

    /// Every slot answers for its own badge, and nothing outside the rail
    /// answers at all.
    #[test]
    fn a_point_on_a_mark_names_that_mark() {
        let badges = KeywordBadge::from_bits(k::FLYING | k::TRAMPLE | k::LIFELINK);
        let row = marks(&badges);
        assert_eq!(
            row,
            vec![
                KeywordBadge::Flying,
                KeywordBadge::Lifelink,
                KeywordBadge::Trample
            ]
        );
        let slot = slot_size(row.len());
        let y = (1.0 / CARD_ASPECT - RAIL_INSET - slot * 0.5) * CARD_ASPECT;
        for (i, badge) in row.iter().enumerate() {
            let x = RAIL_INSET + (i as f32 + 0.5) * slot;
            assert_eq!(badge_at((x, y), &badges), Some(*badge));
        }
        // The middle of the card, and the corner the numbers are getting.
        assert_eq!(badge_at((0.5, 0.5), &badges), None);
        assert_eq!(badge_at((0.9, y), &badges), None);
        // And a card with nothing on the rail has nothing to hit.
        let border_only = KeywordBadge::from_bits(k::HEXPROOF);
        assert_eq!(badge_at((RAIL_INSET + 0.01, y), &border_only), None);
    }
}
