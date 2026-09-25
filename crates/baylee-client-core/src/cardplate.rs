//! What a card's numbers say, on the keyword strip lying on its art.
//!
//! Three rules facts are printed on a Magic card: power, toughness and, on a
//! planeswalker, the loyalty it started with. What the game has made of them
//! since (the layers, marked damage, the loyalty it has now) is printed
//! nowhere. #274 took the numbers off the print onto a frame's ledge; #298
//! took the frame away, and the numbers stand at the left end of the keyword
//! strip ([`crate::cardrail`]), an object lying on the art, whenever the
//! print cannot say them itself ([`Corner::shows_plate`]).
//!
//! The same split as `cardrail`: the plate is *drawn* by the strip's shader,
//! and *what it says* is arithmetic that belongs somewhere it can be tested
//! without a GPU. The constants below are the shader's, mirrored, and a test
//! in `baylee-client` reads the WGSL and fails when the two drift.
//!
//! A card drawn as a flat tint is a card whose art has not loaded, and a 4/4
//! that could block is the thing a player most needs off a card they cannot
//! otherwise read: without a print, the plate is always written.
//!
//! Beside the plate stands the **chip**: the swing, the net power and
//! toughness a permanent's ±1/±1 counters add, written out in the same
//! numerals. It replaced a column of stamped chips —
//! pips to six, a colour per kind of counter — which the owner read as
//! saying nothing, and which was hard to defend once the plate underneath it
//! was writing the answer in figures. A green disc with three pips on it is
//! a rebus for `+3/+3`.
//!
//! What that costs is written down in [`counter_swing`]: the kinds of
//! counter that are not ±P/T had a chip each and now have none on the table.
//!
//! And one shape takes the plate away from both: a **saga's chapter** is a
//! page with a roman numeral on it, not a token somebody put on the card.

use crate::board::{CardGroup, KeywordBadge};
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
    /// One uniform rather than four is not a saving for its own sake: every
    /// uniform is a member of a block that is re-uploaded whole, and a plate
    /// is a thing that changes on the frame a creature is blocked.
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

// ------------------------------------------------- what the counters say

/// The net power and toughness a permanent's ±1/±1 counters add.
///
/// `None` when the permanent wears none of them. Every `Plus` and `Minus`
/// counter is folded into one pair, which is both what a player wants to
/// read and what the rules make true: `+1/+1` and `-1/-1` counters annihilate
/// as a state-based action (CR 704.5q), so a permanent never *has* both kinds
/// of the ordinary one to begin with, and the odd `-0/-1` from a Skulk effect
/// simply adds in.
///
/// What stood here was a column of **chips** — flat stamped discs above the
/// plate, pips to six and numerals above that, one per kind of counter, with
/// colour carrying which kind. The owner's reading of it was that it said
/// nothing, and that is fair: a green disc with three pips on it is a
/// rebus for `+3/+3`, and the plate two millimetres below it was already
/// writing the answer out in numerals. This writes the counters out too.
///
/// The cost is named rather than hidden: charge, time, level, loyalty-on-a-
/// non-planeswalker and keyword counters had a chip each and now have none.
/// This used to say they were "still named in full by the card's badge
/// tooltip", which was the half of the trade that paid for them and which
/// was never built — nothing in the client reads [`CounterEntry`] but this
/// function and `crate::cue`, and both fold `Plus`/`Minus` and walk past the
/// rest. So a Wizard Class at level 2 is drawn exactly as one at level 1.
/// `docs/observed-faults.md` 58 is the measurement and the three routes out.
#[must_use]
pub fn counter_swing(counters: &[CounterEntry]) -> Option<(i16, i16)> {
    let mut power = 0i32;
    let mut toughness = 0i32;
    let mut any = false;
    for entry in counters {
        let (p, t) = match entry.kind {
            CounterKind::Plus { power, toughness } => (i32::from(power), i32::from(toughness)),
            CounterKind::Minus { power, toughness } => (-i32::from(power), -i32::from(toughness)),
            _ => continue,
        };
        let n = i32::from(entry.count);
        power += p * n;
        toughness += t * n;
        any = true;
    }
    // A permanent that wears a `+1/+1` and a `-1/-1` at once is one the
    // engine has not yet run state-based actions on. It nets to nothing, and
    // nothing is what is drawn — an empty line rather than `+0/+0`.
    if !any || (power == 0 && toughness == 0) {
        return None;
    }
    let clamp = |v: i32| v.clamp(-BIAS, CEILING) as i16;
    Some((clamp(power), clamp(toughness)))
}

/// Which numerals the corner writes in a colour that is not its own ink.
///
/// Deathtouch and toxic are the two keywords that change what a creature's
/// *numbers mean* rather than what it can do with them: a 1/1 deathtoucher
/// trades with anything, and the number that does it is the power. So the
/// colour goes on the number, not on a thirteenth mark on a strip that has
/// twelve.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum Tone {
    /// The plate's own ink.
    #[default]
    Plain,
    /// Deathtouch: the **power** alone turns, because that is the half of
    /// the body the keyword acts through. A deathtouching creature's
    /// toughness is an ordinary toughness.
    Deadly,
    /// Toxic: both numbers, because toxic replaces what combat damage to a
    /// player does rather than what one number is worth.
    ///
    /// Not reachable yet — `board::keyword_bits` has no toxic bit and the
    /// engine has no toxic rule, so nothing constructs this. It is written
    /// down because the shader arm is the cheap half and leaving a hole in
    /// the enum would make adding the keyword a change to four files.
    Toxic,
}

/// The tone a permanent's keywords ask for.
///
/// Read off the **badges** rather than off the raw keyword word, so that the
/// colour on a number and the mark on the strip can never disagree about
/// whether a creature has the keyword. The preview, which starts from a view
/// object rather than from a board group, goes through
/// [`KeywordBadge::from_bits`] to get here rather than testing a bit itself.
#[must_use]
pub fn tone_of(badges: &[KeywordBadge]) -> Tone {
    if badges.contains(&KeywordBadge::Deathtouch) {
        Tone::Deadly
    } else {
        Tone::Plain
    }
}

// ------------------------------------------------------------- the corner

/// Everything the strip's numbers say about one card.
///
/// The plate, and the chip beside it: what the counters add. Since #298 the
/// strip carries them, and only when the print cannot say the same thing
/// ([`Corner::shows_plate`]).
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub struct Corner {
    /// What the plate itself says.
    pub plate: Plate,
    /// The net ±1/±1 swing, written in the chip.
    pub swing: Option<(i16, i16)>,
    /// The printed body, when the view names one: what the print's own
    /// power and toughness box says.
    pub printed: Option<(i16, i16)>,
    /// Which numerals are not written in the plate's own ink.
    pub tone: Tone,
}

impl Corner {
    /// What one drawn card's corner holds.
    #[must_use]
    pub fn of(group: &CardGroup) -> Self {
        Self {
            plate: Plate::of(group),
            swing: counter_swing(&group.counters),
            printed: group.base_power.zip(group.base_toughness),
            tone: tone_of(&group.badges),
        }
    }

    /// The same corner for a single object rather than for a group.
    ///
    /// The hover preview draws one card large and builds it out of the view
    /// rather than out of the board model. Without this it drew the *printed*
    /// numbers: a 2/2 under an anthem said 3/3 on the table and 2/2 in the
    /// preview, which is two numbers for one permanent on one screen.
    #[must_use]
    pub fn of_object(object: &baylee_view::PublicObject) -> Self {
        Self {
            plate: Plate::of_parts(
                object.power,
                object.toughness,
                object.loyalty,
                object.damage,
                &object.counters,
            ),
            swing: counter_swing(&object.counters),
            printed: object.base_power.zip(object.base_toughness),
            tone: tone_of(&KeywordBadge::from_bits(object.keywords)),
        }
    }

