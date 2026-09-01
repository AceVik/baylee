//! Mana symbols as glyphs of the `mana` font.
//!
//! `docs/legal.md` §2 allows exactly two ways to draw a mana symbol: pips the
//! client draws itself, or the open-licensed `mana` font (SIL OFL 1.1). This
//! module is the second one. It answers only *which glyph on which disc*, and
//! nothing about how either is painted, so the whole table is testable without
//! a GPU — and a symbol the font cannot spell falls back to a number rather
//! than to an empty box.
//!
//! The font draws a symbol the way the web does: a coloured disc supplied by
//! the page, with a monochrome glyph on top. A hybrid is two glyphs, each
//! clipped to its half of the disc — there is no single hybrid glyph to ask
//! for, which is why [`Pip::Split`] exists.

use baylee_core::color::Color;
use baylee_core::mana::{ManaSymbol, Variable};

/// Codepoints in the `mana` font's private-use block.
///
/// Taken from the font's own stylesheet (`css/mana.css`), which is the only
/// place the mapping is published.
mod glyph {
    /// `{W}`.
    pub const WHITE: char = '\u{e600}';
    /// `{U}`.
    pub const BLUE: char = '\u{e601}';
    /// `{B}`.
    pub const BLACK: char = '\u{e602}';
    /// `{R}`.
    pub const RED: char = '\u{e603}';
    /// `{G}`.
    pub const GREEN: char = '\u{e604}';
    /// `{C}`.
    pub const COLORLESS: char = '\u{e904}';
    /// `{S}`.
    pub const SNOW: char = '\u{e619}';
    /// `{X}`.
    pub const X: char = '\u{e615}';
    /// `{Y}`.
    pub const Y: char = '\u{e616}';
    /// `{Z}`.
    pub const Z: char = '\u{e617}';
    /// The Phyrexian mark, worn on the colour's own disc.
    pub const PHYREXIAN: char = '\u{e618}';
    /// `{∞}`.
    pub const INFINITY: char = '\u{e903}';
    /// `{½}`.
    pub const HALF: char = '\u{e902}';
    /// `{0}`; `{1}`..`{15}` follow it consecutively.
    pub const ZERO: char = '\u{e605}';
    /// `{16}`; `{17}`..`{20}` follow it consecutively.
    pub const SIXTEEN: char = '\u{e62a}';
    /// The largest generic cost the font spells with one glyph.
    pub const LARGEST_GENERIC: u32 = 20;
}

/// The disc a glyph sits on.
///
/// A disc, not a colour: the renderer picks the actual paint, because the
/// same symbol is drawn darker on a card face than on a list row.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum Disc {
    /// White mana.
    White,
    /// Blue mana.
    Blue,
    /// Black mana.
    Black,
    /// Red mana.
    Red,
    /// Green mana.
    Green,
    /// Generic, colorless and the variables.
    Generic,
    /// Snow, which is a property rather than a colour.
    Snow,
}

impl Disc {
    /// The disc a colour is paid on.
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
}

/// How one mana symbol is drawn.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Pip {
    /// One glyph on one disc.
    Solid {
        /// The glyph to set in the `mana` font.
        glyph: char,
        /// The disc under it.
        disc: Disc,
    },
    /// Two glyphs, each clipped to its half of one disc: the hybrids, the
    /// twobrids and the hybrid Phyrexians.
    Split {
        /// Left half.
        left: (char, Disc),
        /// Right half.
        right: (char, Disc),
    },
    /// A generic cost past the font's glyph range, set as digits instead.
    Number {
        /// The amount to spell out.
        value: u32,
    },
}

/// The glyph for a generic cost, when the font has one.
#[must_use]
fn generic_glyph(n: u32) -> Option<char> {
    let base = match n {
        0..=15 => u32::from(glyph::ZERO),
        16..=glyph::LARGEST_GENERIC => u32::from(glyph::SIXTEEN) - 16,
        _ => return None,
    };
    char::from_u32(base + n)
}

