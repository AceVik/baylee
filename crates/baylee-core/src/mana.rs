//! Mana symbols, costs, and pools. Canonical notation: `docs/mana-notation.md`.
//!
//! Costs parse at compile time via [`crate::mana!`] or at runtime via
//! [`ManaCost::try_parse`]. Payment solving lives in the engine; this module
//! holds only the data model.

use crate::color::{Color, ColorPair, ColorSet};
use core::str::FromStr;
use serde::{Deserialize, Serialize};

/// Maximum number of symbols in one mana cost (Progenitus has 10).
pub const MAX_SYMBOLS: usize = 16;

/// Variable mana symbols `{X}`, `{Y}`, `{Z}`.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Serialize, Deserialize)]
pub enum Variable {
    /// `{X}`.
    X,
    /// `{Y}`.
    Y,
    /// `{Z}`.
    Z,
}

/// A single mana symbol within a cost.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Serialize, Deserialize)]
pub enum ManaSymbol {
    /// Generic mana `{n}`; payable with any mana.
    Generic(u32),
    /// Colorless `{C}`.
    Colorless,
    /// One white mana.
    White,
    /// One blue mana.
    Blue,
    /// One black mana.
    Black,
    /// One red mana.
    Red,
    /// One green mana.
    Green,
    /// Hybrid `{A/B}`: one mana of either color.
    Hybrid(ColorPair),
    /// `{2/A}`: two generic OR one colored.
    TwoOrColor(Color),
    /// Phyrexian `{A/P}`: one colored OR 2 life.
    Phyrexian(Color),
    /// Hybrid Phyrexian `{A/B/P}`: one of two colors OR 2 life.
    HybridPhyrexian(ColorPair),
    /// Snow `{S}` (property of the producing source).
    Snow,
    /// Variable `{X}`/`{Y}`/`{Z}`.
    Variable(Variable),
    /// `{½}` (silver-bordered).
    HalfGeneric,
    /// `{∞}` (silver-bordered).
    Infinite,
}

impl ManaSymbol {
    /// Converted-mana-cost contribution (CR 202.3; variables and
    /// silver-bordered symbols contribute 0).
    #[must_use]
    pub const fn cmc_contribution(self) -> u32 {
        match self {
            ManaSymbol::Generic(n) => n,
            ManaSymbol::TwoOrColor(_) => 2,
            ManaSymbol::Variable(_) | ManaSymbol::HalfGeneric | ManaSymbol::Infinite => 0,
            _ => 1,
        }
    }

    /// Amount this symbol adds to the generic (any-mana) part of a cost.
    #[must_use]
    pub const fn generic_contribution(self) -> u32 {
        match self {
            ManaSymbol::Generic(n) => n,
            _ => 0,
        }
    }

    /// Colors referenced by this symbol (hybrid counts both).
    #[must_use]
    pub const fn colors(self) -> ColorSet {
        match self {
            ManaSymbol::White => ColorSet::of(Color::White),
            ManaSymbol::Blue => ColorSet::of(Color::Blue),
            ManaSymbol::Black => ColorSet::of(Color::Black),
            ManaSymbol::Red => ColorSet::of(Color::Red),
            ManaSymbol::Green => ColorSet::of(Color::Green),
            ManaSymbol::Hybrid(p) | ManaSymbol::HybridPhyrexian(p) => ColorSet::of_pair(p),
            ManaSymbol::Phyrexian(c) | ManaSymbol::TwoOrColor(c) => ColorSet::of(c),
            _ => ColorSet::EMPTY,
        }
    }
}

const fn sym_sort_key(s: ManaSymbol) -> u32 {
    // Category order: variables/generic/silver first, then WUBRG-colored,
    // then colorless/snow. Within a category, order by color bits then value.
    let (cat, colors, value) = match s {
        ManaSymbol::Variable(v) => (0u32, 0u32, v as u32),
        ManaSymbol::Generic(n) => (1, 0, n),
        ManaSymbol::HalfGeneric => (2, 0, 0),
        ManaSymbol::Infinite => (2, 0, 1),
        ManaSymbol::White => (3, Color::White.bit() as u32, 0),
        ManaSymbol::Blue => (3, Color::Blue.bit() as u32, 0),
        ManaSymbol::Black => (3, Color::Black.bit() as u32, 0),
        ManaSymbol::Red => (3, Color::Red.bit() as u32, 0),
        ManaSymbol::Green => (3, Color::Green.bit() as u32, 0),
        ManaSymbol::Hybrid(p) => (4, p.bits() as u32, 0),
        ManaSymbol::TwoOrColor(c) => (5, c.bit() as u32, 0),
        ManaSymbol::Phyrexian(c) => (6, c.bit() as u32, 0),
        ManaSymbol::HybridPhyrexian(p) => (7, p.bits() as u32, 0),
        ManaSymbol::Snow => (8, 0, 0),
        ManaSymbol::Colorless => (9, 0, 0),
    };
    (cat << 16) | (colors << 8) | if value > 255 { 255 } else { value }
}

