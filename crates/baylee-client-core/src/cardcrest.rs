//! The identity column: what a permanent *is*, up the card's right edge.
//!
//! Two questions a board cannot otherwise answer, asked of every permanent
//! and true of almost none of them. Is that a commander (CR 903.3)? And is
//! that really a Llanowar Elves — a token with no cardboard behind it at all
//! (CR 111.1), a copy wearing somebody else's face (CR 707.2), or the card
//! it looks like?
//!
//! Both were drawn on the card's **top edge** until September 2026 — a crown
//! centred on it, a provenance mark hard against the top-left corner — and
//! the owner's complaint was the obvious one: that edge is the title bar, so
//! a mark there covers the printed name, which is the thing this client
//! repeats everywhere else. They moved here, to a column the swing's own
//! comment had already named: the plate, the swing above it, and these above
//! that, all on one centre line in the card's right margin. What that
//! margin costs is the ragged right of the rules text, which is the cheapest
//! text on a card at table scale and is not read there at all.
//!
//! Like [`crate::cardrail`] this module draws nothing. It says where the
//! column is and which glyph each row wears; `card_common.wgsl` draws it and
//! a mirror test in `baylee-client` reads the WGSL text and fails when the
//! two drift.

use crate::cardplate::{PLATE_H, PLATE_INSET, PLATE_W, SWING_GAP, SWING_H};
use crate::cardrail::CARD_ASPECT;

/// The rows of the column, bottom to top.
///
/// **Fixed rows, not a packed list**, which is the one decision here worth
/// stating. The rail packs — twelve keywords shrink to fit and a creature
/// with three marks wears them side by side from the left — because there
/// the marks are a *set* and a gap in it would mean nothing. These two are
/// not a set: they answer different questions, and at the seven physical
/// pixels a slot gets on a table card a shield and a squirrel are both a
/// blob. Position is the only thing left that tells them apart, so a token
/// sits in the same place whether or not the permanent is also a commander,
/// and a bare commander leaves the lower row empty rather than sliding down
/// into it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Row {
    /// Nearest the plate: what the permanent is made of.
    ///
    /// The lower row because it is the one a table actually wears — tokens
    /// are on most boards and commanders on few — so the mark that is
    /// usually there is the one anchored to the corner it hangs from.
    Provenance,
    /// Above it: a commander.
    Commander,
}

/// The rows, in the order their slots run upward from the plate.
pub const ROWS: [Row; 2] = [Row::Provenance, Row::Commander];

/// How many glyphs the column can draw.
///
/// Three for two rows: the lower one is a token **or** a copy, which
/// [`crate::board::Provenance`] already makes exclusive.
pub const GLYPH_COUNT: usize = 3;

/// The squirrel: a permanent with no card behind it.
pub const GLYPH_TOKEN: usize = 0;
/// Two cards: a permanent wearing a face that is not its own.
pub const GLYPH_COPY: usize = 1;
/// The shield: a commander.
pub const GLYPH_COMMANDER: usize = 2;

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

/// A slot's size, in card widths — the rail's own, deliberately.
///
/// One alphabet: same square, same distance field, same plate underneath.
/// A mark that measured itself differently would read as a second design
/// rather than as another letter.
pub const COLUMN_SLOT: f32 = 0.115;

/// The gap between two rows, and between the column and the swing below it.
pub const COLUMN_GAP: f32 = 0.014;

/// Where the column's centre line is, in card widths from the left edge.
///
/// The plate's, so the corner is one column. The swing already sits on it.
pub const COLUMN_X: f32 = 1.0 - PLATE_INSET - PLATE_W * 0.5;

// A *margin*, not a stripe through the middle of the card, and inside the
// printed edge. Both sides of that are constants, so they are checked where
// they are written rather than in a test that could only ever pass.
const _: () = assert!(COLUMN_X - COLUMN_SLOT * 0.5 > 0.75);
const _: () = assert!(COLUMN_X + COLUMN_SLOT * 0.5 < 1.0);