/// The glyph one colour wears.
#[must_use]
const fn color_glyph(color: Color) -> char {
    match color {
        Color::White => glyph::WHITE,
        Color::Blue => glyph::BLUE,
        Color::Black => glyph::BLACK,
        Color::Red => glyph::RED,
        Color::Green => glyph::GREEN,
    }
}

/// One half of a hybrid: the colour's glyph on the colour's disc.
#[must_use]
const fn half(color: Color) -> (char, Disc) {
    (color_glyph(color), Disc::of(color))
}

/// How to draw one mana symbol.
///
/// Total: every [`ManaSymbol`] has an answer, so a cost never renders a hole.
#[must_use]
pub fn pip(symbol: ManaSymbol) -> Pip {
    match symbol {
        ManaSymbol::Generic(n) => {
            generic_glyph(n).map_or(Pip::Number { value: n }, |glyph| Pip::Solid {
                glyph,
                disc: Disc::Generic,
            })
        }
        ManaSymbol::Colorless => Pip::Solid {
            glyph: glyph::COLORLESS,
            disc: Disc::Generic,
        },
        ManaSymbol::White
        | ManaSymbol::Blue
        | ManaSymbol::Black
        | ManaSymbol::Red
        | ManaSymbol::Green => {
            // Every one-colour symbol is its colour's glyph on its own disc;
            // going through `ColorSet` keeps that mapping in one place.
            let color = one_color(symbol);
            let (glyph, disc) = half(color);
            Pip::Solid { glyph, disc }
        }
        ManaSymbol::Hybrid(pair) => Pip::Split {
            left: half(pair.first()),
            right: half(pair.second()),
        },
        ManaSymbol::TwoOrColor(color) => Pip::Split {
            left: (generic_glyph(2).unwrap_or(glyph::ZERO), Disc::Generic),
            right: half(color),
        },
        ManaSymbol::Phyrexian(color) => Pip::Solid {
            glyph: glyph::PHYREXIAN,
            disc: Disc::of(color),
        },
        ManaSymbol::HybridPhyrexian(pair) => Pip::Split {
            left: (glyph::PHYREXIAN, Disc::of(pair.first())),
            right: (glyph::PHYREXIAN, Disc::of(pair.second())),
        },
        ManaSymbol::Snow => Pip::Solid {
            glyph: glyph::SNOW,
            disc: Disc::Snow,
        },
        ManaSymbol::Variable(v) => Pip::Solid {
            glyph: match v {
                Variable::X => glyph::X,
                Variable::Y => glyph::Y,
                Variable::Z => glyph::Z,
            },
            disc: Disc::Generic,
        },
        ManaSymbol::HalfGeneric => Pip::Solid {
            glyph: glyph::HALF,
            disc: Disc::Generic,
        },
        ManaSymbol::Infinite => Pip::Solid {
            glyph: glyph::INFINITY,
            disc: Disc::Generic,
        },
    }
}

/// The colour behind a one-colour symbol.
///
/// Only ever called for the five single-colour variants; anything else is a
/// caller bug, and white is the least surprising thing to draw for one.
#[must_use]
fn one_color(symbol: ManaSymbol) -> Color {
    match symbol {
        ManaSymbol::Blue => Color::Blue,
        ManaSymbol::Black => Color::Black,
        ManaSymbol::Red => Color::Red,
        ManaSymbol::Green => Color::Green,
        _ => Color::White,
    }
}

/// The pips of a whole cost, in printed order.
#[must_use]
pub fn cost(cost: &baylee_core::mana::ManaCost) -> Vec<Pip> {
    cost.symbols().map(pip).collect()
}

/// The pip for one colour of mana, for places that count colours rather than
/// read a cost (a deck's colour breakdown, a mana pool).
#[must_use]
pub fn of_color(color: Color) -> Pip {
    let (glyph, disc) = half(color);
    Pip::Solid { glyph, disc }
}

/// Parses a printed cost string and returns its pips, or `None` when the
/// string is not a cost the rules can express.
///
/// The catalog hands the builder cost strings straight from Scryfall, so this
/// is the path most of the UI takes.
#[must_use]
pub fn parse(text: &str) -> Option<Vec<Pip>> {
    if text.trim().is_empty() {
        return None;
    }
    baylee_core::mana::ManaCost::try_parse(text)
        .ok()
        .map(|c| cost(&c))
}

