//! The frame a card's print sits in (#274).
//!
//! Nothing this client draws lies on the print. Scryfall's image terms ask
//! that a card image is not covered, cropped, tinted or stamped, and the
//! artist's name and the copyright line run along the print's bottom edge —
//! exactly where a keyword rail and a power/toughness plate used to be. So a
//! card is a print in a window, and the paper around the window is ours:
//! what the rules have made the card, what this client offers to do with it,
//! and the numbers are all drawn there.
//!
//! The card keeps its size — 1 × 1/[`CARD_ASPECT`] card widths, one table
//! unit wide — so no lane, pile, hit test or shadow moves. The print shrinks
//! to [`PRINT_SCALE`] of the card's width instead, keeps 63:88, and sits
//! [`FRAME_TOP`] below the card's top edge; what is left below it is the
//! ledge, [`FRAME_FOOT`] deep.
//!
//! This is the Rust half of `card_common.wgsl`'s frame. The shader draws the
//! frame; this is what anything laid out *beside* the shader — the table's
//! world-text face, a badge lying on the card — places itself against, and
//! `cardmat`'s tests hold the two to the same digits.

use crate::cardrail::CARD_ASPECT;

/// How wide the frame is beside the print, left and right, in card widths.
///
/// Wider than the black border printed on a modern card (about 0.045), so
/// the frame reads as the card's own paper and not as a second printed
/// border. 5.7 pixels on a card 94 pixels wide on the felt.
pub const FRAME_SIDE: f32 = 0.061;

/// How deep the frame is above the print, in card widths.
///
/// The thinnest side: nothing is drawn up there but the rim's light. The
/// count a merged card stands for is its own object overhanging the corner
/// rather than something the frame has to make room for.
pub const FRAME_TOP: f32 = 0.045;

/// The print's width as a share of the card's: what the two sides leave.
pub const PRINT_SCALE: f32 = 1.0 - 2.0 * FRAME_SIDE;

/// The card's height, in card widths.
pub const CARD_TALL: f32 = 1.0 / CARD_ASPECT;

/// The print's height, in card widths: it keeps the card's own shape.
pub const PRINT_TALL: f32 = PRINT_SCALE * CARD_TALL;

/// How deep the ledge under the print is, in card widths: what the print and
/// the top leave of the card's height.
pub const FRAME_FOOT: f32 = CARD_TALL - FRAME_TOP - PRINT_TALL;

/// How far in from the card's edge an offer or a deed is lit, in card widths.
///
/// No further than the frame's thinnest side, so the light has faded out
/// before it reaches the window on every side.
pub const OFFER_REACH: f32 = 0.045;

/// The frame's own paper, in linear light: a warm slate, about 150 of 255 on
/// screen. Mid-tone, so it can show a night, a wash and an edge against this
/// felt without out-shouting the print it holds.
pub const FRAME_PAPER: [f32; 3] = [0.30, 0.29, 0.27];

/// A commander's paper: oxblood.
///
/// Not gilt, which is this client's word for "yours" and sits too close to
/// the armed ring and the offer's amber: a commander with an ability to
/// activate is the common case. One constant, so a different answer is one
/// line.
pub const COMMANDER_PAPER: [f32; 3] = [0.44, 0.17, 0.15];

/// How dark the paper goes under a summoning-sick creature, as a factor on
/// linear light. The print is not ours to dim, so the night falls on the
/// frame.
pub const FRAME_NIGHT: f32 = 0.47;

// The ledge is a real ledge: deeper than the top, and deep enough for the
// plate it carries.
const _: () = assert!(FRAME_FOOT > 2.0 * FRAME_TOP);
const _: () = assert!(FRAME_FOOT > crate::cardplate::PLATE_H);
// And the print is still most of the card: the frame is a frame, not a mat
// round a stamp.
const _: () = assert!(PRINT_SCALE > 0.85);

/// The print's window on the card, in card widths from the card's top-left
/// corner: `[x0, y0, x1, y1]`, `y` growing down the card.
#[must_use]
pub const fn window() -> [f32; 4] {
    [
        FRAME_SIDE,
        FRAME_TOP,
        FRAME_SIDE + PRINT_SCALE,
        FRAME_TOP + PRINT_TALL,
    ]
}

