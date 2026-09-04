//! What a card's reserved bottom-right corner says.
//!
//! Three rules facts are printed on a Magic card and drawn nowhere in this
//! client on a card showing art: power, toughness, and the damage marked on a
//! creature. A fourth, a planeswalker's loyalty, is not printed at all — it is
//! a number the game keeps. All four live in the corner
//! [`crate::cardrail`] has been leaving empty since the keyword rail was
//! written, which is why the rail is `RAIL_SPAN` wide rather than the whole
//! bottom edge.
//!
//! The same split as `cardrail`: the plate is *drawn* by the card shader, one
//! more layer on the pipeline that already draws eleven pictograms, and *what
//! it says* is arithmetic that belongs somewhere it can be tested without a
//! GPU. The constants below are the shader's, mirrored, and a test in
//! `baylee-client` reads the WGSL and fails when the two drift.
//!
//! Deliberately not gated on whether the card has artwork. A card drawn as a
//! flat tint is a card whose art has not loaded, and a 4/4 that could block is
//! the thing a player most needs off a card they cannot otherwise read.

use crate::board::CardGroup;

/// Nothing to say: a land, or an artifact that is not a creature.
pub const KIND_NONE: u32 = 0;
/// A creature's power and toughness, with the damage marked on it.
pub const KIND_FIGHT: u32 = 1;
/// A planeswalker's loyalty.
pub const KIND_LOYALTY: u32 = 2;

/// Bits per packed number.
pub const SLOT_BITS: u32 = 10;
/// The mask one packed number is read through.
pub const SLOT_MASK: u32 = 0x3ff;
/// Where the two kind bits sit, above the three numbers.
pub const KIND_SHIFT: u32 = 30;

/// What is added to a number before it is packed.
///
/// Power is genuinely negative on a board — a 2/4 given −3/−0 is a −1/4 that
/// is still standing — and a plate that could not say so would be drawing the
/// one number a player is checking. Ten bits with this bias reach −128 to 895,
/// which is not a range Magic troubles.
pub const BIAS: i32 = 128;

/// The largest number a slot can carry once biased.
const CEILING: i32 = SLOT_MASK as i32 - BIAS;

/// What the corner says about one card.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum Plate {
    /// Nothing at all, and the corner stays empty.
    #[default]
    None,
    /// A creature: its projected power and toughness, and marked damage.
    Fight {
        /// Projected power, after every layer.
        power: i16,
        /// Projected toughness, likewise.
        toughness: i16,
        /// Damage marked this turn, which the plate draws as a rising fill
        /// rather than as a third number — see [`Plate::packed`].
        damage: u16,
    },
    /// A planeswalker: the loyalty it currently has.
    Loyalty(u16),
}

impl Plate {
    /// What one drawn card says.
    ///
    /// Loyalty wins over power and toughness, which matters for exactly one
    /// shape — a planeswalker that is also a creature, animated or printed
    /// that way. It is the right way round: loyalty is what that permanent
    /// dies to, and its power says nothing about how close it is. The P/T is
    /// still one held modifier away, on the card's own text face.
    #[must_use]
    pub fn of(group: &CardGroup) -> Self {
        if let Some(loyalty) = group.loyalty {
            return Self::Loyalty(loyalty);
        }
        match (group.power, group.toughness) {
            (Some(power), Some(toughness)) => Self::Fight {
                power,
                toughness,
                damage: group.damage,
            },
            // A creature the view gave only half a body to is not a creature
            // this client will guess the other half of.
            _ => Self::None,
        }
    }

    /// The kind bits, as the shader reads them.
    #[must_use]
    pub const fn kind(self) -> u32 {
        match self {
            Self::None => KIND_NONE,
            Self::Fight { .. } => KIND_FIGHT,
            Self::Loyalty(_) => KIND_LOYALTY,
        }
    }

    /// The whole plate as the single `u32` that rides in `CardParams`.
    ///
    /// Three ten-bit numbers and two kind bits, which is exactly thirty-two.
    /// One uniform rather than four is not a saving for its own sake: the
    /// browser build is WebGL2, every uniform is a member of a block that is
    /// re-uploaded whole, and a plate is a thing that changes on the frame a
    /// creature is blocked.
    ///
    /// Damage is a number here and a *fill* on the card: the plate reads
    /// `2/4` and fills from the bottom to `damage / toughness`, so what a
    /// player reads off it is how close to lethal the creature is rather than
    /// an arithmetic problem in two numerals. It is also the one thing on the
    /// plate that is not printed on a real card, so making it look different
    /// from the printed numbers is the honest treatment.
    ///
    /// An empty corner is the word `0`, and not merely a word whose kind bits
    /// say so. `CardLook` defaults this field to zero for every surface that
    /// draws no body — a card in hand, in a browser, in the printing picker —
    /// and a land on the table that packed its three biases into it would be a
    /// second material for a card that looks exactly the same.
    #[must_use]
    pub fn packed(self) -> u32 {
        let (a, b, c) = match self {
            Self::None => return 0,
            Self::Fight {
                power,
                toughness,
                damage,
            } => (i32::from(power), i32::from(toughness), i32::from(damage)),
            Self::Loyalty(loyalty) => (i32::from(loyalty), 0, 0),
        };
        (self.kind() << KIND_SHIFT)
            | slot(a)
            | (slot(b) << SLOT_BITS)
            | (slot(c) << (SLOT_BITS * 2))
    }
}

