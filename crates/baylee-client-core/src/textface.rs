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
//! # Widths
//!
//! The fitting rules need to know how wide a string is before anything has
//! laid it out, because a two-line name bar is part of the card's material.
//! They take the answer as a function, `width(s)`: the advance of `s` at an em
//! of one. The client answers from the shipped font's own advances; until the
//! font has arrived, [`average_width`] does.
//!
//! Lengths are card widths from the card's top-left corner, `y` growing down
//! the card, and rectangles are `[x0, y0, x1, y1]` — [`crate::cardframe`]'s
//! convention.

use baylee_core::color::{Color, ColorSet};
use baylee_core::types::{SubtypeSet, SupertypeSet, TypeSet};

use crate::cardframe;

/// The dark border round the face, inside the print's window, in card widths.
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
pub const TEXT_FOOT: f32 = 1.155;

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
/// font does for all but 179 of them, and those are refitted when the font
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
    let [x0, _, x1, _] = cardframe::window();
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

/// The face's parts, each `[x0, y0, x1, y1]`.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Regions {
    /// The name, left.
    pub name_bar: [f32; 4],
    /// Where a print has its picture; the cost's first line on the table.
    pub art_box: [f32; 4],
    /// The type line, its top on the seam.
    pub type_bar: [f32; 4],
    /// The rules text.
    pub text_box: [f32; 4],
}

impl Regions {
    /// The parts for bars of these depths.
    #[must_use]
    pub fn new(depths: Depths) -> Self {
        let [x0, x1] = content_x();
        let top = cardframe::FRAME_TOP + BORDER;
        let name_end = top + depths.name_bar();
        let type_end = seam() + depths.type_bar();
        Self {
            name_bar: [x0, top, x1, name_end],
            art_box: [x0, name_end + PINLINE, x1, seam()],
            type_bar: [x0, seam(), x1, type_end],
            text_box: [x0, type_end + BOX_GAP, x1, TEXT_FOOT],
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
        Self::of(name_bar(lines), bar(SMALL_EM))
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

/// The table's name bar for a name on `lines` lines: one line holds the name
/// at its own size, two hold it at the smaller one.
#[must_use]
pub fn name_bar(lines: usize) -> f32 {
    if lines > 1 {
        2.0 * LINE_BOX * SMALL_EM + 2.0 * BAR_PAD
    } else {
        bar(NAME_EM)
    }
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
    let room = line_width();
    let natural = width(name);
    if natural * NAME_EM <= room {
        return Fitted::one(NAME_EM, name);
    }
    if natural * SMALL_EM <= room {
        return Fitted::one(SMALL_EM, name);
    }
    let fits = |s: &str| width(s) * SMALL_EM <= room;
    let (first, rest) = break_line(name, &fits);
    let second = if fits(rest) {
        rest.to_owned()
    } else {
        cut(rest, &fits)
    };
    Fitted {
        em: SMALL_EM,
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
    let room = line_width();
    let natural = width(type_line);
    if natural * SMALL_EM <= room {
        return Fitted::one(SMALL_EM, type_line);
    }
    if natural * TYPE_FLOOR_EM <= room {
        return Fitted::one(TYPE_FLOOR_EM, type_line);
    }
    let fits = |s: &str| width(s) * TYPE_FLOOR_EM <= room;
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
        return Fitted::one(TYPE_FLOOR_EM, &unsuper);
    }
    let Some(subtypes) = subtypes else {
        return Fitted::one(TYPE_FLOOR_EM, &cut(plain, &fits));
    };
    if fits(subtypes) {
        return Fitted::one(TYPE_FLOOR_EM, subtypes);
    }
    let words: Vec<&str> = subtypes.split(' ').collect();
    let kept = (1..words.len()).rev().find_map(|keep| {
        let line = format!("{}{ELLIPSIS}", words[..keep].join(" "));
        fits(&line).then_some(line)
    });
    Fitted::one(TYPE_FLOOR_EM, &kept.unwrap_or_else(|| cut(subtypes, &fits)))
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

/// Bit 0 of [`face_word`]: the window draws the face. A card with no art
/// and no face — a back, a slab under a pile — leaves it clear and is drawn
/// as its flat colour.
pub const FACE_ON: u32 = 1;

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
    /// card's order down it without overlapping, and each has room.
    #[test]
    fn the_face_is_laid_out_inside_the_window_in_a_card_s_order() {
        let [wx0, wy0, wx1, wy1] = cardframe::window();
        let inner = [wx0 + BORDER, wy0 + BORDER, wx1 - BORDER, wy1 - BORDER];
        for lines in [1, 2] {
            let r = Regions::table(lines);
            let order = [r.name_bar, r.art_box, r.type_bar, r.text_box];
            for part in order {
                assert!(
                    inside(part, inner),
                    "{lines} lines: {part:?} leaves {inner:?}"
                );
                assert!(part[3] > part[1], "{lines} lines: {part:?} has no height");
            }
            for pair in order.windows(2) {
                assert!(
                    pair[0][3] <= pair[1][1] + 1e-6,
                    "{lines} lines: {:?} runs into {:?}",
                    pair[0],
                    pair[1]
                );
            }
        }
    }

    /// The type bar's top is the strip's bottom edge, so however many marks
    /// a card has, the strip lies on the art box and never on the type line.
    #[test]
    fn the_strip_lies_on_the_art_box_and_never_on_the_type_line() {
        for lines in [1, 2] {
            let r = Regions::table(lines);
            for n in 0..=cardrail::MARK_ORDER.len() {
                assert!(
                    cardrail::strip_rect(n)[3] <= r.type_bar[1] + 1e-6,
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
        let cost_bottom = r.art_box[1] + bar(SMALL_EM);
        let tallest = cardrail::strip_rect(cardrail::MARK_ORDER.len());
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
                "Legendary Artifact Creature — Human Soldier",
                "Human Soldier",
            ),
            // Subtypes alone still too long: cut from the end, whole.
            (
                "Creature — Human Soldier Warrior Knight Cleric",
                "Human Soldier Warrior…",
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
    /// its 7:1, and the lethal red on every paper at 4.5:1. The light ink
    /// the table wrote in before the bars were drawn stood at 1.1:1 on a
    /// white bar.
    #[test]
    fn the_ink_reads_on_every_bar_and_every_paper() {
        let ink = linear(INK);
        for hue in HUES {
            let bar = hue.tone();
            let paper: [f32; 3] =
                std::array::from_fn(|i| bar[i] + (PAPER_WHITE[i] - bar[i]) * PAPER_MIX);
            let on_bar = contrast(ink, bar);
            assert!(on_bar >= 4.5, "{hue:?}'s bar: {on_bar}");
            let on_paper = contrast(ink, paper);
            assert!(on_paper >= 7.0, "{hue:?}'s paper: {on_paper}");
            let lethal = contrast(linear(LETHAL_INK), paper);
            assert!(lethal >= 4.5, "lethal on {hue:?}'s paper: {lethal}");
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
