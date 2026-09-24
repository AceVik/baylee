//! The identity crests: what a permanent *is*, captioned on its ledge.
//!
//! Two questions a board cannot otherwise answer, asked of every permanent
//! and true of almost none of them. Is that a commander (CR 903.3)? And is
//! that really a Llanowar Elves — a token with no cardboard behind it at all
//! (CR 111.1), a copy wearing somebody else's face (CR 707.2), or the card
//! it looks like?
//!
//! # Four homes, and why this is the fourth
//!
//! The answer was drawn on the card's **top edge** until September 2026 — a
//! crown centred on it, a provenance mark against the top-left corner — and
//! that edge is the printed name. It moved to a **column in the right
//! margin**, which the owner rejected as the wrong place for something that
//! is not a number, and then to **slips**, paper tabs clipped under the name
//! and hanging into the art. All three were on the print, and since #274
//! nothing this client draws is (`docs/legal.md` §3).
//!
//! So the answer is the frame's own **paper** now ([`PAPER`], drawn by
//! `card_common.wgsl`'s `frame_paper`): verdigris for a token, violet for a
//! copy, oxblood for a commander. That is what reads at the 94 pixels a
//! table card is wide, and it is the slips' own idea taken whole — the
//! colour was always the stock the mark was printed on, and the stock is the
//! card's now. The glyphs stay as a **caption**: right-aligned on the ledge,
//! in [`CREST_INK`], legible in the preview and deliberately not relied on
//! at table size. A paper and a glyph are a legibility ladder, not the same
//! claim twice.
//!
//! Like [`crate::cardplate`] this module draws nothing. It says where each
//! crest is, which glyph it is and what it is printed on; `card_common.wgsl`
//! draws it and a mirror test in `baylee-client` reads the WGSL text and
//! fails when the two drift.

/// How many glyphs the crests can draw.
///
/// Three for at most two crests: one of them is a token **or** a copy, which
/// [`crate::board::Provenance`] already makes exclusive.
pub const GLYPH_COUNT: usize = 3;

/// The squirrel: a permanent with no card behind it.
pub const GLYPH_TOKEN: usize = 0;
/// Two cards: a permanent wearing a face that is not its own.
pub const GLYPH_COPY: usize = 1;
/// The shield: a commander.
pub const GLYPH_COMMANDER: usize = 2;

/// How many crests a permanent can ever wear at once.
///
/// A commander that is also a token or a copy is the only pairing the rules
/// allow, so two — and the third glyph is the other half of the first crest.
pub const MAX_CRESTS: usize = 2;

/// The glyph each mark is drawn with.
///
/// The third door a Mana glyph enters this client through, after
/// `manapip::glyph` and [`crate::cardrail::MARK_GLYPHS`]; `docs/legal.md`
/// §2a is why there is a door at all and why there is one per purpose.
/// Codepoints from the font's own stylesheet (`css/mana.css`) — `ms-token`,
/// `ms-ability-copy` and `ms-commander` — and verified against the shipped
/// font by `baylee-client`'s `markatlas`, which refuses to bake a codepoint
/// that rasterises to nothing.
///
/// Checked against the Fan Content Policy's "trademarks and logos that you
/// may not include" table on **17.09.2026**, by reading the page rather
/// than recalling it: fifteen images, and none of these three is among
/// them. What `ms-commander` draws is the **Commander format symbol** — the
/// CSS files it beside the card types, which is not the argument — and it
/// stands on the footing every expansion symbol Scryfall renders already
/// stands on here, `docs/legal.md` §2a's third tier. The planeswalker
/// symbol is on that table and is deliberately *not* used anywhere.
///
/// `ms-token` is a **squirrel**, which is the Mana font's own token mark
/// and the icon Scryfall puts on a token layout. Named here rather than
/// discovered later in a screenshot: nothing about a squirrel says "this
/// permanent has no card", and the argument for it is that it is the
/// picture a player has already met in the two places they would have met
/// one at all.
pub const GLYPHS: [char; GLYPH_COUNT] = [
    '\u{e96d}', // token — a squirrel
    '\u{ea60}', // copy — two cards, one behind the other
    '\u{e9c6}', // commander — the format's shield
];

/// A crest's square, in card widths: the size of a keyword mark, so the
/// card's glyphs are one alphabet at one size.
pub const CREST_W: f32 = 0.085;

/// Where the first crest's right edge sits, in card widths from the card's
/// left edge.
///
/// Right-aligned, because the left of the ledge is the plate's: the numbers
/// are what a fanned card has to show, and a caption for the preview can
/// stand where only an uncovered card shows it.
pub const CREST_X1: f32 = 0.955;

