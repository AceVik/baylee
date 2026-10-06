//! Mana symbols, costs, and pools. Canonical notation: `docs/mana-notation.md`.
//!
//! Costs parse at compile time via [`crate::mana!`] or at runtime via
//! [`ManaCost::try_parse`]. Payment solving lives in the engine; this module
//! holds only the data model.

use crate::color::{Color, ColorPair, ColorSet};
use core::str::FromStr;
use serde::{Deserialize, Serialize};
mod transfer;

/// The mana decision a resolving operation has opened a payment window for.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Serialize, Deserialize)]
pub enum ManaPayment {
    /// A fixed total cost, including its colored requirements.
    Fixed(ManaCost),
    /// Any amount may be generated, then chosen and spent. The amount of
    /// damage is a useful-payment hint, never an upper bound on payment.
    AnyAmount {
        /// Damage the payment can prevent from this event.
        preventable_damage: u32,
    },
}

/// Permissions to spend actual mana as another type, without changing that
/// mana or the cost (CR 609.4b). Shared by payment and client planning.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Serialize, Deserialize)]
pub struct ManaSpending([u8; 6]);

impl Default for ManaSpending {
    fn default() -> Self {
        Self::EXACT
    }
}

impl ManaSpending {
    /// Every type pays itself only.
    pub const EXACT: Self = Self([1, 2, 4, 8, 16, 32]);

    /// Any mana pays any colored requirement; only colorless pays `{C}`.
    pub const ANY_COLOR: Self = Self([31, 31, 31, 31, 31, 63]);

    /// Adds a directed permission. Compatible "as though" effects combine
    /// (CR 609.4a), independent of their registration order.
    pub fn allow(&mut self, from: ManaColor, to: ManaColor) {
        self.0[from.index()] |= 1 << to.index();
        for via in 0..6 {
            for actual in 0..6 {
                if self.0[actual] & (1 << via) != 0 {
                    self.0[actual] |= self.0[via];
                }
            }
        }
    }

    /// Whether one unit of `actual` can pay a requirement for `required`.
    #[must_use]
    pub const fn permits(self, actual: ManaColor, required: ManaColor) -> bool {
        self.0[actual.index()] & (1 << required.index()) != 0
    }
}

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
    /// The one-mana symbol of `c`: `{G}` for green.
    #[must_use]
    pub const fn of_color(c: Color) -> Self {
        match c {
            Color::White => Self::White,
            Color::Blue => Self::Blue,
            Color::Black => Self::Black,
            Color::Red => Self::Red,
            Color::Green => Self::Green,
        }
    }

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

/// How many symbols a cost counts apart from generic mana: every
/// [`ManaSymbol`] but [`ManaSymbol::Generic`], which a cost holds as one
/// amount ([`ManaCost`]).
const KINDS: usize = 42;
// One bit per kind in `ManaCost::present`.
const _: () = assert!(KINDS <= 64);

/// Where each kind starts in [`KIND_SYMBOLS`], which is the canonical order a
/// cost is written in (`docs/mana-notation.md`): the variables, then generic
/// mana (not a kind, written at [`GENERIC_AT`]), the silver-bordered two,
/// WUBRG, hybrid, `{2/C}`, Phyrexian, hybrid Phyrexian, snow and colourless.
/// A colour's symbol sits at its `Color` discriminant past its start, a
/// hybrid pair's at its place in [`PAIRS`].
const VARIABLES: usize = 0;
const HALF: usize = 3;
const INFINITE: usize = 4;
const COLORED: usize = 5;
const HYBRID: usize = 10;
const TWO_OR_COLOR: usize = 20;
const PHYREXIAN: usize = 25;
const HYBRID_PHYREXIAN: usize = 30;
const SNOW: usize = 40;
const COLORLESS: usize = 41;
/// Generic mana is written after the variables and before everything else.
const GENERIC_AT: usize = HALF;
/// The variables' bits in [`ManaCost`]'s `present`: every kind before
/// generic mana.
const VARIABLE_BITS: u64 = (1 << GENERIC_AT) - 1;

/// The ten hybrid pairs (CR 107.4), in the order a cost writes them: by
/// their two colour bits, so `{W/U}` (3) comes before `{W/B}` (5).
const PAIRS: [ColorPair; 10] = [
    ColorPair::new(Color::White, Color::Blue),
    ColorPair::new(Color::White, Color::Black),
    ColorPair::new(Color::Blue, Color::Black),
    ColorPair::new(Color::Red, Color::White),
    ColorPair::new(Color::Blue, Color::Red),
    ColorPair::new(Color::Black, Color::Red),
    ColorPair::new(Color::Green, Color::White),
    ColorPair::new(Color::Green, Color::Blue),
    ColorPair::new(Color::Black, Color::Green),
    ColorPair::new(Color::Red, Color::Green),
];

/// `pair`'s place in [`PAIRS`]. [`ColorPair::new`] makes every pair one of
/// the ten, in one order, so the last place is the one left when the other
/// nine do not match.
const fn pair_rank(pair: ColorPair) -> usize {
    let mut i = 0;
    while i + 1 < PAIRS.len() {
        if PAIRS[i].bits() == pair.bits() {
            return i;
        }
        i += 1;
    }
    i
}

/// Every symbol a cost counts, at the index [`slot`] gives it.
const KIND_SYMBOLS: [ManaSymbol; KINDS] = kind_symbols();

const fn kind_symbols() -> [ManaSymbol; KINDS] {
    let mut out = [ManaSymbol::Colorless; KINDS];
    out[VARIABLES] = ManaSymbol::Variable(Variable::X);
    out[VARIABLES + 1] = ManaSymbol::Variable(Variable::Y);
    out[VARIABLES + 2] = ManaSymbol::Variable(Variable::Z);
    out[HALF] = ManaSymbol::HalfGeneric;
    out[INFINITE] = ManaSymbol::Infinite;
    let mut c = 0;
    while c < Color::ALL.len() {
        let color = Color::ALL[c];
        out[COLORED + color as usize] = ManaSymbol::of_color(color);
        out[TWO_OR_COLOR + color as usize] = ManaSymbol::TwoOrColor(color);
        out[PHYREXIAN + color as usize] = ManaSymbol::Phyrexian(color);
        c += 1;
    }
    let mut p = 0;
    while p < PAIRS.len() {
        out[HYBRID + p] = ManaSymbol::Hybrid(PAIRS[p]);
        out[HYBRID_PHYREXIAN + p] = ManaSymbol::HybridPhyrexian(PAIRS[p]);
        p += 1;
    }
    out[SNOW] = ManaSymbol::Snow;
    out[COLORLESS] = ManaSymbol::Colorless;
    out
}

/// Where a cost keeps a symbol.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Slot {
    /// Generic mana, of this amount, joins the cost's one generic amount.
    Generic(u32),
    /// Any other symbol is counted at this index of [`KIND_SYMBOLS`].
    Kind(usize),
}

const fn slot(symbol: ManaSymbol) -> Slot {
    Slot::Kind(match symbol {
        ManaSymbol::Generic(n) => return Slot::Generic(n),
        ManaSymbol::Variable(v) => VARIABLES + v as usize,
        ManaSymbol::HalfGeneric => HALF,
        ManaSymbol::Infinite => INFINITE,
        ManaSymbol::White => COLORED + Color::White as usize,
        ManaSymbol::Blue => COLORED + Color::Blue as usize,
        ManaSymbol::Black => COLORED + Color::Black as usize,
        ManaSymbol::Red => COLORED + Color::Red as usize,
        ManaSymbol::Green => COLORED + Color::Green as usize,
        ManaSymbol::Hybrid(p) => HYBRID + pair_rank(p),
        ManaSymbol::TwoOrColor(c) => TWO_OR_COLOR + c as usize,
        ManaSymbol::Phyrexian(c) => PHYREXIAN + c as usize,
        ManaSymbol::HybridPhyrexian(p) => HYBRID_PHYREXIAN + pair_rank(p),
        ManaSymbol::Snow => SNOW,
        ManaSymbol::Colorless => COLORLESS,
    })
}

