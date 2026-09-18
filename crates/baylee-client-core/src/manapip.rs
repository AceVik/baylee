//! Mana symbols as glyphs of the `mana` font.
//!
//! This is the only door the client draws a mana symbol through, and
//! `docs/legal.md` §2a is why there is exactly one: the font is OFL 1.1 and
//! the symbols it draws are Wizards' trademarks, some of which — the
//! planeswalker symbol, the guild and clan watermarks — that policy names as
//! off limits outright. A glyph constant therefore lives here and nowhere
//! else, so the set in use can be read off one page.
//! It answers only *which glyph on which disc*, and
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
    /// `{T}`, the tap symbol: a clockwise arrow.
    ///
    /// Not a mana symbol — `ManaCost` cannot hold it and never should — but
    /// it is printed in the same ink on the same disc, and it is the symbol a
    /// player reads most often. Verified against the shipped font rather than
    /// taken from the stylesheet: the glyph at this codepoint rasterises to
    /// the clockwise arrow, and `\u{e61c}` beside it is the older tilted T.
    pub const TAP: char = '\u{e61a}';
    /// `{Q}`, untap: the same arrow the other way round.
    pub const UNTAP: char = '\u{e61b}';
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
    /// A planeswalker's loyalty cost, drawn as the badge the card prints.
    Loyalty(Loyalty),
}

/// Which way a loyalty badge points.
///
/// The shape *is* the sign. That is how the card reads — an upward shield for
/// a cost that adds and a downward one for a cost that takes — and it is what
/// makes the badge legible at the size a list row gives it: the number written
/// on it carries no sign of its own, because a mark eleven pixels across has
/// room for the digits or for the sign, and the shape is already saying the
/// sign.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Tick {
    /// A cost that adds loyalty: the shield points up.
    Up,
    /// A cost that takes it: the shield points down.
    Down,
    /// A cost that changes nothing, which is a shape of its own rather than
    /// an upward badge reading `+0`. A point is a sign, and zero has none.
    Flat,
}

/// A planeswalker's loyalty cost, as the card prints it.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Loyalty {
    /// Which way the badge points.
    pub tick: Tick,
    /// What is written on it, or `None` for the `−X` a few walkers print.
    ///
    /// `u8` because the deepest cost ever printed is `−40` and the engine's
    /// own [`AbilityDef::Loyalty`] carries an `i8`, so nothing the rules can
    /// express reaches past it.
    ///
    /// [`AbilityDef::Loyalty`]: https://docs.rs/baylee-cards-dsl
    pub amount: Option<u8>,
}

impl Loyalty {
    /// What is written on the badge: the digits, or `X`.
    #[must_use]
    pub fn caption(&self) -> String {
        self.amount
            .map_or_else(|| "X".to_string(), |n| n.to_string())
    }
}

/// The brace token a loyalty cost is written as.
///
/// The one place that spells it, because a cost is handed to the renderer as
/// a *string* — [`Pip`] is what comes out the other end, and the door between
/// them is [`symbol`]. `{L+2}`, `{L0}`, `{L-12}`, `{L-X}`; the hyphen is
/// ASCII because this is a token and not prose, and the minus a player reads
/// is the badge's own shape.
#[must_use]
pub fn loyalty_token(cost: i8) -> String {
    match cost.cmp(&0) {
        std::cmp::Ordering::Greater => format!("{{L+{cost}}}"),
        std::cmp::Ordering::Less => format!("{{L-{}}}", cost.unsigned_abs()),
        std::cmp::Ordering::Equal => "{L0}".to_string(),
    }
}

/// The loyalty badge a **printed** cost asks for, if it is one.
///
/// [`loyalty_token`] writes what the client says to itself; this reads what
/// the *card* says. A planeswalker's line begins `+2: `, `0: ` or `−1: ` —
/// prose, with no brace and no `L` — and `generated_lines` carries it that
/// way, because it carries the printed sentence.
///
/// The same parser answers both, so the two doors cannot drift, and its
/// strictness is what makes this one safe to point at any cost prefix: the
/// only unsigned number it accepts is zero, so `{2}{B}, {T}` and
/// `Sacrifice a Land` are refused rather than read as flat badges. A cost is
/// printed the same way in every language, so this asks nothing of the one
/// the player reads in.
#[must_use]
pub fn printed_loyalty(cost: &str) -> Option<Loyalty> {
    loyalty(&format!("L{}", cost.trim()))
}

