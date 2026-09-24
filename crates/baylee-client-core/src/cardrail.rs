//! Where a permanent's keyword marks sit on its card: the strip.
//!
//! The marks are the Mana font's own ability glyphs, baked to distance fields
//! by `baylee-client/src/markatlas.rs` — twelve procedural pictograms in
//! `card_common.wgsl` until September 2026, replaced by the icon a player has
//! already learned somewhere else. *Where* they sit is arithmetic, and
//! arithmetic belongs somewhere it can be tested without a GPU: these
//! constants are the shaders', mirrored, and a test in `baylee-client` reads
//! the WGSL text and fails if the two ever drift.
//!
//! # A strip lying on the art (#274)
//!
//! The marks ran along the card's bottom edge until #274, over the print's
//! artist and copyright line. Nothing this client *paints* lies on the print
//! any more; the marks became an **object**: a dark strip with its own
//! contact shadow, lifted a card's thickness over the card as a child of it,
//! the way a die lies on a real card. It lies on the art, with its bottom
//! edge on the seam where a modern frame's art meets its type line — the
//! owner's "über den Type ins Bild, quasi Kante an Kante mit dem Artwork" —
//! so it covers neither the name and cost, nor the type line, nor the artist.
//! [`M15_SEAM`] is where that seam is, measured.
//!
//! The strip is at the card's **left**, because a lane fans with each card's
//! own left edge exposed: the first mark stays in sight at the tightest
//! pitch. A tapped card turns clockwise, which carries the strip out of that
//! exposed edge — a tapped creature in a fanned lane shows its plate and not
//! its marks. That is accepted: its attack has been declared, and the
//! preview names its keywords.
//!
//! [`badge_at`] answers "what is that little green drop?" for a pointer, and
//! nothing calls it yet — the hover tooltip that will is the next step, and
//! this note stays here until it exists, because a function written and never
//! called is how the client's combat path stayed broken for weeks.

use crate::board::KeywordBadge;

/// The card's aspect, so a length measured in card widths means the same
/// thing on both axes.
pub const CARD_ASPECT: f32 = 63.0 / 88.0;

/// Where a modern frame's art ends, as a share of the print's height.
///
/// Measured, not recalled: on 36 Scryfall `normal` scans (488 × 680) of
/// 2015-frame creatures from 33 sets, ORI to DFT, the dark rule under the
/// art begins on row 378 on 34 of them, 377 on one (ELD) and 379 on one
/// (ZNR). The strip's bottom edge sits on the dominant row; on the earliest print that is one
/// row of 680 over the rule — a sixth of a pixel on a table card — and on
/// none of them is it over the type line's box, which starts five rows lower.
pub const M15_SEAM: f32 = 378.0 / 680.0;

/// Where the strip's left edge sits, in card widths from the card's left.
///
/// Inside the frame's side (0.061), so the strip overlaps the frame and the
/// print's edge the way a die put down at a card's edge does.
pub const STRIP_X0: f32 = 0.025;

/// The strip's inner margin, round its marks.
pub const STRIP_PAD: f32 = 0.012;

/// A mark's square, in card widths: eight physical pixels on a card 94 wide.
///
/// It never shrinks. The rail shrank its marks to fit twelve in its span,
/// and a shrunk eight-pixel glyph is a coloured pip; the strip wraps instead.
pub const MARK: f32 = 0.085;

/// The air between two marks.
pub const MARK_GAP: f32 = 0.012;

/// How many marks a row holds before the strip opens a second one.
///
/// The second row opens **above** the first: the row a creature already
/// wears never moves, and the strip's bottom edge stays on the seam.
pub const PER_ROW: usize = 6;

/// How far the strip's contact shadow spreads past it, in card widths.
///
/// On the left, the right and the top: not below, where the type line is.
/// The strip stands on the seam's rule, and its shadow falls on the art.
pub const SHADOW_MARGIN: f32 = 0.02;

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
    // format**: [`mark_bits`] makes it the bit the strip's shader reads, and
    // the renderer's `markatlas::MARK_CELLS` makes the same index the atlas
    // cell the glyph was baked into. The atlas does not live in this crate,
    // which is the point: this array is read from above and cannot see who
    // is counting on it. Moving one renumbers a value that crosses to the
    // GPU, and the failure it produces is a card drawing
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

/// The twelve-bit word the strip's shader reads: bit `i` set when the card
/// has [`MARK_ORDER`]`[i]`.
///
/// A badge's index *is* this bit, which is the wire-format rule written on
/// [`MARK_ORDER`]. The word is the strip's whole material key, so a lane of
/// twelve Soldiers with the same keywords is one material however long it is.
#[must_use]
pub fn mark_bits(keywords: u128) -> u32 {
    let mut bits = 0;
    for (slot, badge) in (0u32..).zip(MARK_ORDER) {
        if keywords & badge.bit() != 0 {
            bits |= 1 << slot;
        }
    }
    bits
}