/// An immutable mana cost in canonical order.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Serialize, Deserialize)]
pub struct ManaCost {
    symbols: [Option<ManaSymbol>; MAX_SYMBOLS],
    len: u8,
    generic_total: u32,
    cmc: u32,
    has_variable: bool,
}

impl Default for ManaCost {
    fn default() -> Self {
        Self::ZERO
    }
}

impl ManaCost {
    /// The empty cost (lands, tokens, suspend-only cards).
    pub const ZERO: Self = Self {
        symbols: [None; MAX_SYMBOLS],
        len: 0,
        generic_total: 0,
        cmc: 0,
        has_variable: false,
    };

    /// Parses a cost literal, panicking on invalid input.
    ///
    /// # Panics
    /// When `src` is not valid mana notation — a compile error in `const`
    /// contexts (that is the point of [`crate::mana!`]).
    #[must_use]
    pub const fn parse(src: &str) -> Self {
        match Self::try_parse(src) {
            Ok(cost) => cost,
            Err(_) => panic!("invalid mana cost literal in mana!()"),
        }
    }

    /// Parses a cost literal (`"{2}{W/U}{W/P}"`, `""` for zero cost).
    ///
    /// # Errors
    /// A static description of the first syntax violation.
    pub const fn try_parse(src: &str) -> Result<Self, &'static str> {
        let bytes = src.as_bytes();
        let mut cost = Self::ZERO;
        let mut i = 0usize;
        while i < bytes.len() {
            if bytes[i] != b'{' {
                return Err("expected '{'");
            }
            let mut j = i + 1;
            while j < bytes.len() && bytes[j] != b'}' {
                j += 1;
            }
            if j >= bytes.len() {
                return Err("unclosed '{'");
            }
            let sym = match parse_symbol(bytes, i + 1, j) {
                Ok(s) => s,
                Err(e) => return Err(e),
            };
            cost.push_sorted(sym);
            i = j + 1;
        }
        Ok(cost)
    }

    const fn push_sorted(&mut self, sym: ManaSymbol) {
        let mut idx = self.len as usize;
        assert!(idx < MAX_SYMBOLS, "mana cost has too many symbols");
        let key = sym_sort_key(sym);
        while idx > 0 {
            let prev = self.symbols[idx - 1];
            match prev {
                Some(p) if sym_sort_key(p) > key => {
                    self.symbols[idx] = prev;
                    idx -= 1;
                }
                _ => break,
            }
        }
        self.symbols[idx] = Some(sym);
        self.len += 1;
        self.cmc += sym.cmc_contribution();
        self.generic_total += sym.generic_contribution();
        if matches!(sym, ManaSymbol::Variable(_)) {
            self.has_variable = true;
        }
    }

    /// Number of symbols.
    #[must_use]
    pub const fn len(&self) -> u8 {
        self.len
    }

    /// Whether the cost is empty.
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.len == 0
    }

    /// Converted/mana value (CR 202.3).
    #[must_use]
    pub const fn cmc(&self) -> u32 {
        self.cmc
    }

    /// Total generic (any-mana) requirement.
    #[must_use]
    pub const fn generic_total(&self) -> u32 {
        self.generic_total
    }

    /// Whether the cost contains `{X}`/`{Y}`/`{Z}`.
    #[must_use]
    pub const fn has_variable(&self) -> bool {
        self.has_variable
    }

    /// All colors referenced by the cost (hybrid counts both).
    #[must_use]
    pub const fn colors(&self) -> ColorSet {
        let mut set = ColorSet::EMPTY;
        let mut i = 0usize;
        while i < self.len as usize {
            if let Some(s) = self.symbols[i] {
                set = set.union(s.colors());
            }
            i += 1;
        }
        set
    }

    /// Iterates the symbols in canonical order.
    pub fn symbols(&self) -> impl Iterator<Item = ManaSymbol> + '_ {
        self.symbols
            .iter()
            .take(self.len as usize)
            .filter_map(|s| *s)
    }

    /// The cost with `{X}`/`{Y}`/`{Z}` replaced by `Generic(x)` (CR 601.2b).
    #[must_use]
    pub fn with_x(&self, x: u32) -> Self {
        let mut out = Self::ZERO;
        for s in self.symbols() {
            let s = match s {
                ManaSymbol::Variable(_) => ManaSymbol::Generic(x),
                other => other,
            };
            out.push_sorted(s);
        }
        out
    }

    /// The cost with up to `n` generic mana removed (delve/convoke).
    #[must_use]
    pub fn with_less_generic(&self, n: u32) -> Self {
        let mut out = Self::ZERO;
        let mut remaining = n;
        for s in self.symbols() {
            match s {
                ManaSymbol::Generic(amount) => {
                    let cut = amount.min(remaining);
                    remaining -= cut;
                    if amount - cut > 0 {
                        out.push_sorted(ManaSymbol::Generic(amount - cut));
                    }
                }
                other => out.push_sorted(other),
            }
        }
        out
    }

    /// Two costs combined (additional costs like kicker stack onto the
    /// base cost, CR 601.2f).
    #[must_use]
    pub fn combine(&self, other: &ManaCost) -> Self {
        let mut out = *self;
        for s in other.symbols() {
            out.push_sorted(s);
        }
        out
    }
}