/// The loyalty badge a `{L…}` token asks for, if it asks for one.
///
/// Deliberately strict about the shapes a card actually prints: a signed
/// number points, an unsigned one is the zero and nothing else. `{L5}` is not
/// a cost any walker has, and reading it as a flat five would put a badge on
/// screen that no card could have produced.
#[must_use]
fn loyalty(body: &str) -> Option<Loyalty> {
    let rest = body.strip_prefix(['L', 'l'])?;
    let mut chars = rest.chars();
    let (tick, digits) = match chars.next()? {
        '+' => (Tick::Up, chars.as_str()),
        // The token is written with an ASCII hyphen; the printed minus is
        // accepted beside it so a line quoted off a card can use this door
        // too, and both catalogs print U+2212.
        '-' | '\u{2212}' => (Tick::Down, chars.as_str()),
        _ => (Tick::Flat, rest),
    };
    let amount = match digits {
        "" => return None,
        "X" | "x" => None,
        n => Some(n.parse::<u8>().ok()?),
    };
    if tick == Tick::Flat && amount != Some(0) {
        return None;
    }
    Some(Loyalty { tick, amount })
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

/// The pip for one mana in a **pool**.
///
/// [`of_color`] cannot answer this and never will: a `Color` is one of the
/// five (CR 105.1) and mana in a pool may also be colourless, which is not a
/// colour (CR 106.1b). Same table, read from the other set.
#[must_use]
pub fn of_mana(color: baylee_core::mana::ManaColor) -> Pip {
    use baylee_core::mana::ManaColor;
    pip(match color {
        ManaColor::White => ManaSymbol::White,
        ManaColor::Blue => ManaSymbol::Blue,
        ManaColor::Black => ManaSymbol::Black,
        ManaColor::Red => ManaSymbol::Red,
        ManaColor::Green => ManaSymbol::Green,
        ManaColor::Colorless => ManaSymbol::Colorless,
    })
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

/// One piece of a line that mixes prose with printed symbols.
#[derive(Clone, PartialEq, Eq, Debug)]
pub enum Segment {
    /// Prose, set in the interface font.
    Text(String),
    /// A symbol, drawn on its disc.
    Symbol(Pip),
}

/// The pip for one `{...}` symbol, whether or not it is mana.
///
/// `body` is what stands between the braces. Two symbols are not mana at all
/// and cannot be: `ManaCost` holds mana, and the tap symbol is a cost a
/// permanent pays with itself. They are printed in the same ink on the same
/// disc, so they belong in the same table — the alternative was the letter
/// "T" in a sentence full of real symbols.
#[must_use]
pub fn symbol(body: &str) -> Option<Pip> {
    let disc = Disc::Generic;
    match body {
        "T" | "t" => {
            return Some(Pip::Solid {
                glyph: glyph::TAP,
                disc,
            });
        }
        "Q" | "q" => {
            return Some(Pip::Solid {
                glyph: glyph::UNTAP,
                disc,
            });
        }
        _ => {}
    }
    // Before the fallthrough and not after it, the way the two above are:
    // `ManaCost::try_parse` would refuse `{L+2}` and the run would come out
    // as prose *with its braces*, which is the token on screen.
    if let Some(loy) = loyalty(body) {
        return Some(Pip::Loyalty(loy));
    }
    let mut drawn = parse(&format!("{{{body}}}"))?;
    // Exactly one: `{W}{U}` inside one pair of braces is not a symbol, and
    // letting it through would draw two pips where the text has one.
    if drawn.len() == 1 { drawn.pop() } else { None }
}

/// Splits a line into the prose and the symbols written in it.
///
/// `{T}: Add {G}.` is three symbols' worth of meaning in a line the interface
/// has always drawn as letters. Everything outside a brace run stays prose; a
/// brace run this table does not know stays prose **with its braces**, since
/// `{Q}` on screen tells a player more than a hole does, and an unclosed brace
/// is prose to the end of the line rather than a symbol that swallows it.
#[must_use]
pub fn segments(text: &str) -> Vec<Segment> {
    let mut out: Vec<Segment> = Vec::new();
    let mut prose = String::new();
    let mut rest = text;
    while let Some(open) = rest.find('{') {
        let (before, from_brace) = rest.split_at(open);
        prose.push_str(before);
        let Some(close) = from_brace.find('}') else {
            prose.push_str(from_brace);
            rest = "";
            break;
        };
        let body = &from_brace[1..close];
        match symbol(body) {
            Some(pip) => {
                if !prose.is_empty() {
                    out.push(Segment::Text(std::mem::take(&mut prose)));
                }
                out.push(Segment::Symbol(pip));
            }
            None => prose.push_str(&from_brace[..=close]),
        }
        rest = &from_brace[close + 1..];
    }
    prose.push_str(rest);
    if !prose.is_empty() {
        out.push(Segment::Text(prose));
    }
    out
}

/// How much of a prose segment must stay with the mark before it.
///
/// A renderer sets a printed line as a *row of items* — a disc is a node and
/// the words around it are nodes — and a row that wraps may break between any
/// two of them. So `{T}: Add {U} or {B}. This artifact deals 1 damage to you.`
/// wrapped after the last disc and began its second line with the full stop,
/// which reads as a sentence that has come apart. It was invisible in English,
/// where that line fits, and the owner saw it the moment the German arrived —
/// `Erzeuge {U} oder {B}. Dieses Artefakt fügt dir 1 Schadenspunkt zu.`
///
/// The answer is typographic and not a special case: **punctuation belongs to
/// what precedes it.** This counts the bytes of it at the head of a segment,
/// and a renderer glues that much to the mark it follows so the pair cannot be
/// parted. A space ends the run, because the words after it are a new place a
/// line may break and should be.
///
/// Answers 0 for a segment that starts with a letter or a space, which is
/// most of them.
#[must_use]
pub fn clings(text: &str) -> usize {
    text.char_indices()
        .find(|(_, c)| {
            !matches!(
                c,
                '.' | ',' | ':' | ';' | '!' | '?' | ')' | ']' | '\u{2019}' | '\u{201d}'
            )
        })
        .map_or(text.len(), |(at, _)| at)
}

/// One piece of a printed line **quoted** in the interface's own prose.
///
/// The difference from [`Segment`] is one of register, not of parsing. A card
/// *shown* — the hover preview, a deckbuilder row — wears its symbols as the
/// printed discs, because colour identity is a thing a player reads off them.
/// A card *quoted* — the sentence under a stack row, twelve points in the
/// row's own muted ink — has no room for one: the disc is ten pixels across
/// there, and the mark inside it, which is the only part carrying "tap", is
/// exactly the part a disc that small takes the contrast from. So a quotation
/// keeps the mark and drops the disc, the way rules documents have always set
/// mana symbols in body text. It also keeps the stack panel one typesetter's
/// work — its name and its subtitle are real text, and a flex row of words
/// underneath them reads as a second one.
#[derive(Clone, PartialEq, Eq, Debug)]
pub enum Inline {
    /// Prose, set in the interface font.
    Text(String),
    /// One glyph of the `mana` font, set at the prose's size and in its ink.
    Mark(char),
}

/// Splits a line into the prose and the bare marks written in it.
///
/// Built on [`segments`], so the three edge cases are answered in one place:
/// a brace run that is not a symbol keeps its braces, an unclosed brace is
/// prose to the end of the line, and `{W}{U}` inside one pair of braces is
/// not a symbol. What this adds is the two pips that have no single glyph to
/// be. A hybrid is one disc with two glyphs clipped to opposite halves, which
/// is not something a character can be, so it is quoted the way the oracle
/// text writes it — the two marks with a slash between them. A generic cost
/// past the font's range is already digits on the card and stays digits here.
///
/// Neighbouring prose is merged, so a refused symbol does not leave three
/// pieces where the line has one run of text.
#[must_use]
pub fn inline(text: &str) -> Vec<Inline> {
    fn prose(out: &mut Vec<Inline>, run: &str) {
        match out.last_mut() {
            Some(Inline::Text(before)) => before.push_str(run),
            _ => out.push(Inline::Text(run.to_string())),
        }
    }

    let mut out: Vec<Inline> = Vec::new();
    for segment in segments(text) {
        match segment {
            Segment::Text(run) => prose(&mut out, &run),
            Segment::Symbol(Pip::Solid { glyph, .. }) => out.push(Inline::Mark(glyph)),
            Segment::Symbol(Pip::Split { left, right }) => {
                out.push(Inline::Mark(left.0));
                prose(&mut out, "/");
                out.push(Inline::Mark(right.0));
            }
            Segment::Symbol(Pip::Number { value }) => prose(&mut out, &value.to_string()),
            // A quotation keeps the mark and drops the disc, and a loyalty
            // badge is all disc: its shape is the whole symbol, so there is
            // no mark left to keep. It is written out the way the card does
            // instead — which is also what the stack panel has always shown,
            // since a printed line arrives here with `+2` already in it.
            Segment::Symbol(Pip::Loyalty(loy)) => {
                let sign = match loy.tick {
                    Tick::Up => "+",
                    Tick::Down => "\u{2212}",
                    Tick::Flat => "",
                };
                prose(&mut out, &format!("{sign}{}", loy.caption()));
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    /// Punctuation belongs to what precedes it, and a space ends the run.
    ///
    /// The rule a wrapped ability sheet turned on: a row of items may break
    /// between any two of them, so a segment beginning `". Dieses Artefakt"`
    /// put the full stop at the head of the second line.
    #[test]
    fn punctuation_clings_to_the_mark_before_it() {
        assert_eq!(clings(". Dieses Artefakt fügt dir"), 1);
        assert_eq!(clings(": Add "), 1);
        assert_eq!(clings(", "), 1);
        assert_eq!(clings(" or "), 0);
        assert_eq!(clings("Add "), 0);
        assert_eq!(clings(""), 0);
        // A whole segment of punctuation is all cling — the sentence's own
        // full stop after its last symbol.
        assert_eq!(clings("."), 1);
        // And it never splits a character: the byte count is a boundary.
        let text = "… und";
        assert!(text.is_char_boundary(clings(text)));
    }

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
                Pip::Loyalty(loy) => panic!("a mana cost is not a loyalty cost: {loy:?}"),
            }
        }
    }

    /// The tap symbol is the one a player reads most often and the one thing
    /// `ManaCost` can never hold, so it has its own door.
    #[test]
    fn the_tap_symbol_is_drawn_and_is_not_mana() {
        assert_eq!(
            symbol("T"),
            Some(Pip::Solid {
                glyph: glyph::TAP,
                disc: Disc::Generic
            }),
        );
        assert_eq!(
            symbol("Q"),
            Some(Pip::Solid {
                glyph: glyph::UNTAP,
                disc: Disc::Generic
            }),
        );
        assert!(
            ManaCost::try_parse("{T}").is_err(),
            "if this ever parses as mana, the door above is the wrong one",
        );
    }

    /// A brace run that is more than one symbol is not a symbol.
    #[test]
    fn two_symbols_in_one_brace_pair_are_not_a_pip() {
        assert!(symbol("W").is_some());
        assert!(symbol("WU").is_none(), "{{WU}} is not a printed symbol");
        assert!(symbol("nonsense").is_none());
    }

    /// The line the whole thing exists for.
    #[test]
    fn a_line_is_split_into_its_prose_and_its_symbols() {
        let segs = segments("{T}: Add {G}.");
        assert_eq!(
            segs,
            vec![
                Segment::Symbol(symbol("T").expect("tap")),
                Segment::Text(": Add ".to_string()),
                Segment::Symbol(symbol("G").expect("green")),
                Segment::Text(".".to_string()),
            ],
        );
    }

    /// Both ways of writing nothing the table knows: prose keeps its braces,
    /// and an unclosed brace does not eat the rest of the line.
    #[test]
    fn an_unreadable_brace_run_stays_prose_with_its_braces() {
        assert_eq!(
            segments("Pay {Energy} now"),
            vec![Segment::Text("Pay {Energy} now".to_string())],
        );
        assert_eq!(
            segments("half a brace {W"),
            vec![Segment::Text("half a brace {W".to_string())],
        );
        assert_eq!(segments(""), Vec::new(), "an empty line draws nothing");
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

    /// The whole point of the quoted register: three characters of source
    /// become one mark, and the prose either side of it stays one run.
    #[test]
    fn a_quoted_line_keeps_the_mark_and_drops_the_braces() {
        assert_eq!(
            inline("{T}: Add {G}."),
            vec![
                Inline::Mark(glyph::TAP),
                Inline::Text(": Add ".into()),
                Inline::Mark(glyph::GREEN),
                Inline::Text(".".into()),
            ]
        );
    }

    /// A hybrid is one disc with two halves, and no character is that. Quoted,
    /// it is what the oracle text prints minus its braces.
    #[test]
    fn a_hybrid_is_quoted_as_the_two_marks_the_card_prints() {
        assert_eq!(
            inline("{W/U}"),
            vec![
                Inline::Mark(glyph::WHITE),
                Inline::Text("/".into()),
                Inline::Mark(glyph::BLUE),
            ]
        );
    }

    /// Inherited from [`segments`] and worth pinning here, because a reader
    /// that silently ate what it did not know would quote the card wrongly:
    /// a run with no symbol keeps its braces, and neighbouring prose merges
    /// rather than leaving three pieces where the line has one.
    #[test]
    fn a_run_this_table_refuses_is_quoted_as_written() {
        assert_eq!(
            inline("Sacrifice {this}, then draw."),
            vec![Inline::Text("Sacrifice {this}, then draw.".into())]
        );
        assert_eq!(
            inline("Pay {1000000} life."),
            vec![Inline::Text("Pay 1000000 life.".into())]
        );
    }

    /// The token and the badge are one round trip, because the spelling has
    /// exactly one author.
    #[test]
    fn a_loyalty_cost_survives_the_token_it_is_written_as() {
        for (cost, tick, amount) in [
            (2_i8, Tick::Up, Some(2_u8)),
            (0, Tick::Flat, Some(0)),
            (-1, Tick::Down, Some(1)),
            (-12, Tick::Down, Some(12)),
            (-40, Tick::Down, Some(40)),
        ] {
            let token = loyalty_token(cost);
            let body = token.trim_start_matches('{').trim_end_matches('}');
            assert_eq!(
                symbol(body),
                Some(Pip::Loyalty(Loyalty { tick, amount })),
                "{token}"
            );
        }
    }

    /// `−X` is printed by a handful of walkers and is not expressible in the
    /// engine's own `i8`, so it has no token — but the reader answers it, and
    /// the badge says `X` rather than declining and putting `{L-X}` on screen.
    #[test]
    fn the_minus_x_a_few_walkers_print_has_a_badge() {
        let pip = symbol("L-X").expect("a badge");
        let Pip::Loyalty(loy) = pip else {
            panic!("not a badge: {pip:?}")
        };
        assert_eq!(loy.tick, Tick::Down);
        assert_eq!(loy.caption(), "X");
        assert_eq!(symbol("L\u{2212}X"), Some(pip), "the printed minus too");
    }

    /// The counter-test, and the one that matters: the door is narrow enough
    /// that nothing else falls through it. A generic zero is a mana symbol
    /// and must stay one, an unsigned number is not a loyalty cost any card
    /// prints, and a run this refuses keeps its braces.
    #[test]
    fn nothing_but_a_loyalty_cost_comes_through_that_door() {
        assert_eq!(
            symbol("0"),
            Some(Pip::Solid {
                glyph: generic_glyph(0).expect("the font spells zero"),
                disc: Disc::Generic,
            }),
            "{{0}} is generic mana and stays generic mana"
        );
        for refused in ["L5", "L", "L+", "LX", "L+1.5", "Loyal", "L+999"] {
            assert_eq!(symbol(refused), None, "{refused}");
        }
        assert_eq!(
            segments("Sacrifice {L5}."),
            vec![Segment::Text("Sacrifice {L5}.".into())],
            "a refused run keeps its braces"
        );
    }

    /// The printed door reads what a card prints, and — the half that keeps
    /// it safe to point at *any* activation cost — refuses everything else a
    /// cost prefix can be.
    #[test]
    fn a_printed_cost_is_a_badge_only_when_a_walker_printed_it() {
        for (printed, tick, amount) in [
            ("+2", Tick::Up, Some(2_u8)),
            ("+12", Tick::Up, Some(12)),
            ("0", Tick::Flat, Some(0)),
            ("-1", Tick::Down, Some(1)),
            ("\u{2212}1", Tick::Down, Some(1)),
            ("\u{2212}X", Tick::Down, None),
        ] {
            assert_eq!(
                printed_loyalty(printed),
                Some(Loyalty { tick, amount }),
                "{printed}"
            );
        }
        for refused in [
            "",
            // The discriminating one: a cost of one generic mana looks like a
            // zero with braces round it, and is the other door's answer.
            "{0}",
            "{T}",
            "{2}, {T}",
            "{2}{B}, {T}, Sacrifice this",
            "Sacrifice a Land",
            "Level up {2}",
            "2",
            "X",
            "Whenever this creature attacks, draw a card",
        ] {
            assert_eq!(printed_loyalty(refused), None, "{refused}");
        }
    }

    /// Quoted, the badge is written out the way the card writes it — which is
    /// what a printed line already carries, so the stack panel reads the same
    /// before and after this door existed.
    #[test]
    fn a_badge_quoted_in_prose_is_the_cost_the_card_prints() {
        assert_eq!(
            inline("{L+2}: Look at the top card."),
            vec![Inline::Text("+2: Look at the top card.".into())]
        );
        assert_eq!(
            inline("{L-12}: Exile all cards."),
            vec![Inline::Text("\u{2212}12: Exile all cards.".into())]
        );
        assert_eq!(
            inline("{L0}: Draw three cards."),
            vec![Inline::Text("0: Draw three cards.".into())]
        );
    }
}
