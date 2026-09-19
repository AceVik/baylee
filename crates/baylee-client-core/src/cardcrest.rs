//! The identity slips: what a permanent *is*, on a paper tab under its name.
//!
//! Two questions a board cannot otherwise answer, asked of every permanent
//! and true of almost none of them. Is that a commander (CR 903.3)? And is
//! that really a Llanowar Elves — a token with no cardboard behind it at all
//! (CR 111.1), a copy wearing somebody else's face (CR 707.2), or the card
//! it looks like?
//!
//! # Three homes, and why this is the third
//!
//! They were drawn on the card's **top edge** until September 2026 — a crown
//! centred on it, a provenance mark hard against the top-left corner — and
//! the owner's complaint was the obvious one: that edge is the title bar, so
//! a mark there covers the printed name, which is the thing this client
//! repeats everywhere else.
//!
//! They moved to a **column in the right margin**, on the plate's own centre
//! line, so that the whole right-hand corner read as one column. That is the
//! reading the owner rejected next: *„Die Position gefällt mir noch nicht"* —
//! the corner had collected the numbers, what counters did to them, and what
//! the permanent is, and the last of those is not a number.
//!
//! So they are **slips** now: small paper tabs along the card's left margin,
//! clipped under the printed name and hanging into the top corner of the
//! art. A slip is a tab clipped to a document — which is exactly the claim
//! being made, because a commander's shield and a token's squirrel are not
//! printed on the card and never were. It lies *across* the picture rather
//! than beside it for the same reason: a tab that fitted in the air between
//! the name and the art would be too small to read at the seven physical
//! pixels a table card gives it.
//!
//! # What the move bought, and what it spent
//!
//! **Packed, not fixed rows.** The column kept provenance and commander in
//! reserved rows so that position alone told a shield from a squirrel at the
//! seven physical pixels a slot gets on a table card. The slips pack: a lone
//! commander sits in the first slip, where a lone token would. That is only
//! safe because of what replaced position — [`SLIP_PAPER`], a colour per
//! kind. The colour is what pays for the packing, which is the one sentence
//! to keep if the rest of this module is ever rewritten again.
//!
//! **A colour at all** is an override, and the argument it overrides was a
//! good one: every hue in this client is spoken for — the rail tints by
//! keyword, the swing by which way it went, the felt by seat — so a hue here
//! is a claim a player has to look up. What makes it affordable is that a
//! slip is *paper*: the colour is the stock the mark is printed on, not ink
//! added to the mark, and three papers in one place are an alphabet of three
//! rather than a fourth reading of the client's whole palette.
//!
//! **And it moves.** The column deliberately did not: a permanent stops
//! being a commander or a copy only by ceasing to be that permanent
//! (CR 400.7), so a mark that breathed would be making the offer lights'
//! promise about a fact that cannot change. The owner asked for an animation
//! and that argument is withdrawn, not forgotten — which is why the motion
//! is the rarest one on the card ([`SLIP_SHEEN_RATE`], a half-minute apart)
//! and is light moving across paper rather than the mark itself changing.
//!
//! Like [`crate::cardrail`] this module draws nothing. It says where each
//! slip is, which glyph it wears and what it is printed on;
//! `card_common.wgsl` draws it and a mirror test in `baylee-client` reads the
//! WGSL text and fails when the two drift.

/// How many glyphs the slips can draw.
///
/// Three for at most two slips: one of them is a token **or** a copy, which
/// [`crate::board::Provenance`] already makes exclusive.
pub const GLYPH_COUNT: usize = 3;

/// The squirrel: a permanent with no card behind it.
pub const GLYPH_TOKEN: usize = 0;
/// Two cards: a permanent wearing a face that is not its own.
pub const GLYPH_COPY: usize = 1;
/// The shield: a commander.
pub const GLYPH_COMMANDER: usize = 2;

/// How many slips a permanent can ever wear at once.
///
/// A commander that is also a token or a copy is the only pairing the rules
/// allow, so two — and the third glyph is the other half of the first slip.
pub const MAX_SLIPS: usize = 2;

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

/// A mark's square inside its slip, in card widths.
///
/// Deliberately **not** [`crate::cardrail::RAIL_SLOT`], which the column it
/// replaces shared on the argument that the card wears one alphabet. It is
/// smaller because the owner asked for a smaller mark, and it can be smaller
/// because a mark on paper needs less room to read than one on a disc laid
/// over artwork: the paper is the contrast, so the glyph does not have to
/// carry it as size.
pub const SLIP_SLOT: f32 = 0.092;

/// The paper left and right of a mark, in card widths.
///
/// Wider than [`SLIP_PAD_Y`] on purpose, and that is the whole difference
/// between a slip and a badge: a tab is longer than it is deep, and at this
/// size the eye reads the *shape of the paper* long before it resolves what
/// is printed on it.
pub const SLIP_PAD_X: f32 = 0.034;

/// The paper above and below a mark, in card widths.
pub const SLIP_PAD_Y: f32 = 0.016;

/// The air between two slips, in card widths.
pub const SLIP_GAP: f32 = 0.010;

