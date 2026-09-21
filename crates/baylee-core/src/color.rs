//! Colors, color pairs, and color sets (WUBRG).

use serde::{Deserialize, Serialize};

/// One of the five colors of Magic.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug, Serialize, Deserialize)]
pub enum Color {
    /// White (`{W}`).
    White = 0,
    /// Blue (`{U}`).
    Blue = 1,
    /// Black (`{B}`).
    Black = 2,
    /// Red (`{R}`).
    Red = 3,
    /// Green (`{G}`).
    Green = 4,
}

impl Color {
    /// All five colors in WUBRG order.
    pub const ALL: [Color; 5] = [
        Color::White,
        Color::Blue,
        Color::Black,
        Color::Red,
        Color::Green,
    ];

    /// The single bit representing this color in a [`ColorSet`].
    #[inline]
    #[must_use]
    pub const fn bit(self) -> u8 {
        1 << (self as u8)
    }

    /// The canonical one-letter symbol (`W`, `U`, `B`, `R`, `G`).
    #[must_use]
    pub const fn symbol(self) -> char {
        match self {
            Color::White => 'W',
            Color::Blue => 'U',
            Color::Black => 'B',
            Color::Red => 'R',
            Color::Green => 'G',
        }
    }

    /// Parses a one-letter symbol.
    #[must_use]
    pub const fn from_symbol(c: char) -> Option<Color> {
        match c {
            'W' | 'w' => Some(Color::White),
            'U' | 'u' => Some(Color::Blue),
            'B' | 'b' => Some(Color::Black),
            'R' | 'r' => Some(Color::Red),
            'G' | 'g' => Some(Color::Green),
            _ => None,
        }
    }

    /// The English color name.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Color::White => "White",
            Color::Blue => "Blue",
            Color::Black => "Black",
            Color::Red => "Red",
            Color::Green => "Green",
        }
    }
}

/// An unordered pair of distinct colors (hybrid mana symbols).
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Serialize, Deserialize)]
pub struct ColorPair(Color, Color);

impl ColorPair {
    /// Creates a pair in canonical color-wheel order (the order used on
    /// printed cards: `G/U`, not `U/G` — the second color follows the first
    /// on the W→U→B→R→G→W wheel in at most two steps).
    ///
    /// # Panics
    /// When `a == b` (a pair requires two distinct colors).
    #[must_use]
    pub const fn new(a: Color, b: Color) -> Self {
        assert!(a as u8 != b as u8, "ColorPair requires two distinct colors");
        let forward = (b as u8 + 5 - a as u8) % 5;
        let backward = (a as u8 + 5 - b as u8) % 5;
        if forward <= backward {
            Self(a, b)
        } else {
            Self(b, a)
        }
    }

    /// First color in WUBRG order.
    #[must_use]
    pub const fn first(self) -> Color {
        self.0
    }

    /// Second color in WUBRG order.
    #[must_use]
    pub const fn second(self) -> Color {
        self.1
    }

    /// Whether the pair contains the color.
    #[must_use]
    pub const fn contains(self, c: Color) -> bool {
        self.0 as u8 == c as u8 || self.1 as u8 == c as u8
    }

    /// Both bits set.
    #[must_use]
    pub const fn bits(self) -> u8 {
        self.0.bit() | self.1.bit()
    }
}

/// Bitset of colors (WUBRG); the empty set represents *colorless*.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub struct ColorSet(u8);

impl ColorSet {
    /// No colors — colorless.
    pub const EMPTY: Self = Self(0);
    /// All five colors.
    pub const ALL: Self = Self(0b1_1111);

    /// A set containing exactly one color.
    #[must_use]
    pub const fn of(c: Color) -> Self {
        Self(c.bit())
    }

    /// A set containing both colors of a pair.
    #[must_use]
    pub const fn of_pair(p: ColorPair) -> Self {
        Self(p.bits())
    }

    /// A set from a slice of colors (const-friendly for codegen).
    #[must_use]
    pub const fn from_slice(colors: &[Color]) -> Self {
        let mut bits = 0u8;
        let mut i = 0;
        while i < colors.len() {
            bits |= colors[i].bit();
            i += 1;
        }
        Self(bits)
    }

    /// Whether the color is in the set.
    #[inline]
    #[must_use]
    pub const fn contains(self, c: Color) -> bool {
        self.0 & c.bit() != 0
    }

    /// Whether both sets share at least one color.
    #[inline]
    #[must_use]
    pub const fn intersects(self, other: Self) -> bool {
        self.0 & other.0 != 0
    }

    /// Union.
    #[must_use]
    pub const fn union(self, other: Self) -> Self {
        Self(self.0 | other.0)
    }

    /// Intersection.
    #[must_use]
    pub const fn intersection(self, other: Self) -> Self {
        Self(self.0 & other.0)
    }

    /// Difference.
    #[must_use]
    pub const fn difference(self, other: Self) -> Self {
        Self(self.0 & !other.0)
    }

    /// Number of colors in the set.
    #[must_use]
    pub const fn len(self) -> u8 {
        self.0.count_ones() as u8
    }

    /// Whether the set is empty.
    #[must_use]
    pub const fn is_empty(self) -> bool {
        self.0 == 0
    }

    /// Whether the set is empty (CR: colorless is the absence of color).
    #[must_use]
    pub const fn is_colorless(self) -> bool {
        self.is_empty()
    }

    /// Raw bits (WUBRG).
    #[must_use]
    pub const fn bits(self) -> u8 {
        self.0
    }

    /// Iterates the colors in WUBRG order.
    pub fn iter(self) -> impl Iterator<Item = Color> {
        Color::ALL.into_iter().filter(move |c| self.contains(*c))
    }
}

