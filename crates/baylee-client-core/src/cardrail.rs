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
//! own left edge exposed: the first cell stays in sight at the tightest
//! pitch. A tapped card turns clockwise, which carries the strip out of that
//! exposed edge. That is accepted: its attack has been declared, and the
//! preview names its keywords.
//!
//! # The label (#298)
//!
//! #298 took the frame #274 had put round the print away: the print fills
//! the card again. What the frame said moved on to the strip, which the
//! owner allowed on one condition — it reads as an object lying over the
//! card, with its own shadow. So the strip is a label now ([`Strip`]):
//! the chip (`cardplate`), the marks, and the identity crests
//! (`cardcrest`), packed from the left in that order, and a card with none
//! of them wears no strip at all. The sleep moon that stood after the chip
//! went when summoning sickness became a wave over the card (`shellmat`,
//! #298); its place in the packing stays empty. Hexproof, indestructible and
//! shroud, which were the frame's material, are marks like any other. The
//! plate stood at the strip's left end too until the owner, 25.09, read the
//! numbers sitting with the marks as the fault: it is an object of its own
//! at the card's bottom right now (`cardplate::plate_rect`).
//!
//! [`badge_at`] answers "what is that little green drop?" for a pointer, and
//! nothing calls it yet — the hover tooltip that will is the next step, and
//! this note stays here until it exists, because a function written and never
//! called is how the client's combat path stayed broken for weeks.

use crate::board::KeywordBadge;

/// The card's aspect, so a length measured in card widths means the same
/// thing on both axes.
pub const CARD_ASPECT: f32 = 63.0 / 88.0;

/// The card's height, in card widths.
pub const CARD_TALL: f32 = 1.0 / CARD_ASPECT;

/// The black border printed round a modern card, in card widths: about
/// 0.045 of its width. Things of ours that hang over a card's edge — the
/// strip's end, the count — stop on it, short of the name and the cost.
pub const PRINTED_BORDER: f32 = 0.045;

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
/// On the print's own black border, which is about 0.045 wide on a modern
/// frame, so the strip overlaps the card's edge the way a die put down at a
/// card's edge does.
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

/// How long a row of the strip may run, inside its margin, in card widths.
///
/// Seven marks, or the chip with six beside it. A cell that
/// would run past it opens a row **above** the first: the row a creature
/// already wears never moves, and the strip's bottom edge stays on the seam.
/// Seven tenths of a card and not all of it, so the strip reads as a label
/// lying on the art rather than a band across it.
pub const ROW_MAX: f32 = 0.70;

/// How far the strip's contact shadow spreads past it, in card widths.
///
/// On the left, the right and the top: not below, where the type line is.
/// The strip stands on the seam's rule, and its shadow falls on the art.
pub const SHADOW_MARGIN: f32 = 0.02;

/// The keywords that ride the strip, in the order their slots run.
///
/// Every [`KeywordBadge`]. The first twelve are its own order less hexproof,
/// indestructible and shroud, which the card's frame spoke for until #298
/// took the frame away; those three were appended then, by the rule below.
pub const MARK_ORDER: [KeywordBadge; 15] = [
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
    KeywordBadge::Hexproof,
    KeywordBadge::Indestructible,
    KeywordBadge::Shroud,
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
    '\u{e954}', // hexproof
    '\u{e95a}', // indestructible
    '\u{ea88}', // shroud
];

/// The glyph a badge is drawn with.
#[must_use]
pub fn glyph_of(badge: KeywordBadge) -> Option<char> {
    slot_of(badge).map(|i| MARK_GLYPHS[i])
}

/// Which slot a badge occupies.
#[must_use]
pub fn slot_of(badge: KeywordBadge) -> Option<usize> {
    MARK_ORDER.iter().position(|m| *m == badge)
}

