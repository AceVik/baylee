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
//!
//! Above the plate stands the other half of the corner: the **counter chips**,
//! a short column of flat stamped discs, pips to six and numerals from seven.
//! A chip rather than a die, because shape is not a number — nobody reads a d8
//! from a d10 at the size a counter is on a card lying at `CAMERA_LEAN`, and
//! what a player reads off a real die is the numeral on top anyway. Three
//! kinds are drawn and a fourth collapses to `+N`, which is the same honest
//! failure the rail makes when eleven marks share a row: shrink or count, but
//! never hide the tail.
//!
//! And one shape takes the plate away from both: a **saga's chapter** is a
//! page with a roman numeral on it, not a token somebody put on the card.

use crate::board::CardGroup;
use baylee_view::{CounterEntry, CounterKind};

/// Nothing to say: a land, or an artifact that is not a creature.
pub const KIND_NONE: u32 = 0;
/// A creature's power and toughness, with the damage marked on it.
pub const KIND_FIGHT: u32 = 1;
/// A planeswalker's loyalty.
pub const KIND_LOYALTY: u32 = 2;
/// A saga's chapter, drawn as a page rather than as a plate.
pub const KIND_LORE: u32 = 3;

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
    /// A saga: the chapter its lore counters have reached (CR 714.2).
    ///
    /// A page rather than a chip, because a chapter is not a token a player
    /// puts on a card — it is where the card has got to, and the printed saga
    /// frame numbers its chapters down the left edge for exactly that reason.
    Lore(u16),
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
        Self::of_parts(
            group.power,
            group.toughness,
            group.loyalty,
            group.damage,
            &group.counters,
        )
    }

    /// The same, from the five fields a group and a view object both carry.
    fn of_parts(
        power: Option<i16>,
        toughness: Option<i16>,
        loyalty: Option<u16>,
        damage: u16,
        counters: &[CounterEntry],
    ) -> Self {
        if let Some(loyalty) = loyalty {
            return Self::Loyalty(loyalty);
        }
        match (power, toughness) {
            (Some(power), Some(toughness)) => Self::Fight {
                power,
                toughness,
                damage,
            },
            // A creature the view gave only half a body to is not a creature
            // this client will guess the other half of.
            _ => chapter(counters).map_or(Self::None, Self::Lore),
        }
    }

    /// The kind bits, as the shader reads them.
    #[must_use]
    pub const fn kind(self) -> u32 {
        match self {
            Self::None => KIND_NONE,
            Self::Fight { .. } => KIND_FIGHT,
            Self::Loyalty(_) => KIND_LOYALTY,
            Self::Lore(_) => KIND_LORE,
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
            Self::Loyalty(loyalty) | Self::Lore(loyalty) => (i32::from(loyalty), 0, 0),
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

/// A saga's chapter, or `None` for a permanent that is not one.
///
/// Lore counters exist on sagas and nowhere else (CR 714), so no type line is
/// needed to recognise one — which is the only reason this is expressible at
/// all: `CardGroup` carries counters and does not carry subtypes.
fn chapter(counters: &[CounterEntry]) -> Option<u16> {
    counters
        .iter()
        .find(|c| c.kind == CounterKind::Lore && c.count > 0)
        .map(|c| c.count)
}

// ------------------------------------------------------------- counter chips

/// How many chips the column above the plate shows before it starts counting.
pub const CHIPS: usize = 3;

/// Bits one chip is packed into: its tint below, its count above.
pub const CHIP_BITS: u32 = 16;
/// The mask a chip's tint is read through.
pub const CHIP_TINT_MASK: u32 = 0x1f;
/// Where a chip's count sits inside it.
pub const CHIP_COUNT_SHIFT: u32 = 5;
/// The mask a chip's count is read through.
pub const CHIP_COUNT_MASK: u32 = 0x3ff;

/// The largest count a chip draws.
///
/// Three digits is what the numeral path can lay out, so a fourth would be
/// drawn as the wrong number rather than as a wide one — the same failure the
/// plate clamps to avoid. A card with a thousand counters on it is not a
/// board this client is going to render honestly either way.
pub const CHIP_MAX: u16 = 999;

/// An empty chip slot. Zero, so an empty column packs to zero words.
pub const TINT_NONE: u32 = 0;
/// A `+1/+1` counter.
pub const TINT_PLUS: u32 = 1;
/// A `-1/-1` counter.
pub const TINT_MINUS: u32 = 2;
/// A charge counter.
pub const TINT_CHARGE: u32 = 3;
/// A lore counter, on the rare permanent whose plate is not the page.
pub const TINT_LORE: u32 = 4;
/// A time counter (suspend, vanishing).
pub const TINT_TIME: u32 = 5;
/// A level counter.
pub const TINT_LEVEL: u32 = 6;
/// A loyalty counter sitting on something that is not a planeswalker.
pub const TINT_LOYALTY: u32 = 7;
/// A keyword counter.
pub const TINT_KEYWORD: u32 = 8;
/// Everything else, including the engine's custom counters.
pub const TINT_OTHER: u32 = 9;
/// The chip that says how many kinds did not fit.
pub const TINT_MORE: u32 = 10;

/// Which tint a counter is drawn in.
///
/// Colour is the whole of how one chip is told from another at the size a
/// chip is on a card lying at `CAMERA_LEAN` — the count is in pips or
/// numerals, and the *kind* has no other channel left. It is deliberately
/// only half an answer: two players who both know the board can read it, and
/// the full one is the badge tooltip, which names the counter.
#[must_use]
pub const fn tint_of(kind: CounterKind) -> u32 {
    match kind {
        CounterKind::PlusOnePlusOne => TINT_PLUS,
        CounterKind::MinusOneMinusOne => TINT_MINUS,
        CounterKind::Charge => TINT_CHARGE,
        CounterKind::Lore => TINT_LORE,
        CounterKind::Time => TINT_TIME,
        CounterKind::Level => TINT_LEVEL,
        CounterKind::Loyalty => TINT_LOYALTY,
        CounterKind::Lifelink => TINT_KEYWORD,
        // Poison, energy and rad are counters a *player* has. They have no
        // tint of their own because a permanent never wears one, and giving
        // them one would be inventing a colour nothing can show.
        CounterKind::Poison | CounterKind::Energy | CounterKind::Rad | CounterKind::Custom(_) => {
            TINT_OTHER
        }
    }
}

/// The order chips run in, nearest the plate first.
///
/// Fixed rather than the view's order, so a card that gains a second kind of
/// counter does not reshuffle the chips it already had.
fn order_key(kind: CounterKind) -> (u8, u32) {
    let rank = match kind {
        CounterKind::PlusOnePlusOne => 0,
        CounterKind::MinusOneMinusOne => 1,
        CounterKind::Lore => 2,
        CounterKind::Loyalty => 3,
        CounterKind::Charge => 4,
        CounterKind::Level => 5,
        CounterKind::Time => 6,
        CounterKind::Lifelink => 7,
        CounterKind::Poison => 8,
        CounterKind::Energy => 9,
        CounterKind::Rad => 10,
        CounterKind::Custom(_) => 11,
    };
    // Two custom counters would otherwise tie, and a tie is a pair of chips
    // that can swap places between frames for no reason a player can see.
    let id = match kind {
        CounterKind::Custom(id) => id,
        _ => 0,
    };
    (rank, id)
}

/// One counter chip: which counter, and how many there are.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Chip {
    /// Which counter this stands for.
    pub kind: CounterKind,
    /// How many are on the permanent.
    pub count: u16,
}

impl Chip {
    /// The tint this chip is drawn in.
    #[must_use]
    pub const fn tint(self) -> u32 {
        tint_of(self.kind)
    }

    /// The chip as its sixteen bits.
    #[must_use]
    pub fn packed(self) -> u32 {
        packed_chip(self.tint(), self.count)
    }
}

/// A tint and a count as one chip's word.
fn packed_chip(tint: u32, count: u16) -> u32 {
    ((u32::from(count.min(CHIP_MAX)) & CHIP_COUNT_MASK) << CHIP_COUNT_SHIFT) | tint
}

/// The column of chips above the plate.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub struct ChipRow {
    /// The chips drawn, nearest the plate first.
    pub shown: [Option<Chip>; CHIPS],
    /// How many further kinds there was no room for; `0` when they all fit.
    pub more: u8,
}