/// A mana cost: the symbols it holds, written in canonical order.
///
/// Counted, not listed. Every symbol but generic mana has a count, and
/// generic mana is one amount, as a printed cost writes it (`{2}`, never
/// `{1}{1}`: numerical symbols are all generic mana, CR 107.4b). So every
/// cost the rules can build fits, however it was built: a replicate cost
/// paid fifty times (CR 702.56a) is fifty more `{U}` and not fifty more
/// slots, and there is no length for cost arithmetic to run out of. A cost
/// held sixteen symbols in a list until a house seat with fifteen mana
/// priced Lose Focus replicated fifteen times, and the list's assert took
/// the hosted game down.
///
/// A count stops at `u16::MAX`, 65 535 of one symbol, and generic mana and
/// the mana value at `u32::MAX`. No game comes near either: the engine asks
/// for a replicate count or an X of at most fifty, and no card prints more
/// than a handful of one symbol.
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct ManaCost {
    /// How many of each symbol but generic mana, by [`slot`].
    counts: [u16; KINDS],
    /// Which counts are not zero, bit `k` for count `k`: what a reader
    /// walks, so a cost of three kinds is three steps and not forty-two.
    present: u64,
    /// The generic mana, one symbol however many were added; `None` for a
    /// cost that writes none, which `{0}` does.
    generic: Option<u32>,
    /// How many symbols the cost writes, its generic mana as one.
    len: u32,
    /// Mana value (CR 202.3).
    cmc: u32,
}

impl Default for ManaCost {
    fn default() -> Self {
        Self::ZERO
    }
}