/// The badges a card wears on the strip, in slot order.
#[must_use]
pub fn marks(badges: &[KeywordBadge]) -> Vec<KeywordBadge> {
    MARK_ORDER
        .iter()
        .copied()
        .filter(|m| badges.contains(m))
        .collect()
}

/// The word the strip's shader reads: bit `i` set when the card
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

/// The label bits of the strip's word: the identity crests, two bits each,
/// `glyph + 1` or zero for none. Bit 0 was the sleep moon's until
/// summoning sickness became a wave over the card (#298); it stays unused,
/// so the crests keep their places.
pub mod label {
    /// Where the first crest's two bits start.
    pub const CREST_SHIFT: u32 = 1;
    /// How wide one crest's field is.
    pub const CREST_BITS: u32 = 2;
}

/// Everything the strip says about one card, as the three words its shader
/// reads (#298): the keywords, and the label that used to be the frame's —
/// what the counters add and the crests.
///
/// The strip is the material's whole key, so two cards saying the same
/// thing share one material however long the lane is.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct Strip {
    /// The marks, [`mark_bits`].
    pub marks: u32,
    /// The chip: the swing half of [`crate::cardplate::Corner::packed`]
    /// without the plate's tone, zero with no swing or where the card shows
    /// no plate ([`crate::cardplate::Corner::shows_plate`]).
    pub swing: u32,
    /// The crests, [`label`].
    pub label: u32,
}

/// One thing the strip carries, in the order they are packed from the left.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Item {
    /// The chip: what a permanent's ±1/±1 counters add.
    Chip,
    /// A keyword, by its slot in [`MARK_ORDER`].
    Mark(usize),
    /// An identity crest, by its [`crate::cardcrest`] glyph.
    Crest(usize),
}

/// Where one item lies: `[x0, y0, x1, y1]` in card widths from the card's
/// top-left corner, `y` growing down the card.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Cell {
    /// What is drawn there.
    pub item: Item,
    /// Where.
    pub rect: [f32; 4],
}

impl Strip {
    /// The strip for a card: its keyword word, its corner when the card
    /// shows its plate (the chip goes with it), and its crests.
    #[must_use]
    pub fn new(
        marks: u32,
        corner: Option<crate::cardplate::Corner>,
        crests: [Option<usize>; crate::cardcrest::MAX_CRESTS],
    ) -> Self {
        let swing = corner.map_or(0, crate::cardplate::Corner::chip);
        let mut label = 0;
        for (n, glyph) in crests.iter().enumerate() {
            if let Some(glyph) = glyph {
                let at = label::CREST_SHIFT + label::CREST_BITS * n as u32;
                label |= (*glyph as u32 + 1) << at;
            }
        }
        Self {
            marks,
            swing,
            label,
        }
    }

    /// Whether there is nothing to draw, and so no strip.
    #[must_use]
    pub const fn is_empty(self) -> bool {
        self.marks == 0 && self.swing & crate::cardplate::SWING_SET == 0 && self.label == 0
    }

    /// The items this strip carries, in packing order, each with its width
    /// and height in card widths.
    fn items(self) -> Vec<(Item, f32, f32)> {
        use crate::cardplate::{CHIP_W, PLATE_H, SWING_SET};
        let mut out = Vec::new();
        if self.swing & SWING_SET != 0 {
            out.push((Item::Chip, CHIP_W, PLATE_H));
        }
        for slot in 0..MARK_ORDER.len() {
            if self.marks & (1 << slot) != 0 {
                out.push((Item::Mark(slot), MARK, MARK));
            }
        }
        for n in 0..crate::cardcrest::MAX_CRESTS as u32 {
            let field = (self.label >> (label::CREST_SHIFT + label::CREST_BITS * n)) & 3;
            if field != 0 {
                out.push((Item::Crest(field as usize - 1), MARK, MARK));
            }
        }
        out
    }

