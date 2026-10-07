//! The text face: what fills a card's window when there is no print (#259).
//!
//! A token has no print, a printing's art can still be on its way, and a
//! player can ask for text over art. Until #259 the window was then a flat
//! dark tint with a few lines of text centred on it — the owner's "dunkle
//! Metall-Platte" — and a lane of text tokens read as a row of holes. So the
//! window is laid out the way a card is: a border, a name bar, an art box, a
//! type bar and a text box, in the card's own proportions, so that a text
//! card's bars line up with its printed neighbours' in a lane. The power and
//! toughness are not in here: the ledge's plate is the P/T box
//! ([`crate::cardplate`]).
//!
//! It is generic card geometry drawn by our own arithmetic — no frame image,
//! no ornament, no symbol — which is what `docs/legal.md` §2 asks of
//! everything the table shows.
//!
//! # One rule, sized from the text
//!
//! A bar is as tall as one line of its text plus [`BAR_PAD`] above and below:
//! [`bar`]. Alegreya Sans has no line gap and its ascender and descender add
//! up to [`LINE_BOX`] em, so that is a line with its descenders and nothing
//! more. The type bar's top is pinned on the keyword strip's seam
//! ([`crate::cardrail::strip_bottom`]), so the strip lies on the art box and
//! never on the type line; the art box takes whatever the name bar leaves
//! above it.
//!
//! What differs between the table and a preview is only the em. On the table
//! the camera is far, and a name has to be set large for the card's size to
//! be read at all — [`NAME_EM`] is 0.11 card widths, where a print's name is
//! about half that. So the table's name bar is deeper than a print's and a
//! long name has rules of its own ([`fit_name`]); a preview's em is its font
//! size over its card's width in pixels, and lands on a print's proportions by
//! the same arithmetic.
//!
//! # Four layouts (WP6)
//!
//! The table's face is the one above: the art box under the name, the type
//! bar on the strip's seam, because the keyword strip lies on the art box
//! there and must never lie on the type line. The interface lays no strip
//! over a text face, so its faces put the type line under the name, where a
//! player reads it first, and give the art box's empty half of the card to
//! the rules ([`Layout`]):
//!
//! - **a preview** — name, type, the **band** (the colour identity as a
//!   gradient, the subtype words and the keyword chips), the rules, and a
//!   foot that credits the printing. The band is [`BAND_PREVIEW`] deep;
//! - **a long preview** — the same, its band [`BAND_LONG`] deep, for rules
//!   that would otherwise go under [`LONG_PX`];
//! - **a small card** (hand, stack, tray) — name, type, the band as the
//!   keyword strip, and one line of rules at the foot, as deep as the type
//!   line, so the word carries its depth already.
//!
//! The layout rides the face word ([`FACE_LAYOUT_SHIFT`]), so the shader
//! draws the same parts the text was placed in.
//!
//! # Widths
//!
//! The fitting rules need to know how wide a string is before anything has
//! laid it out, because a two-line name bar is part of the card's material.
//! They take the answer as a function, `width(s)`: the advance of `s` at an em
//! of one. The client answers from the shipped font's own advances; until the
//! font has arrived, [`average_width`] does.
//!
//! Lengths are card widths from the card's top-left corner, `y` growing down
//! the card, and rectangles are `[x0, y0, x1, y1]` — [`crate::cardrail`]'s
//! convention.

use baylee_core::color::{Color, ColorSet};
use baylee_core::types::{SubtypeSet, SupertypeSet, TypeSet};

use crate::cardrail::CARD_TALL;

/// The face's window: the whole card, since the print fills it (#298).
const WINDOW: [f32; 4] = [0.0, 0.0, 1.0, CARD_TALL];

/// The dark border round the face, inside the card's edge, in card widths.
pub const BORDER: f32 = 0.040;

/// Air above and below a bar's line of text, in card widths.
pub const BAR_PAD: f32 = 0.010;

/// Alegreya Sans's ascender, in ems: where a line's top is above its
/// baseline. The shipped font's `hhea` table says 0.900.
pub const ASCENT: f32 = 0.900;

/// One line of Alegreya Sans, ascender to descender, in ems: 0.900 and 0.300
/// with no line gap. It is also the line height bevy sets a line at, so a
/// block of `n` lines is `n` of these tall.
pub const LINE_BOX: f32 = 1.2;

/// The pinline between the name bar and the art box, in card widths.
pub const PINLINE: f32 = 0.006;

/// The gap between the type bar and the text box, in card widths.
pub const BOX_GAP: f32 = 0.012;

/// Where the text box ends, in card widths from the card's top. What is left
/// of the window under it is the foot: the border's colour and empty, where a
/// print has its collector line — there is none to write, and a made-up one
/// would be a claim.
///
/// The same share of the face the foot was inside the frame's window before
/// #298 took the frame away: 0.905 of the face's height.
pub const TEXT_FOOT: f32 = 1.264;

/// How far a line of text stands in from the bar's ends, in card widths.
pub const TEXT_INSET: f32 = 0.014;

/// The table's name, as an em in card widths: 11 of `Text2d`'s pixels at the
/// table's hundred to a card width, 10 screen pixels on a card 94 wide.
pub const NAME_EM: f32 = 0.11;

/// The table's smallest name, and its type line and cost, in card widths.
/// Below this a name on a card 94 pixels wide stops being read.
pub const SMALL_EM: f32 = 0.10;

/// The smallest the table's type line is set: a type line that still does
/// not fit loses subtypes rather than shrinking further.
pub const TYPE_FLOOR_EM: f32 = 0.09;

/// What a string is taken to advance per character, in ems, before the font
/// has arrived to say: Alegreya Sans Regular's mean advance over the pool's
/// 2715 card names, 0.420. It answers the one-line-or-two question as the
/// font does for all but 54 of the pool's 2835 names at the face's width
/// since #298 (179 at the frame's narrower one), and those are refitted when the font
/// arrives.
pub const AVERAGE_ADVANCE: f32 = 0.42;

/// What ends a line that had to be cut.
pub const ELLIPSIS: char = '…';

/// What parts a type line's types from its subtypes, in every language the
/// catalog serves.
pub const TYPE_DASH: &str = " — ";

/// A bar holding one line of text set at `em`, in card widths.
#[must_use]
pub const fn bar(em: f32) -> f32 {
    LINE_BOX * em + 2.0 * BAR_PAD
}

/// The face's inside, left to right: the window less its border.
#[must_use]
pub const fn content_x() -> [f32; 2] {
    let [x0, _, x1, _] = WINDOW;
    [x0 + BORDER, x1 - BORDER]
}

/// How long one line of text may be, in card widths.
#[must_use]
pub const fn line_width() -> f32 {
    let [x0, x1] = content_x();
    x1 - x0 - 2.0 * TEXT_INSET
}

/// Where the type bar's top stands: on the strip's seam.
#[must_use]
pub const fn seam() -> f32 {
    crate::cardrail::strip_bottom()
}

/// How a face is laid out down the card ([module docs](self#four-layouts-wp6)).
///
/// The number is the code [`face_word`] carries at [`FACE_LAYOUT_SHIFT`],
/// and `card_common.wgsl`'s `text_face` switches on the same codes.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Default)]
#[repr(u32)]
pub enum Layout {
    /// The table's: the art box under the name, the type bar on the seam.
    #[default]
    Table = 0,
    /// A preview's: the type under the name, the band, the rules, the foot.
    Preview = 1,
    /// A preview whose rules run long: the band at [`BAND_LONG`].
    Long = 2,
    /// A small card in the interface: the band is the keyword strip, and one
    /// line of rules stands at the foot.
    Small = 3,
}

impl Layout {
    /// The layout a three-bit field of the word names, the table's for a
    /// code no layout has, as the shader reads it.
    #[must_use]
    pub const fn from_code(code: u32) -> Self {
        match code & 0x7 {
            1 => Self::Preview,
            2 => Self::Long,
            3 => Self::Small,
            _ => Self::Table,
        }
    }

    /// The band's depth in card widths, for a layout that fixes it: a
    /// preview's. The table's art box and a small card's band take what
    /// their neighbours leave.
    #[must_use]
    pub const fn band(self) -> Option<f32> {
        match self {
            Self::Preview => Some(BAND_PREVIEW),
            Self::Long => Some(BAND_LONG),
            Self::Table | Self::Small => None,
        }
    }

    /// Whether the type line stands under the name (every layout but the
    /// table's).
    #[must_use]
    pub const fn type_under_name(self) -> bool {
        !matches!(self, Self::Table)
    }
}

/// A preview's band: 18 % of the face (WP6), in card widths.
pub const BAND_PREVIEW: f32 = 0.18 * CARD_TALL;

/// A long preview's band, the keyword strip alone: 12 % of the face.
pub const BAND_LONG: f32 = 0.12 * CARD_TALL;

/// The size, in pixels at the default step, under which a preview's rules
/// take the long layout: a text that would be set smaller than this at the
/// full band is given the band's room instead.
pub const LONG_PX: f32 = 16.0;

/// The face's parts, each `[x0, y0, x1, y1]`.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Regions {
    /// The name, left.
    pub name_bar: [f32; 4],
    /// Where a print has its picture: on the table the art box, which holds
    /// the cost's first line and lies under the strip; in the interface the
    /// band of colour, subtypes and keyword chips.
    pub band: [f32; 4],
    /// The type line: on the seam on the table, under the name elsewhere.
    pub type_bar: [f32; 4],
    /// The rules text.
    pub text_box: [f32; 4],
    /// Under the text box, inside the border: where a preview credits its
    /// printing.
    pub foot: [f32; 4],
}

impl Regions {
    /// The table's parts for bars of these depths.
    #[must_use]
    pub fn new(depths: Depths) -> Self {
        Self::laid(Layout::Table, depths)
    }