impl ManaCost {
    /// The empty cost (lands, tokens, suspend-only cards).
    pub const ZERO: Self = Self {
        counts: [0; KINDS],
        present: 0,
        generic: None,
        len: 0,
        cmc: 0,
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
    /// Total: whatever the text, a cost or an error, never a panic, so a
    /// runtime reader may hand it anything.
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
            cost.add(sym, 1);
            i = j + 1;
        }
        Ok(cost)
    }

    /// `n` more of `symbol`: generic mana joins the one generic amount, any
    /// other symbol its count. Saturating at the bounds the type names, and
    /// the length and mana value grow by what was actually added.
    #[allow(clippy::cast_possible_truncation)] // clamped to `u16::MAX` first
    const fn add(&mut self, symbol: ManaSymbol, n: u32) {
        if n == 0 {
            return;
        }
        match slot(symbol) {
            Slot::Generic(amount) => {
                // A cost with no generic symbol grows one.
                let before = if let Some(before) = self.generic {
                    before
                } else {
                    self.len = self.len.saturating_add(1);
                    0
                };
                let after = before.saturating_add(amount.saturating_mul(n));
                self.generic = Some(after);
                self.cmc = self.cmc.saturating_add(after - before);
            }
            Slot::Kind(k) => {
                let before = self.counts[k] as u32;
                let after = before.saturating_add(n);
                let after = if after > u16::MAX as u32 {
                    u16::MAX as u32
                } else {
                    after
                };
                self.counts[k] = after as u16;
                self.present |= 1 << k;
                let added = after - before;
                self.len = self.len.saturating_add(added);
                self.cmc = self
                    .cmc
                    .saturating_add(symbol.cmc_contribution().saturating_mul(added));
            }
        }
    }

    /// Number of symbols, the generic mana written as one.
    #[must_use]
    pub const fn len(&self) -> u32 {
        self.len
    }

    /// Whether the cost is empty.
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.len == 0
    }

    /// Number of different symbols the cost holds: how many items
    /// [`Self::runs`] yields, at most one per kind plus the generic mana.
    #[must_use]
    #[allow(clippy::cast_possible_truncation)] // at most `KINDS + 1`, below 64
    pub const fn kinds(&self) -> u8 {
        self.present.count_ones() as u8 + self.generic.is_some() as u8
    }

    /// Converted/mana value (CR 202.3).
    #[must_use]
    pub const fn cmc(&self) -> u32 {
        self.cmc
    }

    /// Total generic (any-mana) requirement.
    #[must_use]
    pub const fn generic_total(&self) -> u32 {
        match self.generic {
            Some(n) => n,
            None => 0,
        }
    }

    /// Whether the cost contains `{X}`/`{Y}`/`{Z}`.
    #[must_use]
    pub const fn has_variable(&self) -> bool {
        self.present & VARIABLE_BITS != 0
    }

    /// All colors referenced by the cost (hybrid counts both).
    #[must_use]
    pub const fn colors(&self) -> ColorSet {
        let mut set = ColorSet::EMPTY;
        let mut bits = self.present;
        while bits != 0 {
            set = set.union(KIND_SYMBOLS[bits.trailing_zeros() as usize].colors());
            bits &= bits - 1;
        }
        set
    }

    /// Each symbol the cost holds with how many times it holds it, in
    /// canonical order; the generic mana once, as its amount. What
    /// [`Self::symbols`] repeats, and the cheaper walk for a reader that
    /// counts rather than lists (a hash).
    pub fn runs(&self) -> impl Iterator<Item = (ManaSymbol, u16)> + '_ {
        let kinds = move |mut bits: u64| {
            core::iter::from_fn(move || {
                if bits == 0 {
                    return None;
                }
                let k = bits.trailing_zeros() as usize;
                bits &= bits - 1;
                Some((KIND_SYMBOLS[k], self.counts[k]))
            })
        };
        kinds(self.present & VARIABLE_BITS)
            .chain(self.generic.map(|n| (ManaSymbol::Generic(n), 1)))
            .chain(kinds(self.present & !VARIABLE_BITS))
    }

    /// [`Self::runs`] as plain loops, for the one reader on a hot path: the
    /// snapshot hash walks every object's cost on every input a record
    /// keeps, and the chained iterator cost it several times the bytes.
    /// Same runs, same order (`for_each_run_is_runs`).
    #[inline]
    pub fn for_each_run(&self, mut f: impl FnMut(ManaSymbol, u16)) {
        let mut bits = self.present & VARIABLE_BITS;
        while bits != 0 {
            let k = bits.trailing_zeros() as usize;
            bits &= bits - 1;
            f(KIND_SYMBOLS[k], self.counts[k]);
        }
        if let Some(n) = self.generic {
            f(ManaSymbol::Generic(n), 1);
        }
        let mut bits = self.present & !VARIABLE_BITS;
        while bits != 0 {
            let k = bits.trailing_zeros() as usize;
            bits &= bits - 1;
            f(KIND_SYMBOLS[k], self.counts[k]);
        }
    }

    /// Iterates the symbols in canonical order.
    pub fn symbols(&self) -> impl Iterator<Item = ManaSymbol> + '_ {
        self.runs()
            .flat_map(|(symbol, n)| core::iter::repeat_n(symbol, usize::from(n)))
    }

    /// The cost with `{X}`/`{Y}`/`{Z}` replaced by `Generic(x)` (CR 601.2b),
    /// each of them: `{X}{X}{B}` for 2 is `{4}{B}`.
    #[must_use]
    pub fn with_x(&self, x: u32) -> Self {
        let mut out = Self::ZERO;
        for (symbol, n) in self.runs() {
            let symbol = match symbol {
                ManaSymbol::Variable(_) => ManaSymbol::Generic(x),
                other => other,
            };
            out.add(symbol, u32::from(n));
        }
        out
    }

    /// The cost with up to `n` generic mana removed (delve/convoke). A
    /// generic part left at nothing is dropped, `{0}` included: `{0}`
    /// reduced by nothing comes out blank.
    #[must_use]
    pub fn with_less_generic(&self, n: u32) -> Self {
        let mut out = Self::ZERO;
        for (symbol, count) in self.runs() {
            match symbol {
                ManaSymbol::Generic(amount) => {
                    let left = amount.saturating_sub(n);
                    if left > 0 {
                        out.add(ManaSymbol::Generic(left), 1);
                    }
                }
                other => out.add(other, u32::from(count)),
            }
        }
        out
    }

    /// How many Phyrexian symbols, plain or hybrid, the cost holds — each
    /// one payable with its mana or with 2 life (CR 107.4f).
    #[must_use]
    pub fn phyrexian_count(&self) -> u32 {
        let n = self
            .symbols()
            .filter(|s| matches!(s, ManaSymbol::Phyrexian(_) | ManaSymbol::HybridPhyrexian(_)))
            .count();
        u32::try_from(n).unwrap_or(u32::MAX)
    }

    /// The cost with its Phyrexian symbols settled, as CR 601.2b has the
    /// player announce and CR 118.13a times it: the `n`-th Phyrexian symbol
    /// (counted from 0 in the cost's own order) is paid with 2 life when bit
    /// `n` of `life` is set, and so leaves the mana; otherwise it becomes
    /// the mana it is paid with — its colour, or its hybrid pair. The life
    /// is the caller's to charge.
    #[must_use]
    pub fn with_phyrexian_settled(&self, life: u32) -> Self {
        let mut out = Self::ZERO;
        let mut n = 0u32;
        for s in self.symbols() {
            let settled = match s {
                ManaSymbol::Phyrexian(c) => {
                    let by_life = n < 32 && life & (1 << n) != 0;
                    n += 1;
                    (!by_life).then(|| ManaSymbol::of_color(c))
                }
                ManaSymbol::HybridPhyrexian(p) => {
                    let by_life = n < 32 && life & (1 << n) != 0;
                    n += 1;
                    (!by_life).then_some(ManaSymbol::Hybrid(p))
                }
                other => Some(other),
            };
            if let Some(s) = settled {
                out.add(s, 1);
            }
        }
        out
    }

    /// The cost with `n` more generic mana (a cost increase, CR 601.2f).
    ///
    /// The mirror of [`Self::with_less_generic`], and the commander tax is
    /// what needed it: a cost that prints no generic symbol at all has to
    /// grow one rather than stay unchanged.
    #[must_use]
    pub fn with_more_generic(&self, n: u32) -> Self {
        let mut out = *self;
        if n > 0 {
            out.add(ManaSymbol::Generic(n), 1);
        }
        out
    }

    /// Two costs combined (additional costs like kicker stack onto the
    /// base cost, CR 601.2f). The generic mana of both is one symbol.
    #[must_use]
    pub fn combine(&self, other: &ManaCost) -> Self {
        self.combine_n(other, 1)
    }

    /// The cost with `other` added `n` times: a replicate cost paid `n`
    /// times, which CR 702.56a allows "any number of times", on top of the
    /// rest of the total cost (CR 601.2f). Any `n` fits ([`ManaCost`]).
    #[must_use]
    pub fn combine_n(&self, other: &ManaCost, n: u32) -> Self {
        let mut out = *self;
        for (symbol, count) in other.runs() {
            out.add(symbol, u32::from(count).saturating_mul(n));
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
        // Checked: the digits come from whatever text a runtime reader was
        // handed, and `{99999999999}` is an error, not a wrap or a panic.
        n = match n.checked_mul(10) {
            Some(tens) => match tens.checked_add((b - b'0') as u32) {
                Some(next) => next,
                None => return Err("number too large in symbol"),
            },
            None => return Err("number too large in symbol"),
        };
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
                // Two different colours: `ColorPair::new` asserts it.
                if let (Some(a), Some(b)) = (a, b)
                    && a as u8 != b as u8
                {
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
            // Two different colours: `ColorPair::new` asserts it.
            if let (Some(a), Some(b)) = (a, b)
                && a as u8 != b as u8
            {
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

/// A cost reads as its notation, which says what the counts mean.
impl core::fmt::Debug for ManaCost {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "ManaCost(\"{self}\")")
    }
}

/// On the wire a cost is its notation (`"{2}{U}{U}"`, `""` for none): as
/// long as the cost is, and independent of how [`ManaCost`] counts it.
impl Serialize for ManaCost {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.collect_str(self)
    }
}

/// Read back through [`ManaCost::try_parse`], which refuses what is not a
/// cost rather than panicking on it.
impl<'de> Deserialize<'de> for ManaCost {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let text = String::deserialize(deserializer)?;
        Self::try_parse(&text).map_err(serde::de::Error::custom)
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
    /// `NO_EMPTY` expires during cleanup of this turn.
    pub const UNTIL_END_OF_TURN: Self = Self(4);

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
    plain: [u32; 6],
    /// Subset of plain mana produced by snow sources (#158).
    #[serde(default)]
    snow: [u32; 6],
    restricted: Vec<RestrictedMana>,
    /// Subset of plain mana that carries a rider and restricts nothing
    /// (Path of Ancestry, #232), in the order it was made, each entry under
    /// the id its rider is filed by. Ordinary mana (CR 106.6: the rider
    /// "doesn't affect the mana's type"), so every payment may spend it; an
    /// ordinary spend takes the other units first.
    #[serde(default)]
    ridden: Vec<RestrictedMana>,
}

impl ManaPool {
    /// An empty pool.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Adds plain mana.
    pub fn add(&mut self, color: ManaColor, amount: u32) {
        self.plain[color.index()] = self.plain[color.index()].saturating_add(amount);
    }

    /// Add the complete amount, or leave the pool unchanged when the amount
    /// exceeds the representable counter. No mana is silently discarded.
    pub fn try_add(&mut self, color: ManaColor, amount: u32) -> bool {
        let Some(total) = self.plain[color.index()].checked_add(amount) else {
            return false;
        };
        self.plain[color.index()] = total;
        true
    }

    /// Adds unrestricted mana produced by a snow source.
    pub fn add_snow(&mut self, color: ManaColor, amount: u32) {
        let added = amount.min(u32::MAX - self.available(color));
        self.add(color, added);
        self.snow[color.index()] += added;
    }

    /// Add the complete snow-produced amount atomically, preserving both
    /// the total and its snow subset when the counter cannot hold it.
    pub fn try_add_snow(&mut self, color: ManaColor, amount: u32) -> bool {
        let i = color.index();
        let (Some(total), Some(snow)) = (
            self.plain[i].checked_add(amount),
            self.snow[i].checked_add(amount),
        ) else {
            return false;
        };
        self.plain[i] = total;
        self.snow[i] = snow;
        true
    }

    /// Snow-produced mana of this color, already included in `available`.
    #[must_use]
    pub fn snow_available(&self, color: ManaColor) -> u32 {
        self.snow[color.index()]
    }

    /// Spends one mana specifically from a snow source.
    pub fn spend_snow(&mut self, color: ManaColor) -> bool {
        self.spend_snow_units(color, 1)
    }

    /// Spends an amount specifically from snow sources, atomically and
    /// without iterating once per mana unit.
    pub fn spend_snow_units(&mut self, color: ManaColor, amount: u32) -> bool {
        if self.snow[color.index()] < amount {
            return false;
        }
        self.snow[color.index()] -= amount;
        self.plain[color.index()] -= amount;
        self.settle_ridden(color);
        true
    }

    /// Adds restricted mana (riders preserved).
    pub fn add_restricted(&mut self, mana: RestrictedMana) {
        self.restricted.push(mana);
    }

    /// Adds mana that carries a rider and restricts nothing: into the plain
    /// counters (and the snow ones, for [`ManaFlags::SNOW`]), remembering
    /// which units carry the rider.
    pub fn add_ridden(&mut self, mana: RestrictedMana) {
        let before = self.available(mana.color);
        if mana.flags.contains(ManaFlags::SNOW) {
            self.add_snow(mana.color, u32::from(mana.amount));
        } else {
            self.add(mana.color, u32::from(mana.amount));
        }
        let amount = self.available(mana.color) - before;
        if amount > 0 {
            self.ridden.push(RestrictedMana {
                // The increase is bounded by the supplied u16 entry.
                amount: amount as u16,
                ..mana
            });
        }
    }

    /// The units of the plain counters that carry a rider (engine payment
    /// solver).
    #[must_use]
    pub fn ridden(&self) -> &[RestrictedMana] {
        &self.ridden
    }

    /// Spends up to `n` units of the ridden entry with the given id, and
    /// returns the part spent, rider and all. `None` is an id nobody holds
    /// mana for, or `n == 0`.
    pub fn take_ridden_units(&mut self, id: u32, n: u16) -> Option<RestrictedMana> {
        if n == 0 {
            return None;
        }
        let pos = self.ridden.iter().position(|m| m.restriction.0 == id)?;
        let entry = &mut self.ridden[pos];
        let taken = n.min(entry.amount);
        entry.amount -= taken;
        let part = RestrictedMana {
            amount: taken,
            ..*entry
        };
        if entry.amount == 0 {
            self.ridden.remove(pos);
        }
        let i = part.color.index();
        self.plain[i] -= u32::from(taken);
        if part.flags.contains(ManaFlags::SNOW) {
            self.snow[i] -= u32::from(taken);
        } else {
            // A unit of snow mana counted beside it may have been the one
            // that went.
            self.snow[i] = self.snow[i].min(self.plain[i]);
        }
        Some(part)
    }

    /// Keeps the ridden units of `color` within what is left of it, after a
    /// spend that did not say which units it took: the other units went
    /// first, so the rider units still there are the earliest made.
    fn settle_ridden(&mut self, color: ManaColor) {
        let i = color.index();
        let mut room = self.plain[i];
        let mut snow_room = self.snow[i];
        for entry in self.ridden.iter_mut().filter(|m| m.color == color) {
            let snow = entry.flags.contains(ManaFlags::SNOW);
            let fits = if snow { room.min(snow_room) } else { room };
            // Bounded by the existing u16 amount, even with a wider pool.
            entry.amount = u32::from(entry.amount).min(fits) as u16;
            room -= u32::from(entry.amount);
            if snow {
                snow_room -= u32::from(entry.amount);
            }
        }
        self.ridden.retain(|m| m.amount > 0);
    }

    /// Available amount of a plain color.
    #[must_use]
    pub fn available(&self, color: ManaColor) -> u32 {
        self.plain[color.index()]
    }

    /// Tries to spend plain mana; returns success.
    pub fn spend(&mut self, color: ManaColor, amount: u32) -> bool {
        let slot = &mut self.plain[color.index()];
        if *slot >= amount {
            *slot -= amount;
            // Preserve snow mana when ordinary mana can cover the payment.
            self.snow[color.index()] = self.snow[color.index()].min(*slot);
            self.settle_ridden(color);
            true
        } else {
            false
        }
    }

    /// Total mana currently in the pool.
    #[must_use]
    pub fn total(&self) -> u64 {
        let plain: u64 = self.plain.iter().map(|&n| u64::from(n)).sum();
        plain
            + self
                .restricted
                .iter()
                .map(|r| u64::from(r.amount))
                .sum::<u64>()
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
        self.snow = [0; 6];
        self.ridden
            .retain(|r| r.flags.contains(ManaFlags::NO_EMPTY));
        for r in &self.ridden {
            self.plain[r.color.index()] += u32::from(r.amount);
            if r.flags.contains(ManaFlags::SNOW) {
                self.snow[r.color.index()] += u32::from(r.amount);
            }
        }
        self.restricted
            .retain(|r| r.flags.contains(ManaFlags::NO_EMPTY));
    }

    /// End the temporary retention effect; the next step end empties the mana.
    pub fn expire_turn_retention(&mut self) {
        for r in self.ridden.iter_mut().chain(self.restricted.iter_mut()) {
            if r.flags.contains(ManaFlags::UNTIL_END_OF_TURN) {
                r.flags = ManaFlags(
                    r.flags.bits()
                        & !(ManaFlags::NO_EMPTY.bits() | ManaFlags::UNTIL_END_OF_TURN.bits()),
                );
            }
        }
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

    /// Takes up to `n` units off the restricted entry with the given id, and
    /// returns the part taken.
    ///
    /// What is left keeps its restriction, its flags and its place in the
    /// pool, so three restricted {C} that pay a {1} leave two restricted {C}
    /// behind. Taking the whole entry here lost the other two: the surplus
    /// was neither spent nor in the pool afterwards. The entry goes when its
    /// last unit does. `None` is an id nobody holds mana for, or `n == 0`.
    pub fn take_restricted_units(&mut self, id: u32, n: u16) -> Option<RestrictedMana> {
        if n == 0 {
            return None;
        }
        let pos = self.restricted.iter().position(|m| m.restriction.0 == id)?;
        let entry = &mut self.restricted[pos];
        let taken = n.min(entry.amount);
        entry.amount -= taken;
        let part = RestrictedMana {
            amount: taken,
            ..*entry
        };
        if entry.amount == 0 {
            self.restricted.remove(pos);
        }
        Some(part)
    }
}

impl ManaCost {
    /// A cost of one symbol.
    #[must_use]
    pub fn from_symbol(symbol: ManaSymbol) -> Self {
        let mut out = Self::ZERO;
        out.add(symbol, 1);
        out
    }

    /// A generic-only cost `{n}`.
    #[must_use]
    pub fn from_symbol_generic(n: u32) -> Self {
        let mut out = Self::ZERO;
        if n > 0 {
            out.add(ManaSymbol::Generic(n), 1);
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn spending_permissions_are_directed_idempotent_and_combine_in_any_order() {
        let a = (ManaColor::White, ManaColor::Red);
        let b = (ManaColor::Red, ManaColor::Green);
        for edges in [[a, b], [b, a]] {
            let mut spending = ManaSpending::default();
            for (from, to) in edges {
                spending.allow(from, to);
                spending.allow(from, to);
            }
            assert!(spending.permits(ManaColor::White, ManaColor::Green));
            assert!(spending.permits(ManaColor::White, ManaColor::White));
            assert!(!spending.permits(ManaColor::Red, ManaColor::White));
            assert!(!spending.permits(ManaColor::White, ManaColor::Colorless));
            assert!(!spending.permits(ManaColor::Colorless, ManaColor::White));
        }
        for actual in ManaColor::ALL {
            for required in ManaColor::ALL {
                assert_eq!(
                    ManaSpending::EXACT.permits(actual, required),
                    actual == required
                );
                assert_eq!(
                    ManaSpending::ANY_COLOR.permits(actual, required),
                    required != ManaColor::Colorless || actual == required
                );
            }
        }
    }

    /// The loop the snapshot hash walks and the iterator everything else
    /// walks give the same runs in the same order, over costs holding every
    /// kind of symbol, X before generic before the rest, generic as `{0}`
    /// and not at all.
    #[test]
    fn for_each_run_is_runs() {
        for src in [
            "",
            "{0}",
            "{3}",
            "{X}{X}{R}",
            "{X}{Y}{2}{W}{U}{B}{R}{G}{C}",
            "{2/W}{W/U}{G/P}{B/G/P}{S}{S}{1}",
            "{HALFGENERIC}{INFINITY}{Z}",
            "{W}{W}{W}{W}{W}{W}{W}{W}{W}{W}{U}",
        ] {
            let cost = src.parse::<ManaCost>().unwrap_or(ManaCost::ZERO);
            let mut walked = Vec::new();
            cost.for_each_run(|symbol, n| walked.push((symbol, n)));
            assert_eq!(walked, cost.runs().collect::<Vec<_>>(), "{src}");
        }
    }

    fn ridden(color: ManaColor, amount: u16, id: u32) -> RestrictedMana {
        RestrictedMana {
            color,
            amount,
            flags: ManaFlags::default(),
            restriction: RestrictionId(id),
        }
    }

    /// Each Phyrexian symbol is settled on its own bit: paid with life it
    /// leaves the mana, paid with mana it becomes its colour (or its hybrid
    /// pair), and everything else in the cost stays as printed.
    #[test]
    fn a_phyrexian_symbol_is_settled_as_its_mana_or_as_nothing() {
        let pod = ManaCost::parse("{1}{G/P}");
        assert_eq!(pod.phyrexian_count(), 1);
        assert_eq!(pod.with_phyrexian_settled(0), ManaCost::parse("{1}{G}"));
        assert_eq!(pod.with_phyrexian_settled(1), ManaCost::parse("{1}"));
        assert_eq!(ManaCost::parse("{2}{G}").phyrexian_count(), 0);

        let two = ManaCost::parse("{W/P}{U/P}");
        assert_eq!(two.phyrexian_count(), 2);
        let first = two.symbols().next().expect("two symbols");
        let settled_second = two.with_phyrexian_settled(0b01);
        assert_eq!(settled_second.len(), 1, "one paid with life");
        assert_ne!(
            settled_second.symbols().next(),
            Some(match first {
                ManaSymbol::Phyrexian(c) => ManaSymbol::of_color(c),
                other => other,
            }),
            "bit 0 is the first symbol in the cost's order"
        );
        assert!(two.with_phyrexian_settled(0b11).is_empty());

        let hybrid = ManaCost::parse("{G/U/P}");
        assert_eq!(hybrid.phyrexian_count(), 1);
        assert_eq!(
            hybrid
                .with_phyrexian_settled(0)
                .symbols()
                .collect::<Vec<_>>(),
            vec![ManaSymbol::Hybrid(ColorPair::new(
                Color::Green,
                Color::Blue
            ))]
        );
    }

    /// Mana with a rider alone is ordinary mana (CR 106.6, #232): counted in
    /// the plain counters, kept back by an ordinary spend while other units
    /// can pay, spent by one that cannot, and gone at step end.
    #[test]
    fn mana_with_a_rider_is_plain_mana_spent_last() {
        let mut pool = ManaPool::new();
        pool.add_ridden(ridden(ManaColor::Blue, 1, 7));
        pool.add(ManaColor::Blue, 1);
        assert_eq!(pool.available(ManaColor::Blue), 2);
        assert!(pool.restricted().is_empty());

        assert!(pool.spend(ManaColor::Blue, 1));
        assert_eq!(pool.ridden(), [ridden(ManaColor::Blue, 1, 7)], "kept back");
        assert!(pool.spend(ManaColor::Blue, 1));
        assert!(
            pool.ridden().is_empty(),
            "spent when nothing else could pay"
        );

        pool.add_ridden(ridden(ManaColor::Colorless, 2, 8));
        pool.empty_at_step_end();
        assert!(pool.ridden().is_empty());
        assert!(pool.is_empty());
    }

    /// A payment that names the rider's units takes them out of the plain
    /// counters with it, and leaves the rest of the entry where it was.
    #[test]
    fn turn_retained_mana_preserves_only_unspent_units_and_expires_at_cleanup() {
        let mut pool = ManaPool::new();
        pool.add_ridden(RestrictedMana {
            flags: ManaFlags::NO_EMPTY
                .union(ManaFlags::UNTIL_END_OF_TURN)
                .union(ManaFlags::SNOW),
            ..ridden(ManaColor::Blue, 2, 8)
        });
        pool.add(ManaColor::Blue, 1);
        pool.add(ManaColor::Red, 1);
        pool.empty_at_step_end();
        assert_eq!(pool.available(ManaColor::Blue), 2);
        assert_eq!(pool.snow_available(ManaColor::Blue), 2);
        assert_eq!(pool.available(ManaColor::Red), 0);
        assert!(pool.spend_snow(ManaColor::Blue));
        pool.empty_at_step_end();
        assert_eq!(
            pool.available(ManaColor::Blue),
            1,
            "spent mana does not return"
        );
        assert_eq!(pool.snow_available(ManaColor::Blue), 1);
        pool.expire_turn_retention();
        pool.empty_at_step_end();
        assert!(pool.is_empty());
        assert!(pool.ridden().is_empty());
    }

    #[test]
    fn taking_rider_units_spends_them() {
        let mut pool = ManaPool::new();
        pool.add_ridden(ridden(ManaColor::Green, 2, 3));
        pool.add(ManaColor::Green, 1);
        let taken = pool.take_ridden_units(3, 1).expect("the entry is there");
        assert_eq!(taken, ridden(ManaColor::Green, 1, 3));
        assert_eq!(pool.available(ManaColor::Green), 2);
        assert_eq!(pool.ridden(), [ridden(ManaColor::Green, 1, 3)]);
        assert_eq!(pool.take_ridden_units(9, 1), None, "nobody holds id 9");
    }

    /// Snow mana with a rider is in both subsets, and an ordinary spend
    /// keeps both as long as a unit that is neither can pay.
    #[test]
    fn snow_mana_with_a_rider_is_kept_in_both_subsets() {
        let mut pool = ManaPool::new();
        pool.add_ridden(RestrictedMana {
            flags: ManaFlags::SNOW,
            ..ridden(ManaColor::Red, 1, 4)
        });
        pool.add(ManaColor::Red, 1);
        assert_eq!(pool.snow_available(ManaColor::Red), 1);
        assert!(pool.spend(ManaColor::Red, 1));
        assert_eq!(pool.snow_available(ManaColor::Red), 1);
        assert_eq!(pool.ridden().len(), 1);
        assert!(pool.spend_snow(ManaColor::Red));
        assert!(pool.ridden().is_empty(), "the snow unit was the ridden one");
    }

    #[test]
    fn snow_mana_is_a_subset_preserved_until_needed_and_cleared_at_step_end() {
        let mut pool = ManaPool::new();
        pool.add(ManaColor::Green, 1);
        pool.add_snow(ManaColor::Green, 2);
        assert!(pool.spend(ManaColor::Green, 1));
        assert_eq!(pool.snow_available(ManaColor::Green), 2);
        assert!(pool.spend_snow(ManaColor::Green));
        assert_eq!(pool.available(ManaColor::Green), 1);
        assert!(pool.spend(ManaColor::Green, 1));
        assert!(!pool.spend_snow(ManaColor::Green));
        pool.add_snow(ManaColor::Blue, 1);
        pool.empty_at_step_end();
        assert!(pool.is_empty());
        assert!(!pool.spend_snow(ManaColor::Blue));
    }

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

    /// Every cost that can be written must read back as itself. The two
    /// directions are used by different crates — the deck builder writes them,
    /// the card files read them — so a symbol either side got wrong would show
    /// up as a card with a subtly different cost rather than as an error.
    #[test]
    fn every_symbol_survives_a_round_trip() {
        let costs = [
            "",
            "{0}",
            "{5}",
            "{W}",
            "{U}",
            "{B}",
            "{R}",
            "{G}",
            "{C}",
            "{S}",
            "{X}",
            "{Y}",
            "{Z}",
            "{2}{W}{U}",
            "{W/U}",
            "{B/R}{B/R}",
            "{2/G}",
            "{W/P}",
            "{W/U/P}",
            "{½}",
            "{∞}",
            "{X}{X}{R}",
            "{10}{G}{G}",
        ];
        for src in costs {
            let cost = ManaCost::try_parse(src).expect(src);
            let written = cost.to_string();
            assert_eq!(
                ManaCost::try_parse(&written).expect(&written),
                cost,
                "{src} wrote as {written}"
            );
        }
    }

    /// Canonical order is what a cost is stored in, so writing normalises it:
    /// two spellings of the same cost produce one string, and a client that
    /// keys anything on that string cannot see them as two different cards.
    #[test]
    fn writing_a_cost_normalises_its_order() {
        let a = ManaCost::try_parse("{W}{2}").expect("parses");
        let b = ManaCost::try_parse("{2}{W}").expect("parses");
        assert_eq!(a.to_string(), "{2}{W}");
        assert_eq!(a.to_string(), b.to_string());
    }

    /// A land has no cost and prints as nothing rather than as `{0}` — the
    /// deck builder shows this string as-is beside the card name.
    #[test]
    fn no_cost_writes_as_nothing() {
        assert_eq!(ManaCost::ZERO.to_string(), "");
        assert_eq!(
            ManaCost::try_parse("{0}").expect("parses").to_string(),
            "{0}"
        );
    }

    /// CR 601.2b: the announced number replaces every variable symbol, and
    /// a card printing two of them (`{X}{X}`) charges it twice. Until it is
    /// announced, `{X}` is nothing — which is what makes an unannounced
    /// `X` spell's mana value zero on the stack.
    #[test]
    fn announcing_x_replaces_every_variable_symbol() {
        let one = ManaCost::parse("{X}{R}");
        assert_eq!(one.cmc(), 1, "an unannounced X counts nothing");
        assert!(one.has_variable());
        assert_eq!(one.with_x(3).to_string(), "{3}{R}");
        assert_eq!(one.with_x(3).cmc(), 4);
        assert!(
            !one.with_x(3).has_variable(),
            "and it is no longer variable"
        );

        let twice = ManaCost::parse("{X}{X}{B}");
        assert_eq!(twice.with_x(2).cmc(), 5, "two symbols, twice the number");
        // Announcing nothing leaves a `{0}` symbol standing, where
        // `with_less_generic` drops a generic part it has emptied. The
        // arithmetic agrees either way — this is what the cost *prints*,
        // and Magic prints `{0}` beside nothing else. Pinned rather than
        // endorsed: a reader that draws a pip per symbol draws one here.
        assert_eq!(one.with_x(0).to_string(), "{0}{R}");
        assert_eq!(one.with_x(0).cmc(), 1, "which costs what it should");
    }

    /// Delve and convoke take generic mana off and **only** generic mana: a
    /// reduction larger than the cost has is not a cost that pays its
    /// coloured pips for you, and it must not wrap the subtraction either.
    #[test]
    fn a_reduction_eats_generic_mana_and_never_a_coloured_pip() {
        let cost = ManaCost::parse("{3}{U}{U}");
        assert_eq!(cost.with_less_generic(0), cost);
        assert_eq!(cost.with_less_generic(1).to_string(), "{2}{U}{U}");
        assert_eq!(cost.with_less_generic(3).to_string(), "{U}{U}");
        assert_eq!(
            cost.with_less_generic(99).to_string(),
            "{U}{U}",
            "a reduction bigger than the cost leaves the colours standing"
        );
        assert_eq!(cost.with_less_generic(99).cmc(), 2);
        assert_eq!(
            ManaCost::parse("{U}{U}").with_less_generic(2).to_string(),
            "{U}{U}",
            "and a cost with no generic part is untouched"
        );
    }

    /// The mirror, and the commander tax is what needed it: a cost printing
    /// no generic symbol at all has to **grow** one rather than stay as it
    /// was. Two taxes in a row add up on the one symbol instead of writing
    /// two.
    #[test]
    fn a_cost_increase_grows_a_generic_symbol_that_was_not_printed() {
        let plain = ManaCost::parse("{G}{G}");
        assert_eq!(plain.with_more_generic(0), plain, "no tax, no change");
        assert_eq!(plain.with_more_generic(2).to_string(), "{2}{G}{G}");
        assert_eq!(
            plain.with_more_generic(2).with_more_generic(2).to_string(),
            "{4}{G}{G}",
            "the second tax joins the first rather than writing a second symbol"
        );
        assert_eq!(
            ManaCost::parse("{1}{W}").with_more_generic(2).to_string(),
            "{3}{W}"
        );
        assert_eq!(
            ManaCost::ZERO.with_more_generic(3).to_string(),
            "{3}",
            "a free spell taxed is not a free spell"
        );
    }

    /// CR 601.2f: an additional cost stacks onto the base cost. Kicker is
    /// the shape, and the result is one cost in canonical order rather than
    /// two lists laid end to end.
    #[test]
    fn an_additional_cost_stacks_onto_the_one_that_was_printed() {
        let base = ManaCost::parse("{1}{G}");
        let kicker = ManaCost::parse("{2}{R}");
        let both = base.combine(&kicker);

        assert_eq!(both.cmc(), base.cmc() + kicker.cmc());
        assert_eq!(
            both.colors(),
            ColorSet::from_slice(&[Color::Red, Color::Green])
        );
        assert_eq!(
            both.to_string(),
            kicker.combine(&base).to_string(),
            "combining is not an order the reader can see"
        );
        assert_eq!(base.combine(&ManaCost::ZERO), base);
        assert!(
            base.combine(&ManaCost::parse("{X}")).has_variable(),
            "and a variable carried in is still variable"
        );
    }

    /// CR 702.56a: a replicate cost is paid "any number of times", and every
    /// payment is part of the total cost (CR 601.2f). Lose Focus is `{1}{U}`
    /// with replicate `{U}`. A house seat with fifteen mana priced it
    /// replicated fifteen times, seventeen symbols, and the cost that held
    /// sixteen in a list asserted and took the hosted game down (the fuzzer,
    /// 2026-09-29). Counted, fifty payments are fifty more `{U}` in one cost.
    #[test]
    fn a_replicate_cost_paid_any_number_of_times_is_one_cost() {
        let lose_focus = ManaCost::parse("{1}{U}");
        let replicate = ManaCost::parse("{U}");
        // Payment by payment, as the house priced it: the loop that asserted
        // at its fifteenth turn.
        let mut paid = lose_focus;
        for _ in 0..50 {
            paid = paid.combine(&replicate);
        }
        assert_eq!(
            paid,
            lose_focus.combine_n(&replicate, 50),
            "fifty payments at once are the same fifty payments"
        );
        assert_eq!(paid.cmc(), 52);
        assert_eq!(paid.len(), 52, "one {{1}} and fifty-one {{U}}");
        assert_eq!(paid.generic_total(), 1);
        assert_eq!(
            paid.symbols().filter(|s| *s == ManaSymbol::Blue).count(),
            51
        );
        let written = paid.to_string();
        assert_eq!(written, format!("{{1}}{}", "{U}".repeat(51)));
        assert_eq!(ManaCost::try_parse(&written), Ok(paid), "and it reads back");
        assert_eq!(
            lose_focus.combine_n(&replicate, 0),
            lose_focus,
            "paid no times, it is the spell's own cost"
        );
    }

    /// Numerical symbols are all generic mana (CR 107.4b), and a cost holds
    /// its generic mana as one symbol however it was built, as a printed
    /// cost writes it: two `{1}` on `{W}` are `{2}{W}`, and `{X}{X}` for two
    /// is `{4}`.
    #[test]
    fn generic_mana_is_one_symbol_however_it_was_added() {
        assert_eq!(ManaCost::parse("{1}{1}"), ManaCost::parse("{2}"));
        let one = ManaCost::parse("{1}");
        let twice = ManaCost::parse("{W}").combine(&one).combine(&one);
        assert_eq!(twice.to_string(), "{2}{W}");
        assert_eq!(twice.len(), 2);
        assert_eq!(ManaCost::parse("{X}{X}{B}").with_x(2).to_string(), "{4}{B}");
        assert_eq!(
            ManaCost::parse("{0}")
                .combine(&ManaCost::parse("{2}"))
                .to_string(),
            "{2}",
            "a {{0}} joins the generic mana rather than standing beside it"
        );
    }

    /// `try_parse` is total: a runtime reader hands it text it did not
    /// write, and what no cost can hold is an error, never a panic. `{W/W}`
    /// asserted in `ColorPair::new`, and a number past `u32::MAX` wrapped in
    /// a release build and panicked in a debug one.
    #[test]
    fn reading_a_cost_refuses_what_no_cost_holds_without_panicking() {
        for text in ["{W/W}", "{U/U/P}", "{4294967296}", "{99999999999999999999}"] {
            assert!(ManaCost::try_parse(text).is_err(), "{text}");
        }
        assert_eq!(
            ManaCost::try_parse("{4294967295}").map(|c| c.cmc()),
            Ok(u32::MAX)
        );
    }

    /// The canonical order, pinned symbol by symbol: every symbol a cost
    /// counts, once, in the order a cost is written in, and each at the one
    /// index its slot names.
    #[test]
    fn every_symbol_has_one_place_in_the_written_order() {
        for (k, symbol) in KIND_SYMBOLS.iter().enumerate() {
            assert_eq!(slot(*symbol), Slot::Kind(k), "{symbol:?}");
        }
        let every = KIND_SYMBOLS
            .iter()
            .rev()
            .fold(ManaCost::parse("{3}"), |cost, s| {
                cost.combine(&ManaCost::from_symbol(*s))
            });
        assert_eq!(
            every.to_string(),
            "{X}{Y}{Z}{3}{½}{∞}{W}{U}{B}{R}{G}\
             {W/U}{W/B}{U/B}{R/W}{U/R}{B/R}{G/W}{G/U}{B/G}{R/G}\
             {2/W}{2/U}{2/B}{2/R}{2/G}{W/P}{U/P}{B/P}{R/P}{G/P}\
             {W/U/P}{W/B/P}{U/B/P}{R/W/P}{U/R/P}{B/R/P}{G/W/P}{G/U/P}{B/G/P}{R/G/P}\
             {S}{C}"
        );
        assert_eq!(ManaCost::try_parse(&every.to_string()), Ok(every));
        // The most kinds a cost holds, each walked once.
        assert_eq!(every.kinds(), 43);
        assert_eq!(every.runs().count(), 43);
        assert_eq!(ManaCost::ZERO.kinds(), 0);
        assert_eq!(ManaCost::parse("{0}").kinds(), 1, "{{0}} is a symbol");
        assert_eq!(ManaCost::parse("{2}{U}{U}").kinds(), 2);
    }

    /// On the wire a cost is its notation, of any length, and text that is
    /// no cost is refused as an error.
    #[test]
    fn a_cost_travels_as_its_notation() {
        let cost = ManaCost::parse("{1}{U}").combine_n(&ManaCost::parse("{U}"), 20);
        let json = serde_json::to_string(&cost).expect("a cost writes");
        assert_eq!(json, format!("\"{cost}\""));
        assert_eq!(
            serde_json::from_str::<ManaCost>(&json).expect("and reads"),
            cost
        );
        assert_eq!(
            serde_json::to_string(&ManaCost::ZERO).expect("writes"),
            "\"\""
        );
        assert!(serde_json::from_str::<ManaCost>("\"{W/W}\"").is_err());
    }

    /// What a cost *is* made of, read four ways: the total, the generic
    /// half, whether it is variable at all, and which colours it demands.
    /// A hybrid symbol demands both of its colours and costs one.
    #[test]
    fn a_cost_answers_what_it_is_made_of() {
        let hybrid = ManaCost::parse("{2}{W/U}{B}");
        assert_eq!(hybrid.cmc(), 4);
        assert_eq!(hybrid.generic_total(), 2);
        assert!(!hybrid.has_variable());
        assert_eq!(
            hybrid.colors(),
            ColorSet::from_slice(&[Color::White, Color::Blue, Color::Black]),
            "a hybrid pip is both of its colours"
        );
        assert!(!hybrid.is_empty());
        assert!(ManaCost::ZERO.is_empty());
        assert_eq!(ManaCost::ZERO.cmc(), 0);
        assert_eq!(ManaCost::ZERO.colors(), ColorSet::EMPTY);
    }
    /// Every mana symbol there is, so the two questions below are asked of
    /// the whole enum and not of the ones somebody remembered.
    const EVERY_SYMBOL: [ManaSymbol; 15] = [
        ManaSymbol::Generic(3),
        ManaSymbol::Colorless,
        ManaSymbol::White,
        ManaSymbol::Blue,
        ManaSymbol::Black,
        ManaSymbol::Red,
        ManaSymbol::Green,
        ManaSymbol::Hybrid(ColorPair::new(Color::White, Color::Blue)),
        ManaSymbol::TwoOrColor(Color::White),
        ManaSymbol::Phyrexian(Color::White),
        ManaSymbol::HybridPhyrexian(ColorPair::new(Color::White, Color::Blue)),
        ManaSymbol::Snow,
        ManaSymbol::Variable(Variable::X),
        ManaSymbol::HalfGeneric,
        ManaSymbol::Infinite,
    ];

    /// A distinct number per variant, and the **guard** the list above needs.
    ///
    /// [`ManaSymbol::cmc_contribution`] and
    /// [`ManaSymbol::generic_contribution`] both end in a catch-all arm, so
    /// a symbol added tomorrow compiles there and silently contributes one
    /// to every mana value and nothing to every generic bucket. This match
    /// is exhaustive, so the same symbol stops the build here instead —
    /// which is the whole reason the answers are worth writing down.
    fn variant_index(symbol: ManaSymbol) -> usize {
        match symbol {
            ManaSymbol::Generic(_) => 0,
            ManaSymbol::Colorless => 1,
            ManaSymbol::White => 2,
            ManaSymbol::Blue => 3,
            ManaSymbol::Black => 4,
            ManaSymbol::Red => 5,
            ManaSymbol::Green => 6,
            ManaSymbol::Hybrid(_) => 7,
            ManaSymbol::TwoOrColor(_) => 8,
            ManaSymbol::Phyrexian(_) => 9,
            ManaSymbol::HybridPhyrexian(_) => 10,
            ManaSymbol::Snow => 11,
            ManaSymbol::Variable(_) => 12,
            ManaSymbol::HalfGeneric => 13,
            ManaSymbol::Infinite => 14,
        }
    }

    /// What each symbol is worth to a mana value (CR 202.3) and what it puts
    /// in the generic bucket, which are two different numbers on the one
    /// symbol where it matters.
    ///
    /// `{2/W}` is that symbol: CR 202.3f makes its mana value **two**, and
    /// it adds **nothing** generic, because a cost with one is not a cost
    /// with two generic mana in it — the choice between two mana and one
    /// white is made at payment. Reading either number off the other would
    /// be wrong in a different direction each way.
    ///
    /// The three that are worth nothing are worth nothing for two different
    /// reasons. `{X}` is CR 202.3b: outside the stack X is zero, and a card
    /// in hand is outside the stack. `{½}` and `{∞}` are silver-bordered and
    /// have no rules answer at all, so nought is this repo's choice rather
    /// than the rules' — pinned so that a game which one day means to play
    /// them has to say so.
    #[test]
    fn every_mana_symbol_is_worth_what_the_rules_say_it_is() {
        let expected: Vec<(usize, u32, u32)> = vec![
            // (variant, mana value, generic)
            (0, 3, 3),  // {3}
            (1, 1, 0),  // {C}
            (2, 1, 0),  // {W}
            (3, 1, 0),  // {U}
            (4, 1, 0),  // {B}
            (5, 1, 0),  // {R}
            (6, 1, 0),  // {G}
            (7, 1, 0),  // {W/U}
            (8, 2, 0),  // {2/W}
            (9, 1, 0),  // {W/P}
            (10, 1, 0), // {W/U/P}
            (11, 1, 0), // {S}
            (12, 0, 0), // {X}
            (13, 0, 0), // {½}
            (14, 0, 0), // {∞}
        ];
        let seen: Vec<(usize, u32, u32)> = EVERY_SYMBOL
            .iter()
            .map(|s| {
                (
                    variant_index(*s),
                    s.cmc_contribution(),
                    s.generic_contribution(),
                )
            })
            .collect();
        assert_eq!(seen, expected);
        assert_eq!(
            seen.iter().map(|(v, ..)| *v).collect::<Vec<_>>(),
            (0..15).collect::<Vec<_>>(),
            "the list is every variant exactly once, in declaration order"
        );
    }

    /// A whole cost is the sum of its symbols, which is worth checking on a
    /// card that makes the difference visible: Reaper King's five `{2/A}`
    /// symbols are a mana value of ten and not one generic mana anywhere,
    /// and a reader that took the mana value for the generic requirement
    /// would offer it for ten of any colour.
    #[test]
    fn a_cost_of_hybrids_costs_what_its_symbols_cost() {
        let king = ManaCost::parse("{2/W}{2/U}{2/B}{2/R}{2/G}");
        assert_eq!(king.cmc(), 10);
        assert_eq!(king.generic_total(), 0);
        assert_eq!(king.symbols().count(), 5);
        assert_eq!(
            king.colors(),
            ColorSet::from_slice(&[
                Color::White,
                Color::Blue,
                Color::Black,
                Color::Red,
                Color::Green,
            ]),
            "each of them is its one colour"
        );

        // The Phyrexian pair beside it, which pays in life and is still a
        // coloured pip of mana value one each (CR 202.3g).
        let pitch = ManaCost::parse("{2}{W/P}{W/U/P}");
        assert_eq!((pitch.cmc(), pitch.generic_total()), (4, 2));
        assert!(!pitch.has_variable());

        // And `{X}`, which is nought until it is announced.
        let fireball = ManaCost::parse("{X}{R}");
        assert_eq!((fireball.cmc(), fireball.generic_total()), (1, 0));
        assert!(fireball.has_variable());
        assert_eq!(fireball.with_x(4).cmc(), 5);
    }

    /// Restricted mana is taken **by its restriction and not by its
    /// position**, which is what lets a solver spend Cavern of Souls's mana
    /// on the creature it named and leave the rest of the pool alone.
    #[test]
    fn restricted_mana_is_taken_by_the_restriction_that_named_it() {
        let mut pool = ManaPool::new();
        let rider = |id: u32, color: ManaColor| RestrictedMana {
            color,
            amount: 1,
            flags: ManaFlags::NO_EMPTY,
            restriction: RestrictionId(id),
        };
        pool.add_restricted(rider(7, ManaColor::Green));
        pool.add_restricted(rider(9, ManaColor::Red));
        assert_eq!(pool.total(), 2);

        assert!(
            pool.take_restricted(4).is_none(),
            "a restriction nobody is holding mana for"
        );
        let taken = pool.take_restricted(9).expect("the second one");
        assert_eq!(taken.color, ManaColor::Red);
        assert_eq!(pool.restricted().len(), 1);
        assert_eq!(
            pool.restricted()[0].restriction,
            RestrictionId(7),
            "and the one nobody asked for is untouched"
        );
        assert_eq!(pool.total(), 1);
    }

    /// Units come off an entry in place: the rest keeps its restriction and
    /// its position, and the entry goes with its last unit.
    #[test]
    fn restricted_units_are_taken_and_the_rest_keeps_its_restriction() {
        let mut pool = ManaPool::new();
        let entry = |id: u32, amount: u16| RestrictedMana {
            color: ManaColor::Colorless,
            amount,
            flags: ManaFlags::NONE,
            restriction: RestrictionId(id),
        };
        pool.add_restricted(entry(3, 3));
        pool.add_restricted(entry(5, 1));

        let taken = pool.take_restricted_units(3, 1).expect("one of three");
        assert_eq!(taken, entry(3, 1), "the part taken carries the restriction");
        assert_eq!(
            pool.restricted(),
            &[entry(3, 2), entry(5, 1)],
            "two are left, under the same id and in the same place"
        );

        let rest = pool
            .take_restricted_units(3, 9)
            .expect("more than there is");
        assert_eq!(rest.amount, 2, "an entry gives up what it has and no more");
        assert_eq!(
            pool.restricted(),
            &[entry(5, 1)],
            "and goes with its last unit"
        );

        assert!(
            pool.take_restricted_units(3, 1).is_none(),
            "an id nobody holds"
        );
        assert!(
            pool.take_restricted_units(5, 0).is_none(),
            "nothing asked for"
        );
        assert_eq!(
            pool.restricted(),
            &[entry(5, 1)],
            "and neither touched anything"
        );
    }

    /// `colors_available` reads the **plain** part of the pool and nothing
    /// else, which is a limitation rather than an oversight: restricted mana
    /// is spendable only on what its rider names, so a caller told it was
    /// "available" would offer a spell the payment then refuses.
    ///
    /// Pinned because it is invisible from the name. A pool holding one
    /// green mana under a restriction answers the same as an empty one.
    #[test]
    fn a_colour_under_a_restriction_is_not_a_colour_available() {
        let mut pool = ManaPool::new();
        pool.add_restricted(RestrictedMana {
            color: ManaColor::Green,
            amount: 1,
            flags: ManaFlags::NO_EMPTY,
            restriction: RestrictionId(3),
        });
        assert_eq!(pool.total(), 1, "the mana is in the pool");
        assert!(
            pool.colors_available().is_empty(),
            "and green is not one of the colours it reports"
        );

        pool.add(ManaColor::Green, 1);
        assert_eq!(pool.colors_available(), ColorSet::of(Color::Green));
        assert_eq!(pool.available(ManaColor::Green), 1, "the plain one only");
    }
}