const fn parse_color_byte(b: u8) -> Option<Color> {
    match b {
        b'W' | b'w' => Some(Color::White),
        b'U' | b'u' => Some(Color::Blue),
        b'B' | b'b' => Some(Color::Black),
        b'R' | b'r' => Some(Color::Red),
        b'G' | b'g' => Some(Color::Green),
        _ => None,
    }
}

const fn parse_number(bytes: &[u8], start: usize, end: usize) -> Result<u32, &'static str> {
    if start >= end {
        return Err("empty number");
    }
    let mut n: u32 = 0;
    let mut i = start;
    while i < end {
        let b = bytes[i];
        if !b.is_ascii_digit() {
            return Err("invalid number in symbol");
        }
        n = n * 10 + (b - b'0') as u32;
        i += 1;
    }
    Ok(n)
}

const fn parse_symbol(bytes: &[u8], start: usize, end: usize) -> Result<ManaSymbol, &'static str> {
    let len = end - start;
    if len == 0 {
        return Err("empty symbol");
    }
    // Silver-bordered specials (multi-byte UTF-8).
    if len == 2 && bytes[start] == 0xC2 && bytes[start + 1] == 0xBD {
        return Ok(ManaSymbol::HalfGeneric);
    }
    if len == 3 && bytes[start] == 0xE2 && bytes[start + 1] == 0x88 && bytes[start + 2] == 0x9E {
        return Ok(ManaSymbol::Infinite);
    }
    // Hybrid / split symbols contain '/'.
    let mut slash1 = None;
    let mut slash2 = None;
    let mut i = start;
    while i < end {
        if bytes[i] == b'/' {
            if slash1.is_none() {
                slash1 = Some(i);
            } else if slash2.is_none() {
                slash2 = Some(i);
            } else {
                return Err("too many '/' in symbol");
            }
        }
        i += 1;
    }
    if let Some(s1) = slash1 {
        let p0_len = s1 - start;
        if let Some(s2) = slash2 {
            // Three parts: {A/B/P} hybrid phyrexian.
            let p2_len = end - (s2 + 1);
            if p0_len == 1 && (s2 - s1 - 1) == 1 && p2_len == 1 && bytes[s2 + 1] == b'P' {
                let a = parse_color_byte(bytes[start]);
                let b = parse_color_byte(bytes[s1 + 1]);
                if let (Some(a), Some(b)) = (a, b) {
                    return Ok(ManaSymbol::HybridPhyrexian(ColorPair::new(a, b)));
                }
            }
            return Err("invalid three-part hybrid symbol");
        }
        let p1_len = end - (s1 + 1);
        // {2/A}: two-or-color.
        if p0_len == 1 && bytes[start] == b'2' && p1_len == 1 {
            if let Some(c) = parse_color_byte(bytes[s1 + 1]) {
                return Ok(ManaSymbol::TwoOrColor(c));
            }
            return Err("invalid 2-or-color symbol");
        }
        // {A/P}: phyrexian.
        if p1_len == 1 && bytes[s1 + 1] == b'P' && p0_len == 1 {
            if let Some(c) = parse_color_byte(bytes[start]) {
                return Ok(ManaSymbol::Phyrexian(c));
            }
            return Err("invalid phyrexian symbol");
        }
        // {A/B}: hybrid.
        if p0_len == 1 && p1_len == 1 {
            let a = parse_color_byte(bytes[start]);
            let b = parse_color_byte(bytes[s1 + 1]);
            if let (Some(a), Some(b)) = (a, b) {
                return Ok(ManaSymbol::Hybrid(ColorPair::new(a, b)));
            }
        }
        return Err("invalid hybrid symbol");
    }
    // Single-part symbols.
    if len == 1 {
        let b = bytes[start];
        if b.is_ascii_digit() {
            return Ok(ManaSymbol::Generic((b - b'0') as u32));
        }
        if let Some(c) = parse_color_byte(b) {
            return Ok(match c {
                Color::White => ManaSymbol::White,
                Color::Blue => ManaSymbol::Blue,
                Color::Black => ManaSymbol::Black,
                Color::Red => ManaSymbol::Red,
                Color::Green => ManaSymbol::Green,
            });
        }
        return match b {
            b'C' | b'c' => Ok(ManaSymbol::Colorless),
            b'S' | b's' => Ok(ManaSymbol::Snow),
            b'X' | b'x' => Ok(ManaSymbol::Variable(Variable::X)),
            b'Y' | b'y' => Ok(ManaSymbol::Variable(Variable::Y)),
            b'Z' | b'z' => Ok(ManaSymbol::Variable(Variable::Z)),
            _ => Err("unknown symbol"),
        };
    }
    // Digits: generic mana.
    let n = match parse_number(bytes, start, end) {
        Ok(n) => n,
        Err(e) => return Err(e),
    };
    Ok(ManaSymbol::Generic(n))
}