    /// The parts of a face laid out as `layout`, for bars of these depths.
    #[must_use]
    pub fn laid(layout: Layout, depths: Depths) -> Self {
        let [x0, x1] = content_x();
        let top = WINDOW[1] + BORDER;
        let name_end = top + depths.name_bar();
        let foot = [x0, TEXT_FOOT, x1, WINDOW[3] - BORDER];
        if !layout.type_under_name() {
            let type_end = seam() + depths.type_bar();
            return Self {
                name_bar: [x0, top, x1, name_end],
                band: [x0, name_end + PINLINE, x1, seam()],
                type_bar: [x0, seam(), x1, type_end],
                text_box: [x0, type_end + BOX_GAP, x1, TEXT_FOOT],
                foot,
            };
        }
        let type_top = name_end + PINLINE;
        let type_end = type_top + depths.type_bar();
        let band_top = type_end + PINLINE;
        // A small card's text box is one line at the type line's size: the
        // type bar's depth.
        let text_top = layout.band().map_or(TEXT_FOOT - depths.type_bar(), |band| {
            band_top + band + BOX_GAP
        });
        let band_end = text_top - BOX_GAP;
        Self {
            name_bar: [x0, top, x1, name_end],
            band: [x0, band_top, x1, band_end],
            type_bar: [x0, type_top, x1, type_end],
            text_box: [x0, text_top, x1, TEXT_FOOT],
            foot,
        }
    }

    /// The table's parts, for a name set on `lines` lines.
    #[must_use]
    pub fn table(lines: usize) -> Self {
        Self::new(Depths::table(lines))
    }
}

/// The step a bar's depth is carried in by [`face_word`]: a 512th of a card
/// width, under a pixel on the largest preview.
pub const DEPTH_STEP: f32 = 1.0 / 512.0;

/// How deep a face's two bars are, in [`DEPTH_STEP`]s.
///
/// The one number both halves read: [`Regions`] places the text by it and
/// [`face_word`] hands it to the shader, which draws the bars by it, so a
/// name stands on its bar to the bit. A depth is rounded **up** to the step,
/// so a bar always holds the lines it was sized for.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct Depths {
    /// The name bar's.
    pub name: u8,
    /// The type bar's.
    pub kind: u8,
}

impl Depths {
    /// Bars this deep, in card widths, each rounded up to the step.
    #[must_use]
    pub fn of(name_bar: f32, type_bar: f32) -> Self {
        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)] // clamped
        let steps = |depth: f32| (depth / DEPTH_STEP).ceil().clamp(0.0, 255.0) as u8;
        Self {
            name: steps(name_bar),
            kind: steps(type_bar),
        }
    }

    /// The table's, for a name on `lines` lines.
    #[must_use]
    pub fn table(lines: usize) -> Self {
        Sizes::TABLE.depths(lines)
    }

    /// The name bar's depth, in card widths.
    #[must_use]
    pub fn name_bar(self) -> f32 {
        f32::from(self.name) * DEPTH_STEP
    }

    /// The type bar's depth, in card widths.
    #[must_use]
    pub fn type_bar(self) -> f32 {
        f32::from(self.kind) * DEPTH_STEP
    }
}

/// The table's name bar for a name on `lines` lines ([`Sizes::name_bar`]).
#[must_use]
pub fn name_bar(lines: usize) -> f32 {
    Sizes::TABLE.name_bar(lines)
}

/// The overlay's name, as a share of the card's width in pixels, held
/// between two sizes in pixels.
///
/// Nine pixels at the least: a 92-pixel hand card's name is what a row of
/// them is read by (WP6). Every top clamp is over its size on a 308-pixel
/// preview even times the smallest step's 0.702, so the preview at the
/// default scale is the same at every step and the steps bite on the small
/// faces only.
pub const UI_NAME: (f32, [f32; 2]) = (0.082, [9.0, 36.0]);

/// The overlay's type line and its name stepped down, likewise.
///
/// Smaller than the rules on a preview, as on a print — 16 px at 308 — and
/// eight pixels at the least, which a small card's one line of rules is set
/// at too.
pub const UI_TYPE: (f32, [f32; 2]) = (0.052, [8.0, 24.0]);

/// The overlay's rules text at its own size, likewise: 19 px on a 308-pixel
/// preview at every step (WP6), since the top clamp, 28 px times the
/// smallest step's 0.702, is still over it.
pub const UI_BODY: (f32, [f32; 2]) = (0.062, [6.0, 28.0]);

/// The band's subtype words and keyword chips, likewise: 12 px on a preview,
/// 8 on a hand card.
pub const UI_CHIP: (f32, [f32; 2]) = (0.040, [8.0, 18.0]);

/// The foot's credit line, likewise: 11 px on a preview.
pub const UI_FOOT: (f32, [f32; 2]) = (0.036, [7.0, 16.0]);

/// The smallest the rules text is stepped down to at the default step, in
/// pixels ([`Step::body_floor`]). Past it the text box scrolls rather than
/// shrinking long rules below a readable size.
pub const BODY_FLOOR_PX: f32 = 14.0;

/// The interface's five text steps' factors (`DESIGN-v5.md` §8): geometric,
/// 1.125 apart, the fourth the size the client had before them.
pub const STEP_FACTORS: [f32; 5] = [0.702, 0.790, 0.889, 1.000, 1.125];

/// The rules' floor at each step, in pixels: the factor's, held up where a
/// sentence stops being read (WP6).
pub const BODY_FLOORS: [f32; 5] = [12.0, 13.0, 14.0, 14.0, 16.0];

/// One of the interface's five text steps.
///
/// A face in the interface multiplies only its clamps by the step's factor
/// and takes the step's floor for its rules: the shares of the card's width
/// stay, so a 308-pixel preview keeps its 19-pixel rules at every step, and
/// the step bites on the small faces the clamps hold. The table's faces do
/// not follow it. The setting that chooses it is the shell's
/// (`ClientSettings::text_size`, WP0b-1); until then a face is set at
/// [`Step::DEFAULT`].
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct Step(u8);

impl Step {
    /// The smallest step.
    pub const XS: Self = Self(1);
    /// The fourth step, the size before there were steps.
    pub const DEFAULT: Self = Self(4);
    /// The largest step.
    pub const XL: Self = Self(5);

    /// Step `n` of five, held to the five there are.
    #[must_use]
    pub const fn new(n: u8) -> Self {
        Self(if n < 1 {
            1
        } else if n > 5 {
            5
        } else {
            n
        })
    }

    /// Which step it is, one to five.
    #[must_use]
    pub const fn number(self) -> u8 {
        self.0
    }

    /// What the step multiplies an interface clamp by.
    #[must_use]
    pub const fn factor(self) -> f32 {
        STEP_FACTORS[self.0 as usize - 1]
    }

    /// The rules' floor at this step, in pixels.
    #[must_use]
    pub const fn body_floor(self) -> f32 {
        BODY_FLOORS[self.0 as usize - 1]
    }
}

impl Default for Step {
    fn default() -> Self {
        Self::DEFAULT
    }
}

/// The gap between two blocks of rules text, in lines of it.
pub const BLOCK_GAP: f32 = 0.3;

/// The text box's right margin, in card widths: where its scrollbar stands,
/// kept whether or not the bar is shown, so the text never reflows when it
/// appears.
pub const SCROLL_MARGIN: f32 = 0.024;

/// The sizes a face sets its lines at, and how long a name may run, in card
/// widths.
///
/// The table's are [`Sizes::TABLE`], large because the felt is far. The
/// overlay's follow the card's width in pixels ([`Sizes::overlay`]): the
/// same rule at its own em, which keeps a 92-pixel hand card and a
/// 308-pixel preview in a print's proportions without a second table of
/// constants.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Sizes {
    /// The name, on one line.
    pub name: f32,
    /// The name stepped down, on one line or two, and the type line.
    pub small: f32,
    /// The smallest the type line is set.
    pub type_floor: f32,
    /// How long a line of the name may run: the line, less whatever shares
    /// its bar.
    pub name_room: f32,
    /// How long the type line may run: the line, less whatever shares its
    /// bar (a preview's set and rarity).
    pub type_room: f32,
}

impl Sizes {
    /// The table's: [`NAME_EM`], [`SMALL_EM`], [`TYPE_FLOOR_EM`], and the
    /// whole line for the name, since its cost stands on the art box.
    pub const TABLE: Self = Self {
        name: NAME_EM,
        small: SMALL_EM,
        type_floor: TYPE_FLOOR_EM,
        name_room: line_width(),
        type_room: line_width(),
    };

    /// The overlay's, on a card `card_px` pixels wide whose cost takes
    /// `cost` card widths at the name bar's right end, as on a print, at
    /// the default step.
    #[must_use]
    pub fn overlay(card_px: f32, cost: f32) -> Self {
        Self::overlay_at(card_px, cost, Step::DEFAULT)
    }

    /// [`Self::overlay`] at text step `step`.
    #[must_use]
    pub fn overlay_at(card_px: f32, cost: f32, step: Step) -> Self {
        let small = ui_em_at(UI_TYPE, card_px, step);
        Self {
            name: ui_em_at(UI_NAME, card_px, step),
            small,
            type_floor: small * TYPE_FLOOR_EM / SMALL_EM,
            name_room: line_width() - if cost > 0.0 { cost + TEXT_INSET } else { 0.0 },
            type_room: line_width(),
        }
    }

    /// These sizes with `aside` card widths of the type bar's right end
    /// given to something else.
    #[must_use]
    pub fn beside_type(self, aside: f32) -> Self {
        Self {
            type_room: line_width() - if aside > 0.0 { aside + TEXT_INSET } else { 0.0 },
            ..self
        }
    }

    /// The name bar for a name on `lines` lines: one line holds the name at
    /// its own size, two hold it at the smaller one.
    #[must_use]
    pub fn name_bar(&self, lines: usize) -> f32 {
        if lines > 1 {
            2.0 * LINE_BOX * self.small + 2.0 * BAR_PAD
        } else {
            bar(self.name)
        }
    }