    /// Whether the strip carries the plate (#298).
    ///
    /// The print fills the card again, and a creature's print has its own
    /// power and toughness box: a plate beside it saying the same numbers is
    /// noise on every vanilla creature of the board. So a body is written
    /// only where it says something the print cannot: a body the layers
    /// changed or the view cannot set against a printed one, marked damage,
    /// a card drawn without its print, or a print whose box the next card of
    /// a fanned lane lies on (`covered`). Loyalty and a saga's chapter are
    /// counts the print never had, and are always written.
    #[must_use]
    pub fn shows_plate(&self, print: bool, covered: bool) -> bool {
        match self.plate {
            Plate::None => false,
            Plate::Loyalty(_) | Plate::Lore(_) => true,
            Plate::Fight {
                power,
                toughness,
                damage,
            } => damage > 0 || !print || covered || self.printed != Some((power, toughness)),
        }
    }

    /// The two uniforms the strip's shader reads: the plate, and the swing
    /// with the tone.
    #[must_use]
    pub fn packed(self) -> [u32; 2] {
        let swing = match self.swing {
            None => 0,
            Some((power, toughness)) => {
                SWING_SET | slot(i32::from(power)) | (slot(i32::from(toughness)) << SLOT_BITS)
            }
        };
        [
            self.plate.packed(),
            swing | (self.tone_bits() << TONE_SHIFT),
        ]
    }

    /// The chip's word, which the strip carries ([`crate::cardrail::Strip`]):
    /// the swing without the tone, zero with no swing.
    #[must_use]
    pub fn chip(self) -> u32 {
        self.packed()[1] & (SWING_SET | ((1 << (2 * SLOT_BITS)) - 1))
    }

    /// The plate object's two words ([`plate_rect`]): the plate, and the ink
    /// it writes in, the tone with [`PLATE_NIGHT`] on a creature that cannot
    /// attack yet (CR 302.6).
    #[must_use]
    pub fn plate_words(self, night: bool) -> [u32; 2] {
        [
            self.plate.packed(),
            (self.tone_bits() << TONE_SHIFT) | if night { PLATE_NIGHT } else { 0 },
        ]
    }

    /// The tone, as the two bits the shader switches on.
    const fn tone_bits(self) -> u32 {
        match self.tone {
            Tone::Plain => TONE_PLAIN,
            Tone::Deadly => TONE_DEADLY,
            Tone::Toxic => TONE_TOXIC,
        }
    }
}

/// Set on the swing word when there *is* a swing, so that a `0/0` net and an
/// absent one are different states rather than the same zero.
pub const SWING_SET: u32 = 1 << 20;
/// Where the tone sits in the swing word.
pub const TONE_SHIFT: u32 = 21;
/// The plate writes in its own ink.
pub const TONE_PLAIN: u32 = 0;
/// Deathtouch: the power alone.
pub const TONE_DEADLY: u32 = 1;
/// Toxic: both numbers.
pub const TONE_TOXIC: u32 = 2;
/// Set on the plate's ink word ([`Corner::plate_words`]) when its creature
/// is asleep: the plate writes in moon-grey.
pub const PLATE_NIGHT: u32 = 1;
/// Set on the plate's ink word when the one looking sees its card from the
/// far side, as an opponent's across the table: the plate's face is drawn a
/// half turn round its own middle, so its numbers read upright to them (the
/// PO, 25.09: a `6/1` upside down reads `1/9`). The body is as symmetric as
/// the turn, so it covers what it covered.
pub const PLATE_TURNED: u32 = 2;

/// The largest chapter drawn in roman numerals.
///
/// Five, which is one past the longest saga printed. A sixth chapter — or a
/// lore counter put somewhere strange by a card that says so — falls back to
/// the arabic numerals the plate already draws, because `VI` needs a second
/// composition rule and an honest number beats a pretty one.
pub const ROMAN_MAX: u16 = 5;

// ---------------------------------------------------------------- the plate
//
// Since #298 the numbers stand on the keyword strip (`cardrail`), at its left
// end, because a lane fans with each card's own **left** edge exposed
// (`layout::MIN_VISIBLE_FRACTION`): the plate comes first, then the chip.
// These are their sizes; where they stand is the strip's layout.

/// The plate's width, in card widths.
///
/// The width the corner always had. The strip starts it at
/// `cardrail::STRIP_X0 + cardrail::STRIP_PAD` = 0.037, so it ends at 0.233,
/// inside the 0.26 of a card the tightest fan still shows: every creature in
/// a crowded lane shows its body.
pub const PLATE_W: f32 = 0.196;

/// The margin inside the plate, in card widths.
pub const PLATE_PAD: f32 = 0.012;

/// How tall the plate's figures are, in card widths: seven physical pixels
/// on a card 94 wide.
pub const PLATE_CAP: f32 = 0.075;

/// The plate's height, in card widths: its figures and its margin.
pub const PLATE_H: f32 = PLATE_CAP + 2.0 * PLATE_PAD;

/// The air between the plate and the chip, in card widths.
pub const CHIP_GAP: f32 = 0.010;

/// The chip's width, in card widths.
///
/// The chip is what the counters did: the net swing of a permanent's ±1/±1
/// counters, on green stock for growth and violet for a shrink, standing
/// beside the plate.
pub const CHIP_W: f32 = 0.100;

/// How wide the plate is on the strip with the chip beside it, for the
/// plate's `kind` bits: a saga's page is square, every other plate
/// [`PLATE_W`].
#[must_use]
pub const fn plate_width(kind: u32, chip: bool) -> f32 {
    let plate = if kind == KIND_LORE { PLATE_H } else { PLATE_W };
    if chip {
        plate + CHIP_GAP + CHIP_W
    } else {
        plate
    }
}

/// The plate has room for a glyph once its own margin is taken out of it.
const _: () = assert!(PLATE_CAP > 0.02);

/// The characters the corner writes with, in the order their cells sit in the
/// atlas.
///
/// A typeface and no longer a stencil. What stood here was a 4×6 bitmap per
/// glyph, sampled bilinearly — authored when the corner had no other way to
/// put a numeral on a card, and it showed: a stroke was one cell of four, so
/// a `2/2` was six strokes on a grid twenty-three cells wide and the eye read
/// the grid rather than the number. Two complaints came off it together and
/// they were one fault. It looked **blurry**, because a 4×6 mask smoothed up
/// to eleven physical pixels is a blur by construction and has no edge to
/// sharpen. And it looked **off-centre**, because every glyph was given the
/// same four cells: `1` drew its flag in the leftmost two and `/` ran corner
/// to corner, so the ink inside a fixed box sat wherever the picture put it.
///
/// A real face answers both at once — a distance field has an edge at any
/// size, and a glyph's *advance* is what centring a line of type means. The
/// face is `AlegreyaSans-Bold.ttf`, which this client already ships and sets
/// its interface in, under the SIL OFL; nothing new is downloaded and
/// `docs/legal.md` §2 is untouched, because a digit is nobody's trademark.
pub const TEXT_CHARS: [char; 16] = [
    '0', '1', '2', '3', '4', '5', '6', '7', '8', '9', '-', '/', '+', 'I', 'V', '×',
];

/// The index of `-` in [`TEXT_CHARS`].
pub const GLYPH_MINUS: usize = 10;
/// The index of `/` in [`TEXT_CHARS`].
pub const GLYPH_SLASH: usize = 11;
/// The index of `+` in [`TEXT_CHARS`].
pub const GLYPH_PLUS: usize = 12;
/// The index of the roman `I` in [`TEXT_CHARS`].
pub const GLYPH_I: usize = 13;
/// The index of the roman `V` in [`TEXT_CHARS`].
///
/// A saga's chapter is a roman numeral on every card that prints one, and
/// `III` written in arabic ones would read as one hundred and eleven.
pub const GLYPH_V: usize = 14;
/// The index of the multiplication sign in [`TEXT_CHARS`], which a merged
/// card's count opens with ([`count_word`]).
///
/// Load-bearing rather than decoration: the count sits over the printed
/// cost, and a bare `12` there reads as twelve generic mana.
pub const GLYPH_TIMES: usize = 15;