impl core::fmt::Display for ManaSymbol {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            ManaSymbol::Generic(n) => write!(f, "{n}"),
            ManaSymbol::Colorless => write!(f, "C"),
            ManaSymbol::White => write!(f, "W"),
            ManaSymbol::Blue => write!(f, "U"),
            ManaSymbol::Black => write!(f, "B"),
            ManaSymbol::Red => write!(f, "R"),
            ManaSymbol::Green => write!(f, "G"),
            ManaSymbol::Hybrid(p) => write!(f, "{}/{}", p.first().symbol(), p.second().symbol()),
            ManaSymbol::TwoOrColor(c) => write!(f, "2/{}", c.symbol()),
            ManaSymbol::Phyrexian(c) => write!(f, "{}/P", c.symbol()),
            ManaSymbol::HybridPhyrexian(p) => {
                write!(f, "{}/{}/P", p.first().symbol(), p.second().symbol())
            }
            ManaSymbol::Snow => write!(f, "S"),
            ManaSymbol::Variable(Variable::X) => write!(f, "X"),
            ManaSymbol::Variable(Variable::Y) => write!(f, "Y"),
            ManaSymbol::Variable(Variable::Z) => write!(f, "Z"),
            ManaSymbol::HalfGeneric => write!(f, "½"),
            ManaSymbol::Infinite => write!(f, "∞"),
        }
    }
}

impl core::fmt::Display for ManaCost {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        for s in self.symbols() {
            write!(f, "{{{s}}}")?;
        }
        Ok(())
    }
}

/// Mana parsing error (runtime).
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("invalid mana cost: {0}")]
pub struct ManaParseError(&'static str);

impl FromStr for ManaCost {
    type Err = ManaParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::try_parse(s).map_err(ManaParseError)
    }
}

/// Colors of mana that can exist in a mana pool.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Serialize, Deserialize)]
pub enum ManaColor {
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
    /// Colorless.
    Colorless = 5,
}