impl core::fmt::Debug for ColorSet {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "ColorSet({self})")
    }
}

impl core::fmt::Display for ColorSet {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        if self.is_empty() {
            return write!(f, "C");
        }
        for c in self.iter() {
            write!(f, "{}", c.symbol())?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn color_set_ops() {
        let wu = ColorSet::from_slice(&[Color::White, Color::Blue]);
        assert!(wu.contains(Color::White));
        assert!(!wu.contains(Color::Red));
        assert_eq!(wu.len(), 2);
        assert_eq!(wu.to_string(), "WU");
        assert!(ColorSet::EMPTY.is_colorless());
        let pair = ColorPair::new(Color::Green, Color::Blue);
        assert_eq!(pair.first(), Color::Green); // wheel order: "G/U"
        assert_eq!(pair.second(), Color::Blue);
        assert!(pair.contains(Color::Green));
        assert_eq!(
            ColorPair::new(Color::Blue, Color::White).first(),
            Color::White
        );
    }

    /// The ten pairs Magic prints, and the order it prints them in: the
    /// second colour follows the first on the W→U→B→R→G→W wheel in at most
    /// two steps. Both ways round give the same pair, which is what makes
    /// this a canonical form rather than a preference — a guild stored one
    /// way and looked up the other would be two different keys.
    #[test]
    fn every_pair_is_canonical_whichever_way_round_it_is_built() {
        use Color::{Black, Blue, Green, Red, White};
        // Allied first (one step), then enemy (two): Orzhov is W/B and
        // never B/W, Simic is G/U and never U/G.
        const PRINTED: [(Color, Color); 10] = [
            (White, Blue),
            (Blue, Black),
            (Black, Red),
            (Red, Green),
            (Green, White),
            (White, Black),
            (Blue, Red),
            (Black, Green),
            (Red, White),
            (Green, Blue),
        ];
        assert_eq!(PRINTED.len(), 10, "every unordered pair of five colours");
        for (a, b) in PRINTED {
            for pair in [ColorPair::new(a, b), ColorPair::new(b, a)] {
                assert_eq!((pair.first(), pair.second()), (a, b), "{a:?}/{b:?}");
                assert!(pair.contains(a) && pair.contains(b));
                assert_eq!(pair.bits(), a.bit() | b.bit());
                assert_eq!(ColorSet::of_pair(pair).len(), 2);
            }
        }
    }

    /// A pair is two *distinct* colours, and the refusal is a panic in a
    /// `const fn` — so a card written with one is a build error rather than
    /// a guild that is half a colour.
    #[test]
    #[should_panic(expected = "two distinct colors")]
    fn a_pair_of_one_colour_is_refused() {
        let _ = ColorPair::new(Color::Red, Color::Red);
    }

    /// The symbol is the round trip a decklist, a mana cost and a card
    /// filter all go through. `C` is not a colour — colorless is the
    /// absence of one — so it has no `Color` to come back as.
    #[test]
    fn a_colour_survives_its_own_symbol() {
        for c in Color::ALL {
            assert_eq!(Color::from_symbol(c.symbol()), Some(c), "{c:?}");
            assert!(!c.name().is_empty());
        }
        assert_eq!(Color::from_symbol('C'), None, "colorless is not a colour");
        assert_eq!(Color::from_symbol('X'), None, "and neither is X");
        for c in Color::ALL {
            assert_eq!(
                Color::from_symbol(c.symbol().to_ascii_lowercase()),
                Some(c),
                "a decklist writes its colours in either case"
            );
        }
    }

    /// Each colour owns one bit, and a set prints and iterates in WUBRG
    /// order however it was built — the order a player reads a cost in.
    #[test]
    fn a_set_reads_in_wubrg_order_however_it_was_built() {
        let bits: Vec<u8> = Color::ALL.iter().map(|c| c.bit()).collect();
        let mut distinct = bits.clone();
        distinct.sort_unstable();
        distinct.dedup();
        assert_eq!(distinct.len(), 5, "one bit each");

        let backwards = ColorSet::from_slice(&[Color::Green, Color::Black, Color::White]);
        assert_eq!(backwards.to_string(), "WBG");
        assert_eq!(
            backwards.iter().collect::<Vec<_>>(),
            vec![Color::White, Color::Black, Color::Green]
        );
        assert_eq!(ColorSet::EMPTY.to_string(), "C", "colorless prints as C");
        assert_eq!(format!("{backwards:?}"), "ColorSet(WBG)");
    }

    /// The set algebra the layer system and every colour filter read.
    /// `difference` is the one with a direction, and the identities are
    /// what a filter asking "colours you do not have" depends on.
    #[test]
    fn the_set_algebra_holds_on_the_five() {
        let wu = ColorSet::from_slice(&[Color::White, Color::Blue]);
        let ub = ColorSet::from_slice(&[Color::Blue, Color::Black]);
        let all = ColorSet::from_slice(&Color::ALL);

        assert_eq!(wu.union(ub).len(), 3);
        assert_eq!(
            wu.intersection(ub),
            ColorSet::of(Color::Blue),
            "the shared colour and nothing else"
        );
        assert_eq!(wu.difference(ub), ColorSet::of(Color::White));
        assert_eq!(
            ub.difference(wu),
            ColorSet::of(Color::Black),
            "it has a direction"
        );
        assert!(wu.intersects(ub));
        assert!(!wu.intersects(ColorSet::of(Color::Red)));
        assert_eq!(all.len(), 5);
        assert_eq!(all.difference(all), ColorSet::EMPTY);
        assert!(all.difference(all).is_colorless());
        assert_eq!(wu.union(wu), wu);
        assert_eq!(ColorSet::EMPTY.union(wu), wu);
    }
}