/// Everything the reserved corner says about one card.
///
/// The plate and the chips are decided together because one of them can
/// silence the other: a saga's page *is* its lore counter, and drawing both
/// would be the same fact twice, once as a number and once as a die.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub struct Corner {
    /// What the plate itself says.
    pub plate: Plate,
    /// What sits above it.
    pub chips: ChipRow,
}

impl Corner {
    /// What one drawn card's corner holds.
    #[must_use]
    pub fn of(group: &CardGroup) -> Self {
        Self::of_parts(
            group.power,
            group.toughness,
            group.loyalty,
            group.damage,
            &group.counters,
        )
    }

    /// The same corner for a single object rather than for a group.
    ///
    /// The hover preview draws one card large and builds it out of the view
    /// rather than out of the board model. Without this it drew the *printed*
    /// numbers: a 2/2 under an anthem said 3/3 on the table and 2/2 in the
    /// preview, which is two numbers for one permanent on one screen.
    #[must_use]
    pub fn of_object(object: &baylee_view::PublicObject) -> Self {
        Self::of_parts(
            object.power,
            object.toughness,
            object.loyalty,
            object.damage,
            &object.counters,
        )
    }

    fn of_parts(
        power: Option<i16>,
        toughness: Option<i16>,
        loyalty: Option<u16>,
        damage: u16,
        counters: &[CounterEntry],
    ) -> Self {
        let plate = Plate::of_parts(power, toughness, loyalty, damage, counters);
        let mut chips: Vec<Chip> = counters
            .iter()
            .filter(|c| c.count > 0 && !silenced(c.kind, plate))
            .map(|c| Chip {
                kind: c.kind,
                count: c.count,
            })
            .collect();
        chips.sort_by_key(|c| order_key(c.kind));
        let mut row = ChipRow::default();
        for (slot, chip) in row.shown.iter_mut().zip(chips.iter().copied()) {
            *slot = Some(chip);
        }
        row.more = u8::try_from(chips.len().saturating_sub(CHIPS)).unwrap_or(u8::MAX);
        Self { plate, chips: row }
    }