/// The em size one text cell is baked at, in cell units.
///
/// Chosen by the **tallest** glyph rather than by the digits, which is what
/// keeps every cell on one baseline: `/` runs from -115 to 680 font units,
/// 0.795 em, and at this size that is 0.755 of a cell — leaving 0.12 either
/// side, which is more than [`TEXT_RANGE`] needs. Sizing on the digits
/// instead would have pushed the slash through its own wall and cut the one
/// glyph that separates a power from a toughness.
pub const TEXT_EM: f32 = 0.95;

/// Where a text cell's baseline sits, measured down from the cell's top.
///
/// Placed so that a **lining figure** is centred in its cell: the digits run
/// from 0.011 em below the baseline to 0.596 above it, so their middle is
/// 0.2925 em up and the baseline goes that far below the cell's centre. Every
/// other glyph is then hung off the same baseline and lands where type says
/// it should — the hyphen at x-height, the slash overshooting both ways.
pub const TEXT_BASELINE: f32 = 0.5 + 0.2925 * TEXT_EM;

/// How far either side of an outline a text cell's distance reaches, in cell
/// units.
///
/// Much shorter than the rail's [`crate::cardrail::MARK_ORDER`] marks
/// need, and deliberately: a mark is drawn with a halo (`exp(-d * 24)`) and
/// wants field to burn off into, while a numeral is a hard edge and a fill.
/// Spending the margin on range instead of on size would shrink the digits
/// for nothing.
pub const TEXT_RANGE: f32 = 0.10;

/// A lining figure's height, in cell units — what the shader sizes a line of
/// type by.
///
/// The digits' own band (0.607 em from the deepest overshoot to the tallest
/// cap) at [`TEXT_EM`]. A line is scaled so that *this* fills the height it
/// is given, not so that the cell does: a cell is mostly margin, and sizing
/// by it would draw every number two thirds as tall as its plate.
pub const TEXT_CAP: f32 = 0.607 * TEXT_EM;

/// Each glyph's advance, in cell units.
///
/// The digits are **tabular** — one advance for all ten, taken from the
/// `tnum` figures — which is the difference between a plate that grows a
/// creature from `9/9` to `10/10` and one that also jiggles the `/` sideways
/// when a 1 replaces an 8. `lnum` is asked for in the same breath, because
/// this face's default figures are *oldstyle*: `3`, `4`, `5`, `7` and `9`
/// descend a tenth of an em below the baseline, which on a P/T is exactly the
/// crooked look this whole change is here to remove.
///
/// Font units over a thousand, times [`TEXT_EM`].
/// `markatlas::the_advances_are_the_shipped_font_s_own` re-measures every one
/// of them out of the file and is what stops this table drifting from it.
pub const TEXT_ADV: [f32; TEXT_CHARS.len()] = [
    0.475 * TEXT_EM, // 0
    0.475 * TEXT_EM, // 1
    0.475 * TEXT_EM, // 2
    0.475 * TEXT_EM, // 3
    0.475 * TEXT_EM, // 4
    0.475 * TEXT_EM, // 5
    0.475 * TEXT_EM, // 6
    0.475 * TEXT_EM, // 7
    0.475 * TEXT_EM, // 8
    0.475 * TEXT_EM, // 9
    0.304 * TEXT_EM, // -
    0.284 * TEXT_EM, // /
    0.480 * TEXT_EM, // +
    0.295 * TEXT_EM, // I
    0.587 * TEXT_EM, // V
    0.480 * TEXT_EM, // ×
];

/// The fewest permanents a drawn card has to stand for before it says how
/// many: one card standing for one permanent is what every other card is.
pub const COUNT_MIN: u32 = 2;

/// The largest count written out. Three digits is all the shader's
/// `digits_of` writes, and a pile past it says `×999`, which is still more
/// than anybody counts.
pub const COUNT_MAX: u32 = 999;

/// What the count badge says for a card standing for `members` permanents:
/// `0` for none, else the count, clamped to [`COUNT_MAX`].
///
/// The #210 measurement is why this exists: 54 Goblins drew as one Goblin
/// and 16 Plains as one Plains, because a merged card's only cue was the
/// thickness of the slab under it. The badge ([`badge_rect`]) stands at the
/// card's top-right corner, written in the plate's own numerals on the
/// plate's own dark body, so it reads as one of this client's numbers
/// rather than as something printed.
#[must_use]
pub fn count_word(members: usize) -> u32 {
    let n = u32::try_from(members).unwrap_or(u32::MAX);
    if n < COUNT_MIN { 0 } else { n.min(COUNT_MAX) }
}

/// How wide `×count` is at the plate's figure height, padded by the plate's
/// padding on either side, in card widths: `×` and up to three tabular
/// digits.
///
/// What the badge would need for its words. Two digits is all it is given
/// ([`BADGE_W`]); [`badge_cap`] sets a third smaller to fit.
#[must_use]
pub fn count_width(count: u32) -> f32 {
    let digits = match count {
        0..=9 => 1.0,
        10..=99 => 2.0,
        _ => 3.0,
    };
    let cells = TEXT_ADV[GLYPH_TIMES] + digits * TEXT_ADV[0];
    cells * PLATE_CAP / TEXT_CAP + 2.0 * PLATE_PAD
}

/// The count badge's height, in card widths (#261).
///
/// The badge is an object of its own, not paint on the card: `×N` in the
/// plate's ink with its own drop shadow and no plate behind it (the owner,
/// #298), standing at the card's top-right corner ([`BadgePlace`]). Taller
/// than the plate ([`PLATE_H`]) by the air a thing standing proud of the
/// card wants round its figures.
pub const BADGE_H: f32 = 0.12;

/// Where a merged card's count badge stands: at the card's right edge, and
/// over its top edge where the rows leave the room (the owner, 25.09: "put
/// the xN to the right edge and a bit higher").
///
/// Either way it lies on no other card's print (the owner, 25.09;
/// `docs/legal.md` §3) and on nothing of its own card's print but the
/// printed border.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum BadgePlace {
    /// Over the card's top-right corner, clear of its top edge, and upright
    /// whatever the card does: where a row leaves [`BADGE_RISE`] of felt
    /// above its cards, which a duel's rows do (0.19). No card of the row
    /// reaches up there however far the row fans, so a merged card fans
    /// like any other.
    Above,
    /// Off the card's right edge, flush with its top, turning with a tapped
    /// card: where the rows stand too close for [`Self::Above`], 0.009 above
    /// a card at a ring table. The row then holds the gap after a merged
    /// card open ([`crate::layout::HELD_PITCH`]).
    Beside,
}

impl BadgePlace {
    /// Where the badge stands on a row that leaves `margin` of felt above
    /// and below its untapped cards, in card widths.
    #[must_use]
    pub fn for_margin(margin: f32) -> Self {
        if margin >= BADGE_RISE {
            Self::Above
        } else {
            Self::Beside
        }
    }
}

/// Where the badge's left end stands [`BadgePlace::Beside`], in card widths
/// from the card's left edge.
///
/// Its words grow **right** from here, off the card (#298). The shadow's
/// left end stays on the print's own printed border
/// ([`crate::cardrail::PRINTED_BORDER`]), so the mana cost is never under
/// any of it.
pub const BADGE_LEFT: f32 = 1.0 - crate::cardrail::PRINTED_BORDER - BADGE_DROP[0] + BADGE_BLUR;