/// The air between two crests, in card widths.
pub const CREST_GAP: f32 = 0.012;

/// The paper each identity is, by its glyph index: the frame's own stock
/// for a token, a copy and a commander.
///
/// Three papers, and each says the thing its mark says without being read as
/// ink: verdigris is the colour of a thing conjured rather than printed,
/// violet is what the swing already uses for a permanent that is not quite
/// what it was, and a commander is oxblood — [`crate::cardframe::COMMANDER_PAPER`],
/// which says why it is not the gilt it was on the slips.
///
/// **Card stock, not writing paper.** These are *linear* values — the card
/// shader mixes in linear light and the framebuffer converts — displayed
/// around 175 of 255: dark enough to hold ink, to show a sleeping creature's
/// night and the hexproof wash, and to sit beside a print without
/// out-shouting it.
pub const PAPER: [[f32; 3]; GLYPH_COUNT] = [
    [0.396, 0.440, 0.418], // token — verdigris
    [0.429, 0.385, 0.506], // copy — violet
    crate::cardframe::COMMANDER_PAPER,
];

/// The ink every crest is printed in, on all three papers.
///
/// Near-black, and darker than the slips' `0.035` it replaces, because of
/// the oxblood: a mid-tone paper holds neither a light ink nor a merely dark
/// one, and the slips' ink measured 3.5:1 on it. This one is 4.8:1 there and
/// better on the other two, and [`the ink test`](self) runs over **every**
/// paper rather than the one it was drawn against. On a sleeping creature's
/// night paper the crest is quieter still, which is fine for a caption.
pub const CREST_INK: [f32; 3] = [0.008, 0.007, 0.006];

/// The crests a permanent wears, packed, from the right.
///
/// Provenance first, because it is the one a table actually wears — tokens
/// are on most boards and commanders on few — so the crest that is usually
/// there is the one anchored to the ledge's end.
///
/// Packed rather than slotted: a lone commander takes the first crest. The
/// column this replaces could not do that, because at seven pixels a shield
/// and a squirrel are both a blob and position was all that told them apart.
/// [`PAPER`] is what took that job over.
#[must_use]
pub fn marks(provenance: crate::board::Provenance, commander: bool) -> [Option<usize>; MAX_CRESTS] {
    let mut out = [None; MAX_CRESTS];
    let mut n = 0;
    let first = match provenance {
        crate::board::Provenance::Printed => None,
        crate::board::Provenance::Token => Some(GLYPH_TOKEN),
        crate::board::Provenance::Copy => Some(GLYPH_COPY),
    };
    if let Some(glyph) = first {
        out[n] = Some(glyph);
        n += 1;
    }
    if commander {
        out[n] = Some(GLYPH_COMMANDER);
    }
    out
}