impl ManaColor {
    /// All six mana colors, index order.
    pub const ALL: [ManaColor; 6] = [
        ManaColor::White,
        ManaColor::Blue,
        ManaColor::Black,
        ManaColor::Red,
        ManaColor::Green,
        ManaColor::Colorless,
    ];

    /// Pool array index.
    #[inline]
    #[must_use]
    pub const fn index(self) -> usize {
        self as usize
    }

    /// Converts a [`Color`] (never colorless).
    #[must_use]
    pub const fn from_color(c: Color) -> Self {
        match c {
            Color::White => ManaColor::White,
            Color::Blue => ManaColor::Blue,
            Color::Black => ManaColor::Black,
            Color::Red => ManaColor::Red,
            Color::Green => ManaColor::Green,
        }
    }
}

/// Opaque reference to a spend-restriction descriptor owned by the engine
/// ("spend only to cast creature spells", …).
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Default, Serialize, Deserialize)]
#[serde(transparent)]
pub struct RestrictionId(pub u32);

/// Flags on a produced mana unit.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Default, Debug, Serialize, Deserialize)]
#[serde(transparent)]
pub struct ManaFlags(u8);

impl ManaFlags {
    /// No flags.
    pub const NONE: Self = Self(0);
    /// Does not empty from the pool as steps/phases end.
    pub const NO_EMPTY: Self = Self(1);
    /// Produced by a snow source.
    pub const SNOW: Self = Self(2);

    /// Whether all flags of `other` are set.
    #[must_use]
    pub const fn contains(self, other: Self) -> bool {
        self.0 & other.0 == other.0
    }

    /// Union.
    #[must_use]
    pub const fn union(self, other: Self) -> Self {
        Self(self.0 | other.0)
    }

    /// Raw bits.
    #[must_use]
    pub const fn bits(self) -> u8 {
        self.0
    }
}

/// Mana with riders, kept separately from the plain counters.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Serialize, Deserialize)]
pub struct RestrictedMana {
    /// Color of the mana.
    pub color: ManaColor,
    /// Amount.
    pub amount: u16,
    /// Flags (no-empty, snow).
    pub flags: ManaFlags,
    /// Spend restriction; `RestrictionId(0)` = unrestricted.
    pub restriction: RestrictionId,
}

/// A player's mana pool: six plain counters plus restricted mana.
#[derive(Clone, PartialEq, Eq, Hash, Debug, Default, Serialize, Deserialize)]
pub struct ManaPool {
    plain: [u16; 6],
    restricted: Vec<RestrictedMana>,
}

impl ManaPool {
    /// An empty pool.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Adds plain mana.
    pub fn add(&mut self, color: ManaColor, amount: u16) {
        self.plain[color.index()] = self.plain[color.index()].saturating_add(amount);
    }

    /// Adds restricted mana (riders preserved).
    pub fn add_restricted(&mut self, mana: RestrictedMana) {
        self.restricted.push(mana);
    }

    /// Available amount of a plain color.
    #[must_use]
    pub fn available(&self, color: ManaColor) -> u16 {
        self.plain[color.index()]
    }

    /// Tries to spend plain mana; returns success.
    pub fn spend(&mut self, color: ManaColor, amount: u16) -> bool {
        let slot = &mut self.plain[color.index()];
        if *slot >= amount {
            *slot -= amount;
            true
        } else {
            false
        }
    }

    /// Total mana currently in the pool.
    #[must_use]
    pub fn total(&self) -> u32 {
        let plain: u32 = self.plain.iter().map(|&n| u32::from(n)).sum();
        plain
            + self
                .restricted
                .iter()
                .map(|r| u32::from(r.amount))
                .sum::<u32>()
    }

