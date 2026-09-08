//! Floating mana, as a row the HUD can draw.
//!
//! A mana pool is the one zone in Magic with no card in it, which is why it
//! took until now to get a place on screen: everything else the client draws
//! is an object it can point at. It is nonetheless a zone in the sense that
//! matters to a player — mana goes into it when a land is tapped and comes out
//! of it when a spell is paid for, and between those two moments it is the
//! only evidence that the tap did what was intended.
//!
//! That evidence was missing, and it hid a real defect. Jasmine Dragon Tea
//! Shop prints two mana abilities — `{T}: Add {C}` and `{T}: Add one mana of
//! any color`, the second spendable only on Allies — and the client's mana
//! planner reduces every permanent to the *one* tap it can read, which is the
//! first. So the land tapped for `{C}` whatever the player wanted, and with
//! nothing on screen saying what had been produced there was no way to see it
//! happen. The row below is the fix's other half: the ability is reachable by
//! hand, and what it made is now visible.
//!
//! Renderer-free on purpose, like the rest of this crate: what is floating,
//! in what order, with what label, is decided here and drawn elsewhere.

use baylee_core::mana::{ManaColor, ManaSymbol};
use baylee_view::ManaPoolView;

use crate::manapip::{Pip, pip};

/// One kind of mana floating in a seat's pool.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Floating {
    /// Which colour it is. `Colorless` is a colour here, as in the rules.
    pub color: ManaColor,
    /// How to draw the symbol.
    pub pip: Pip,
    /// How many.
    ///
    /// Always drawn as a numeral beside the pip, never as a count of repeated
    /// pips: colour alone must not carry meaning, and six discs in a row is a
    /// number the player has to count.
    pub count: u16,
    /// Whether this mana may only be spent on certain spells (CR 106.6).
    ///
    /// The view does not say *what* it may be spent on — that is a rules
    /// question the engine answers at the moment of payment — so the row says
    /// only that a restriction exists, and the seat that just chose it knows
    /// the rest.
    pub restricted: bool,
}

/// The symbol for one mana colour.
#[must_use]
pub fn symbol(color: ManaColor) -> Pip {
    pip(match color {
        ManaColor::White => ManaSymbol::White,
        ManaColor::Blue => ManaSymbol::Blue,
        ManaColor::Black => ManaSymbol::Black,
        ManaColor::Red => ManaSymbol::Red,
        ManaColor::Green => ManaSymbol::Green,
        ManaColor::Colorless => ManaSymbol::Colorless,
    })
}

/// What is floating, in the order a player reads a cost: WUBRG then colorless,
/// plain mana first and restricted mana after all of it.
///
/// Empty kinds are left out, so an empty pool is an empty row and the HUD has
/// one thing to test rather than seven zeroes to hide. Restricted mana comes
/// last as a block rather than beside its own colour, because "what can I
/// spend freely" is the question asked first and a mixed row answers it only
/// after reading every entry.
#[must_use]
pub fn row(pool: &ManaPoolView) -> Vec<Floating> {
    let mut out = Vec::new();
    for color in ManaColor::ALL {
        let count = plain(pool, color);
        if count > 0 {
            out.push(Floating {
                color,
                pip: symbol(color),
                count,
                restricted: false,
            });
        }
    }
    for color in ManaColor::ALL {
        let count = pool.restricted[color.index()];
        if count > 0 {
            out.push(Floating {
                color,
                pip: symbol(color),
                count,
                restricted: true,
            });
        }
    }
    out
}

/// The unrestricted mana of one colour.
#[must_use]
pub fn plain(pool: &ManaPoolView, color: ManaColor) -> u16 {
    match color {
        ManaColor::White => pool.white,
        ManaColor::Blue => pool.blue,
        ManaColor::Black => pool.black,
        ManaColor::Red => pool.red,
        ManaColor::Green => pool.green,
        ManaColor::Colorless => pool.colorless,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pool() -> ManaPoolView {
        ManaPoolView::default()
    }

    #[test]
    fn an_empty_pool_is_an_empty_row() {
        assert!(row(&pool()).is_empty());
        assert!(pool().is_empty());
    }

    #[test]
    fn plain_mana_comes_out_in_the_printed_order() {
        let mut p = pool();
        p.green = 1;
        p.white = 2;
        p.colorless = 3;
        let colors: Vec<ManaColor> = row(&p).iter().map(|f| f.color).collect();
        assert_eq!(
            colors,
            vec![ManaColor::White, ManaColor::Green, ManaColor::Colorless],
            "WUBRG then colorless, whatever order it was added in"
        );
    }

    #[test]
    fn restricted_mana_keeps_its_colour_and_comes_after_the_plain() {
        let mut p = pool();
        p.white = 1;
        p.restricted[ManaColor::White.index()] = 2;
        let entries = row(&p);
        assert_eq!(entries.len(), 2, "one plain white and one restricted white");
        assert_eq!(
            (entries[0].color, entries[0].restricted),
            (ManaColor::White, false)
        );
        assert_eq!(
            (entries[1].color, entries[1].restricted),
            (ManaColor::White, true)
        );
        assert_eq!(entries[1].count, 2);
    }

    /// The reason `VIEW_VERSION` went to 14: an uncoloured total could not
    /// tell these two pools apart, and they are the two outcomes of the choice
    /// a Cavern of Souls asks.
    #[test]
    fn two_restricted_choices_are_different_rows() {
        let mut white = pool();
        white.restricted[ManaColor::White.index()] = 1;
        let mut blue = pool();
        blue.restricted[ManaColor::Blue.index()] = 1;
        assert_ne!(row(&white), row(&blue));
        assert_eq!(white.total(), blue.total(), "the totals never differed");
    }

    #[test]
    fn the_total_counts_restricted_mana_too() {
        let mut p = pool();
        p.red = 2;
        p.restricted[ManaColor::Black.index()] = 3;
        assert_eq!(p.total(), 5);
        assert_eq!(p.restricted_total(), 3);
        assert!(!p.is_empty());
    }
}