/// One number, biased and clamped into its ten bits.
///
/// Clamped rather than wrapped, because the failure a wrap produces is a 40/40
/// drawn as a 1/1 — a number that is wrong and looks right, which is the worst
/// thing a board can show a player.
fn slot(value: i32) -> u32 {
    (value.clamp(-BIAS, CEILING) + BIAS) as u32 & SLOT_MASK
}

/// How far in from the printed edge the plate sits, in card widths.
///
/// The rail's own inset, so the two rows share a baseline and a margin.
pub const PLATE_INSET: f32 = crate::cardrail::RAIL_INSET;

/// The plate's width, in card widths.
///
/// Exactly what the rail left: `RAIL_INSET + RAIL_SPAN` is 0.752 and the plate
/// runs from there to `1 - RAIL_INSET`. The reserved fifth of §1.4 was
/// reserved to this number, and taking a hair more would push a card with
/// eleven keywords into it.
pub const PLATE_W: f32 = 1.0 - 2.0 * crate::cardrail::RAIL_INSET - crate::cardrail::RAIL_SPAN;

/// The plate's height, in card widths — the rail's slot, so both rows are one
/// band along the bottom of the card rather than two things at two heights.
pub const PLATE_H: f32 = crate::cardrail::RAIL_SLOT;

/// The margin inside the plate, in card widths.
pub const PLATE_PAD: f32 = 0.014;

/// The plate has room for a glyph once its own margin is taken out of it.
///
/// A compile-time assertion rather than a test, because all three of these are
/// constants: a padding raised past the height would leave the shader dividing
/// by a negative and drawing nothing at all, which is the failure that looks
/// exactly like "this card has no body".
const _: () = assert!(PLATE_H > 2.0 * PLATE_PAD + 0.02);

/// A glyph cell's grid, which is what the packed glyphs below are drawn on.
pub const GLYPH_W: u32 = 4;
/// Rows in a glyph.
pub const GLYPH_H: u32 = 6;

/// The glyphs, packed a row per nibble, bit `3 - column` within it — which is
/// what lets each literal below be read as the picture it draws.
///
/// A stencil rather than a typeface, and for the same reason the felt is value
/// noise and the marks are signed distance fields: ornament is the easiest
/// thing to borrow by accident and arithmetic borrows nothing
/// (`docs/legal.md` §2). It is also the only kind of glyph the card pipeline
/// can draw at all — there is no text on the 3D table, Bevy has no 3D text,
/// and projecting a UI numeral onto a card would have to chase its tap
/// rotation, its hover lift and its place in a stack every frame.
pub const GLYPHS: [u32; 12] = [
    glyph([0b0110, 0b1001, 0b1001, 0b1001, 0b1001, 0b0110]), // 0
    glyph([0b0100, 0b1100, 0b0100, 0b0100, 0b0100, 0b1110]), // 1
    glyph([0b0110, 0b1001, 0b0001, 0b0010, 0b0100, 0b1111]), // 2
    glyph([0b1110, 0b0001, 0b0110, 0b0001, 0b1001, 0b0110]), // 3
    glyph([0b0010, 0b0110, 0b1010, 0b1111, 0b0010, 0b0010]), // 4
    glyph([0b1111, 0b1000, 0b1110, 0b0001, 0b1001, 0b0110]), // 5
    glyph([0b0110, 0b1000, 0b1110, 0b1001, 0b1001, 0b0110]), // 6
    glyph([0b1111, 0b0001, 0b0010, 0b0010, 0b0100, 0b0100]), // 7
    glyph([0b0110, 0b1001, 0b0110, 0b1001, 0b1001, 0b0110]), // 8
    glyph([0b0110, 0b1001, 0b1001, 0b0111, 0b0001, 0b0110]), // 9
    glyph([0b0000, 0b0000, 0b0000, 0b1110, 0b0000, 0b0000]), // minus
    glyph([0b0001, 0b0001, 0b0010, 0b0100, 0b1000, 0b1000]), // slash
];

/// The index of `−` in [`GLYPHS`].
pub const GLYPH_MINUS: usize = 10;
/// The index of `/` in [`GLYPHS`].
pub const GLYPH_SLASH: usize = 11;

/// Packs six drawn rows into the word the shader reads.
const fn glyph(rows: [u32; GLYPH_H as usize]) -> u32 {
    rows[0] | (rows[1] << 4) | (rows[2] << 8) | (rows[3] << 12) | (rows[4] << 16) | (rows[5] << 20)
}

