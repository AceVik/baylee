//! Where a permanent's keyword marks sit on its card.
//!
//! The marks themselves are drawn by the card shader — one distance-field
//! sample per mark out of the atlas `baylee-client/src/markatlas.rs` bakes
//! from the Mana font, so that a creature with six keywords is still one
//! draw. They were twelve procedural pictograms in `card_common.wgsl` until
//! September 2026, and what replaced them is the icon a player has already
//! learned somewhere else. *Where* they sit is
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
pub const MARK_ORDER: [KeywordBadge; 12] = [
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
    // **Appended, never inserted, and retired in place.**
    //
    // The reason is not that a mark would move on screen — `marks` packs, so
    // a creature with one keyword draws one mark at the left end whatever
    // else is in this table. It is that **a badge's index here *is* its wire
    // format**: the renderer's `cardmat::glow_bits` zips this array with
    // `glow::MARK_SHIFT` to make the bit the GPU reads, and its
    // `markatlas::MARK_CELLS` makes the same index the atlas cell the glyph
    // was baked into. Neither lives in this crate, which is the point: this
    // array is read from above and cannot see who is counting on it. Moving one renumbers a value that
    // crosses to the GPU, and the failure it produces is a card drawing
    // *another keyword's* glyph — a picture that lies while every round trip
    // stays green and nothing in the suite can see it.
    //
    // So nothing is ever removed from this array. A badge the rail stops
    // drawing retires as a `None` in its own slot — which costs no visible
    // space at all, again because `marks` packs — and the array becomes
    // `[Option<KeywordBadge>; N]` on the day that first happens. Nothing has
    // retired yet, so it is still a plain array; the rule is written down now
    // because the moment it is needed is the moment somebody reaches for
    // `remove` instead.
    KeywordBadge::Prowess,
];

/// The glyph each mark is drawn with, in [`MARK_ORDER`]'s order.
///
/// Codepoints in the Mana font's private-use block, taken from the font's own
/// stylesheet (`css/mana.css`, the `ms-ability-*` classes) — the only place
/// the mapping is published — and verified against the shipped font by
/// `baylee-client`'s `markatlas`, which refuses to bake a codepoint that
/// rasterises to nothing. A wrong number here draws an empty box and no
/// compiler can see it.
///
/// This table, `manapip::glyph` and [`crate::cardcrest::GLYPHS`] are the only
/// three doors a Mana glyph enters this client through, and `docs/legal.md`
/// §2a is why there is one per purpose: the font is open-licensed, the
/// symbols on it are Wizards' marks, and some of what the font draws — the
/// planeswalker symbol, the guild and clan watermarks — that policy names as
/// off limits outright. A set that can be read off one page is a set somebody
/// can audit.
pub const MARK_GLYPHS: [char; MARK_ORDER.len()] = [
    '\u{e952}', // flying
    '\u{e950}', // first strike
    '\u{e94d}', // double strike
    '\u{e94b}', // deathtouch
    '\u{e953}', // haste
    '\u{ea4b}', // lifelink
    '\u{e95d}', // menace
    '\u{e960}', // reach
    '\u{e964}', // trample
    '\u{e968}', // vigilance
    '\u{e94c}', // defender
    '\u{e982}', // prowess
];

/// The glyph a badge is drawn with, or `None` for one the border draws.
#[must_use]
pub fn glyph_of(badge: KeywordBadge) -> Option<char> {
    slot_of(badge).map(|i| MARK_GLYPHS[i])
}

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
/// Marks shrink rather than spill: twelve of them are twelve coloured pips
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

    /// Every mark has a glyph, no two marks share one, and every one of them
    /// is in the private-use block the Mana font maps.
    ///
    /// The length is the compiler's already; what it cannot see is a
    /// duplicate, which is how two keywords come to wear the same picture,
    /// and a codepoint outside the block, which is how one comes to wear
    /// none. Whether the *font* has it is `markatlas`'s question, because
    /// that needs the file.
    #[test]
    fn every_mark_has_its_own_glyph_in_the_private_use_block() {
        for (i, g) in MARK_GLYPHS.iter().enumerate() {
            assert!(
                ('\u{e000}'..='\u{f8ff}').contains(g),
                "{:?} is drawn with {g:?}, which is not a private-use codepoint",
                MARK_ORDER[i]
            );
            assert_eq!(
                MARK_GLYPHS.iter().filter(|o| *o == g).count(),
                1,
                "{:?} shares its glyph with another mark",
                MARK_ORDER[i]
            );
        }
        assert_eq!(glyph_of(KeywordBadge::Flying), Some(MARK_GLYPHS[0]));
        assert_eq!(glyph_of(KeywordBadge::Hexproof), None);
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