    /// Every item and where it lies: packed from the left along the seam,
    /// a cell that would run past [`ROW_MAX`] opening a row above.
    ///
    /// The first row is as tall as the chip when there is one and a mark
    /// otherwise, and every cell stands on its row's middle line. The
    /// strip's shader lays itself out by the same steps, and `cardmat`'s
    /// tests hold the two to the same constants.
    #[must_use]
    pub fn cells(self) -> Vec<Cell> {
        self.layout().0
    }

    /// [`Self::cells`], and the top edge of the highest row.
    fn layout(self) -> (Vec<Cell>, f32) {
        let items = self.items();
        let first = if matches!(items.first(), Some((Item::Chip, ..))) {
            crate::cardplate::PLATE_H
        } else {
            MARK
        };
        let mut out = Vec::with_capacity(items.len());
        let (mut x, mut row) = (0.0_f32, 0_u32);
        let mut top = strip_bottom() - STRIP_PAD;
        for (item, w, h) in items {
            if x > 0.0 && x + w > ROW_MAX {
                row += 1;
                x = 0.0;
            }
            let (bottom, tall) = row_bottom(first, row);
            top = top.min(bottom - tall);
            let mid = bottom - tall * 0.5;
            let x0 = STRIP_X0 + STRIP_PAD + x;
            out.push(Cell {
                item,
                rect: [x0, mid - h * 0.5, x0 + w, mid + h * 0.5],
            });
            x += w + MARK_GAP;
        }
        (out, top)
    }

    /// The strip's body round its cells, `[x0, y0, x1, y1]` like a
    /// [`Cell`]'s: empty for a strip with nothing on it.
    #[must_use]
    pub fn rect(self) -> [f32; 4] {
        let y1 = strip_bottom();
        let (cells, top) = self.layout();
        if cells.is_empty() {
            return [STRIP_X0, y1, STRIP_X0, y1];
        }
        let x1 = cells.iter().map(|c| c.rect[2]).fold(STRIP_X0, f32::max);
        [STRIP_X0, top - STRIP_PAD, x1 + STRIP_PAD, y1]
    }

    /// The widest and tallest strip there is: every mark, both crests and
    /// the chip.
    #[must_use]
    pub fn largest() -> Self {
        use crate::cardplate::SWING_SET;
        Self {
            marks: (1 << MARK_ORDER.len()) - 1,
            swing: SWING_SET,
            label: (1 << label::CREST_SHIFT) | (3 << (label::CREST_SHIFT + label::CREST_BITS)),
        }
    }

    /// The item under a point on the card, in the card's own UV: `(0,0)` at
    /// its top-left and `(1,1)` at its bottom-right, which is what both the
    /// mesh and the UI node hand a shader.
    #[must_use]
    pub fn at(self, uv: (f32, f32)) -> Option<Item> {
        // Width units, so a mark is square.
        let (x, y) = (uv.0, uv.1 / CARD_ASPECT);
        self.cells()
            .into_iter()
            .find(|c| (c.rect[0]..=c.rect[2]).contains(&x) && (c.rect[1]..=c.rect[3]).contains(&y))
            .map(|c| c.item)
    }
}

/// Row `row`'s bottom edge in card widths from the card's top, and its
/// height, for a first row `first` tall.
fn row_bottom(first: f32, row: u32) -> (f32, f32) {
    let seam = strip_bottom() - STRIP_PAD;
    if row == 0 {
        return (seam, first);
    }
    let above = first + MARK_GAP + (row - 1) as f32 * (MARK + MARK_GAP);
    (seam - above, MARK)
}

/// Where the strip's bottom edge sits, in card widths from the card's top:
/// on [`M15_SEAM`] of the print, which fills the card.
#[must_use]
pub const fn strip_bottom() -> f32 {
    M15_SEAM * CARD_TALL
}