/// [`mark_bits`] for a card whose keywords arrive as badges — a board
/// group's, which is where the table reads them.
#[must_use]
pub fn badge_bits(badges: &[KeywordBadge]) -> u32 {
    mark_bits(badges.iter().fold(0, |word, badge| word | badge.bit()))
}

/// How many rows `n` marks take, and how many marks the widest row holds.
#[must_use]
pub const fn rows_and_columns(n: usize) -> (usize, usize) {
    let rows = n.div_ceil(PER_ROW);
    let columns = if n < PER_ROW { n } else { PER_ROW };
    (rows, columns)
}

/// How long `k` marks side by side are with their gaps, in card widths.
const fn run(k: usize) -> f32 {
    if k == 0 {
        return 0.0;
    }
    (k as f32) * MARK + ((k - 1) as f32) * MARK_GAP
}

/// Where the strip's bottom edge sits, in card widths from the card's top:
/// on [`M15_SEAM`], through the frame's window.
#[must_use]
pub const fn strip_bottom() -> f32 {
    crate::cardframe::FRAME_TOP + M15_SEAM * crate::cardframe::PRINT_TALL
}

/// The strip holding `n` marks, `[x0, y0, x1, y1]` in card widths from the
/// card's top-left corner, `y` growing down the card. Empty for none.
#[must_use]
pub const fn strip_rect(n: usize) -> [f32; 4] {
    let (rows, columns) = rows_and_columns(n);
    let y1 = strip_bottom();
    if n == 0 {
        return [STRIP_X0, y1, STRIP_X0, y1];
    }
    let w = 2.0 * STRIP_PAD + run(columns);
    let h = 2.0 * STRIP_PAD + run(rows);
    [STRIP_X0, y1 - h, STRIP_X0 + w, y1]
}

/// The quad the strip is drawn on: the largest strip with its shadow round
/// it on three sides, `[x0, y0, x1, y1]` like [`strip_rect`].
///
/// One quad for every card, however many marks it has: the shader sizes the
/// strip inside it from the word, so there is one mesh and the only key a
/// strip's material has is its marks.
#[must_use]
pub const fn quad_rect() -> [f32; 4] {
    let [x0, y0, x1, y1] = strip_rect(MARK_ORDER.len());
    [
        x0 - SHADOW_MARGIN,
        y0 - SHADOW_MARGIN,
        x1 + SHADOW_MARGIN,
        y1,
    ]
}

