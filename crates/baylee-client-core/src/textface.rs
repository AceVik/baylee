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

use baylee_core::types::SupertypeSet;

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
    /// The parts for a name bar and a type bar of these heights.
    #[must_use]
    pub fn new(name_bar: f32, type_bar: f32) -> Self {
        let [x0, x1] = content_x();
        let top = cardframe::FRAME_TOP + BORDER;
        let name_end = top + name_bar;
        let type_end = seam() + type_bar;
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
        Self::new(name_bar(lines), bar(SMALL_EM))
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