/// Crest `n`'s square on the card, `[x0, y0, x1, y1]` in card widths from
/// the card's top-left corner, the first at the ledge's right end and the
/// second one gap to its left, both on the ledge's middle line.
#[must_use]
pub const fn crest_rect(n: usize) -> [f32; 4] {
    let x1 = CREST_X1 - (n as f32) * (CREST_W + CREST_GAP);
    let mid = crate::cardplate::ledge_mid();
    [x1 - CREST_W, mid - CREST_W * 0.5, x1, mid + CREST_W * 0.5]
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::board::Provenance;

    #[test]
    fn a_crest_says_what_the_permanent_is_and_nothing_else() {
        assert_eq!(
            marks(Provenance::Token, false),
            [Some(GLYPH_TOKEN), None],
            "a token wears one crest"
        );
        assert_eq!(marks(Provenance::Copy, false), [Some(GLYPH_COPY), None]);
        assert_eq!(marks(Provenance::Printed, false), [None, None]);
        assert_eq!(
            marks(Provenance::Printed, true),
            [Some(GLYPH_COMMANDER), None],
            "a bare commander takes the first crest rather than leaving it empty"
        );
        assert_eq!(
            marks(Provenance::Token, true),
            [Some(GLYPH_TOKEN), Some(GLYPH_COMMANDER)],
            "and provenance stays in front of it"
        );
    }

    /// The packing is only defensible while the papers are three different
    /// colours, so this is the assertion the whole layout rests on.
    ///
    /// Not a spelling check: the three are compared as *hues*, because two
    /// papers a few degrees apart are one paper at the six pixels of frame a
    /// table card shows, whatever their triples look like in a diff.
    #[test]
    fn every_paper_is_its_own_colour() {
        fn hue(rgb: [f32; 3]) -> f32 {
            let [red, green, blue] = rgb;
            let max = red.max(green).max(blue);
            let min = red.min(green).min(blue);
            let chroma = max - min;
            assert!(chroma > 0.02, "{rgb:?} is grey and has no hue to compare");
            let sixth = if (max - red).abs() < f32::EPSILON {
                ((green - blue) / chroma).rem_euclid(6.0)
            } else if (max - green).abs() < f32::EPSILON {
                (blue - red) / chroma + 2.0
            } else {
                (red - green) / chroma + 4.0
            };
            sixth * 60.0
        }
        // Sixty degrees is not a round number picked in advance: the papers
        // were placed first and the closest surviving pair — oxblood against
        // violet since #274 — measures about 100, so this is the bound with
        // air under it.
        const APART: f32 = 60.0;
        for (a, one) in PAPER.iter().enumerate() {
            for (b, other) in PAPER.iter().enumerate().skip(a + 1) {
                let d = (hue(*one) - hue(*other)).abs();
                let d = d.min(360.0 - d);
                assert!(
                    d >= APART,
                    "papers {a} and {b} are {d:.1}° apart: {one:?} and {other:?}"
                );
            }
        }
    }

    /// One ink on three papers, and the bound runs over **every** paper.
    ///
    /// A pair tuned on the paper it happened to be drawn against is the
    /// failure this exists to stop: a crest is not one colour pair but three,
    /// and the third is the one nobody looks at — which is what happened
    /// when the commander's paper went from gilt to oxblood and the slips'
    /// ink dropped to 3.5:1 on it.
    ///
    /// The luminance is taken from the triples **directly** and they are not
    /// decoded first, because the card shader mixes in linear light and
    /// these are what it mixes. Decoding them as if they were display values
    /// is not a rounding difference: it reads an ink at 3.5:1 as 6.3:1, which
    /// is the difference between a pair that passes and a pair that is
    /// written down as passing.
    #[test]
    fn the_ink_reads_on_every_paper() {
        fn luma(rgb: [f32; 3]) -> f32 {
            0.2126 * rgb[0] + 0.7152 * rgb[1] + 0.0722 * rgb[2]
        }
        for (i, paper) in PAPER.iter().enumerate() {
            let (hi, lo) = {
                let (a, b) = (luma(*paper), luma(CREST_INK));
                if a > b { (a, b) } else { (b, a) }
            };
            let ratio = (hi + 0.05) / (lo + 0.05);
            assert!(
                ratio >= 4.5,
                "paper {i} ({paper:?}) holds the ink at only {ratio:.2}:1"
            );
        }
    }

    /// Two crests stand apart, the first at the ledge's right end and the
    /// second exactly one gap to its left — the arithmetic that puts a glyph
    /// over its neighbour if it drifts — and neither reaches the chip.
    #[test]
    fn the_crests_run_in_from_the_right_and_never_overlap() {
        assert!((crest_rect(0)[2] - CREST_X1).abs() < 1e-6);
        for n in 1..MAX_CRESTS {
            let (this, before) = (crest_rect(n), crest_rect(n - 1));
            assert!(
                (before[0] - this[2] - CREST_GAP).abs() < 1e-6,
                "crest {n} is not one gap left of crest {}",
                n - 1
            );
        }
        let last = crest_rect(MAX_CRESTS - 1)[0];
        let chip = crate::cardplate::chip_rect()[2];
        assert!(
            last > chip,
            "the crests reach {last}, over the chip ending at {chip}"
        );
    }

    /// A wrong codepoint draws an empty box and no compiler can see it, so
    /// the table is at least held to being a table.
    #[test]
    fn every_glyph_is_in_the_private_use_block_and_appears_once() {
        for (i, g) in GLYPHS.iter().enumerate() {
            assert!(
                ('\u{e000}'..='\u{f8ff}').contains(g),
                "glyph {i} ({g:?}) is not a private-use codepoint"
            );
            assert_eq!(
                GLYPHS.iter().filter(|o| *o == g).count(),
                1,
                "glyph {i} ({g:?}) is drawn twice"
            );
        }
        // The planeswalker symbol is on the Fan Content Policy's do-not-use
        // table (`docs/legal.md` §2a) and the font draws it at E623.
        assert!(
            !GLYPHS.contains(&'\u{e623}'),
            "the planeswalker symbol is not ours to draw"
        );
    }
}