/// How far below the card's top edge the badge's top stands
/// [`BadgePlace::Beside`], in card widths.
///
/// Flush with the top edge, not over it: a ring table's rows stand
/// `lane_height − CARD_HEIGHT` apart, 0.019, so a badge standing proud of
/// the top edge there would lie on the next row's cards. Down by the
/// shadow's own rise, so the shadow stays below the edge too.
pub const BADGE_TOP: f32 = 0.008;
// The body sits below the card's top edge, not on it.
const _: () = assert!(BADGE_TOP > 0.0);

/// The felt left between a card's top edge and the lowest of its badge's
/// shadow [`BadgePlace::Above`], in card widths: the badge stands clear of
/// its own row, and of the card fanned over its own card's right.
pub const BADGE_AIR: f32 = 0.01;

/// Where the badge's top stands [`BadgePlace::Above`], in card widths down
/// the card: over its top edge by its own height, its shadow's fall and
/// [`BADGE_AIR`].
pub const BADGE_OVER: f32 = -(BADGE_H + BADGE_DROP[1] + BADGE_BLUR + BADGE_AIR);

/// The badge's corner radius: the plate's own proportion of its height.
pub const BADGE_CORNER: f32 = 0.034;

/// Where the badge's shadow falls, `[x, y]` in card widths, `y` down the
/// card: down and a little right, away from the card's own print.
pub const BADGE_DROP: [f32; 2] = [0.006, 0.012];

/// How soft the badge's shadow is: how far past the dropped body it fades
/// out, in card widths.
pub const BADGE_BLUR: f32 = 0.02;

/// The widest the badge grows: `×99`.
///
/// Held there so the overhang is bounded whatever the count. A tapped
/// card's badge beside it turns with it, and its right edge is then its
/// side facing the row nearer the card's player: a ring table leaves a
/// tapped card 0.217 of its lane to that side, and a `×999` at the plate's
/// figure height would reach 0.240 with its shadow and lie on that row's
/// cards. So three digits are set smaller instead ([`badge_cap`]), which is
/// still the true count, and a count that rare is read up close anyway.
pub const BADGE_W: f32 = {
    let cells = TEXT_ADV[GLYPH_TIMES] + 2.0 * TEXT_ADV[0];
    cells * PLATE_CAP / TEXT_CAP + 2.0 * PLATE_PAD
};
// `badge_rect` clamps between the two, and a clamp with its bounds crossed
// panics.
const _: () = assert!(BADGE_H < BADGE_W);

/// The figure height `×count` is set at on the badge: the plate's, or less
/// where that would not fit [`BADGE_W`].
#[must_use]
pub fn badge_cap(count: u32) -> f32 {
    let wants = count_width(count) - 2.0 * PLATE_PAD;
    let room = BADGE_W - 2.0 * PLATE_PAD;
    PLATE_CAP * (room / wants).min(1.0)
}

/// The badge's body for `count` at `place`: `[x0, y0, x1, y1]` in card
/// widths from the card's top-left corner, `y` growing down the card.
///
/// Over the card it ends at the card's right edge and grows left; beside it
/// it starts on the printed border and grows right, off the card.
#[must_use]
pub fn badge_rect(count: u32, place: BadgePlace) -> [f32; 4] {
    let w = count_width(count).clamp(BADGE_H, BADGE_W);
    match place {
        BadgePlace::Above => [1.0 - w, BADGE_OVER, 1.0, BADGE_OVER + BADGE_H],
        BadgePlace::Beside => [BADGE_LEFT, BADGE_TOP, BADGE_LEFT + w, BADGE_TOP + BADGE_H],
    }
}

/// How far the badge reaches past its card's right edge
/// [`BadgePlace::Beside`], shadow and all, in card widths: the widest
/// body's overhang, its shadow's drop and its blur.
///
/// What a row has to leave free after a merged card
/// ([`crate::layout::HELD_PITCH`]), so the badge lies on the felt and on no
/// other card (the owner, 25.09). `the_badge_reaches_as_far_as_its_quad`
/// holds it to [`badge_quad_rect`].
pub const BADGE_REACH: f32 = BADGE_LEFT + BADGE_W + BADGE_DROP[0] + BADGE_BLUR - 1.0;

/// How far the badge stands over its card's top edge [`BadgePlace::Above`],
/// shadow and all, in card widths: the felt a row has to leave above its
/// cards for it ([`BadgePlace::for_margin`]).
pub const BADGE_RISE: f32 = -(BADGE_OVER + BADGE_DROP[1] - BADGE_BLUR);

/// The quad every badge at `place` is drawn in, the same way round as
/// [`badge_rect`]: the widest body and all of its shadow, so one mesh serves
/// every count, and the shader lays the body out from its right end.
#[must_use]
pub fn badge_quad_rect(place: BadgePlace) -> [f32; 4] {
    let [x0, y0, x1, y1] = badge_rect(COUNT_MAX, place);
    let [dx, dy] = BADGE_DROP;
    [
        (x0 + dx - BADGE_BLUR).min(x0),
        (y0 + dy - BADGE_BLUR).min(y0),
        (x1 + dx + BADGE_BLUR).max(x1),
        (y1 + dy + BADGE_BLUR).max(y1),
    ]
}

// ------------------------------------------------------ where the plate stands
//
// The plate was the strip's first cell (#298) until the owner, 25.09, read
// the numbers sitting with the keyword marks as the fault: it stands at the
// card's bottom right now, where the print's own power and toughness box
// is, an object of its own with its own shadow. The chip, the moon and the
// crests stay on the strip. Where it stands is one rule for every pitch of
// its row: as far right as the part of its card that stays in sight allows
// (the PM, 25.09).

/// The printed power and toughness box, and the bottom border's words, as
/// shares of the print: measured on the 112 Scryfall `normal` scans in the
/// art cache, 2003 and 2015 frames, 25.09.
///
/// The box runs from 0.745 to 0.945 of the print's width and from 0.885 to
/// 0.95 of its height on every one; its right end is at most 0.953, where
/// the black border begins.
pub const PRINTED_BOX: [f32; 4] = [0.745, 0.885, 0.945, 0.95];

/// Where the bottom border's words begin, as a share of the print's height:
/// the artist and the collector's line on the left from 0.92, the ©/™ line
/// on the right from 0.945. The plate never reaches down to them.
pub const FOOT_TEXT: f32 = 0.92;

/// The plate's left edge beside the printed box, in card widths: on the
/// black border's inner line, so the plate straddles the border and the
/// box is never under it.
pub const PLATE_BESIDE: f32 = 1.0 - crate::cardrail::PRINTED_BORDER;

/// Where the plate's bottom edge lies on its own card, as a share of the
/// print's height: just above the printed box's top edge.
///
/// The one height that is clear of the box, the artist's line and the ©/™
/// line at any x, so the plate can be right-aligned to whatever part of its
/// card stays in sight without ever covering one of them. What it covers
/// there is the foot of its own card's text box, which the preview reads in
/// full.
pub const PLATE_FOOT: f32 = 0.880;
const _: () = assert!(PLATE_FOOT < PRINTED_BOX[1] && PLATE_FOOT < FOOT_TEXT);

/// The air the plate keeps from the card beside it and from its own card's
/// edge, in card widths.
pub const PLATE_AIR: f32 = 0.012;

/// How far the plate's shadow reaches past its body on each side, in card
/// widths, `[left, top, right, bottom]`: the badge's drop and blur, so the
/// two objects stand at one height over the table.
pub const PLATE_SHADE: [f32; 4] = [
    BADGE_BLUR - BADGE_DROP[0],
    BADGE_BLUR - BADGE_DROP[1],
    BADGE_DROP[0] + BADGE_BLUR,
    BADGE_DROP[1] + BADGE_BLUR,
];