/// How far above the card's centre the print's centre sits, in card widths.
///
/// The ledge is deeper than the top, so the print sits high on the card;
/// anything centred on the print — the table's text face — is lifted by
/// this much.
#[must_use]
pub const fn window_lift() -> f32 {
    (FRAME_FOOT - FRAME_TOP) * 0.5
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Linear light to 8-bit display levels, the way the framebuffer
    /// converts it.
    fn levels(c: [f32; 3]) -> [f32; 3] {
        c.map(|v| {
            let s = if v <= 0.003_130_8 {
                v * 12.92
            } else {
                1.055 * v.powf(1.0 / 2.4) - 0.055
            };
            s * 255.0
        })
    }

    fn furthest(a: [f32; 3], b: [f32; 3]) -> f32 {
        let (a, b) = (levels(a), levels(b));
        (0..3).map(|i| (a[i] - b[i]).abs()).fold(0.0, f32::max)
    }

    /// The print keeps a card's shape and the frame closes round it.
    ///
    /// The window is exactly 63:88, so nothing of the print is cropped or
    /// stretched, and the two sides and the print fill the width. (That the
    /// ledge is a real ledge is asserted where the constants are.)
    #[test]
    fn the_print_keeps_its_shape_and_the_frame_closes_round_it() {
        let [x0, y0, x1, y1] = window();
        assert!(((x1 - x0) / (y1 - y0) - CARD_ASPECT).abs() < 1e-6);
        assert!((x0 + (x1 - x0) + FRAME_SIDE - 1.0).abs() < 1e-6);
        assert!((y1 + FRAME_FOOT - CARD_TALL).abs() < 1e-6);
    }

    /// Signed distance to the card's own rounded outline, in card widths:
    /// negative inside. `card_common.wgsl`'s `corner_sdf`, for a point
    /// already in card widths.
    fn card_sdf(x: f32, y: f32) -> f32 {
        const CORNER: f32 = 0.0476;
        let (hx, hy) = (0.5, CARD_TALL * 0.5);
        let (qx, qy) = (
            (x - hx).abs() - (hx - CORNER),
            (y - hy).abs() - (hy - CORNER),
        );
        let outside = qx.max(0.0).hypot(qy.max(0.0));
        outside + qx.max(qy).min(0.0) - CORNER
    }

    /// Everything the ledge carries lies off the print and on the card.
    ///
    /// The Rust half of #274's rule, over every rectangle the frame draws in:
    /// the plate, the chip and both crests. Each must lie wholly below the
    /// window — not overlap it by a hair — and each of its corners inside
    /// the card's rounded outline, so the ledge's end does not hang off the
    /// card either. The shader is held to the same numbers by `cardmat`'s
    /// mirror tests, and to the window itself by the live diff.
    #[test]
    fn nothing_on_the_ledge_lies_on_the_print() {
        use crate::cardcrest::{MAX_CRESTS, crest_rect};
        use crate::cardplate::{chip_rect, plate_rect};
        let [wx0, _, wx1, wy1] = window();
        let mut rects = vec![("the plate", plate_rect()), ("the chip", chip_rect())];
        for n in 0..MAX_CRESTS {
            rects.push(("a crest", crest_rect(n)));
        }
        for (what, [x0, y0, x1, y1]) in rects {
            let clear = y0 >= wy1 || x1 <= wx0 || x0 >= wx1;
            assert!(
                clear,
                "{what} at [{x0}, {y0}, {x1}, {y1}] lies on the print, which ends at {wy1}"
            );
            for (x, y) in [(x0, y0), (x1, y0), (x0, y1), (x1, y1)] {
                assert!(
                    card_sdf(x, y) <= 0.0,
                    "{what} reaches ({x}, {y}), off the card"
                );
            }
        }
    }

    /// An offer is light on the rim and has faded out before the print.
    #[test]
    fn an_offer_fades_before_it_reaches_the_print() {
        for (side, deep) in [
            ("the sides", FRAME_SIDE),
            ("the top", FRAME_TOP),
            ("the ledge", FRAME_FOOT),
        ] {
            assert!(
                OFFER_REACH <= deep + 1e-6,
                "an offer reaches {OFFER_REACH} in, past {side} at {deep}"
            );
        }
    }

    /// The night is a night on every paper a card can be made of, and every
    /// identity is its own paper.
    ///
    /// Twenty display levels is this client's floor for a change a player is
    /// meant to see at a glance. The night has to clear it on the palest and
    /// the darkest paper alike, and each identity has to clear it against the
    /// plain frame, or a token would be told from a card only in the preview.
    #[test]
    fn the_night_and_every_identity_are_seen_on_the_frame() {
        use crate::cardcrest::{GLYPH_COMMANDER, GLYPH_COPY, GLYPH_TOKEN, PAPER};
        let papers = [
            ("plain", FRAME_PAPER),
            ("token", PAPER[GLYPH_TOKEN]),
            ("copy", PAPER[GLYPH_COPY]),
            ("commander", PAPER[GLYPH_COMMANDER]),
        ];
        for (name, paper) in papers {
            let night = paper.map(|v| v * FRAME_NIGHT);
            let drop = furthest(paper, night);
            assert!(drop >= 20.0, "night on {name} paper is {drop} levels");
        }
        for (name, paper) in &papers[1..] {
            let apart = furthest(*paper, FRAME_PAPER);
            assert!(apart >= 20.0, "{name} paper is {apart} levels from plain");
        }
    }
}