    /// The three uniforms the shader reads: the plate, then two chip words.
    ///
    /// Two words rather than one because a chip is a tint *and* a count, and
    /// four of those do not fit in thirty-two bits without capping a count so
    /// low that a proliferate deck would out-count it.
    #[must_use]
    pub fn packed(self) -> [u32; 3] {
        let at = |i: usize| self.chips.shown[i].map_or(0, Chip::packed);
        let more = if self.chips.more == 0 {
            0
        } else {
            packed_chip(TINT_MORE, u16::from(self.chips.more))
        };
        [
            self.plate.packed(),
            at(0) | (at(1) << CHIP_BITS),
            at(2) | (more << CHIP_BITS),
        ]
    }
}

/// Whether the plate already says what this counter says.
///
/// Only ever a saga's page. `+1/+1` and `-1/-1` counters *are* folded into the
/// projected power and toughness, and their chips are still drawn: a 3/3 and a
/// 1/1 wearing two +1/+1 counters are different permanents, and the plate says
/// `3/3` for both. Only the chip says which one dies to a Sudden Spoiling.
const fn silenced(kind: CounterKind, plate: Plate) -> bool {
    matches!((kind, plate), (CounterKind::Lore, Plate::Lore(_)))
}

/// A chip's diameter, in card widths.
pub const CHIP_D: f32 = 0.098;

/// The gap between two chips, and between the first chip and the plate band.
pub const CHIP_GAP: f32 = 0.018;

/// The chip column's centre, in card widths — the plate's own centre, so the
/// corner reads as one column rather than two things near each other.
pub const CHIP_X: f32 = 1.0 - PLATE_INSET - PLATE_W * 0.5;

/// How far above the card's bottom edge the column starts, in card widths.
///
/// Measured from the bottom rather than the top so that the shader can build
/// it out of the same three constants without knowing the card's height —
/// and taken from the plate's *band* rather than from the plate, so the column
/// does not slide down the card when a creature stops being one.
pub const CHIP_BASE: f32 = PLATE_INSET + PLATE_H + CHIP_GAP;

/// The largest count drawn as pips. Above it, a chip shows numerals.
///
/// Six because that is where a die stops, and because a pip pattern is read
/// pre-attentively only while it is a pattern somebody already knows.
pub const PIP_MAX: u16 = 6;