/// The plate's width for its `kind`: a saga's page is square
/// ([`PLATE_H`]), every other plate [`PLATE_W`].
#[must_use]
pub const fn plate_body_width(kind: u32) -> f32 {
    if kind == KIND_LORE { PLATE_H } else { PLATE_W }
}

/// Which of its places the plate stands in.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum PlateSpot {
    /// Beside the printed box, straddling the right-hand border: where the
    /// row leaves the plate and its shadow room before the next card.
    Beside,
    /// On its own card, over the foot of its text box, right-aligned to the
    /// part of the card in sight: in a fanned row, where the next card
    /// reaches the place beside the box.
    OnCard,
    /// Under a tapped card, upright, in its lane's own air.
    Below,
}

/// What a row leaves a card's plate, in card widths along the row, every
/// one measured from where the card's left edge is while it lies untapped.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct PlateRoom {
    /// Where the next card along the row begins, its pile's cards included,
    /// or the lane's end: an untapped card's plate stands clear of it.
    pub right: f32,
    /// The stretch of the lane's air under a tapped card that no print
    /// reaches into: from where the untapped card before it ends to where
    /// the untapped card after it begins, its pile included, or the lane's
    /// ends. A tapped neighbour is a card's width tall and stays out of the
    /// air, so only untapped ones bound it; only a tapped card's plate
    /// reads it.
    pub below: [f32; 2],
}

impl PlateRoom {
    /// A card with nothing beside it: the preview's.
    pub const OPEN: Self = Self {
        right: f32::INFINITY,
        below: [f32::NEG_INFINITY, f32::INFINITY],
    };
}

/// The plate's body: `[x0, y0, x1, y1]` in card widths from the card's
/// top-left corner while it lies untapped, `y` down the card, and which
/// place that is. `None` for a tapped card whose row leaves its plate no
/// room at all: a tapped card between two untapped ones in the tightest
/// fans, whose neighbours' prints fill the air under it.
///
/// A tapped card's plate stands upright (`table::Upright`), so its body is
/// in the same frame: where it would lie beside the card untapped, which is
/// the seat's own. `badge` is a count badge that turns with the card
/// ([`BadgePlace::Beside`]); under a tapped card it hangs where the plate
/// would, so the plate stands to its left.
#[must_use]
pub fn plate_rect(
    kind: u32,
    tapped: bool,
    room: PlateRoom,
    badge: Option<[f32; 4]>,
) -> Option<([f32; 4], PlateSpot)> {
    use crate::cardrail::CARD_TALL;
    let (w, h) = (plate_body_width(kind), PLATE_H);
    if tapped {
        // The tapped card's own box, seen upright: a card's height along the
        // row and its width across it.
        let [left, _, right, foot] = turned([0.0, 0.0, 1.0, CARD_TALL]);
        let mut x1 = right.min(room.below[1] - PLATE_AIR);
        if let Some(badge) = badge {
            x1 = x1.min(turned(badge)[0] - PLATE_AIR);
        }
        // Its shadow too stays off the print before it, which lies under
        // it; the one after lies over it and hides what reaches it.
        let x0 = x1 - w;
        if x0 < left.max(room.below[0] + PLATE_SHADE[0] + PLATE_AIR) {
            return None;
        }
        let y0 = foot + PLATE_AIR;
        return Some(([x0, y0, x1, y0 + h], PlateSpot::Below));
    }
    let mid = f32::midpoint(PRINTED_BOX[1], PRINTED_BOX[3]) * CARD_TALL;
    let beside = [PLATE_BESIDE, mid - 0.5 * h, PLATE_BESIDE + w, mid + 0.5 * h];
    if beside[2] + PLATE_SHADE[2] + PLATE_AIR <= room.right {
        return Some((beside, PlateSpot::Beside));
    }
    // Never off its own card's left border: where a tapped card beside it
    // leaves less than a plate, the plate lies partly under that card rather
    // than on the felt or on another print.
    let x1 = (room.right - PLATE_AIR)
        .min(PLATE_BESIDE)
        .max(crate::cardrail::PRINTED_BORDER + w);
    let y1 = PLATE_FOOT * CARD_TALL;
    Some(([x1 - w, y1 - h, x1, y1], PlateSpot::OnCard))
}

/// Where something lying on a card untapped stands once the card has turned
/// a quarter clockwise to tap, in the same frame: `rect` as
/// [`plate_rect`]'s, and so is the answer.
///
/// For what turns with its card, a count badge beside it, seen from what
/// stays upright, the plate under it.
#[must_use]
pub fn turned(rect: [f32; 4]) -> [f32; 4] {
    use crate::cardrail::CARD_TALL;
    let [x0, y0, x1, y1] = rect;
    // About the card's centre, (0.5, CARD_TALL / 2): the right edge goes
    // down, the top edge right.
    let (cx, cy) = (0.5, 0.5 * CARD_TALL);
    [cx + cy - y1, cy - cx + x0, cx + cy - y0, cy - cx + x1]
}