    /// The bars' depths for a name on `lines` lines.
    #[must_use]
    pub fn depths(&self, lines: usize) -> Depths {
        Depths::of(self.name_bar(lines), bar(self.small))
    }
}

/// One of the overlay's sizes on a card `card_px` pixels wide, as an em in
/// card widths.
#[must_use]
pub fn ui_em(size: (f32, [f32; 2]), card_px: f32) -> f32 {
    ui_em_at(size, card_px, Step::DEFAULT)
}

/// [`ui_em`] at text step `step`: the clamps times its factor, the share as
/// it is.
#[must_use]
pub fn ui_em_at((share, [lo, hi]): (f32, [f32; 2]), card_px: f32, step: Step) -> f32 {
    let f = step.factor();
    (card_px * share).clamp(lo * f, hi * f) / card_px
}

/// How wide the rules text's column is, in card widths: the face's inside
/// less [`TEXT_INSET`] on the left and [`SCROLL_MARGIN`] on the right.
#[must_use]
pub const fn column() -> f32 {
    let [x0, x1] = content_x();
    x1 - x0 - TEXT_INSET - SCROLL_MARGIN
}

/// The size the rules text is set at in a box `height` card widths deep, as
/// an em: its own size if all of it fits, else a pixel smaller at a time
/// down to [`BODY_FLOOR_PX`], and there it stays and the box scrolls. A card
/// too small for its own size to reach the floor keeps its own size.
///
/// `depth` says how deep the text stands at an em, in card widths: the
/// renderer's model of its own layout, since only the renderer knows how it
/// sets a line (`manaui::rich_depth` in the client).
#[must_use]
pub fn fit_body(card_px: f32, height: f32, depth: impl Fn(f32) -> f32) -> f32 {
    fit_body_at(card_px, height, Step::DEFAULT, depth)
}

/// [`fit_body`] at text step `step`: its clamps and its floor
/// ([`Step::body_floor`]).
#[must_use]
pub fn fit_body_at(card_px: f32, height: f32, step: Step, depth: impl Fn(f32) -> f32) -> f32 {
    let own = ui_em_at(UI_BODY, card_px, step) * card_px;
    let floor = step.body_floor();
    let mut px = own;
    // The last step lands on the floor itself, which a card's own size is
    // seldom a whole number of pixels over.
    while px > floor && depth(px / card_px) > height {
        px = (px - 1.0).max(floor);
    }
    px / card_px
}

/// A string set to fit: the em it is set at and its lines, top first.
#[derive(Clone, PartialEq, Debug)]
pub struct Fitted {
    /// Its em, in card widths.
    pub em: f32,
    /// One line, or two.
    pub lines: Vec<String>,
}

impl Fitted {
    fn one(em: f32, text: &str) -> Self {
        Self {
            em,
            lines: vec![text.to_owned()],
        }
    }
}

/// [`AVERAGE_ADVANCE`] per character: what a string is taken to measure
/// before the font can say.
#[must_use]
pub fn average_width(text: &str) -> f32 {
    #[allow(clippy::cast_precision_loss)] // a name is short
    let chars = text.chars().count() as f32;
    chars * AVERAGE_ADVANCE
}

/// The table's name, set to fit the name bar.
///
/// At [`NAME_EM`] on one line if it fits; else at [`SMALL_EM`] on one line;
/// else at [`SMALL_EM`] on two, broken after the last word that fits — the
/// name bar then takes its two-line height. Never a third line and never
/// smaller: a second line that still runs over is cut with an ellipsis.
#[must_use]
pub fn fit_name(name: &str, width: impl Fn(&str) -> f32) -> Fitted {
    fit_name_in(&Sizes::TABLE, name, width)
}

/// A name set to fit its bar by [`fit_name`]'s rule, at `sizes`.
#[must_use]
pub fn fit_name_in(sizes: &Sizes, name: &str, width: impl Fn(&str) -> f32) -> Fitted {
    let room = sizes.name_room;
    let natural = width(name);
    if natural * sizes.name <= room {
        return Fitted::one(sizes.name, name);
    }
    if natural * sizes.small <= room {
        return Fitted::one(sizes.small, name);
    }
    let fits = |s: &str| width(s) * sizes.small <= room;
    let (first, rest) = break_line(name, &fits);
    let second = if fits(rest) {
        rest.to_owned()
    } else {
        cut(rest, &fits)
    };
    Fitted {
        em: sizes.small,
        lines: vec![first.to_owned(), second],
    }
}

/// The table's type line, set to fit the type bar on one line.
///
/// At [`SMALL_EM`] if it fits, else at [`TYPE_FLOOR_EM`]. Past that it gives
/// up words from the **front**, because on the table the subtypes are the
/// half that says something — a Soldier matters to a tribe, and "Creature"
/// is already said by the plate and the frame: first the supertypes
/// ("Legendary Creature — Elf Druid" → "Creature — Elf Druid"), then the
/// card types and the dash ("Elf Druid"). Only then are subtypes dropped from
/// the end, whole, with an ellipsis. A line with no subtypes keeps its card
/// types. One line always: the type bar's height is fixed, so the text box
/// never moves.
///
/// A supertype is known by its English word ([`SupertypeSet::from_word`]),
/// which is what the engine prints. A translated line has none the table can
/// tell apart, so it goes from the whole line straight to its subtypes.
#[must_use]
pub fn fit_type(type_line: &str, width: impl Fn(&str) -> f32) -> Fitted {
    fit_type_in(&Sizes::TABLE, type_line, width)
}

/// A type line set to fit its bar by [`fit_type`]'s rule, at `sizes`.
#[must_use]
pub fn fit_type_in(sizes: &Sizes, type_line: &str, width: impl Fn(&str) -> f32) -> Fitted {
    let room = sizes.type_room;
    let floor = sizes.type_floor;
    let natural = width(type_line);
    if natural * sizes.small <= room {
        return Fitted::one(sizes.small, type_line);
    }
    if natural * floor <= room {
        return Fitted::one(floor, type_line);
    }
    let fits = |s: &str| width(s) * floor <= room;
    let (types, subtypes) = match type_line.split_once(TYPE_DASH) {
        Some((types, subtypes)) => (types, Some(subtypes)),
        None => (type_line, None),
    };
    let plain = types
        .split(' ')
        .filter(|word| SupertypeSet::from_word(word).is_none())
        .collect::<Vec<_>>()
        .join(" ");
    let plain = if plain.is_empty() { types } else { &plain };
    let unsuper = match subtypes {
        Some(subtypes) => format!("{plain}{TYPE_DASH}{subtypes}"),
        None => plain.to_owned(),
    };
    if fits(&unsuper) {
        return Fitted::one(floor, &unsuper);
    }
    let Some(subtypes) = subtypes else {
        return Fitted::one(floor, &cut(plain, &fits));
    };
    if fits(subtypes) {
        return Fitted::one(floor, subtypes);
    }
    let words: Vec<&str> = subtypes.split(' ').collect();
    let kept = (1..words.len()).rev().find_map(|keep| {
        let line = format!("{}{ELLIPSIS}", words[..keep].join(" "));
        fits(&line).then_some(line)
    });
    Fitted::one(floor, &kept.unwrap_or_else(|| cut(subtypes, &fits)))
}

/// A preview's type line, set to fit its bar on one line: at `sizes.small`
/// if it fits, else at the floor, else cut from the **end**, whole words
/// first, with an ellipsis.
///
/// The other way round from [`fit_type_in`], because a preview's band says
/// the subtypes again in words of their own ([`subtype_words`]): the half of
/// the line only the line says is the front, "Legendary Creature".
#[must_use]
pub fn fit_type_front_in(sizes: &Sizes, type_line: &str, width: impl Fn(&str) -> f32) -> Fitted {
    let room = sizes.type_room;
    let floor = sizes.type_floor;
    let natural = width(type_line);
    if natural * sizes.small <= room {
        return Fitted::one(sizes.small, type_line);
    }
    if natural * floor <= room {
        return Fitted::one(floor, type_line);
    }
    let fits = |s: &str| width(s) * floor <= room;
    let words: Vec<&str> = type_line.split(' ').collect();
    let kept = (1..words.len()).rev().find_map(|keep| {
        let line = format!(
            "{}{ELLIPSIS}",
            words[..keep].join(" ").trim_end_matches(" —")
        );
        fits(&line).then_some(line)
    });
    Fitted::one(floor, &kept.unwrap_or_else(|| cut(type_line, &fits)))
}

/// The subtypes a type line names, in its own words: what follows
/// [`TYPE_DASH`], or nothing.
///
/// Split into words where the line's words are as many as the face's
/// subtypes (`count`), so "Ally Wizard" reads as two; joined whole where
/// they are not, so a subtype of two words ("Time Lord") is not taken apart.
#[must_use]
pub fn subtype_words(type_line: &str, count: usize) -> Vec<String> {
    let Some((_, subtypes)) = type_line.split_once(TYPE_DASH) else {
        return Vec::new();
    };
    let words: Vec<&str> = subtypes.split_whitespace().collect();
    if words.is_empty() {
        return Vec::new();
    }
    if words.len() == count {
        words.into_iter().map(str::to_owned).collect()
    } else {
        vec![words.join(" ")]
    }
}