/// How far the first slip's left edge sits from the card's, in card widths.
///
/// [`crate::cardrail::RAIL_INSET`]'s number, because the rail runs along the
/// bottom edge from the same margin and two different insets on one card
/// would read as a drawing that does not line up with itself.
pub const SLIP_INSET: f32 = 0.052;

/// Where a slip's top edge sits, in card widths from the card's top.
///
/// The **top edge** is what this places, and the slip hangs down from it
/// into the top of the art — which is not a compromise but the shape being
/// asked for. A modern frame closes its title bar at about 0.092 of the
/// card's *height* and opens the art immediately: there is no band between
/// them a [`SLIP_H`]-deep tab would fit in, and a tab that fitted in the
/// air above the art would be a tab too small to read. A slip is clipped to
/// a document and lies across what the document says, the same way the rail
/// and the plate lie across the bottom of the art.
///
/// It is a **constant** and does not chase the frame: an old border, a full
/// art card and a saga all put something different at that height, and a
/// slip that moved per printing would be a slip that moved when a player
/// swapped one printing for another.
pub const SLIP_TOP: f32 = 0.151;

/// A slip's paper, in card widths.
pub const SLIP_W: f32 = SLIP_SLOT + 2.0 * SLIP_PAD_X;
/// A slip's depth, in card widths.
pub const SLIP_H: f32 = SLIP_SLOT + 2.0 * SLIP_PAD_Y;

/// The stock each mark is printed on, by its glyph index.
///
/// Three papers, and each says the thing its mark says without being read as
/// ink: gilt is the client's word for *this one of yours* and already rims
/// the viewing seat's mat, verdigris is the colour of a thing conjured
/// rather than printed, and violet is what the swing already uses for a
/// permanent that is not quite what it was.
///
/// **Card stock, not writing paper**, and that is a measurement rather than
/// a taste. These are *linear* values — the card shader mixes in linear
/// light and the framebuffer converts — so the first draft's `0.72, 0.80,
/// 0.76` displayed at 221 of 255, and a sheen mixed 45% toward white on a
/// sheet that pale moves it **16 levels**, which the same shader's own
/// measurements put below the 20 a mark that does not move at all already
/// swings. At 55% of those values the paper displays around 175 and the same
/// sheen moves 44 to 70. A slip that is too pale cannot catch the light.
pub const SLIP_PAPER: [[f32; 3]; GLYPH_COUNT] = [
    [0.396, 0.440, 0.418], // token — verdigris
    [0.429, 0.385, 0.506], // copy — violet
    [0.495, 0.418, 0.220], // commander — gilt
];

/// The ink every mark is printed in, on all three papers.
///
/// Darker than the saga page's `SEPIA`, which is the client's other ink on
/// paper, because these papers are darker than its parchment: `SEPIA` on
/// this stock measures 2.0:1. The pair that ships is 5.6:1 at worst, and
/// [`the ink test`](self) runs over **every** paper rather than the one it
/// was drawn against.
pub const SLIP_INK: [f32; 3] = [0.035, 0.030, 0.025];

/// How far a sheen mixes its paper toward white at the band's centre.
///
/// Measured rather than chosen: at this strength the gilt slip swings 70
/// display levels and the quietest of the three swings 44, against the 20 a
/// still mark already swings from the rail's ink pulse and the 89 the
/// shader's own notes record for a pip that travelled 1.7 pixels. Below
/// about 0.3 it stops being an event and becomes a wriggle.
pub const SLIP_SHEEN: f32 = 0.45;

/// How often a slip catches the light, as a rate over the rail's own beat.
///
/// `card_common.wgsl` runs every impulse through `fract(ph * K)`, whose
/// period is `1 / (BEAT * K)` seconds — 0.8696 / K, and **not** the
/// 5.464 / K it would be if the term were a `sin`. At this rate that is a
/// half-minute, which makes it the rarest motion on the card: the rail's
/// slowest keyword is menace at 24.1 s.
///
/// Rare on purpose. The column this replaced argued that a mark here must
/// not move at all, because what it says cannot change; a sheen every thirty
/// seconds is the smallest thing that answers the owner's request without
/// turning a permanent fact into a signal a player watches for.
pub const SLIP_SHEEN_RATE: f32 = 0.0290;