/// The quad the strip is drawn on: the largest strip with its shadow round
/// it on three sides, `[x0, y0, x1, y1]` like a [`Cell`]'s.
///
/// One quad for every card, however much its strip says: the shader sizes
/// the strip inside it from its words, so there is one mesh.
#[must_use]
pub fn quad_rect() -> [f32; 4] {
    let [x0, y0, x1, y1] = Strip::largest().rect();
    [
        x0 - SHADOW_MARGIN,
        y0 - SHADOW_MARGIN,
        x1 + SHADOW_MARGIN,
        y1,
    ]
}

/// The badge under a point on the card, in the card's own UV.
#[must_use]
pub fn badge_at(uv: (f32, f32), strip: Strip) -> Option<KeywordBadge> {
    match strip.at(uv)? {
        Item::Mark(slot) => Some(MARK_ORDER[slot]),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::board::keyword_bits as k;
    use crate::cardplate::{CHIP_W, Corner, Plate};

    /// Every badge is a mark, and the first twelve kept their slots.
    ///
    /// Both halves matter: a badge with no slot is a keyword the card no
    /// longer says anywhere now the frame is gone (#298), and a slot that
    /// moved would draw another keyword's glyph, because the slot is the
    /// wire format.
    #[test]
    fn every_badge_is_a_mark_and_the_first_twelve_kept_their_slots() {
        for badge in KeywordBadge::ALL {
            assert_eq!(
                MARK_ORDER.iter().filter(|m| **m == badge).count(),
                1,
                "{badge:?} is not on the strip exactly once"
            );
        }
        assert_eq!(MARK_ORDER.len(), KeywordBadge::ALL.len());
        assert_eq!(slot_of(KeywordBadge::Flying), Some(0));
        assert_eq!(slot_of(KeywordBadge::Prowess), Some(11));
        assert_eq!(slot_of(KeywordBadge::Hexproof), Some(12));
        assert_eq!(slot_of(KeywordBadge::Shroud), Some(14));
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
        assert_eq!(glyph_of(KeywordBadge::Shroud), Some('\u{ea88}'));
    }

    /// A strip for every shape of label: no corner, a creature's with and
    /// without a chip, a saga's with and without one and a planeswalker's;
    /// asleep and awake; no crest, one and two; and each with every count
    /// of marks.
    fn every_strip() -> Vec<Strip> {
        let fight = Corner {
            plate: Plate::Fight {
                power: 12,
                toughness: 12,
                damage: 3,
            },
            ..Corner::default()
        };
        let swung = Corner {
            swing: Some((2, 2)),
            ..fight
        };
        let corners = [
            None,
            Some(fight),
            Some(swung),
            Some(Corner {
                plate: Plate::Lore(3),
                ..Corner::default()
            }),
            Some(Corner {
                plate: Plate::Lore(3),
                swing: Some((1, 1)),
                ..Corner::default()
            }),
            Some(Corner {
                plate: Plate::Loyalty(7),
                ..Corner::default()
            }),
        ];
        let crests = [[None, None], [Some(0), None], [Some(1), Some(2)]];
        let mut out = Vec::new();
        for corner in corners {
            for crest in crests {
                for n in 0..=MARK_ORDER.len() {
                    out.push(Strip::new((1 << n) - 1, corner, crest));
                }
            }
        }
        out
    }

    /// The strip lies on the art, and nowhere a player reads rules from.
    ///
    /// Measured in the print's height, which is the card's since #298: a
    /// modern frame's name and cost end by 0.105 of it, and the artist and
    /// copyright line start at 0.93 on every portrait layout. On a modern
    /// frame the strip's bottom edge is the seam, so it is over the art and
    /// not over the type line. All of that holds for the largest strip there
    /// is as well as for one mark, since a second row grows upwards.
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
        // ELD's rule, the earliest of the 36, and ELD's type line, five rows
        // under it and the earliest of those.
        const EARLIEST_RULE: f32 = 377.0 / 680.0;
        const EARLIEST_TYPE_LINE: f32 = 382.0 / 680.0;
        let print = |y: f32| y * CARD_ASPECT;
        for strip in every_strip() {
            if strip.is_empty() {
                continue;
            }
            let [_, y0, _, y1] = strip.rect();
            assert!(
                print(y0) >= 0.105,
                "{strip:?} reaches {} of the print, over the name",
                print(y0)
            );
            // Edge to edge with the art: not above the earliest rule, which
            // would leave a band of art showing under the strip, and not as
            // far down as the earliest type line.
            assert!(
                print(y1) >= EARLIEST_RULE - 1e-5 && print(y1) < EARLIEST_TYPE_LINE,
                "{strip:?} ends at {} of the print, off the seam measured between \
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

    /// The label leads, the marks follow in slot order and the crests close
    /// the strip; no two cells overlap, and every strip fits the one quad.
    #[test]
    fn the_label_leads_and_the_crests_close_the_strip() {
        let [qx0, qy0, qx1, qy1] = quad_rect();
        for strip in every_strip() {
            let cells = strip.cells();
            let rank = |item: Item| match item {
                Item::Chip => (0, 0),
                Item::Mark(slot) => (2, slot),
                Item::Crest(_) => (3, 0),
            };
            for pair in cells.windows(2) {
                assert!(
                    rank(pair[0].item) <= rank(pair[1].item),
                    "{strip:?} packs {:?} before {:?}",
                    pair[0].item,
                    pair[1].item
                );
            }
            for (i, a) in cells.iter().enumerate() {
                for b in &cells[i + 1..] {
                    let apart = a.rect[2] <= b.rect[0] + 1e-6
                        || b.rect[2] <= a.rect[0] + 1e-6
                        || a.rect[3] <= b.rect[1] + 1e-6
                        || b.rect[3] <= a.rect[1] + 1e-6;
                    assert!(apart, "{strip:?}: {a:?} overlaps {b:?}");
                }
                let [x0, y0, x1, y1] = a.rect;
                assert!(
                    x1 - STRIP_X0 - STRIP_PAD <= ROW_MAX + 1e-6,
                    "{strip:?}: {a:?} runs past the row"
                );
                assert!(
                    x0 >= qx0 && y0 >= qy0 && x1 <= qx1 && y1 <= qy1,
                    "{strip:?}: {a:?} is off the quad"
                );
            }
            let [x0, y0, x1, y1] = strip.rect();
            assert!(x0 >= qx0 && y0 >= qy0 && x1 <= qx1 && y1 <= qy1);
        }
        // The quad stays on the card.
        assert!(
            qx0 > 0.0 && qx1 < 1.0,
            "the strip's quad runs off the card: {qx0}..{qx1}"
        );
    }

    /// An eighth mark opens a row above, and a card with more marks is never
    /// a shorter or a narrower strip.
    #[test]
    fn an_eighth_mark_opens_a_row_above() {
        let marks = |n: u32| Strip::new((1 << n) - 1, None, [None, None]);
        let seven = marks(7).rect();
        let eight = marks(8).rect();
        assert!((seven[3] - eight[3]).abs() < 1e-6, "the first row moved");
        assert!(
            (seven[1] - eight[1] - (MARK + MARK_GAP)).abs() < 1e-6,
            "the second row is not one row above the first"
        );
        for n in 1..MARK_ORDER.len() as u32 {
            assert!(marks(n + 1).rect()[1] <= marks(n).rect()[1] + 1e-6);
            assert!(marks(n + 1).rect()[2] >= marks(n).rect()[2] - 1e-6);
        }
    }

    /// The chip and the first mark are whole in the edge of a card the
    /// tightest fan shows.
    #[test]
    fn the_chip_and_the_first_mark_survive_the_tightest_fan() {
        let edge = crate::layout::MIN_VISIBLE_FRACTION;
        let chip = STRIP_X0 + STRIP_PAD + CHIP_W;
        assert!(chip < edge, "the chip ends at {chip}");
        let mark = STRIP_X0 + STRIP_PAD + MARK;
        assert!(mark < edge, "the first mark ends at {mark}");
    }

    /// The word is the badge list's order, and nothing but the marks.
    #[test]
    fn the_word_is_one_bit_per_mark_in_order() {
        assert_eq!(mark_bits(k::FLYING), 1);
        assert_eq!(mark_bits(k::FLYING | k::DEFENDER), 1 | 1 << 10);
        assert_eq!(mark_bits(k::HEXPROOF | k::SHROUD), 1 << 12 | 1 << 14);
        assert_eq!(mark_bits(u128::MAX), (1 << MARK_ORDER.len()) - 1);
        // And the table's door, which reads a group's badges, says the same.
        let word = k::FLYING | k::TRAMPLE | k::HEXPROOF | k::PROWESS;
        assert_eq!(badge_bits(&KeywordBadge::from_bits(word)), mark_bits(word));
    }

    /// The label's word packs each crest where the shader reads it, its
    /// first bit, the sleep moon's once, left empty; and a strip with
    /// nothing to say is empty.
    #[test]
    fn the_label_word_carries_the_crests() {
        let strip = Strip::new(0, None, [Some(1), Some(2)]);
        assert_eq!(strip.label & 1, 0, "the moon's bit is set");
        let crest = |n: u32| (strip.label >> (label::CREST_SHIFT + label::CREST_BITS * n)) & 3;
        assert_eq!(
            (crest(0), crest(1)),
            (2, 3),
            "a crest is its glyph plus one"
        );
        assert_eq!(
            strip.cells().iter().map(|c| c.item).collect::<Vec<_>>(),
            vec![Item::Crest(1), Item::Crest(2)]
        );
        assert!(Strip::new(0, None, [None, None]).is_empty());
        // A corner the strip does not carry leaves no trace in the key, and
        // nor does a plate: it is an object of its own (`plate_rect`).
        let none = Corner::default();
        assert!(Strip::new(0, Some(none), [None, None]).is_empty());
        let plate = Corner {
            plate: Plate::Loyalty(3),
            ..none
        };
        assert!(Strip::new(0, Some(plate), [None, None]).is_empty());
        let chip = Strip::new(
            0,
            Some(Corner {
                swing: Some((1, 0)),
                ..plate
            }),
            [None, None],
        );
        assert_eq!(
            chip.cells().iter().map(|c| c.item).collect::<Vec<_>>(),
            vec![Item::Chip]
        );
    }

    /// Every mark answers for its own badge, and nothing else on the strip
    /// or off it answers at all.
    #[test]
    fn a_point_on_a_mark_names_that_mark() {
        let word = k::FLYING | k::TRAMPLE | k::LIFELINK;
        let corner = Corner {
            plate: Plate::Loyalty(3),
            swing: Some((0, 2)),
            ..Corner::default()
        };
        let strip = Strip::new(mark_bits(word), Some(corner), [None, None]);
        let cells = strip.cells();
        let middle = |c: &Cell| {
            (
                f32::midpoint(c.rect[0], c.rect[2]),
                f32::midpoint(c.rect[1], c.rect[3]) * CARD_ASPECT,
            )
        };
        let named: Vec<_> = cells
            .iter()
            .filter_map(|c| badge_at(middle(c), strip))
            .collect();
        assert_eq!(
            named,
            vec![
                KeywordBadge::Flying,
                KeywordBadge::Lifelink,
                KeywordBadge::Trample
            ]
        );
        // The chip is on the strip and is not a keyword; the middle of the
        // card and the gap between two marks are neither.
        assert_eq!(badge_at(middle(&cells[0]), strip), None);
        assert_eq!(strip.at(middle(&cells[0])), Some(Item::Chip));
        assert_eq!(badge_at((0.5, 0.2), strip), None);
        let gap = f32::midpoint(cells[1].rect[2], cells[2].rect[0]);
        assert_eq!(badge_at((gap, middle(&cells[1]).1), strip), None);
    }
}