#[cfg(test)]
mod tests {
    use super::*;
    use baylee_core::mana::ManaCost;

    /// Every symbol the rules can express has a glyph or a number — the one
    /// thing that must never happen is a symbol drawn as an empty box.
    #[test]
    fn every_symbol_the_parser_accepts_has_something_to_draw() {
        // Two costs because `MAX_SYMBOLS` is sixteen; between them they name
        // every variant of `ManaSymbol`.
        let mut pips = Vec::new();
        for text in [
            "{2}{W}{U}{B}{R}{G}{C}{S}{X}{Y}{Z}{W/U}{2/R}{G/P}{W/U/P}",
            "{½}{∞}",
        ] {
            let cost = ManaCost::parse(text);
            let drawn = super::cost(&cost);
            assert_eq!(drawn.len(), cost.symbols().count());
            pips.extend(drawn);
        }
        for pip in pips {
            match pip {
                Pip::Solid { glyph, .. } => assert!(!glyph.is_control()),
                Pip::Split { left, right } => {
                    assert!(!left.0.is_control() && !right.0.is_control());
                }
                Pip::Number { value } => {
                    assert!(value > glyph::LARGEST_GENERIC, "small costs have glyphs");
                }
            }
        }
    }

    /// The generic run is two blocks in the font, not one; an off-by-one here
    /// draws `{16}` as `{17}` and nobody would notice from the code.
    #[test]
    fn the_generic_glyphs_run_in_two_blocks() {
        assert_eq!(generic_glyph(0), Some('\u{e605}'));
        assert_eq!(generic_glyph(1), Some('\u{e606}'));
        assert_eq!(generic_glyph(15), Some('\u{e614}'));
        assert_eq!(generic_glyph(16), Some('\u{e62a}'));
        assert_eq!(generic_glyph(20), Some('\u{e62e}'));
        assert_eq!(generic_glyph(21), None);
    }

    /// `{1000000}` is a real printed cost and the font stops at twenty.
    #[test]
    fn a_cost_past_the_glyph_range_is_drawn_as_digits() {
        assert_eq!(
            pip(ManaSymbol::Generic(1_000_000)),
            Pip::Number { value: 1_000_000 }
        );
    }

    /// A hybrid is two halves in printed order, each wearing its own colour.
    #[test]
    fn a_hybrid_keeps_the_printed_order_of_its_two_colours() {
        let pips = super::cost(&ManaCost::parse("{W/U}"));
        let Pip::Split { left, right } = pips[0] else {
            panic!("a hybrid is a split pip");
        };
        assert_eq!(left, (glyph::WHITE, Disc::White));
        assert_eq!(right, (glyph::BLUE, Disc::Blue));
    }

    /// A twobrid pays two generic *or* one colour, and reads that way round.
    #[test]
    fn a_twobrid_leads_with_the_two() {
        let pips = super::cost(&ManaCost::parse("{2/R}"));
        let Pip::Split { left, right } = pips[0] else {
            panic!("a twobrid is a split pip");
        };
        assert_eq!(left.1, Disc::Generic);
        assert_eq!(right, (glyph::RED, Disc::Red));
    }

    /// Phyrexian mana wears its colour's disc and the Phyrexian mark, not the
    /// colour's own glyph — that is what tells it apart from plain `{G}`.
    #[test]
    fn phyrexian_mana_keeps_its_colour_but_not_its_glyph() {
        assert_eq!(
            pip(ManaSymbol::Phyrexian(Color::Green)),
            Pip::Solid {
                glyph: glyph::PHYREXIAN,
                disc: Disc::Green,
            }
        );
    }

    /// The UI mostly holds cost *strings*, so the string path has to work.
    #[test]
    fn a_printed_cost_string_parses_to_the_same_pips() {
        assert_eq!(
            parse("{3}{W}{U}"),
            Some(super::cost(&ManaCost::parse("{3}{W}{U}")))
        );
        assert_eq!(parse(""), None);
        assert_eq!(parse("   "), None);
        assert_eq!(parse("{Q}"), None);
    }
}