/// The keyword chips a face's rules text gives its band: each keyword of
/// every line that is nothing but keywords, in the card's own words and in
/// the order it prints them.
///
/// Read off the printed text rather than named from the card's keyword set,
/// because the words are then the card's own in the player's language — a
/// German face says "Fliegend" because its printing does, and nothing here
/// has to translate a keyword (or could get it wrong). A keyword line is a
/// rules block with no sentence in it: no full stop, colon, dash or bullet,
/// no mana symbol and short ([`CHIP_LINE_MAX`] characters); its keywords
/// are its parts between commas. Reminder text never is one, since the face
/// keeps it in blocks of its own.
#[must_use]
pub fn keyword_chips<'a>(rules: impl IntoIterator<Item = &'a str>) -> Vec<String> {
    rules
        .into_iter()
        .filter(|line| is_keyword_line(line))
        .flat_map(|line| line.split([',', ';']))
        .map(str::trim)
        .filter(|chip| !chip.is_empty())
        .map(str::to_owned)
        .collect()
}

/// How long a keyword line may be, in characters: longer, and it is a
/// sentence that has lost its full stop.
pub const CHIP_LINE_MAX: usize = 48;

/// Whether a rules block is a line of keywords ([`keyword_chips`]).
fn is_keyword_line(line: &str) -> bool {
    let line = line.trim();
    !line.is_empty()
        && line.chars().count() <= CHIP_LINE_MAX
        && !line.contains(['.', ':', '—', '•', '{', '!', '?', '"', '“', '„'])
}

/// The first sentence of a face's rules, for a face with room for one line:
/// the first rules block up to and including its first full stop.
#[must_use]
pub fn first_sentence(rules: &str) -> &str {
    let rules = rules.trim();
    // A full stop inside a word ("1.5") is not a sentence's end; one followed
    // by a space or the end is.
    let mut at = None;
    for (i, ch) in rules.char_indices() {
        if ch == '.' {
            let next = rules[i + 1..].chars().next();
            if next.is_none_or(char::is_whitespace) {
                at = Some(i + 1);
                break;
            }
        }
    }
    at.map_or(rules, |end| &rules[..end])
}

// ------------------------------------------------------------- the colours

/// How round a bar's and a box's corners are, in card widths.
pub const BAR_CORNER: f32 = 0.012;

/// A part's colour: one of the five, gold for two or more, grey for none.
///
/// The number is what the shader reads out of [`face_word`]: the tones below
/// are in this order, and `card_common.wgsl`'s `face_hue` is the same table.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
#[repr(u32)]
pub enum Hue {
    /// White.
    White = 0,
    /// Blue.
    Blue = 1,
    /// Black.
    Black = 2,
    /// Red.
    Red = 3,
    /// Green.
    Green = 4,
    /// Two colours or more.
    Gold = 5,
    /// No colour, and every land's bars.
    Grey = 6,
}

impl Hue {
    /// The hue of one colour.
    #[must_use]
    pub const fn of(color: Color) -> Self {
        match color {
            Color::White => Self::White,
            Color::Blue => Self::Blue,
            Color::Black => Self::Black,
            Color::Red => Self::Red,
            Color::Green => Self::Green,
        }
    }

    /// The hue a four-bit field of [`face_word`] names, grey for a code no
    /// hue has, as the shader's `face_hue` reads it.
    #[must_use]
    pub const fn from_code(code: u32) -> Self {
        match code & 0xf {
            0 => Self::White,
            1 => Self::Blue,
            2 => Self::Black,
            3 => Self::Red,
            4 => Self::Green,
            5 => Self::Gold,
            _ => Self::Grey,
        }
    }

    /// The bars' colour, in linear light.
    ///
    /// Chosen as sRGB and converted once: white (0.90, 0.87, 0.78), blue
    /// (0.55, 0.70, 0.86), black (0.53, 0.51, 0.55), red (0.88, 0.58, 0.48),
    /// green (0.58, 0.76, 0.58), gold (0.86, 0.74, 0.42), grey
    /// (0.70, 0.70, 0.68). Black's is the one [`INK`] stands off least, at
    /// 4.7:1; the rest are over 7:1.
    #[must_use]
    pub const fn tone(self) -> [f32; 3] {
        match self {
            Self::White => [0.7874, 0.7293, 0.5705],
            Self::Blue => [0.2633, 0.448, 0.7106],
            Self::Black => [0.2429, 0.2234, 0.2633],
            Self::Red => [0.7484, 0.2957, 0.196],
            Self::Green => [0.2957, 0.5382, 0.2957],
            Self::Gold => [0.7106, 0.5071, 0.1473],
            Self::Grey => [0.448, 0.448, 0.42],
        }
    }
}

/// What the text box's paper is mixed towards from its bars' colour, in
/// linear light: a warm white, sRGB (0.93, 0.92, 0.88).
pub const PAPER_WHITE: [f32; 3] = [0.8481, 0.8276, 0.7484];

/// How far the paper goes from the bars' colour to [`PAPER_WHITE`]. Light,
/// like a printed text box, which is what keeps a text card from reading as
/// a hole in a row of prints.
pub const PAPER_MIX: f32 = 0.65;

/// The art box's colour at its top and its foot, as a factor on the bars'
/// colour in linear light: the same hue, dark, where a print has a picture.
pub const ART_TOP: f32 = 0.35;

/// See [`ART_TOP`].
pub const ART_FOOT: f32 = 0.20;

/// How far the art box's cloth lifts and sinks its colour, in linear light.
pub const CLOTH: f32 = 0.04;

/// The border's colour and the pinline's, in linear light: sRGB
/// (0.06, 0.06, 0.07).
pub const BORDER_INK: [f32; 3] = [0.0049, 0.0049, 0.006];

/// What a bar's top edge gains and its bottom edge loses, in linear light,
/// so it reads as a raised plate.
pub const BEVEL: f32 = 0.06;

/// The face's ink, in sRGB: every word on its bars and its paper.
///
/// Dark, because the bars and the paper are light, as a printed card's are.
/// The bar it stands off least is black's, at 4.7:1 (WCAG asks 4.5 of body
/// text); the paper is over 11:1 whatever its colour.
pub const INK: [f32; 3] = [0.09, 0.10, 0.12];

/// The ink of reminder text and of a placeholder, in sRGB: quieter than
/// [`INK`], and still 4.5:1 off every paper.
pub const MUTED_INK: [f32; 3] = [0.32, 0.33, 0.36];

/// The text box's paper under bars of `hue`, in linear light: the bars'
/// colour [`PAPER_MIX`] of the way to [`PAPER_WHITE`].
#[must_use]
pub fn paper(hue: Hue) -> [f32; 3] {
    let bar = hue.tone();
    std::array::from_fn(|i| bar[i] + (PAPER_WHITE[i] - bar[i]) * PAPER_MIX)
}

/// The cost's other ink, in sRGB, for an art box too dark for [`INK`]
/// ([`cost_ink`]).
pub const LIGHT_INK: [f32; 3] = [0.85, 0.82, 0.72];

/// The body's numbers once damage has made the toughness matter, in sRGB: a
/// red dark enough to read on the paper, where they stand.
pub const LETHAL_INK: [f32; 3] = [0.62, 0.13, 0.10];

/// An sRGB colour in linear light.
#[must_use]
pub fn linear(srgb: [f32; 3]) -> [f32; 3] {
    srgb.map(|c| {
        if c <= 0.040_45 {
            c / 12.92
        } else {
            ((c + 0.055) / 1.055).powf(2.4)
        }
    })
}

/// How far apart two linear colours stand, as WCAG measures it: from 1:1,
/// the same, to 21:1, black on white.
#[must_use]
pub fn contrast(a: [f32; 3], b: [f32; 3]) -> f32 {
    let luminance = |[r, g, b]: [f32; 3]| 0.2126 * r + 0.7152 * g + 0.0722 * b;
    let (a, b) = (luminance(a), luminance(b));
    (a.max(b) + 0.05) / (a.min(b) + 0.05)
}

/// Which ink the cost is written in.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum CostInk {
    /// [`INK`].
    Dark,
    /// [`LIGHT_INK`].
    Light,
}

impl CostInk {
    /// The ink, in sRGB.
    #[must_use]
    pub const fn srgb(self) -> [f32; 3] {
        match self {
            Self::Dark => INK,
            Self::Light => LIGHT_INK,
        }
    }
}

/// The ink the cost is written in: whichever of [`INK`] and [`LIGHT_INK`]
/// stands further off the art box under it.
///
/// The cost stands at the top of the art box's right half, whose colour is
/// the second of `word`'s art hues at [`ART_TOP`]. That is dark on a black
/// card and light enough on a white one that the light ink would stand at
/// 2.3:1 on it. The hues between are mid-tones, where the better ink stands
/// between 3.4:1 (grey) and 3.9:1 (gold): over WCAG's 3:1 for large text and
/// under its 4.5:1 for body text, and the cost is 10 px.
#[must_use]
pub fn cost_ink(word: u32) -> CostInk {
    let tone = Hue::from_code(word >> (FACE_BARS_SHIFT + 8)).tone();
    let under = tone.map(|c| c * ART_TOP);
    if contrast(linear(INK), under) >= contrast(linear(LIGHT_INK), under) {
        CostInk::Dark
    } else {
        CostInk::Light
    }
}

/// How deep the cost's line on a table face's art box is, in card widths:
/// one line at [`SMALL_EM`] in its bar.
pub const COST_LINE: f32 = LINE_BOX * SMALL_EM + 2.0 * BAR_PAD;

/// A table face's colour discs, each its centre `[x, y]` in card widths and
/// its hue: one for a card whose art box is one colour, two side by side
/// for two ([`DISC_RADIUS`]). `card_common.wgsl`'s `text_face` draws them by
/// the same arithmetic.
#[must_use]
pub fn discs(word: u32, regions: &Regions) -> Vec<([f32; 2], Hue)> {
    let a = Hue::from_code(word >> (FACE_BARS_SHIFT + 4));
    let b = Hue::from_code(word >> (FACE_BARS_SHIFT + 8));
    let y = regions.band[1] + COST_LINE + DISC_DROP + DISC_RADIUS;
    if a == b {
        vec![([0.5, y], a)]
    } else {
        let half = DISC_PAIR * 0.5;
        vec![([0.5 - half, y], a), ([0.5 + half, y], b)]
    }
}

