//! What a card's numbers say, on the ledge under its print.
//!
//! Three rules facts are printed on a Magic card and drawn nowhere in this
//! client on a card showing art: power, toughness, and the damage marked on a
//! creature. A fourth, a planeswalker's loyalty, is not printed at all — it is
//! a number the game keeps. All four lived in the bottom-right corner of the
//! print, which [`crate::cardrail`] left empty for them, until #274 took
//! everything this client draws off the print: they stand on the frame's
//! ledge now, at its left end (see "the ledge" below).
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

/// Everything the ledge's numbers say about one card.
///
/// The plate, and the chip beside it: what the counters add and, drawn
/// large, what the printing says the body was. The plate is the only one
/// that is always there.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub struct Corner {
    /// What the plate itself says.
    pub plate: Plate,
    /// The net ±1/±1 swing, written in the chip.
    pub swing: Option<(i16, i16)>,
    /// The printed body, written in the chip when the card is drawn large —
    /// and only when it is not the body the plate is already showing.
    pub base: Option<(i16, i16)>,
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
            base: base_body(Plate::of(group), group.base_power, group.base_toughness),
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
            base: base_body(
                Plate::of_parts(
                    object.power,
                    object.toughness,
                    object.loyalty,
                    object.damage,
                    &object.counters,
                ),
                object.base_power,
                object.base_toughness,
            ),
            tone: tone_of(&KeywordBadge::from_bits(object.keywords)),
        }
    }

    /// The three uniforms the shader reads.
    ///
    /// The plate, the swing, and the printed body.
    #[must_use]
    pub fn packed(self) -> [u32; 3] {
        let swing = match self.swing {
            None => 0,
            Some((power, toughness)) => {
                SWING_SET | slot(i32::from(power)) | (slot(i32::from(toughness)) << SLOT_BITS)
            }
        };
        let base = match self.base {
            None => 0,
            Some((power, toughness)) => {
                BASE_SET | slot(i32::from(power)) | (slot(i32::from(toughness)) << SLOT_BITS)
            }
        };
        [
            self.plate.packed(),
            swing | (self.tone_bits() << TONE_SHIFT),
            base,
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
/// The same flag on the base word, for the same reason: a printed `0/0`
/// Walking Ballista is a real body.
pub const BASE_SET: u32 = 1 << 20;

/// The printed body, when it is worth writing in the chip.
///
/// Only when it **differs** from what the plate is showing, which is the
/// whole design of the appendage. A creature drawn at its printed size has
/// its printed size on the plate already, and a second figure beside every
/// untouched creature on the board is noise with no information in it. When
/// they differ, that difference is exactly the thing a player is trying to
/// work out. (The plate used to sit on the print's own P/T box and hide it;
/// since #274 the print shows it, but a card drawn from its tint or its text
/// has no print to read it off.)
///
/// A permanent whose plate is not a body (a planeswalker, a saga, a land)
/// gets nothing: there is no printed body to set against it.
#[must_use]
pub fn base_body(plate: Plate, power: Option<i16>, toughness: Option<i16>) -> Option<(i16, i16)> {
    let Plate::Fight {
        power: shown,
        toughness: shown_t,
        ..
    } = plate
    else {
        return None;
    };
    match (power, toughness) {
        (Some(p), Some(t)) if (p, t) != (shown, shown_t) => Some((p, t)),
        _ => None,
    }
}
/// Where the tone sits in the swing word.
pub const TONE_SHIFT: u32 = 21;
/// The plate writes in its own ink.
pub const TONE_PLAIN: u32 = 0;
/// Deathtouch: the power alone.
pub const TONE_DEADLY: u32 = 1;
/// Toxic: both numbers.
pub const TONE_TOXIC: u32 = 2;

/// The largest chapter drawn in roman numerals.
///
/// Five, which is one past the longest saga printed. A sixth chapter — or a
/// lore counter put somewhere strange by a card that says so — falls back to
/// the arabic numerals the plate already draws, because `VI` needs a second
/// composition rule and an honest number beats a pretty one.
pub const ROMAN_MAX: u16 = 5;

/// How tall a card has to be *drawn* before the chip writes the printed
/// body, as the pixel size the shader already measures.
///
/// It is not written on the table and that is the measurement, not a
/// preference: a card there is about 94 physical pixels wide, which would
/// put a second line in the chip at three, and three-pixel figures are a
/// smudge that says only "something is here" — on every pumped creature at
/// once. The damage band's rules appear on the same terms and through the
/// same `aa` (`TICK_AA`), so the ledge already has this behaviour and a
/// player has already met it: push the camera in, or hover the card, and the
/// ledge says more.
pub const BASE_AA: f32 = 0.004;

// ---------------------------------------------------------------- the ledge
//
// Since #274 the numbers are not drawn on the print at all: they stand on
// the frame's ledge under it (`cardframe::FRAME_FOOT` deep), and read from
// the left, because a lane fans with each card's own **left** edge exposed
// (`layout::MIN_VISIBLE_FRACTION`). The plate comes first, then the chip,
// and the identity crest (`cardcrest`) is right-aligned at the other end;
// all three are centred on the ledge's own middle line.

/// How far in from the card's left edge the plate starts, in card widths.
pub const LEDGE_PAD: f32 = 0.030;

/// The plate's width, in card widths.
///
/// The width the corner always had. What changed is where it stands: it ends
/// at `LEDGE_PAD + PLATE_W` = 0.226, inside the 0.26 of a card the tightest
/// fan still shows, so every creature in a crowded lane shows its body.
pub const PLATE_W: f32 = 0.196;

/// The margin inside the plate, in card widths.
///
/// Was 0.020 when the plate was 0.115 deep. The ledge is 0.125, and the
/// figures did not shrink to pay for it: the margin did, from a little under
/// two physical pixels to a little over one, and [`PLATE_CAP`] is where it
/// was.
pub const PLATE_PAD: f32 = 0.012;

/// How tall the plate's figures are, in card widths: seven physical pixels
/// on a card 94 wide, which is the floor the ledge was measured against.
pub const PLATE_CAP: f32 = 0.075;

/// The plate's height, in card widths: its figures and its margin.
pub const PLATE_H: f32 = PLATE_CAP + 2.0 * PLATE_PAD;

/// The air between the plate and the chip, in card widths.
pub const CHIP_GAP: f32 = 0.010;

/// The chip's width, in card widths.
///
/// The chip is what the counters did — the net swing of a permanent's ±1/±1
/// counters, on green stock for growth and violet for a shrink — and, drawn
/// large, the printed body under it. It used to be two lines, one standing
/// on the plate and one hanging under it; the ledge has no room above or
/// below a plate, so they stand beside it.
pub const CHIP_W: f32 = 0.100;

/// The ledge's middle line, in card widths from the card's top edge.
#[must_use]
pub const fn ledge_mid() -> f32 {
    crate::cardframe::CARD_TALL - crate::cardframe::FRAME_FOOT * 0.5
}

/// The plate's rectangle on the card, `[x0, y0, x1, y1]` in card widths from
/// the card's top-left corner, `y` growing down the card.
///
/// A saga's page is the square at the left of this rectangle.
#[must_use]
pub const fn plate_rect() -> [f32; 4] {
    let y0 = ledge_mid() - PLATE_H * 0.5;
    [LEDGE_PAD, y0, LEDGE_PAD + PLATE_W, y0 + PLATE_H]
}

/// The chip's rectangle, likewise.
#[must_use]
pub const fn chip_rect() -> [f32; 4] {
    let [_, y0, x1, y1] = plate_rect();
    [x1 + CHIP_GAP, y0, x1 + CHIP_GAP + CHIP_W, y1]
}

/// The plate has room for a glyph once its own margin is taken out of it,
/// and fits on the ledge with air round it.
///
/// Compile-time rather than tests, because every term is a constant: a
/// plate taller than the ledge would sit on the print, which is the one
/// place #274 exists to keep it off.
const _: () = assert!(PLATE_CAP > 0.02);
const _: () = assert!(PLATE_H < crate::cardframe::FRAME_FOOT);

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

/// What the count pill says for a card standing for `members` permanents:
/// `0` for none, else the count, clamped to [`COUNT_MAX`].
///
/// The #210 measurement is why this exists: 54 Goblins drew as one Goblin
/// and 16 Plains as one Plains, because a merged card's only cue was the
/// thickness of the slab under it. The pill sits in the top-right corner,
/// [`COUNT_INSET`] in, over the printed cost — the one corner that is dead on the battlefield,
/// and empty on the tokens and lands that merge most — written in the
/// plate's own numerals on the plate's own dark body, so it reads as one of
/// this client's numbers rather than as something printed.
#[must_use]
pub fn count_word(members: usize) -> u32 {
    let n = u32::try_from(members).unwrap_or(u32::MAX);
    if n < COUNT_MIN { 0 } else { n.min(COUNT_MAX) }
}

/// How wide the count pill is for `count`, in card widths — the shader's own
/// arithmetic, mirrored so the widest one can be held to the title bar.
///
/// The figures are the plate's height, so the pill is as tall as the plate
/// and grows sideways with its digits: `×` and up to three tabular digits,
/// padded by the plate's padding on either side.
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

/// How far in from the card's top and right edges the count pill sits, in
/// card widths.
pub const COUNT_INSET: f32 = 0.052;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::board::Provenance;
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
            // that is not about the appendage never draws it.
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

    /// The ledge reads plate, then chip, from the left, and the plate is
    /// whole in the strip of a card the tightest fan still shows.
    ///
    /// The fan is why the left: a lane offsets each card to the right of the
    /// one under it and draws it higher, so what is left of a covered card is
    /// its own left edge, `MIN_VISIBLE_FRACTION` of it at the tightest
    /// pitch. A plate that ran past that edge would show a creature's power
    /// and not its toughness in exactly the lane that most needs both.
    #[test]
    fn the_plate_leads_the_ledge_and_survives_the_tightest_fan() {
        let [_, py0, px1, py1] = plate_rect();
        let [cx0, cy0, cx1, cy1] = chip_rect();
        assert!(
            px1 < crate::layout::MIN_VISIBLE_FRACTION,
            "the plate ends at {px1}, past the {} a fanned card shows",
            crate::layout::MIN_VISIBLE_FRACTION
        );
        assert!(
            (cx0 - px1 - CHIP_GAP).abs() < 1e-6,
            "the chip is not one gap past the plate"
        );
        assert!((cx1 - cx0 - CHIP_W).abs() < 1e-6);
        assert!(
            (py0 - cy0).abs() < 1e-6 && (py1 - cy1).abs() < 1e-6,
            "one row"
        );
        assert!(
            (f32::midpoint(py0, py1) - ledge_mid()).abs() < 1e-6,
            "the plate is not centred on the ledge"
        );
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

    /// The printed body is drawn only when the plate is not already showing
    /// it.
    #[test]
    fn the_base_appears_when_it_differs_and_never_otherwise() {
        let pumped = CardGroup {
            base_power: Some(2),
            base_toughness: Some(2),
            ..group(Some(5), Some(5), None)
        };
        assert_eq!(Corner::of(&pumped).base, Some((2, 2)));

        // A creature at its printed size says it once.
        assert_eq!(Corner::of(&group(Some(2), Some(2), None)).base, None);

        // A permanent whose plate is not a body has no printed body under
        // it to hide: a planeswalker that is also a creature plates its
        // loyalty, and a `3/3` under that would be a second number nobody
        // asked for.
        let walker = CardGroup {
            base_power: Some(2),
            base_toughness: Some(2),
            ..group(Some(5), Some(5), Some(4))
        };
        assert_eq!(Corner::of(&walker).base, None);

        // And it survives its word.
        let [_, _, word] = Corner::of(&pumped).packed();
        assert_ne!(word & BASE_SET, 0);
        assert_eq!((word & SLOT_MASK) as i32 - BIAS, 2);
        assert_eq!(((word >> SLOT_BITS) & SLOT_MASK) as i32 - BIAS, 2);
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
    /// three zeroes.
    #[test]
    fn the_swing_survives_the_packing() {
        let mut swung = CardGroup {
            counters: counted(&[(CounterKind::MINUS_ONE, 2)]),
            ..group(Some(5), Some(5), None)
        };
        swung.badges = vec![crate::board::KeywordBadge::Deathtouch];
        let [_, word, _] = Corner::of(&swung).packed();
        assert_ne!(word & SWING_SET, 0, "the word does not say there is one");
        assert_eq!((word & SLOT_MASK) as i32 - BIAS, -2);
        assert_eq!(((word >> SLOT_BITS) & SLOT_MASK) as i32 - BIAS, -2);
        assert_eq!(word >> TONE_SHIFT, TONE_DEADLY);

        // A corner with nothing to say packs to nothing, which is what lets
        // every card in a hand share one material.
        assert_eq!(Corner::of(&group(None, None, None)).packed(), [0, 0, 0]);
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

    /// The pill grows leftwards from the right edge and must never leave the
    /// printed name less than half the title bar, however many digits it
    /// writes.
    #[test]
    fn the_widest_count_leaves_the_name_half_the_title_bar() {
        let widest = count_width(COUNT_MAX);
        assert!(
            widest < 0.5 * (1.0 - 2.0 * COUNT_INSET),
            "a ×999 pill takes {widest} of the title bar"
        );
        // And it is wider with every digit, so a count that grows past ten
        // is not drawn in the same box as one below it.
        assert!(count_width(9) < count_width(10) && count_width(99) < count_width(100));
    }
}