/// The badge under a point on the card, in the card's own UV.
///
/// `uv` is `(0,0)` at the card's top-left and `(1,1)` at its bottom-right,
/// which is what both the mesh and the UI node hand a shader. The first row
/// is the one on the seam; a seventh mark opens the row above it.
#[must_use]
pub fn badge_at(uv: (f32, f32), badges: &[KeywordBadge]) -> Option<KeywordBadge> {
    let row = marks(badges);
    let [x0, _, _, y1] = strip_rect(row.len());
    // Width-units, so a mark is square.
    let (x, y) = (uv.0, uv.1 / CARD_ASPECT);
    let across = x - x0 - STRIP_PAD;
    let up = y1 - STRIP_PAD - y;
    if across < 0.0 || up < 0.0 {
        return None;
    }
    let pitch = MARK + MARK_GAP;
    let (column, level) = ((across / pitch) as usize, (up / pitch) as usize);
    let on_mark = across - column as f32 * pitch <= MARK && up - level as f32 * pitch <= MARK;
    if column >= PER_ROW || !on_mark {
        return None;
    }
    row.get(level * PER_ROW + column).copied()
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

    /// The strip lies on the art, and nowhere a player reads rules from.
    ///
    /// Measured in the **print's** height, through the frame's window, since
    /// that is what the layouts were measured in: a modern frame's name and
    /// cost end by 0.105 of it, and the artist and copyright line start at
    /// 0.93 on every portrait layout. On a modern frame the strip's bottom
    /// edge is the seam, so it is over the art and not over the type line.
    /// All of that holds for twelve marks as well as for one, since a second
    /// row grows upwards.
    ///
    /// The seam is a measurement and this is where it is written down: 36
    /// Scryfall `normal` scans (488 × 680) of 2015-frame creatures from 33
    /// sets — one each from ORI, BFZ, KLD, AKH, XLN, DOM, M19, GRN, M20, ELD,
    /// THB, IKO, M21, ZNR, KHM, STX, AFR, MID, NEO, SNC, DMU, BRO, ONE, WOE,
    /// LCI, MKM, OTJ, BLB, DSK and DFT, and six from the local cache (three
    /// FDN, one each XLN, OGW and MSC) — profiled down the
    /// middle half of the card. The dark rule under the art starts on row
    /// 378 on 34 of them, 377 on ELD's and 379 on ZNR's; the type line's box
    /// starts five rows below it on all 36. [`M15_SEAM`] is row 378.
    #[test]
    fn the_strip_lies_on_the_art_between_the_name_and_the_type_line() {
        use crate::cardframe::{FRAME_TOP, PRINT_TALL};
        // ELD's rule, the earliest of the 36, and ELD's type line, five rows
        // under it and the earliest of those.
        const EARLIEST_RULE: f32 = 377.0 / 680.0;
        const EARLIEST_TYPE_LINE: f32 = 382.0 / 680.0;
        let print = |y: f32| (y - FRAME_TOP) / PRINT_TALL;
        for n in 1..=MARK_ORDER.len() {
            let [_, y0, _, y1] = strip_rect(n);
            assert!(
                print(y0) >= 0.105,
                "{n} marks reach {} of the print, over the name",
                print(y0)
            );
            assert!(
                print(y1) <= 0.93,
                "{n} marks reach {} of the print, over the artist",
                print(y1)
            );
            // Edge to edge with the art: not above the earliest rule, which
            // would leave a band of art showing under the strip, and not as
            // far down as the earliest type line.
            assert!(
                print(y1) >= EARLIEST_RULE - 1e-5 && print(y1) < EARLIEST_TYPE_LINE,
                "{n} marks end at {} of the print, off the seam measured between \
                 {EARLIEST_RULE} and {EARLIEST_TYPE_LINE}",
                print(y1)
            );
        }
        // And the quad the shadow is drawn in does not reach below it either.
        assert!(
            (quad_rect()[3] - strip_bottom()).abs() < 1e-6,
            "a shadow under the seam"
        );
    }

    /// A second row opens above the first, and a card with more marks is
    /// never a shorter strip.
    #[test]
    fn a_seventh_mark_opens_a_row_above() {
        let one = strip_rect(PER_ROW);
        let seven = strip_rect(PER_ROW + 1);
        assert!((one[3] - seven[3]).abs() < 1e-6, "the first row moved");
        assert!(
            seven[1] < one[1] - MARK,
            "the second row is not above the first"
        );
        for n in 1..MARK_ORDER.len() {
            assert!(strip_rect(n + 1)[1] <= strip_rect(n)[1] + 1e-6);
            assert!(strip_rect(n + 1)[2] >= strip_rect(n)[2] - 1e-6);
        }
    }

    /// The first mark is whole in the edge of a card the tightest fan shows,
    /// and the widest strip stays on the card.
    #[test]
    fn the_first_mark_survives_the_tightest_fan() {
        let first = STRIP_X0 + STRIP_PAD + MARK;
        assert!(
            first < crate::layout::MIN_VISIBLE_FRACTION,
            "the first mark ends at {first}"
        );
        let [x0, _, x1, _] = quad_rect();
        assert!(
            x0 > 0.0 && x1 < 1.0,
            "the strip's quad runs off the card: {x0}..{x1}"
        );
    }

    /// The word is the badge list's order, and nothing but the marks.
    #[test]
    fn the_word_is_one_bit_per_mark_in_order() {
        assert_eq!(mark_bits(k::FLYING), 1);
        assert_eq!(mark_bits(k::FLYING | k::DEFENDER), 1 | 1 << 10);
        assert_eq!(
            mark_bits(k::HEXPROOF | k::INDESTRUCTIBLE),
            0,
            "the paper says those"
        );
        assert_eq!(mark_bits(u128::MAX), (1 << MARK_ORDER.len()) - 1);
        // And the table's door, which reads a group's badges, says the same.
        let word = k::FLYING | k::TRAMPLE | k::HEXPROOF | k::PROWESS;
        assert_eq!(badge_bits(&KeywordBadge::from_bits(word)), mark_bits(word));
    }

    /// Every mark answers for its own badge, and nothing outside the strip
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
        let [x0, _, _, y1] = strip_rect(row.len());
        let y = (y1 - STRIP_PAD - MARK * 0.5) * CARD_ASPECT;
        for (i, badge) in row.iter().enumerate() {
            let x = x0 + STRIP_PAD + (i as f32) * (MARK + MARK_GAP) + MARK * 0.5;
            assert_eq!(badge_at((x, y), &badges), Some(*badge));
        }
        // The middle of the card, the gap between two marks, and past the
        // last one.
        assert_eq!(badge_at((0.5, 0.2), &badges), None);
        assert_eq!(
            badge_at((x0 + STRIP_PAD + MARK + MARK_GAP * 0.5, y), &badges),
            None
        );
        assert_eq!(badge_at((0.9, y), &badges), None);
        // And a card with nothing on the strip has nothing to hit.
        let paper_only = KeywordBadge::from_bits(k::HEXPROOF);
        assert_eq!(badge_at((x0 + STRIP_PAD + 0.01, y), &paper_only), None);
    }
}