/// Bit 0 of [`face_word`]: the window draws the face. A card with no art
/// and no face — a back, a slab under a pile — leaves it clear and is drawn
/// as its flat colour.
pub const FACE_ON: u32 = 1;

/// Where the face's [`Layout`] starts in [`face_word`]: three bits after
/// [`FACE_ON`], the table's layout being zero.
pub const FACE_LAYOUT_SHIFT: u32 = 1;

/// `word` with its layout set to `layout`.
#[must_use]
pub const fn laid(word: u32, layout: Layout) -> u32 {
    word & !(0x7 << FACE_LAYOUT_SHIFT) | (layout as u32) << FACE_LAYOUT_SHIFT
}

/// The layout a face word carries.
#[must_use]
pub const fn layout_of(word: u32) -> Layout {
    Layout::from_code(word >> FACE_LAYOUT_SHIFT)
}

/// How far the band lightens towards [`PAPER_WHITE`] across it, left to
/// right: the colour deep where its words begin and light under the chips'
/// far end, the identity as a gradient (WP6).
///
/// Only ever towards white, so the band is nowhere darker than its bars'
/// colour, and [`INK`] stands off every point of it at least as far as it
/// stands off the bars: 4.7:1 on black's, the least.
pub const BAND_LIFT: f32 = 0.35;

/// How deep the band's colour goes where its words begin: this share of
/// itself, so a white or a gold band stands off the paper under it...
pub const BAND_DEEP: f32 = 0.6;

/// ...but never under this luminance, at which [`INK`] still stands off it
/// at 4.5:1 — so black's band, already that dark, is not darkened at all.
pub const BAND_LUMA: f32 = 0.23;

/// What the band's colour is multiplied by where its words begin, for a
/// colour of luminance `luma` ([`BAND_DEEP`], [`BAND_LUMA`]). The shader's
/// `band_depth` is the same arithmetic.
#[must_use]
pub fn band_depth(luma: f32) -> f32 {
    (BAND_LUMA / luma.max(1e-4)).clamp(BAND_DEEP, 1.0)
}

/// A colour's relative luminance, as WCAG weighs linear light.
#[must_use]
pub fn luminance([r, g, b]: [f32; 3]) -> f32 {
    0.2126 * r + 0.7152 * g + 0.0722 * b
}

/// A keyword chip's paper, in sRGB: a slate dark enough that
/// [`LIGHT_INK`] reads on it whatever the band under it is.
pub const CHIP_PAPER: [f32; 3] = [0.12, 0.14, 0.19];

/// The foot's credit line's ink, in sRGB, on [`BORDER_INK`]: quiet, and
/// over 7:1 still.
pub const FOOT_INK: [f32; 3] = [0.66, 0.66, 0.70];

/// A table face's colour disc: its radius, and where its centre stands under
/// the cost's line, in card widths. One disc in the art box for a card of
/// one colour, two side by side for two, gold for more and grey for none —
/// the symbol band of the battlefield face (WP6).
pub const DISC_RADIUS: f32 = 0.075;

/// See [`DISC_RADIUS`]: how far below the cost's line the disc's top stands.
pub const DISC_DROP: f32 = 0.012;

/// See [`DISC_RADIUS`]: how far apart two discs' centres stand.
pub const DISC_PAIR: f32 = 0.17;

/// Where the bars' hue starts in [`face_word`]; the art box's two follow it
/// four bits apart.
pub const FACE_BARS_SHIFT: u32 = 4;

/// Where the name bar's depth starts in [`face_word`], eight bits of
/// [`DEPTH_STEP`]s.
pub const FACE_NAME_SHIFT: u32 = 16;

/// Where the type bar's depth starts, eight bits after the name bar's.
pub const FACE_TYPE_SHIFT: u32 = 24;

/// The word a card's material carries for its face: on, the hues of its bars
/// and of the art box's two halves, and how deep its two bars are.
///
/// The colours are the card's colour identity as the rules have it now. A
/// card of one colour is that colour; two or more have gold bars, and a
/// two-colour card's art box runs from one to the other, left to right in
/// the order the colours are always written. A land's bars are grey whatever
/// it makes, as on a printed land; its art box takes the colour of its one
/// basic land type (CR 305.6: that type is what taps for the colour), and
/// grey without exactly one.
#[must_use]
pub fn face_word(colors: ColorSet, types: TypeSet, subtypes: SubtypeSet, depths: Depths) -> u32 {
    use baylee_core::generated::subtypes::land;
    let (bars, art) = if types.contains(TypeSet::LAND) {
        let basic: Vec<Color> = [
            (land::PLAINS, Color::White),
            (land::ISLAND, Color::Blue),
            (land::SWAMP, Color::Black),
            (land::MOUNTAIN, Color::Red),
            (land::FOREST, Color::Green),
        ]
        .into_iter()
        .filter(|(id, _)| subtypes.contains(*id))
        .map(|(_, color)| color)
        .collect();
        let art = match basic.as_slice() {
            [one] => Hue::of(*one),
            _ => Hue::Grey,
        };
        (Hue::Grey, [art, art])
    } else {
        let hues: Vec<Hue> = colors.iter().map(Hue::of).collect();
        match hues.as_slice() {
            [] => (Hue::Grey, [Hue::Grey; 2]),
            [one] => (*one, [*one; 2]),
            [a, b] => (Hue::Gold, [*a, *b]),
            _ => (Hue::Gold, [Hue::Gold; 2]),
        }
    };
    FACE_ON
        | (bars as u32) << FACE_BARS_SHIFT
        | (art[0] as u32) << (FACE_BARS_SHIFT + 4)
        | (art[1] as u32) << (FACE_BARS_SHIFT + 8)
        | u32::from(depths.name) << FACE_NAME_SHIFT
        | u32::from(depths.kind) << FACE_TYPE_SHIFT
}

/// The table's first sentence ([`first_sentence`]), as an em in card widths:
/// 8.5 of `Text2d`'s pixels at the table's hundred to a card width, eight
/// screen pixels on a card 94 wide (WP6).
pub const SENTENCE_EM: f32 = 0.085;

/// How many lines the table's first sentence may take: two stand clear of
/// the plate at the text box's bottom right.
pub const SENTENCE_LINES: usize = 2;

/// `text` set on at most `lines` lines `room` card widths long at `em`,
/// broken after the last word that fits, the last line cut with an
/// ellipsis where the rest still runs over.
#[must_use]
pub fn fit_lines(
    text: &str,
    em: f32,
    room: f32,
    lines: usize,
    width: impl Fn(&str) -> f32,
) -> Vec<String> {
    let fits = |s: &str| width(s) * em <= room;
    let mut out = Vec::new();
    let mut rest = text.trim();
    while !rest.is_empty() && out.len() < lines {
        if fits(rest) {
            out.push(rest.to_owned());
            return out;
        }
        if out.len() + 1 == lines {
            // Whole words first, so a symbol (`{T}`) is never cut in two.
            let words: Vec<&str> = rest.split(' ').collect();
            let kept = (1..words.len()).rev().find_map(|keep| {
                let line = format!("{}{ELLIPSIS}", words[..keep].join(" "));
                fits(&line).then_some(line)
            });
            out.push(kept.unwrap_or_else(|| cut(rest, &fits)));
            return out;
        }
        let (line, after) = break_line(rest, &fits);
        out.push(line.to_owned());
        rest = after.trim_start();
    }
    out
}

/// `text` broken into a first line that fits and the rest: after the last
/// space that leaves the first line fitting, or — for a first word that is
/// longer than a line on its own — after its last character that fits.
fn break_line<'a>(text: &'a str, fits: &impl Fn(&str) -> bool) -> (&'a str, &'a str) {
    let at_space = text
        .match_indices(' ')
        .map(|(at, _)| at)
        .take_while(|&at| fits(&text[..at]))
        .last();
    if let Some(at) = at_space {
        return (&text[..at], &text[at + 1..]);
    }
    let at_char = text
        .char_indices()
        .map(|(at, _)| at)
        .skip(1)
        .take_while(|&at| fits(&text[..at]))
        .last()
        .unwrap_or_else(|| text.chars().next().map_or(0, char::len_utf8));
    (&text[..at_char], &text[at_char..])
}