/// The six die faces, a bit per cell of a 3×3 grid, bit `row * 3 + (2 - col)`
/// — the same "written as it is drawn" convention as [`GLYPHS`].
pub const PIPS: [u32; 6] = [
    0b000_010_000, // 1
    0b100_000_001, // 2
    0b100_010_001, // 3
    0b101_000_101, // 4
    0b101_010_101, // 5
    0b101_101_101, // 6
];

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
pub const GLYPHS: [u32; 15] = [
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
    glyph([0b0000, 0b0100, 0b1110, 0b0100, 0b0000, 0b0000]), // plus
    glyph([0b1110, 0b0100, 0b0100, 0b0100, 0b0100, 0b1110]), // roman I
    glyph([0b1001, 0b1001, 0b1001, 0b1001, 0b0110, 0b0100]), // roman V
];

/// The index of `−` in [`GLYPHS`].
pub const GLYPH_MINUS: usize = 10;
/// The index of `/` in [`GLYPHS`].
pub const GLYPH_SLASH: usize = 11;
/// The index of `+` in [`GLYPHS`], which only the overflow chip draws.
pub const GLYPH_PLUS: usize = 12;
/// The index of the roman `I` in [`GLYPHS`].
pub const GLYPH_I: usize = 13;
/// The index of the roman `V` in [`GLYPHS`].
///
/// Serifed, and not the arabic `1` with its flag and foot: a saga's chapter is
/// a roman numeral on every card that prints one, and `III` written in arabic
/// ones would read as one hundred and eleven.
pub const GLYPH_V: usize = 14;

/// The largest chapter drawn in roman numerals.
///
/// Five, which is one past the longest saga printed. A sixth chapter — or a
/// lore counter put somewhere strange by a card that says so — falls back to
/// the arabic numerals the plate already draws, because `VI` needs a second
/// composition rule and an honest number beats a pretty one.
pub const ROMAN_MAX: u16 = 5;

/// Packs six drawn rows into the word the shader reads.
const fn glyph(rows: [u32; GLYPH_H as usize]) -> u32 {
    rows[0] | (rows[1] << 4) | (rows[2] << 8) | (rows[3] << 12) | (rows[4] << 16) | (rows[5] << 20)
}

#[cfg(test)]
mod tests {
    use super::*;
    use baylee_core::ids::ObjectId;
    use baylee_view::ObjectStatus;

    fn counted(kinds: &[(CounterKind, u16)]) -> Vec<CounterEntry> {
        kinds
            .iter()
            .map(|&(kind, count)| CounterEntry { kind, count })
            .collect()
    }

    fn with_counters(counters: Vec<CounterEntry>) -> CardGroup {
        CardGroup {
            counters,
            ..group(None, None, None)
        }
    }

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

    /// Every pip pattern is the die face it claims to be.
    #[test]
    fn a_pip_face_has_as_many_pips_as_it_says() {
        for (i, face) in PIPS.iter().enumerate() {
            assert_eq!(
                face.count_ones() as usize,
                i + 1,
                "the {} face has the wrong number of pips",
                i + 1
            );
            assert_eq!(face >> 9, 0, "the {} face runs out of its grid", i + 1);
        }
        // A die is symmetric under a half turn, which is what makes the
        // patterns readable without counting: rotating the grid 180° has to
        // give the same face back. It is also the cheapest check that no
        // literal above was typed one cell off.
        for (i, face) in PIPS.iter().enumerate() {
            let mut turned = 0u32;
            for bit in 0..9 {
                if (face >> bit) & 1 == 1 {
                    turned |= 1 << (8 - bit);
                }
            }
            assert_eq!(turned, *face, "the {} face is not symmetric", i + 1);
        }
    }

    /// A saga has no body, so the corner is free for its chapter — and the
    /// chapter is then *not* also a chip, because that is one fact twice.
    #[test]
    fn a_saga_wears_its_chapter_as_a_page_and_not_as_a_counter() {
        let saga = with_counters(counted(&[(CounterKind::Lore, 2)]));
        let corner = Corner::of(&saga);
        assert_eq!(corner.plate, Plate::Lore(2));
        assert_eq!(corner.chips, ChipRow::default());
        assert_eq!(corner.plate.packed() >> KIND_SHIFT, KIND_LORE);

        // A saga that is also a creature is a creature: the plate says what
        // it dies to, and the chapter goes back to being a chip.
        let creature = CardGroup {
            power: Some(3),
            toughness: Some(4),
            ..saga
        };
        let corner = Corner::of(&creature);
        assert!(matches!(corner.plate, Plate::Fight { .. }));
        assert_eq!(
            corner.chips.shown[0].map(|c| c.kind),
            Some(CounterKind::Lore)
        );
    }