/// The quad a plate is drawn in: its body `rect` with the shadow round it,
/// as wide as the widest plate and flush with the body's right end, so one
/// mesh serves every plate and the shader lays the body out from the right.
#[must_use]
pub fn plate_quad(rect: [f32; 4]) -> [f32; 4] {
    let [_, y0, x1, y1] = rect;
    let [l, t, r, b] = PLATE_SHADE;
    [x1 - PLATE_W - l, y0 - t, x1 + r, y1 + b]
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::board::{Provenance, Section};
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
            // The printed body is the projected one by default, so a test
            // that is not about the plate's predicate never reads it.
            base_power: power,
            base_toughness: toughness,
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
            provenance: Provenance::Printed,
            original: None,
            summoning_sick: false,
            activatable: false,
            commander: false,
            individual: None,
            proposed: None,
            section: Section::Centre,
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

    /// A saga's page is square and every other plate is the plate's width,
    /// and the chip stands one gap past either.
    #[test]
    fn the_chip_stands_one_gap_past_the_plate() {
        assert!((plate_width(KIND_FIGHT, false) - PLATE_W).abs() < 1e-6);
        assert!((plate_width(KIND_LOYALTY, false) - PLATE_W).abs() < 1e-6);
        assert!((plate_width(KIND_LORE, false) - PLATE_H).abs() < 1e-6);
        for kind in [KIND_FIGHT, KIND_LOYALTY, KIND_LORE] {
            assert!(
                (plate_width(kind, true) - plate_width(kind, false) - CHIP_GAP - CHIP_W).abs()
                    < 1e-6
            );
        }
    }

    /// The named indices name the characters they claim to.
    ///
    /// Five constants point into [`TEXT_CHARS`] and the shader writes with
    /// the numbers, not the names — so a character inserted into that table
    /// renumbers every glyph after it and the plate silently starts drawing
    /// a `+` where it meant a `/`. That is the one failure of this table
    /// that looks like a rendering bug rather than a typo.
    #[test]
    fn the_named_glyphs_are_where_the_table_puts_them() {
        assert_eq!(TEXT_CHARS[GLYPH_MINUS], '-');
        assert_eq!(TEXT_CHARS[GLYPH_SLASH], '/');
        assert_eq!(TEXT_CHARS[GLYPH_PLUS], '+');
        assert_eq!(TEXT_CHARS[GLYPH_I], 'I');
        assert_eq!(TEXT_CHARS[GLYPH_V], 'V');
        // The ten digits are the ten leading cells, which is what lets the
        // shader turn a digit into a cell with one addition.
        for (i, c) in TEXT_CHARS.iter().take(10).enumerate() {
            assert_eq!(*c, char::from_digit(i as u32, 10).unwrap());
        }
    }

    /// Every glyph advances, and the ten digits advance alike.
    ///
    /// Tabular figures are the whole reason the table is written out rather
    /// than derived: a proportional `1` is a fifth narrower than a `0` in
    /// this face, and a creature growing from `9/9` to `10/10` would shunt
    /// its own slash sideways. The advances themselves are pinned against
    /// the shipped file by `markatlas`, which can open it; this end only
    /// knows the shape they have to have.
    #[test]
    fn the_digits_share_one_advance_and_nothing_advances_by_nothing() {
        for (i, a) in TEXT_ADV.iter().enumerate() {
            assert!(*a > 0.0, "glyph {i} has no width");
            assert!(*a < 1.0, "glyph {i} is wider than its cell");
        }
        for (i, a) in TEXT_ADV.iter().enumerate().take(10).skip(1) {
            assert!((a - TEXT_ADV[0]).abs() < 1e-6, "digit {i} is not tabular");
        }
    }

    /// A text cell has room for its glyph and for the field around it.
    ///
    /// The tallest glyph is the slash, 0.795 em, and the baseline is placed
    /// by the *digits* — so the two can disagree, and the way they disagree
    /// is a slash whose tip is flattened against the cell wall with no
    /// distance left to measure into. Both ends are checked, because a
    /// baseline moved to fix the top would push the tail out of the bottom.
    #[test]
    fn the_tallest_glyph_still_clears_its_cell_walls() {
        // Slash, in em above and below the baseline.
        let (up, down) = (0.680, 0.115);
        let top = TEXT_BASELINE - up * TEXT_EM;
        let bottom = TEXT_BASELINE + down * TEXT_EM;
        assert!(top >= TEXT_RANGE, "the slash reaches the cell's top: {top}");
        assert!(
            1.0 - bottom >= TEXT_RANGE,
            "the slash reaches the cell's bottom: {bottom}"
        );
        // And a lining figure is centred, which is what the baseline is for.
        let mid = TEXT_BASELINE - 0.2925 * TEXT_EM;
        assert!((mid - 0.5).abs() < 1e-6, "the figures sit at {mid}");
    }

    /// A saga has no body, so the corner is free for its chapter — and the
    /// chapter is then *not* also a swing, because a lore counter moves no
    /// numbers.
    #[test]
    fn a_saga_wears_its_chapter_as_a_page() {
        let saga = with_counters(counted(&[(CounterKind::Lore, 2)]));
        let corner = Corner::of(&saga);
        assert_eq!(corner.plate, Plate::Lore(2));
        assert_eq!(corner.swing, None);
        assert_eq!(corner.plate.packed() >> KIND_SHIFT, KIND_LORE);

        // A saga that is also a creature is a creature: the plate says what
        // it dies to.
        let creature = CardGroup {
            power: Some(3),
            toughness: Some(4),
            ..saga
        };
        assert!(matches!(Corner::of(&creature).plate, Plate::Fight { .. }));
    }

    /// The counters that moved the printed numbers say by how much.
    ///
    /// Which is the whole question the line exists for: a 3/3 and a 1/1
    /// wearing two `+1/+1` counters both plate as `3/3`, and a corner that
    /// said nothing about them would be showing two different permanents
    /// identically. A `-0/-1` is a minus counter and counts as one.
    #[test]
    fn the_swing_is_the_net_of_every_plus_and_minus_counter() {
        let body = |counters| CardGroup {
            counters,
            ..group(Some(3), Some(3), None)
        };
        assert_eq!(
            Corner::of(&body(counted(&[(CounterKind::PLUS_ONE, 2)]))).swing,
            Some((2, 2))
        );
        assert_eq!(
            Corner::of(&body(counted(&[(CounterKind::MINUS_ONE, 3)]))).swing,
            Some((-3, -3))
        );
        assert_eq!(
            Corner::of(&body(counted(&[
                (CounterKind::PLUS_ONE, 4),
                (
                    CounterKind::Minus {
                        power: 0,
                        toughness: 1
                    },
                    2
                ),
            ])))
            .swing,
            Some((4, 2))
        );
        // Counters that move no numbers are not a swing at all.
        assert_eq!(
            Corner::of(&body(counted(&[
                (CounterKind::Charge, 3),
                (CounterKind::Time, 1)
            ])))
            .swing,
            None
        );
        // Nor is a pair that has not yet annihilated (CR 704.5q). The engine
        // removes them before anybody is asked a question; until it has,
        // `+0/+0` is a truthful nothing and `None` draws nothing.
        assert_eq!(
            Corner::of(&body(counted(&[
                (CounterKind::PLUS_ONE, 1),
                (CounterKind::MINUS_ONE, 1)
            ])))
            .swing,
            None
        );
    }

    /// The strip carries a creature's body only where the print cannot say
    /// it (#298): a body the layers changed, marked damage, no print, or a
    /// print the next card lies on. Loyalty and a chapter always.
    #[test]
    fn the_plate_is_written_only_where_the_print_cannot_say_it() {
        let printed = |power, toughness| CardGroup {
            base_power: Some(2),
            base_toughness: Some(2),
            ..group(Some(power), Some(toughness), None)
        };
        let vanilla = Corner::of(&printed(2, 2));
        assert!(
            !vanilla.shows_plate(true, false),
            "a vanilla 2/2 in the open"
        );
        assert!(
            vanilla.shows_plate(true, true),
            "a 2/2 the next card lies on"
        );
        assert!(vanilla.shows_plate(false, false), "a 2/2 without its print");
        assert!(
            Corner::of(&printed(5, 5)).shows_plate(true, false),
            "a pumped 2/2"
        );
        let hurt = CardGroup {
            damage: 1,
            ..printed(2, 2)
        };
        assert!(
            Corner::of(&hurt).shows_plate(true, false),
            "a 2/2 with damage"
        );
        // A body the view cannot set against a printed one is written.
        let unknown = CardGroup {
            base_power: None,
            base_toughness: None,
            ..group(Some(2), Some(2), None)
        };
        assert!(Corner::of(&unknown).shows_plate(true, false));
        // Loyalty, even at its printed number; nothing, never.
        let walker = CardGroup {
            base_power: Some(5),
            base_toughness: Some(5),
            ..group(Some(5), Some(5), Some(4))
        };
        assert!(Corner::of(&walker).shows_plate(true, false));
        assert!(!Corner::of(&group(None, None, None)).shows_plate(false, true));
    }

    /// Deathtouch colours a number, and only through the strip's own badges.
    #[test]
    fn deathtouch_turns_the_corner_deadly() {
        let mut deadly = group(Some(1), Some(1), None);
        deadly.badges = vec![crate::board::KeywordBadge::Deathtouch];
        assert_eq!(Corner::of(&deadly).tone, Tone::Deadly);

        let mut harmless = group(Some(1), Some(1), None);
        harmless.badges = vec![crate::board::KeywordBadge::Flying];
        assert_eq!(Corner::of(&harmless).tone, Tone::Plain);
    }

    /// The swing and the tone survive their word, and an empty corner is
    /// two zeroes.
    #[test]
    fn the_swing_survives_the_packing() {
        let mut swung = CardGroup {
            counters: counted(&[(CounterKind::MINUS_ONE, 2)]),
            ..group(Some(5), Some(5), None)
        };
        swung.badges = vec![crate::board::KeywordBadge::Deathtouch];
        let [_, word] = Corner::of(&swung).packed();
        assert_ne!(word & SWING_SET, 0, "the word does not say there is one");
        assert_eq!((word & SLOT_MASK) as i32 - BIAS, -2);
        assert_eq!(((word >> SLOT_BITS) & SLOT_MASK) as i32 - BIAS, -2);
        assert_eq!(word >> TONE_SHIFT, TONE_DEADLY);

        // A corner with nothing to say packs to nothing, which is what lets
        // every strip without a plate share one material.
        assert_eq!(Corner::of(&group(None, None, None)).packed(), [0, 0]);
    }

    /// One permanent is a card like every other and says nothing; two and
    /// up say how many, and a pile past three digits says the most three
    /// digits can.
    #[test]
    fn a_card_standing_for_several_says_how_many() {
        assert_eq!(count_word(0), 0);
        assert_eq!(count_word(1), 0, "a lone card wears no count");
        assert_eq!(count_word(2), 2);
        assert_eq!(count_word(54), 54);
        assert_eq!(count_word(999), 999);
        assert_eq!(count_word(4000), COUNT_MAX, "clamped, not wrapped");
    }

    /// Nothing of the badge, body or shadow, reaches the name or the cost
    /// (#298): beside the card its whole quad starts on the print's own
    /// printed border, and over it the whole quad stands above the card's
    /// top edge, for every count, so the rule holds by geometry rather than
    /// by what the shader happens to draw.
    #[test]
    fn nothing_of_the_badge_reaches_past_the_printed_border() {
        let [x0, ..] = badge_quad_rect(BadgePlace::Beside);
        let border = 1.0 - crate::cardrail::PRINTED_BORDER;
        assert!(
            x0 >= border - 1e-6,
            "the badge's quad starts at {x0}, inside the printed border at {border}"
        );
        let [.., y1] = badge_quad_rect(BadgePlace::Above);
        assert!(
            y1 <= -BADGE_AIR + 1e-6,
            "the badge's quad over the card reaches {y1} down it"
        );
        for count in COUNT_MIN..=COUNT_MAX {
            let [x0, _, x1, _] = badge_rect(count, BadgePlace::Beside);
            assert!(x0 >= border, "×{count} starts inside the printed border");
            assert!(x1 > 1.0, "×{count} does not hang off the card");
        }
    }

    /// The room a row leaves after a merged card, and above its cards, is
    /// measured against the quad the badge is drawn in, not against a second
    /// reckoning of it.
    #[test]
    fn the_badge_reaches_as_far_as_its_quad() {
        let [.., x1, _] = badge_quad_rect(BadgePlace::Beside);
        assert!(
            (BADGE_REACH - (x1 - 1.0)).abs() < 1e-6,
            "the quad reaches {} past the card, BADGE_REACH says {BADGE_REACH}",
            x1 - 1.0
        );
        let [_, y0, ..] = badge_quad_rect(BadgePlace::Above);
        assert!(
            (BADGE_RISE + y0).abs() < 1e-6,
            "the quad stands {} over the card, BADGE_RISE says {BADGE_RISE}",
            -y0
        );
    }

    /// And beside the card nothing of it stands above the card's top edge,
    /// where a ring table's next row starts 0.019 later with its power and
    /// toughness.
    #[test]
    fn nothing_of_the_badge_beside_stands_above_the_card() {
        let [_, y0, ..] = badge_quad_rect(BadgePlace::Beside);
        assert!(y0 >= 0.0, "the badge's quad starts {y0} above the card");
    }

    /// Every count fits its badge, and every badge its quad.
    ///
    /// The words are `×` and the digits at [`badge_cap`] with the plate's
    /// padding either side, and the badge is at least as wide as they are:
    /// a count that spilled out of its body would be ink on the felt. Up to
    /// two digits they are the plate's own figures; three are set smaller,
    /// but never so small that a count is a smudge.
    #[test]
    fn every_count_fits_its_badge_and_every_badge_its_quad() {
        for place in [BadgePlace::Above, BadgePlace::Beside] {
            let [qx0, qy0, qx1, qy1] = badge_quad_rect(place);
            let mut last = 0.0;
            for count in COUNT_MIN..=COUNT_MAX {
                let [x0, y0, x1, y1] = badge_rect(count, place);
                let words = (count_width(count) - 2.0 * PLATE_PAD) * badge_cap(count) / PLATE_CAP
                    + 2.0 * PLATE_PAD;
                assert!(
                    words <= x1 - x0 + 1e-5,
                    "×{count} needs {words} and its badge is {}",
                    x1 - x0
                );
                assert!(
                    x0 >= qx0 && y0 >= qy0 && x1 <= qx1 && y1 <= qy1,
                    "×{count} leaves its quad {place:?}"
                );
                assert!(x1 - x0 >= last, "×{count} is narrower than a smaller count");
                last = x1 - x0;
                let cap = badge_cap(count);
                if count < 100 {
                    assert!(
                        (cap - PLATE_CAP).abs() < 1e-6,
                        "×{count} is not the plate's figures"
                    );
                }
                assert!(cap >= 0.7 * PLATE_CAP, "×{count} is set at {cap}");
            }
        }
        // One mesh serves both places.
        let size = |[x0, y0, x1, y1]: [f32; 4]| (x1 - x0, y1 - y0);
        let (a, b) = (
            size(badge_quad_rect(BadgePlace::Above)),
            size(badge_quad_rect(BadgePlace::Beside)),
        );
        assert!((a.0 - b.0).abs() < 1e-6 && (a.1 - b.1).abs() < 1e-6);
    }

    /// Beside the card, a tapped card turns its top-right corner a quarter
    /// clockwise, and the badge with it, so the badge's overhang points at
    /// the row nearer the card's player: a tapped card has
    /// `(lane_height − CARD_WIDTH) / 2` of its lane to that side, and that
    /// row's card stands `(lane_height − CARD_HEIGHT) / 2` beyond. The widest
    /// badge and its shadow stay inside both, at the tightest lane any table
    /// from two to eight seats is laid out with.
    #[test]
    fn a_tapped_card_s_badge_stays_off_the_next_row() {
        use crate::layout::{CARD_HEIGHT, CARD_WIDTH, Seat, TableLayout};
        use baylee_core::ids::PlayerId;
        let lane = (2..=8u8)
            .flat_map(|n| {
                [0.45_f32, 1.0, 1.78, 2.8].map(|aspect| {
                    let seats: Vec<Seat> = (0..n).map(|p| Seat::alone(PlayerId::new(p))).collect();
                    TableLayout::seated(&seats, aspect, None)
                        .slots
                        .iter()
                        .map(crate::layout::SeatSlot::lane_height)
                        .fold(f32::INFINITY, f32::min)
                })
            })
            .fold(f32::INFINITY, f32::min);
        assert!(
            lane > CARD_HEIGHT,
            "the premise: a card fits its lane ({lane})"
        );
        let room = (lane - CARD_WIDTH) * 0.5 + (lane - CARD_HEIGHT) * 0.5;
        assert!(
            BADGE_REACH < room,
            "the badge reaches {BADGE_REACH} past a tapped card and the next row is {room} away"
        );
    }

    /// Every room a row can leave a card, from the tightest fan past a
    /// comfortable pitch and a card with nothing beside it.
    fn rooms() -> impl Iterator<Item = PlateRoom> {
        (0..=200)
            .map(|n| {
                let right = 0.10 + n as f32 * 0.01;
                PlateRoom {
                    right,
                    below: [f32::NEG_INFINITY, right],
                }
            })
            .chain(std::iter::once(PlateRoom::OPEN))
    }

    fn overlaps(a: [f32; 4], b: [f32; 4]) -> bool {
        a[0] < b[2] && b[0] < a[2] && a[1] < b[3] && b[1] < a[3]
    }

    /// The plate lies on nothing of its own card that a player reads, and on
    /// nothing the law asks to be seen (`docs/legal.md` §3): not the name
    /// and cost, not the type line, not the strip, not the printed box, not
    /// the artist's line and not the ©/™ line, at any pitch its row can
    /// fan to. Measured in the print's height, which is the card's since
    /// #298; `PRINTED_BOX` and `FOOT_TEXT` say where they were measured.
    #[test]
    fn the_plate_lies_on_nothing_its_card_says() {
        use crate::cardrail::{CARD_TALL, Strip};
        let height = |share: f32| share * CARD_TALL;
        let said = [
            ("the name and cost", [0.0, 0.0, 1.0, height(0.105)]),
            ("the type line", [0.0, height(0.55), 1.0, height(0.62)]),
            ("the strip", Strip::largest().rect()),
            (
                "the printed box",
                [
                    PRINTED_BOX[0],
                    height(PRINTED_BOX[1]),
                    PRINTED_BOX[2],
                    height(PRINTED_BOX[3]),
                ],
            ),
            (
                "the artist's and the ©/™ line",
                [0.0, height(FOOT_TEXT), PRINTED_BOX[2], CARD_TALL],
            ),
        ];
        let mut spots = Vec::new();
        for kind in [KIND_FIGHT, KIND_LOYALTY, KIND_LORE] {
            for room in rooms() {
                let (body, spot) = plate_rect(kind, false, room, None).expect("an untapped plate");
                spots.push(spot);
                for (what, rect) in said {
                    assert!(
                        !overlaps(body, rect),
                        "kind {kind} with {room:?} lies on {what}: {body:?}"
                    );
                }
            }
        }
        // Both places are reached, or the loop above said nothing about one.
        assert!(spots.contains(&PlateSpot::Beside) && spots.contains(&PlateSpot::OnCard));
    }

    /// Beside the printed box when the row leaves the plate and its shadow
    /// room, and right-aligned to the part of the card in sight when it does
    /// not: never under the next card, and never on the felt past it.
    #[test]
    fn the_plate_stands_as_far_right_as_its_card_is_in_sight() {
        for room in rooms() {
            let (body, spot) = plate_rect(KIND_FIGHT, false, room, None).expect("untapped");
            match spot {
                PlateSpot::Beside => {
                    assert!((body[0] - PLATE_BESIDE).abs() < 1e-6);
                    assert!(
                        body[2] + PLATE_SHADE[2] + PLATE_AIR <= room.right + 1e-6,
                        "{room:?}: beside the box, its shadow reaches the next card"
                    );
                }
                // Less room than a plate is a tapped card beside it in the
                // tightest fan, and the plate keeps to its own card.
                PlateSpot::OnCard
                    if room.right < crate::cardrail::PRINTED_BORDER + PLATE_W + PLATE_AIR =>
                {
                    assert!((body[0] - crate::cardrail::PRINTED_BORDER).abs() < 1e-6);
                }
                PlateSpot::OnCard => {
                    assert!(
                        body[2] <= room.right - PLATE_AIR + 1e-6,
                        "{room:?}: the plate reaches under the next card: {body:?}"
                    );
                    assert!(
                        body[2] >= (room.right - PLATE_AIR).min(PLATE_BESIDE) - 1e-6,
                        "{room:?}: the plate stands short of the edge in sight: {body:?}"
                    );
                    assert!(body[0] >= crate::cardrail::PRINTED_BORDER - 1e-6);
                }
                PlateSpot::Below => panic!("an untapped plate below its card"),
            }
        }
        assert_eq!(
            plate_rect(KIND_FIGHT, false, PlateRoom::OPEN, None).map(|(_, spot)| spot),
            Some(PlateSpot::Beside),
            "a card with nothing beside it"
        );
    }

    /// A tapped card's plate stands upright under it, in its lane's own air:
    /// under the card, clear of the cards on either side and of a count
    /// badge turned with the card, at every table this client lays out.
    #[test]
    fn a_tapped_cards_plate_stands_in_its_lanes_air() {
        use crate::cardrail::CARD_TALL;
        use crate::layout::{CARD_HEIGHT, TableLayout};
        use baylee_core::ids::PlayerId;
        let [left, _, right, foot] = turned([0.0, 0.0, 1.0, CARD_TALL]);
        let badge = turned(badge_quad_rect(BadgePlace::Beside));
        let mut placed = 0;
        for room in rooms() {
            for with_badge in [false, true] {
                let at = plate_rect(
                    KIND_FIGHT,
                    true,
                    room,
                    with_badge.then_some(badge_quad_rect(BadgePlace::Beside)),
                );
                let Some((body, spot)) = at else {
                    assert!(
                        room.below[1] - PLATE_AIR - left < PLATE_W + 1e-6 || with_badge,
                        "{room:?}: room for a plate and none placed"
                    );
                    continue;
                };
                placed += 1;
                assert_eq!(spot, PlateSpot::Below);
                assert!(body[1] >= foot + PLATE_AIR - 1e-6, "on the card: {body:?}");
                assert!(body[0] >= left - 1e-6 && body[2] <= right + 1e-6);
                assert!(body[2] <= room.below[1] - PLATE_AIR + 1e-6);
                if with_badge {
                    assert!(!overlaps(body, badge), "{room:?}: on the badge: {body:?}");
                }
            }
        }
        assert!(placed > 100, "only {placed} tapped plates placed");
        // An untapped card before it reaches under a tapped card, and the
        // plate stands clear of it or not at all.
        let squeezed = PlateRoom {
            right: 1.2,
            below: [0.6, 1.2],
        };
        let (body, _) = plate_rect(KIND_FIGHT, true, squeezed, None).expect("room enough");
        assert!(body[0] - PLATE_SHADE[0] >= 0.6 + PLATE_AIR - 1e-6);
        assert_eq!(
            plate_rect(
                KIND_FIGHT,
                true,
                PlateRoom {
                    right: 0.7,
                    below: [0.6, 0.7]
                },
                None
            ),
            None,
            "no room between the two prints"
        );
        // A tapped card after it stays out of the air, and the plate keeps
        // its tapped card's bottom right however close that card lies.
        let tapped_next = PlateRoom {
            right: 0.3,
            below: [f32::NEG_INFINITY, f32::INFINITY],
        };
        let (body, _) = plate_rect(KIND_FIGHT, true, tapped_next, None).expect("the air is free");
        assert!((body[2] - right).abs() < 1e-6, "{body:?}");
        // The shadow stays inside the lane at the tightest table there is.
        let lowest = foot + PLATE_AIR + PLATE_H + PLATE_SHADE[3];
        for seats in 1..=8u8 {
            let players: Vec<PlayerId> = (0..seats).map(PlayerId::new).collect();
            for aspect in [16.0 / 9.0, 4.0 / 3.0] {
                for slot in &TableLayout::new(&players, aspect, None).slots {
                    let lane_foot =
                        0.5 * CARD_TALL + 0.5 * slot.lane_height() / CARD_HEIGHT * CARD_TALL;
                    assert!(
                        lowest <= lane_foot,
                        "{seats} seats at {aspect}: the plate's shadow reaches {lowest}, the lane ends at {lane_foot}"
                    );
                }
            }
        }
    }

    /// What turns with a card turns about its centre, clockwise: its right
    /// edge goes down and its top edge right, and a quarter turn four times
    /// over is where it started.
    #[test]
    fn a_quarter_turn_is_a_quarter_turn() {
        use crate::cardrail::CARD_TALL;
        let card = [0.0, 0.0, 1.0, CARD_TALL];
        let [x0, y0, x1, y1] = turned(card);
        assert!((x1 - x0 - CARD_TALL).abs() < 1e-6 && (y1 - y0 - 1.0).abs() < 1e-6);
        assert!(
            (f32::midpoint(x0, x1) - 0.5).abs() < 1e-6
                && (f32::midpoint(y0, y1) - 0.5 * CARD_TALL).abs() < 1e-6
        );
        // The top-right corner, where a badge beside the card stands, comes
        // to the bottom right.
        let corner = turned([0.9, 0.0, 1.0, 0.1]);
        assert!(corner[0] > 0.5 && corner[1] > 0.5 * CARD_TALL, "{corner:?}");
        let round = turned(turned(turned(corner)));
        for (a, b) in round.iter().zip([0.9, 0.0, 1.0, 0.1]) {
            assert!((a - b).abs() < 1e-5, "{round:?}");
        }
    }
}