/// `text` cut to the longest start that fits with an ellipsis after it.
fn cut(text: &str, fits: &impl Fn(&str) -> bool) -> String {
    let at = text
        .char_indices()
        .map(|(at, _)| at)
        .take_while(|&at| fits(&format!("{}{ELLIPSIS}", text[..at].trim_end())))
        .last()
        .unwrap_or(0);
    format!("{}{ELLIPSIS}", text[..at].trim_end())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cardrail;

    /// What every character advances by [`mono`]: chosen so that no line
    /// length in the tests below lands on a size's edge.
    const UNIT: f32 = 0.3;

    /// A width oracle that is not the font: every character [`UNIT`] wide,
    /// so a test's arithmetic is a count.
    fn mono(s: &str) -> f32 {
        #[allow(clippy::cast_precision_loss)]
        let n = s.chars().count() as f32;
        n * UNIT
    }

    /// How many characters a line of the table holds at `em`, by [`mono`].
    fn holds(em: f32) -> usize {
        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
        let n = (line_width() / (UNIT * em)).floor() as usize;
        n
    }

    fn inside(inner: [f32; 4], outer: [f32; 4]) -> bool {
        inner[0] >= outer[0] - 1e-6
            && inner[1] >= outer[1] - 1e-6
            && inner[2] <= outer[2] + 1e-6
            && inner[3] <= outer[3] + 1e-6
    }

    /// Every part lies in the window inside the border, the parts stand in a
    /// card's order down it without overlapping, and each has room: the
    /// table's (name, art box, type, rules, foot) and the interface's three
    /// layouts (name, type, band, rules, foot), at the depths a 56- (an
    /// attachment), a 72- (the stack), a 92- (the hand), a 308-
    /// and a 480-pixel card's bars take.
    #[test]
    fn the_face_is_laid_out_inside_the_window_in_a_card_s_order() {
        let [wx0, wy0, wx1, wy1] = WINDOW;
        let inner = [wx0 + BORDER, wy0 + BORDER, wx1 - BORDER, wy1 - BORDER];
        let mut faces = Vec::new();
        for lines in [1, 2] {
            let r = Regions::table(lines);
            faces.push((
                format!("table, {lines} lines"),
                [r.name_bar, r.band, r.type_bar, r.text_box, r.foot],
            ));
            for px in [56.0, 72.0, 92.0, 308.0, 480.0] {
                for step in [Step::XS, Step::DEFAULT, Step::XL] {
                    let depths = Sizes::overlay_at(px, 0.2, step).depths(lines);
                    for layout in [Layout::Preview, Layout::Long, Layout::Small] {
                        let r = Regions::laid(layout, depths);
                        faces.push((
                            format!("{layout:?} at {px} px, {step:?}, {lines} lines"),
                            [r.name_bar, r.type_bar, r.band, r.text_box, r.foot],
                        ));
                    }
                }
            }
        }
        for (what, order) in faces {
            for part in order {
                assert!(inside(part, inner), "{what}: {part:?} leaves {inner:?}");
                assert!(part[3] > part[1], "{what}: {part:?} has no height");
            }
            for pair in order.windows(2) {
                assert!(
                    pair[0][3] <= pair[1][1] + 1e-6,
                    "{what}: {:?} runs into {:?}",
                    pair[0],
                    pair[1]
                );
            }
        }
    }

    /// The interface's faces give the rules the card's middle (WP6): a
    /// preview's text box is more than half the face — where the art box's
    /// gradient took 45 % of it before — a long preview's deeper by the band
    /// it gave up, and a small card's band is the keyword strip over one
    /// line of rules as deep as the type line.
    #[test]
    fn the_interface_gives_the_rules_the_middle_of_the_card() {
        let share = |r: [f32; 4]| (r[3] - r[1]) / CARD_TALL;
        let depths = Sizes::overlay(308.0, 0.2).depths(1);
        let preview = Regions::laid(Layout::Preview, depths);
        let long = Regions::laid(Layout::Long, depths);
        assert!(share(preview.text_box) > 0.5, "{preview:?}");
        assert!((share(preview.band) - 0.18).abs() < 1e-4);
        assert!((share(long.band) - 0.12).abs() < 1e-4);
        assert!(
            (long.text_box[1] + BAND_PREVIEW - BAND_LONG - preview.text_box[1]).abs() < 1e-6,
            "the long face's rules take the band's room"
        );
        assert!(
            preview.type_bar[1] < preview.band[1],
            "the type under the name"
        );

        let small = Regions::laid(Layout::Small, Sizes::overlay(92.0, 0.2).depths(1));
        let line = small.text_box[3] - small.text_box[1];
        assert!((line - (small.type_bar[3] - small.type_bar[1])).abs() < 1e-6);
        assert!(share(small.band) > 0.3, "{small:?}");

        // The word carries the layout, and a word without one is the table's.
        let word = face_word(
            ColorSet::EMPTY,
            TypeSet::CREATURE,
            SubtypeSet::EMPTY,
            depths,
        );
        assert_eq!(layout_of(word), Layout::Table);
        for layout in [Layout::Table, Layout::Preview, Layout::Long, Layout::Small] {
            let set = laid(word, layout);
            assert_eq!(layout_of(set), layout);
            assert_eq!(
                set & !(0x7 << FACE_LAYOUT_SHIFT),
                word,
                "only the layout moved"
            );
            assert_eq!(laid(set, Layout::Table), word);
        }
        assert_eq!(Layout::from_code(7), Layout::Table);
    }

    /// A step multiplies the clamps and nothing else: the 308-pixel
    /// preview's rules are 19 px at every step, and the hand card's
    /// clamped sizes follow it; the floor is the step's.
    #[test]
    fn a_step_moves_the_clamps_and_the_floor_and_nothing_else() {
        for n in 1..=5 {
            let step = Step::new(n);
            let px = ui_em_at(UI_BODY, 308.0, step) * 308.0;
            assert!((px - 19.096).abs() < 1e-3, "{step:?}: {px}");
            // The share (7.5 px) or the clamp, whichever is larger.
            let name = ui_em_at(UI_NAME, 92.0, step) * 92.0;
            let want = (UI_NAME.0 * 92.0).max(UI_NAME.1[0] * step.factor());
            assert!((name - want).abs() < 1e-3, "{step:?}: {name}");
        }
        // Nothing on a 308-pixel preview moves with the step.
        for size in [UI_NAME, UI_TYPE, UI_BODY, UI_CHIP, UI_FOOT] {
            let at = |step| ui_em_at(size, 308.0, step);
            for n in 1..=5 {
                assert!(
                    (at(Step::new(n)) - at(Step::DEFAULT)).abs() < 1e-6,
                    "{size:?}"
                );
            }
        }
        assert_eq!(Step::new(0), Step::XS);
        assert_eq!(Step::new(9), Step::XL);
        assert_eq!(Step::default(), Step::DEFAULT);
        assert!((Step::DEFAULT.factor() - 1.0).abs() < f32::EPSILON);
        assert!((Step::DEFAULT.body_floor() - BODY_FLOOR_PX).abs() < f32::EPSILON);
        assert_eq!(
            BODY_FLOORS,
            [12.0, 13.0, 14.0, 14.0, 16.0],
            "the floors the design names"
        );
        // A long text stops at its step's floor.
        let deep = |em: f32| 100.0 * em;
        for step in [Step::XS, Step::XL] {
            let got = fit_body_at(308.0, 0.3, step, deep) * 308.0;
            assert!(
                (got - step.body_floor()).abs() < 1.0 && got >= step.body_floor(),
                "{step:?}: {got}"
            );
        }
    }

    /// A keyword line gives a chip a keyword, in the card's own words, and a
    /// sentence gives none.
    #[test]
    fn a_line_of_keywords_gives_the_band_its_chips() {
        assert_eq!(
            keyword_chips([
                "Flying, vigilance",
                "Whenever another Ally enters the battlefield under your control, draw a card.",
                "Protection from black",
            ]),
            ["Flying", "vigilance", "Protection from black"]
        );
        assert_eq!(keyword_chips(["Fliegend"]), ["Fliegend"]);
        for sentence in [
            "This spell can't be countered.",
            "Equip {2}",
            "{T}: Add {G}",
            "Choose one —",
            "I — Create a 1/1 token",
            "Whenever a creature you control attacks, it gets +1/+0 until end of turn",
        ] {
            assert!(keyword_chips([sentence]).is_empty(), "{sentence}");
        }
    }

    /// The band's subtype words are the type line's, one a word where the
    /// card has as many subtypes, and whole where it does not.
    #[test]
    fn the_band_names_the_subtypes_the_type_line_does() {
        assert_eq!(
            subtype_words("Creature — Merfolk Wizard Ally", 3),
            ["Merfolk", "Wizard", "Ally"]
        );
        assert_eq!(
            subtype_words("Artifact Creature — Time Lord", 1),
            ["Time Lord"]
        );
        assert!(subtype_words("Instant", 0).is_empty());
        assert_eq!(
            subtype_words("Kreatur — Meervolk Zauberer", 2),
            ["Meervolk", "Zauberer"]
        );
    }

    /// One line of rules is the first sentence, and a full stop inside a
    /// number does not end it.
    #[test]
    fn a_small_card_reads_the_first_sentence() {
        assert_eq!(
            first_sentence("{T}: Draw a card for each Ally you control. Then discard."),
            "{T}: Draw a card for each Ally you control."
        );
        assert_eq!(first_sentence("Flying"), "Flying");
        assert_eq!(first_sentence("Pay 1.5 life. Draw."), "Pay 1.5 life.");
    }

    /// The table's sentence takes its lines whole and cuts the last at a
    /// word, so a symbol is never cut in two.
    #[test]
    fn a_sentence_is_set_on_its_lines_and_cut_at_a_word() {
        let em = 1.0;
        let room = mono("abcdefghij");
        assert_eq!(fit_lines("abc def", em, room, 2, mono), ["abc def"]);
        assert_eq!(
            fit_lines("abc def ghi jkl", em, room, 2, mono),
            ["abc def", "ghi jkl"]
        );
        let cut = fit_lines("{T}: abc def ghi jkl mno pqr", em, room, 2, mono);
        assert_eq!(cut.len(), 2);
        assert_eq!(cut[0], "{T}: abc");
        assert!(cut[1].ends_with(ELLIPSIS), "{cut:?}");
        for line in &cut {
            assert!(mono(line) <= room + 1e-6, "{line}");
        }
        assert!(fit_lines("", em, room, 2, mono).is_empty());
    }

    /// A preview's type line keeps its front and gives up its end, since the
    /// band says the subtypes again.
    #[test]
    fn a_preview_s_type_line_keeps_its_front() {
        let sizes = Sizes::overlay(308.0, 0.2).beside_type(0.15);
        let room = sizes.type_room;
        let width = mono;
        let line = "Legendary Enchantment Artifact Creature — Human Soldier Warrior";
        assert!(
            width(line) * sizes.type_floor > room,
            "{line} fits as it is"
        );
        let got = fit_type_front_in(&sizes, line, width);
        assert!(got.lines[0].starts_with("Legendary"), "{got:?}");
        assert!(got.lines[0].ends_with(ELLIPSIS), "{got:?}");
        assert!(width(&got.lines[0]) * got.em <= room + 1e-6, "{got:?}");
        assert!(
            !got.lines[0].contains(" —…"),
            "a dash left hanging: {got:?}"
        );
    }

    /// The table's colour discs: one for a card of one colour (or none, or
    /// many: gold), two for two, each under the cost's line and inside the
    /// art box above the tallest one-row strip.
    #[test]
    fn a_table_face_wears_a_disc_per_colour() {
        let r = Regions::table(2);
        let word = |colors: &[Color]| {
            face_word(
                ColorSet::from_slice(colors),
                TypeSet::CREATURE,
                SubtypeSet::EMPTY,
                Depths::table(2),
            )
        };
        assert_eq!(discs(word(&[Color::Blue]), &r).len(), 1);
        assert_eq!(discs(word(&[]), &r)[0].1, Hue::Grey);
        let pair = discs(word(&[Color::White, Color::Black]), &r);
        assert_eq!(
            pair.iter().map(|d| d.1).collect::<Vec<_>>(),
            [Hue::White, Hue::Black]
        );
        let one_row = cardrail::Strip::new(0b111, None, [None, None]).rect();
        for ([x, y], _) in pair {
            assert!(y - DISC_RADIUS >= r.band[1] + COST_LINE, "under the cost");
            assert!(y + DISC_RADIUS <= one_row[1], "over a one-row strip");
            assert!(x - DISC_RADIUS > r.band[0] && x + DISC_RADIUS < r.band[2]);
        }
    }

    /// Every ink on the band reads: [`INK`] on the band at its darkest,
    /// which is a bar's colour, and [`LIGHT_INK`] on a chip's paper; and the
    /// foot's credit on the border.
    #[test]
    fn the_band_s_words_and_chips_and_the_foot_read() {
        let ink = linear(INK);
        // Every hue, and every mix of two the gradient passes through, at
        // its depth where the words begin and lifted where the chips end.
        for a in HUES {
            for b in HUES {
                for t in [0.0, 0.25, 0.5, 0.75, 1.0] {
                    let hue: [f32; 3] =
                        std::array::from_fn(|i| a.tone()[i] + (b.tone()[i] - a.tone()[i]) * t);
                    let deep = hue.map(|c| c * band_depth(luminance(hue)));
                    for lift in [0.0, BAND_LIFT] {
                        let band: [f32; 3] =
                            std::array::from_fn(|i| deep[i] + (PAPER_WHITE[i] - deep[i]) * lift);
                        assert!(contrast(ink, band) >= 4.5, "{a:?}–{b:?} {t} at {lift}");
                    }
                }
            }
        }
        // And a light band stands off its card's paper, which is what the
        // depth is for.
        let white = Hue::White.tone();
        let band = white.map(|c| c * band_depth(luminance(white)));
        assert!(contrast(band, paper(Hue::White)) > 1.5);
        assert!(contrast(linear(LIGHT_INK), linear(CHIP_PAPER)) >= 7.0);
        assert!(contrast(linear(FOOT_INK), BORDER_INK) >= 7.0);
    }

    /// The type bar's top is the strip's bottom edge, so however many marks
    /// a card has, the strip lies on the art box and never on the type line.
    #[test]
    fn the_strip_lies_on_the_art_box_and_never_on_the_type_line() {
        for lines in [1, 2] {
            let r = Regions::table(lines);
            for n in 0..=cardrail::MARK_ORDER.len() {
                let strip = cardrail::Strip::new((1 << n) - 1, None, [None, None]);
                assert!(
                    strip.rect()[3] <= r.type_bar[1] + 1e-6,
                    "{n} marks reach into the type bar"
                );
            }
        }
    }

    /// The table sets the cost on the art box's first line, and the strip
    /// grows up the art box from the seam: the tallest strip must still end
    /// below the cost line, with a two-line name pushing that line as far
    /// down as it goes.
    #[test]
    fn the_tallest_strip_stays_under_the_cost_line() {
        let r = Regions::table(2);
        let cost_bottom = r.band[1] + COST_LINE;
        let tallest = cardrail::Strip::largest().rect();
        assert!(
            cost_bottom <= tallest[1],
            "the cost line ends at {cost_bottom} and a full strip starts at {}",
            tallest[1]
        );
    }

    /// Every bar holds its line with the descenders in: the table's type bar
    /// at the type line's own size, and the name bar at one line or two.
    #[test]
    fn every_bar_holds_its_lines() {
        let r = Regions::table(1);
        assert!(r.type_bar[3] - r.type_bar[1] >= LINE_BOX * SMALL_EM);
        assert!(r.name_bar[3] - r.name_bar[1] >= LINE_BOX * NAME_EM);
        let two = Regions::table(2);
        assert!(two.name_bar[3] - two.name_bar[1] >= 2.0 * LINE_BOX * SMALL_EM);
        // And the text box is still a box: the card keeps room for rules.
        assert!(two.text_box[3] - two.text_box[1] > 0.2);
    }

    /// A depth rides the word rounded up to its step and never down: a bar
    /// the shader draws is at least as deep as the lines it was sized for,
    /// and less than a step deeper.
    #[test]
    fn a_depth_is_carried_rounded_up() {
        for depth in [name_bar(1), name_bar(2), bar(SMALL_EM), 0.1, 0.118, 0.072] {
            let carried = Depths::of(depth, depth);
            assert_eq!(carried.name, carried.kind);
            let back = carried.name_bar();
            assert!(
                back >= depth && back < depth + DEPTH_STEP,
                "{depth} carried as {back}"
            );
        }
        assert_eq!(
            Depths::of(1.0, -1.0),
            Depths {
                name: u8::MAX,
                kind: 0
            }
        );
    }

    /// Each arm of the name's rule, by a width oracle whose sums are counts.
    #[test]
    fn a_name_steps_down_once_then_breaks_then_is_cut() {
        let big = holds(NAME_EM);
        let small = holds(SMALL_EM);
        assert!(small > big, "the smaller size holds no more");

        let short = "a".repeat(big);
        assert_eq!(fit_name(&short, mono), Fitted::one(NAME_EM, &short));

        let middling = "b".repeat(big + 1);
        assert_eq!(fit_name(&middling, mono), Fitted::one(SMALL_EM, &middling));

        // Two words, neither fitting beside the other: one a line.
        let word = "c".repeat(small - 2);
        let two = format!("{word} {word}");
        let fitted = fit_name(&two, mono);
        assert!((fitted.em - SMALL_EM).abs() < f32::EPSILON);
        assert_eq!(fitted.lines, vec![word.clone(), word.clone()]);

        // Three lines' worth: the second line is cut and says so.
        let three = format!("{word} {word} {word}");
        let fitted = fit_name(&three, mono);
        assert_eq!(fitted.lines.len(), 2);
        assert_eq!(fitted.lines[0], word);
        assert!(fitted.lines[1].ends_with(ELLIPSIS), "{fitted:?}");
        for line in &fitted.lines {
            assert!(
                mono(line) * SMALL_EM <= line_width() + 1e-6,
                "{line:?} runs over"
            );
        }
    }

    /// A word longer than a line on its own is broken inside it rather than
    /// cut, so nothing of a two-line name is lost that two lines could hold.
    #[test]
    fn a_word_longer_than_a_line_is_broken_inside_it() {
        let small = holds(SMALL_EM);
        let long = "ä".repeat(small + 3);
        let fitted = fit_name(&long, mono);
        assert_eq!(fitted.lines.len(), 2);
        assert_eq!(fitted.lines.concat(), long, "a character went missing");
        assert_eq!(fitted.lines[0].chars().count(), small);
    }

    /// The type line shrinks once, then gives up words from the front —
    /// supertypes, then the card types and the dash — and only then cuts
    /// subtypes from the end, whole. A line with no subtypes keeps its card
    /// types, and a translated one goes straight to its subtypes.
    #[test]
    fn a_type_line_shrinks_once_then_gives_up_its_front_then_its_end() {
        let small = holds(SMALL_EM);
        let floor = holds(TYPE_FLOOR_EM);
        assert!(floor > small);
        let set = |line: &str| fit_type(line, mono);
        let floor_line = |text: &str| Fitted::one(TYPE_FLOOR_EM, text);

        let fits = "t".repeat(small);
        assert_eq!(set(&fits), Fitted::one(SMALL_EM, &fits));
        let shrinks = "t".repeat(small + 1);
        assert_eq!(set(&shrinks), floor_line(&shrinks));

        // Each line below is too long for the floor as it stands, and each
        // answer fits it: the arithmetic is a count, so check it once here.
        let cases = [
            // The supertype goes first.
            (
                "Legendary Creature — Elf Druid Ally",
                "Creature — Elf Druid Ally",
            ),
            // No supertype to give: the card types and the dash go.
            (
                "Artifact Creature — Human Soldier Ally",
                "Human Soldier Ally",
            ),
            // Both, when one is not enough.
            (
                "Legendary Enchantment Artifact Creature — Human Soldier",
                "Human Soldier",
            ),
            // Subtypes alone still too long: cut from the end, whole.
            (
                "Creature — Human Soldier Warrior Knight Cleric",
                "Human Soldier Warrior Knight…",
            ),
            // No subtypes: the card types stay.
            (
                "Legendary Snow Enchantment Artifact",
                "Enchantment Artifact",
            ),
            // A translated line has no supertype the table knows.
            (
                "Legendäre Kreatur — Elf Druide Verbündeter",
                "Elf Druide Verbündeter",
            ),
        ];
        for (line, want) in cases {
            assert!(
                mono(line) * TYPE_FLOOR_EM > line_width(),
                "{line:?} fits as it is"
            );
            assert!(
                mono(want) * TYPE_FLOOR_EM <= line_width(),
                "{want:?} does not fit"
            );
            assert_eq!(set(line), floor_line(want), "{line:?}");
        }

        // No subtypes and no supertype to give: cut from the end.
        let crowded = "Enchantment Artifact Creature Land";
        let got = set(crowded);
        assert!(got.lines[0].starts_with("Enchantment"), "{got:?}");
        assert!(got.lines[0].ends_with(ELLIPSIS), "{got:?}");
        assert!(mono(&got.lines[0]) * TYPE_FLOOR_EM <= line_width() + 1e-6);
    }

    /// A face wears its colours: one is that colour; two are gold bars over
    /// an art box running from one to the other in the order colours are
    /// written; three or more are gold; none is grey. A land's bars are grey
    /// whatever it makes, and its art box is its one basic land type's colour
    /// (CR 305.6), grey with two.
    #[test]
    fn a_face_wears_its_colours_and_a_land_its_basic_type() {
        use baylee_core::generated::subtypes::land;
        let hues = |word: u32| {
            let at = |shift: u32| (word >> shift) & 0xf;
            (
                at(FACE_BARS_SHIFT),
                at(FACE_BARS_SHIFT + 4),
                at(FACE_BARS_SHIFT + 8),
            )
        };
        let (w, g, gold, grey) = (
            Hue::White as u32,
            Hue::Green as u32,
            Hue::Gold as u32,
            Hue::Grey as u32,
        );
        let of = |colors: &[Color], types: TypeSet, subtypes: &[baylee_core::ids::SubtypeId]| {
            hues(face_word(
                ColorSet::from_slice(colors),
                types,
                SubtypeSet::from_slice(subtypes),
                Depths::table(1),
            ))
        };
        let creature = TypeSet::CREATURE;

        assert_eq!(of(&[Color::Green], creature, &[]), (g, g, g));
        // Written white before green whichever way round it was asked.
        assert_eq!(
            of(&[Color::Green, Color::White], creature, &[]),
            (gold, w, g)
        );
        assert_eq!(
            of(&[Color::White, Color::Blue, Color::Green], creature, &[]),
            (gold, gold, gold)
        );
        assert_eq!(of(&[], TypeSet::ARTIFACT, &[]), (grey, grey, grey));

        let land = TypeSet::LAND;
        assert_eq!(of(&[], land, &[land::FOREST]), (grey, g, g));
        assert_eq!(
            of(&[], land, &[land::FOREST, land::PLAINS]),
            (grey, grey, grey)
        );
        // An animated land is still a land: its colour does not make its bars.
        assert_eq!(
            of(&[Color::Green], land.union(creature), &[land::FOREST]),
            (grey, g, g)
        );

        // The depths ride whole, one byte each, and nothing overlaps.
        let depths = Depths {
            name: 0xa5,
            kind: 0x5a,
        };
        let word = face_word(ColorSet::EMPTY, creature, SubtypeSet::EMPTY, depths);
        assert_eq!(word & FACE_ON, FACE_ON);
        assert_eq!(word >> FACE_NAME_SHIFT & 0xff, 0xa5);
        assert_eq!(word >> FACE_TYPE_SHIFT, 0x5a);
        assert_eq!(hues(word), (grey, grey, grey));
    }

    /// The overlay keeps the table's rule at its own em: its bars follow
    /// the card's width in pixels, its name gives up the room its cost
    /// takes beside it, and the text box takes what the thinner bars leave.
    #[test]
    fn the_overlay_sets_the_table_s_rule_at_its_own_em() {
        assert_eq!(Sizes::TABLE.depths(2), Depths::table(2));

        let hand = Sizes::overlay(92.0, 0.0);
        assert!(
            (hand.name - 9.0 / 92.0).abs() < 1e-4,
            "held at 9 px: {hand:?}"
        );
        assert!((hand.small - 8.0 / 92.0).abs() < 1e-4, "held at 8 px");
        assert!(hand.type_floor < hand.small);
        let preview = Sizes::overlay(480.0, 0.2);
        assert!((preview.name - 36.0 / 480.0).abs() < 1e-5, "held at 36 px");
        assert!((preview.name_room - (line_width() - 0.2 - TEXT_INSET)).abs() < 1e-6);
        assert!((Sizes::overlay(384.0, 0.0).name_room - line_width()).abs() < 1e-6);

        let table = Regions::new(Sizes::TABLE.depths(1));
        let ui = Regions::new(preview.depths(1));
        assert!(ui.name_bar[3] < table.name_bar[3], "a thinner name bar");
        assert!(ui.text_box[1] < table.text_box[1], "and a deeper text box");
        assert!(
            (ui.type_bar[1] - table.type_bar[1]).abs() < f32::EPSILON,
            "both on the seam"
        );

        // The name steps down, then breaks, inside the room the cost left.
        let room = preview.name_room;
        let long = "A".repeat(60);
        let fitted = fit_name_in(&preview, &long, mono);
        assert_eq!(fitted.lines.len(), 2);
        for line in &fitted.lines {
            assert!(mono(line) * preview.small <= room + 1e-6, "{line}");
        }
    }

    /// The rules text keeps its own size if it fits, steps down a pixel at a
    /// time to the floor if it does not, and stops there; a card too small
    /// to reach the floor keeps its own size.
    #[test]
    fn rules_text_steps_down_to_the_floor_and_no_further() {
        let px = |em: f32| em * 384.0;
        let height = 0.3;
        let own = px(ui_em(UI_BODY, 384.0));
        // `n` lines of text, and nothing else, at an em.
        let lines = |n: f32| move |em: f32| n * LINE_BOX * em;
        assert!(
            lines(1.0)(own / 384.0) <= height,
            "one line fits at its own size"
        );
        assert!((px(fit_body(384.0, height, lines(1.0))) - own).abs() < 1e-3);

        // A box just deep enough for six lines six pixels under its own
        // size, and so too shallow at every size between: set there.
        let snug = lines(6.0)((own - 6.0) / 384.0) + 1e-6;
        let got = px(fit_body(384.0, snug, lines(6.0)));
        assert!((got - (own - 6.0)).abs() < 1e-3, "set at {got} px");

        // Too deep for the floor: set at the floor exactly, though the
        // floor is not a whole number of pixels under its own size, and the
        // box scrolls.
        assert!((own - BODY_FLOOR_PX).fract() > 0.1);
        let got = px(fit_body(384.0, height, lines(59.0)));
        assert!((got - BODY_FLOOR_PX).abs() < 1e-3, "set at {got} px");

        // A hand card's own 6 px is under the floor already.
        let tiny = fit_body(92.0, height, lines(59.0)) * 92.0;
        assert!((tiny - 6.0).abs() < 1e-3, "set at {tiny} px");
    }

    /// Every hue the word can carry, in its code's order.
    const HUES: [Hue; 7] = [
        Hue::White,
        Hue::Blue,
        Hue::Black,
        Hue::Red,
        Hue::Green,
        Hue::Gold,
        Hue::Grey,
    ];

    /// The face's words read on whatever the shader draws under them: the
    /// ink on every bar at WCAG's 4.5:1 for body text and on every paper at
    /// its 7:1, and the lethal red and the muted ink on every paper at 4.5:1. The light ink
    /// the table wrote in before the bars were drawn stood at 1.1:1 on a
    /// white bar.
    #[test]
    fn the_ink_reads_on_every_bar_and_every_paper() {
        let ink = linear(INK);
        for hue in HUES {
            let bar = hue.tone();
            let paper = paper(hue);
            let on_bar = contrast(ink, bar);
            assert!(on_bar >= 4.5, "{hue:?}'s bar: {on_bar}");
            let on_paper = contrast(ink, paper);
            assert!(on_paper >= 7.0, "{hue:?}'s paper: {on_paper}");
            let lethal = contrast(linear(LETHAL_INK), paper);
            assert!(lethal >= 4.5, "lethal on {hue:?}'s paper: {lethal}");
            let muted = contrast(linear(MUTED_INK), paper);
            assert!(muted >= 4.5, "muted on {hue:?}'s paper: {muted}");
        }
    }

    /// The cost is written in whichever ink its art box lets it — dark on
    /// white, light on black — and never under 3:1, the most the mid-tones
    /// allow either ink. A hue reads back from its code as the shader reads
    /// it, and a code no hue has is grey.
    #[test]
    fn the_cost_takes_the_ink_its_art_box_lets_it() {
        for hue in HUES {
            assert_eq!(Hue::from_code(hue as u32), hue);
            let word = FACE_ON | (hue as u32) << (FACE_BARS_SHIFT + 8);
            let under = hue.tone().map(|c| c * ART_TOP);
            let on_art = contrast(linear(cost_ink(word).srgb()), under);
            assert!(on_art >= 3.0, "{hue:?}'s art box: {on_art}");
        }
        assert_eq!(Hue::from_code(0xf), Hue::Grey);
        let white = face_word(
            ColorSet::from_slice(&[Color::White]),
            TypeSet::CREATURE,
            SubtypeSet::EMPTY,
            Depths::table(1),
        );
        assert_eq!(cost_ink(white), CostInk::Dark);
        let black = face_word(
            ColorSet::from_slice(&[Color::Black]),
            TypeSet::CREATURE,
            SubtypeSet::EMPTY,
            Depths::table(1),
        );
        assert_eq!(cost_ink(black), CostInk::Light);
        // The right-hand hue is the one under the cost.
        let orzhov = face_word(
            ColorSet::from_slice(&[Color::White, Color::Black]),
            TypeSet::CREATURE,
            SubtypeSet::EMPTY,
            Depths::table(1),
        );
        assert_eq!(cost_ink(orzhov), CostInk::Light);
    }

    /// The average says what the font would say about an ordinary name to
    /// within a sensible margin: it is only a stand-in, but it must not put a
    /// short name on two lines. The shipped font's advances make "Llanowar
    /// Elves" 5.739 em.
    #[test]
    fn the_average_is_near_the_font_on_an_ordinary_name() {
        let average = average_width("Llanowar Elves");
        assert!((average - 5.739).abs() < 0.3, "{average}");
        assert_eq!(fit_name("Llanowar Elves", average_width).lines.len(), 1);
    }
}