    /// Whether the pool is completely empty.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.total() == 0
    }

    /// Colors currently available (plain part only).
    #[must_use]
    pub fn colors_available(&self) -> ColorSet {
        let mut set = ColorSet::EMPTY;
        for c in [
            Color::White,
            Color::Blue,
            Color::Black,
            Color::Red,
            Color::Green,
        ] {
            if self.plain[ManaColor::from_color(c).index()] > 0 {
                set = set.union(ColorSet::of(c));
            }
        }
        set
    }

    /// Empties the pool except mana flagged [`ManaFlags::NO_EMPTY`]
    /// (CR 106.4 — called as steps and phases end).
    pub fn empty_at_step_end(&mut self) {
        self.plain = [0; 6];
        self.restricted
            .retain(|r| r.flags.contains(ManaFlags::NO_EMPTY));
    }

    /// Restricted entries (engine payment solver).
    #[must_use]
    pub fn restricted(&self) -> &[RestrictedMana] {
        &self.restricted
    }

    /// Removes and returns the restricted entry with the given id.
    pub fn take_restricted(&mut self, id: u32) -> Option<RestrictedMana> {
        let pos = self.restricted.iter().position(|m| m.restriction.0 == id)?;
        Some(self.restricted.remove(pos))
    }
}

impl ManaCost {
    /// A cost of one symbol.
    #[must_use]
    pub fn from_symbol(symbol: ManaSymbol) -> Self {
        let mut out = Self::ZERO;
        out.push_sorted(symbol);
        out
    }

    /// A generic-only cost `{n}`.
    #[must_use]
    pub fn from_symbol_generic(n: u32) -> Self {
        let mut out = Self::ZERO;
        if n > 0 {
            out.push_sorted(ManaSymbol::Generic(n));
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_simple_costs() {
        let c = ManaCost::parse("{2}{U}{U}");
        assert_eq!(c.len(), 3);
        assert_eq!(c.cmc(), 4);
        assert_eq!(c.generic_total(), 2);
        assert_eq!(c.to_string(), "{2}{U}{U}");
    }

    #[test]
    fn parses_hybrid_and_phyrexian() {
        let c = ManaCost::parse("{2}{W/U}{W/P}{G/U/P}");
        assert_eq!(c.len(), 4);
        // {2}=2, hybrid=1, phyrexian=1, hybrid-phyrexian=1
        assert_eq!(c.cmc(), 5);
        assert!(c.colors().contains(Color::White));
        assert!(c.colors().contains(Color::Green));
        assert!(c.colors().contains(Color::Blue));
        assert!(!c.colors().contains(Color::Red));
        assert_eq!(c.to_string(), "{2}{W/U}{W/P}{G/U/P}");
    }

    #[test]
    fn parses_two_or_color_and_snow_and_variables() {
        let c = ManaCost::parse("{2/W}{S}{X}");
        assert_eq!(c.cmc(), 3); // 2/W → 2, S → 1, X → 0
        assert!(c.has_variable());
        assert_eq!(c.to_string(), "{X}{2/W}{S}");
    }

    #[test]
    fn parses_silver_bordered() {
        let c = ManaCost::parse("{½}{∞}");
        assert_eq!(c.len(), 2);
        assert_eq!(c.cmc(), 0);
        let big = ManaCost::parse("{1000000}");
        assert_eq!(big.cmc(), 1_000_000);
    }

    #[test]
    fn canonical_order_is_order_insensitive() {
        assert_eq!(ManaCost::parse("{R}{W}"), ManaCost::parse("{W}{R}"));
        assert_eq!(ManaCost::parse("{W}{W}{U}{U}{B}{B}{R}{R}{G}{G}").cmc(), 10);
    }

    #[test]
    fn rejects_invalid() {
        assert!(ManaCost::try_parse("{Q}").is_err());
        assert!(ManaCost::try_parse("2W").is_err());
        assert!(ManaCost::try_parse("{W").is_err());
        assert!(ManaCost::try_parse("{W/X}").is_err());
        assert!(ManaCost::try_parse("{}").is_err());
    }

    #[test]
    fn empty_cost() {
        assert!(ManaCost::parse("").is_empty());
        assert_eq!(ManaCost::ZERO.cmc(), 0);
    }

    #[test]
    fn pool_basics() {
        let mut pool = ManaPool::new();
        pool.add(ManaColor::Red, 3);
        pool.add_restricted(RestrictedMana {
            color: ManaColor::Colorless,
            amount: 2,
            flags: ManaFlags::NO_EMPTY,
            restriction: RestrictionId(0),
        });
        assert_eq!(pool.total(), 5);
        assert!(pool.spend(ManaColor::Red, 2));
        assert!(!pool.spend(ManaColor::Red, 2));
        pool.empty_at_step_end();
        assert_eq!(pool.total(), 2); // no-empty survived
        assert!(pool.colors_available().is_empty());
    }
}