    /// The two counters that change the printed numbers are still drawn.
    ///
    /// Which is the whole question this test exists for: a 3/3 and a 1/1
    /// wearing two `+1/+1` counters both plate as `3/3`, and a client that
    /// dropped the chip would be showing two different permanents identically.
    #[test]
    fn a_counter_that_moved_the_numbers_is_still_a_chip() {
        for kind in [CounterKind::PlusOnePlusOne, CounterKind::MinusOneMinusOne] {
            let body = CardGroup {
                counters: counted(&[(kind, 2)]),
                ..group(Some(3), Some(3), None)
            };
            let corner = Corner::of(&body);
            assert_eq!(
                corner.chips.shown[0],
                Some(Chip { kind, count: 2 }),
                "{kind:?} lost its chip"
            );
        }
    }

    /// Three fit, a fourth kind is counted, and a count of zero is not a chip.
    #[test]
    fn a_fourth_kind_is_counted_rather_than_hidden() {
        let many = with_counters(counted(&[
            (CounterKind::Charge, 3),
            (CounterKind::Time, 1),
            (CounterKind::Level, 2),
            (CounterKind::Lifelink, 1),
            (CounterKind::Custom(7), 4),
            (CounterKind::Poison, 0),
        ]));
        let corner = Corner::of(&many);
        assert_eq!(corner.chips.more, 2);
        let kinds: Vec<_> = corner
            .chips
            .shown
            .iter()
            .flatten()
            .map(|c| c.kind)
            .collect();
        // Fixed order, nearest the plate first — never the view's order.
        assert_eq!(
            kinds,
            vec![CounterKind::Charge, CounterKind::Level, CounterKind::Time]
        );
        let [_, _, b] = corner.packed();
        assert_eq!((b >> CHIP_BITS) & CHIP_TINT_MASK, TINT_MORE);
        assert_eq!((b >> (CHIP_BITS + CHIP_COUNT_SHIFT)) & CHIP_COUNT_MASK, 2);
    }

    /// Chips round-trip through their two words, and an empty column is zero.
    #[test]
    fn every_chip_survives_the_packing() {
        let three = with_counters(counted(&[
            (CounterKind::PlusOnePlusOne, 1),
            (CounterKind::Charge, 12),
            (CounterKind::Time, 4000),
        ]));
        let [_, a, b] = Corner::of(&three).packed();
        let read = |word: u32, half: u32| {
            let chip = (word >> (half * CHIP_BITS)) & 0xffff;
            (
                chip & CHIP_TINT_MASK,
                (chip >> CHIP_COUNT_SHIFT) & CHIP_COUNT_MASK,
            )
        };
        assert_eq!(read(a, 0), (TINT_PLUS, 1));
        assert_eq!(read(a, 1), (TINT_CHARGE, 12));
        // Clamped, not wrapped: 4000 & 0x3ff would be 928, a smaller number
        // that looks exactly as real as the right one.
        assert_eq!(read(b, 0), (TINT_TIME, u32::from(CHIP_MAX)));
        assert_eq!(read(b, 1), (TINT_NONE, 0));
        assert_eq!(Corner::of(&group(None, None, None)).packed(), [0, 0, 0]);
    }

    /// The column stands above the band the plate is reserved in, and stays
    /// inside the card.
    #[test]
    fn the_chip_column_clears_the_plate_and_the_cards_edge() {
        let height = 1.0 / crate::cardrail::CARD_ASPECT;
        let plate_top = height - PLATE_INSET - PLATE_H;
        let first = height - CHIP_BASE - CHIP_D * 0.5;
        assert!(
            first + CHIP_D * 0.5 <= plate_top + 1e-6,
            "the first chip overlaps the plate"
        );
        // Four chips, the overflow one included, and the top of the last is
        // still on the card rather than off its edge.
        let last = height - CHIP_BASE - CHIP_D * 0.5 - 3.0 * (CHIP_D + CHIP_GAP);
        assert!(last - CHIP_D * 0.5 > 0.0, "the column runs off the top");
    }
}