/// The bottom of the lowest row, in card widths from the top edge.
///
/// A **constant**, and that is the point rather than an accident of how it
/// is written. Every term is fixed — the plate's box and the swing's are
/// reserved whether or not either is drawn — so a Treasure token, which has
/// no plate at all and no counters on it, wears its squirrel exactly where
/// a 5/5 with three `+1/+1` counters wears one. A column that started at
/// the top of whatever happened to be below it would move under a creature
/// as it took a counter.
pub const COLUMN_BOTTOM: f32 =
    1.0 / CARD_ASPECT - PLATE_INSET - PLATE_H - SWING_GAP - SWING_H - COLUMN_GAP;

/// The bottom of row `n`, in card widths from the top edge.
#[must_use]
pub fn row_bottom(n: usize) -> f32 {
    COLUMN_BOTTOM - (n as f32) * (COLUMN_SLOT + COLUMN_GAP)
}

/// The glyph row `n` wears, given what the permanent is.
///
/// `None` for a row this permanent has nothing to put in.
#[must_use]
pub fn glyph_at(n: usize, provenance: crate::board::Provenance, commander: bool) -> Option<usize> {
    match ROWS.get(n)? {
        Row::Provenance => match provenance {
            crate::board::Provenance::Printed => None,
            crate::board::Provenance::Token => Some(GLYPH_TOKEN),
            crate::board::Provenance::Copy => Some(GLYPH_COPY),
        },
        Row::Commander => commander.then_some(GLYPH_COMMANDER),
    }
}

/// The column's topmost edge, in card widths from the top edge.
#[must_use]
pub fn column_top() -> f32 {
    row_bottom(ROWS.len() - 1) - COLUMN_SLOT
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::board::Provenance;
    use crate::cardrail::RAIL_SLOT;

    #[test]
    fn a_row_says_what_the_permanent_is_and_nothing_else() {
        assert_eq!(glyph_at(0, Provenance::Token, false), Some(GLYPH_TOKEN));
        assert_eq!(glyph_at(0, Provenance::Copy, false), Some(GLYPH_COPY));
        assert_eq!(glyph_at(0, Provenance::Printed, true), None);
        assert_eq!(
            glyph_at(1, Provenance::Printed, true),
            Some(GLYPH_COMMANDER)
        );
        assert_eq!(glyph_at(1, Provenance::Token, false), None);
        assert_eq!(glyph_at(2, Provenance::Token, true), None);
    }

    /// The whole argument for fixed rows: a token does not move when the
    /// permanent turns out to be a commander as well.
    #[test]
    fn a_marks_row_does_not_depend_on_the_other_mark() {
        for commander in [false, true] {
            assert_eq!(
                glyph_at(0, Provenance::Token, commander),
                Some(GLYPH_TOKEN),
                "the token left its row when commander was {commander}"
            );
        }
        for provenance in [Provenance::Printed, Provenance::Token, Provenance::Copy] {
            assert_eq!(
                glyph_at(1, provenance, true),
                Some(GLYPH_COMMANDER),
                "the crest left its row on {provenance:?}"
            );
        }
    }

    #[test]
    fn the_column_is_one_alphabet_with_the_rail() {
        assert!(
            (COLUMN_SLOT - RAIL_SLOT).abs() < f32::EPSILON,
            "the column stopped sharing the rail's slot"
        );
    }

    /// It has to stand clear of the swing below it and inside the card
    /// above it — the two ways a column of a fixed size goes wrong.
    ///
    /// The other two ways are the horizontal ones, and those are constants
    /// on both sides, so they are `const _: () = assert!` beside
    /// [`COLUMN_X`] rather than a test that could only ever pass.
    #[test]
    fn the_column_stands_between_the_swing_and_the_cards_own_edge() {
        let swing_top = 1.0 / CARD_ASPECT - PLATE_INSET - PLATE_H - SWING_GAP - SWING_H;
        assert!(
            COLUMN_BOTTOM < swing_top,
            "the column's foot ({COLUMN_BOTTOM}) is inside the swing ({swing_top})"
        );
        assert!(
            column_top() > 0.0,
            "the column ({}) runs off the top of the card",
            column_top()
        );
    }

    #[test]
    fn the_rows_climb_and_never_overlap() {
        for n in 1..ROWS.len() {
            assert!(
                row_bottom(n) < row_bottom(n - 1) - COLUMN_SLOT,
                "row {n} overlaps the one below it"
            );
            assert!(
                (row_bottom(n - 1) - COLUMN_SLOT - row_bottom(n) - COLUMN_GAP).abs() < 1e-6,
                "row {n} is not one gap above row {}",
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