/// The marks a permanent wears, packed, left to right.
///
/// Provenance first, because it is the one a table actually wears — tokens
/// are on most boards and commanders on few — so the mark that is usually
/// there is the one anchored to the margin it hangs from.
///
/// Packed rather than slotted: a lone commander takes the first slip. The
/// column this replaces could not do that, because at seven pixels a shield
/// and a squirrel are both a blob and position was all that told them apart.
/// [`SLIP_PAPER`] is what took that job over.
#[must_use]
pub fn marks(provenance: crate::board::Provenance, commander: bool) -> [Option<usize>; MAX_SLIPS] {
    let mut out = [None; MAX_SLIPS];
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

/// The left edge of slip `n`, in card widths from the card's left edge.
#[must_use]
pub fn slip_x(n: usize) -> f32 {
    SLIP_INSET + (n as f32) * (SLIP_W + SLIP_GAP)
}

/// How far the slips reach across the card when a permanent wears them all.
#[must_use]
pub fn slips_right() -> f32 {
    slip_x(MAX_SLIPS - 1) + SLIP_W
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::board::Provenance;
    // The rail's, and not `layout`'s: the shader measures the card the rail's
    // way, and these are the constants that have to agree with the shader.
    use crate::cardrail::CARD_ASPECT;

    #[test]
    fn a_slip_says_what_the_permanent_is_and_nothing_else() {
        assert_eq!(
            marks(Provenance::Token, false),
            [Some(GLYPH_TOKEN), None],
            "a token wears one slip"
        );
        assert_eq!(marks(Provenance::Copy, false), [Some(GLYPH_COPY), None]);
        assert_eq!(marks(Provenance::Printed, false), [None, None]);
        assert_eq!(
            marks(Provenance::Printed, true),
            [Some(GLYPH_COMMANDER), None],
            "a bare commander takes the first slip rather than leaving it empty"
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
    /// papers a few degrees apart are one paper at the ten pixels a slip is
    /// drawn at, whatever their triples look like in a diff.
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
        // were placed first and the closest surviving pair measures 106.8,
        // so this is the bound with air under it.
        const APART: f32 = 60.0;
        for (a, one) in SLIP_PAPER.iter().enumerate() {
            for (b, other) in SLIP_PAPER.iter().enumerate().skip(a + 1) {
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
    /// failure this exists to stop: a slip is not one colour pair but three,
    /// and the third is the one nobody looks at.
    ///
    /// The luminance is taken from the triples **directly** and they are not
    /// decoded first, because the card shader mixes in linear light and
    /// these are what it mixes. Decoding them as if they were display values
    /// is not a rounding difference: it reads the shipping ink at 3.5:1 as
    /// 6.3:1, which is the difference between a pair that passes and a pair
    /// that is written down as passing.
    #[test]
    fn the_ink_reads_on_every_paper() {
        fn luma(rgb: [f32; 3]) -> f32 {
            0.2126 * rgb[0] + 0.7152 * rgb[1] + 0.0722 * rgb[2]
        }
        for (i, paper) in SLIP_PAPER.iter().enumerate() {
            let (hi, lo) = {
                let (a, b) = (luma(*paper), luma(SLIP_INK));
                if a > b { (a, b) } else { (b, a) }
            };
            let ratio = (hi + 0.05) / (lo + 0.05);
            assert!(
                ratio >= 4.5,
                "paper {i} ({paper:?}) holds the ink at only {ratio:.2}:1"
            );
        }
    }

    /// A slip is a tab, which means it is wider than it is deep.
    #[test]
    fn a_slip_is_longer_than_it_is_deep() {
        // A `const` block, which clippy asks for and which is the stronger
        // statement anyway: both sides are constants, so this is a claim the
        // *compiler* refuses rather than one a test run reports.
        const {
            assert!(SLIP_W > SLIP_H * 1.2, "a slip is a badge, not a tab");
        }
    }

    /// Clipped under the name, into the top corner of the art, and clear of
    /// the card's other margin.
    ///
    /// The vertical bound is the interesting one and it is *one-and-a-half*
    /// sided, which is the honest shape. Above 0.092 of the card's height
    /// the slip is on the printed name, and that is the fault the whole
    /// module moved away from twice — a hard floor. Below, it may hang into
    /// the art, because a tab does; what it may not do is reach the art's
    /// own middle, where the picture's subject is, so the ceiling is a
    /// quarter of the card and not the art's top edge.
    #[test]
    fn a_slip_is_clipped_under_the_name_and_hangs_into_the_art() {
        // Height fractions of a modern frame, turned into the width units
        // everything here is measured in.
        let name_bottom = 0.092 / CARD_ASPECT;
        let quarter = 0.250 / CARD_ASPECT;
        assert!(
            SLIP_TOP >= name_bottom,
            "a slip at {SLIP_TOP} is still on the printed name, which ends at {name_bottom}"
        );
        assert!(
            SLIP_TOP + SLIP_H <= quarter,
            "a slip reaching {} is into the middle of the picture, which starts at {quarter}",
            SLIP_TOP + SLIP_H
        );
        assert!(
            slips_right() < 1.0 - SLIP_INSET,
            "two slips reach {} and the card's other margin starts at {}",
            slips_right(),
            1.0 - SLIP_INSET
        );
    }

    /// Two slips stand apart, and the second is exactly one gap past the
    /// first — the arithmetic that puts a mark under its neighbour if it
    /// drifts.
    #[test]
    fn the_slips_run_left_to_right_and_never_overlap() {
        for n in 1..MAX_SLIPS {
            assert!(
                slip_x(n) > slip_x(n - 1) + SLIP_W,
                "slip {n} overlaps the one before it"
            );
            assert!(
                (slip_x(n) - slip_x(n - 1) - SLIP_W - SLIP_GAP).abs() < 1e-6,
                "slip {n} is not one gap past slip {}",
                n - 1
            );
        }
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