#[cfg(test)]
mod tests {
    use super::*;
    use baylee_core::ids::ObjectId;
    use baylee_view::ObjectStatus;

    fn group(power: Option<i16>, toughness: Option<i16>, loyalty: Option<u16>) -> CardGroup {
        CardGroup {
            representative: ObjectId::new(1, 0),
            members: vec![ObjectId::new(1, 0)],
            name: "x".into(),
            power,
            toughness,
            damage: 0,
            loyalty,
            status: ObjectStatus::default(),
            counters: Vec::new(),
            badges: Vec::new(),
            art: None,
            is_token: false,
            summoning_sick: false,
            activatable: false,
            individual: None,
        }
    }

    #[test]
    fn a_creature_shows_its_body_and_a_land_shows_nothing() {
        assert_eq!(
            Plate::of(&group(Some(2), Some(3), None)),
            Plate::Fight {
                power: 2,
                toughness: 3,
                damage: 0
            }
        );
        assert_eq!(Plate::of(&group(None, None, None)), Plate::None);
        // Half a body is not a body. The view has no shape that produces this,
        // and the plate still refuses to invent the other half.
        assert_eq!(Plate::of(&group(Some(2), None, None)), Plate::None);
    }

    /// The one precedence question, and the reason it goes that way.
    #[test]
    fn an_animated_planeswalker_shows_its_loyalty_and_not_its_power() {
        assert_eq!(
            Plate::of(&group(Some(5), Some(5), Some(3))),
            Plate::Loyalty(3)
        );
    }

    /// Round-trips through the packing, which the shader is the only other
    /// reader of — so this test is the only thing on this side that can catch
    /// a slot boundary put in the wrong place.
    #[test]
    fn every_number_survives_the_packing() {
        let unpack = |word: u32, i: u32| ((word >> (SLOT_BITS * i)) & SLOT_MASK) as i32 - BIAS;
        for (power, toughness, damage) in [
            (0i16, 1i16, 0u16),
            (2, 2, 1),
            (-1, 4, 3),
            (13, 13, 12),
            (99, 99, 99),
        ] {
            let word = Plate::Fight {
                power,
                toughness,
                damage,
            }
            .packed();
            assert_eq!(word >> KIND_SHIFT, KIND_FIGHT);
            assert_eq!(unpack(word, 0), i32::from(power));
            assert_eq!(unpack(word, 1), i32::from(toughness));
            assert_eq!(unpack(word, 2), i32::from(damage));
        }
        let word = Plate::Loyalty(7).packed();
        assert_eq!(word >> KIND_SHIFT, KIND_LOYALTY);
        assert_eq!(unpack(word, 0), 7);
        assert_eq!(Plate::None.packed(), 0);
    }

    /// A number too big to pack comes out big, never small.
    #[test]
    fn an_absurd_number_clamps_rather_than_wrapping() {
        let word = Plate::Fight {
            power: 9999,
            toughness: 9999,
            damage: 0,
        }
        .packed();
        assert_eq!(((word & SLOT_MASK) as i32) - BIAS, CEILING);
        let word = Plate::Fight {
            power: -9999,
            toughness: 1,
            damage: 0,
        }
        .packed();
        assert_eq!(((word & SLOT_MASK) as i32) - BIAS, -BIAS);
    }

    /// The plate ends exactly where the rail's inset does, and starts exactly
    /// where its span does. Both are asserted rather than assumed, because the
    /// rail's constants are what reserved this corner in the first place.
    #[test]
    fn the_plate_is_what_the_rail_left() {
        let start = 1.0 - crate::cardrail::RAIL_INSET - PLATE_W;
        assert!(
            (start - (crate::cardrail::RAIL_INSET + crate::cardrail::RAIL_SPAN)).abs() < 1e-6,
            "the plate starts at {start}, the rail ends at {}",
            crate::cardrail::RAIL_INSET + crate::cardrail::RAIL_SPAN
        );
    }

    /// Every glyph is drawn inside its own grid.
    ///
    /// The literals above are read as pictures, which is what makes them
    /// legible and also what makes a stray fifth column easy to type. Four
    /// bits per row is all the shader will read, so a fifth would vanish into
    /// the neighbouring row instead of failing.
    #[test]
    fn no_glyph_runs_out_of_its_cell() {
        for (i, word) in GLYPHS.iter().enumerate() {
            assert_eq!(word >> (4 * GLYPH_H), 0, "glyph {i} has a seventh row");
            for row in 0..GLYPH_H {
                let bits = (word >> (row * 4)) & 0xf;
                assert!(bits <= 0b1111, "glyph {i} row {row}");
            }
        }
        // And they are all distinct, which catches the copy-paste that gives
        // two digits one picture — the only way this table can be wrong and
        // still look right on a board of 2/2s.
        for (i, a) in GLYPHS.iter().enumerate() {
            for (j, b) in GLYPHS.iter().enumerate().skip(i + 1) {
                assert_ne!(a, b, "glyphs {i} and {j} draw the same picture");
            }
        }
    }
}
