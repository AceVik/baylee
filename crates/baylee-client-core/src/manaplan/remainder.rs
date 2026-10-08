//! What a payment window still asks for once the pool has paid what it can.
//!
//! The view carries the whole cost (`PlayerView::owed`) and the pool beside
//! it, and the pool strip used to draw the cost whole: `{2}{G}` owed stayed
//! `{2}{G}` after the player had tapped a Forest by hand, and the subtraction
//! was the player's to do (TODO client item 8). This is that subtraction, and
//! it is the planner's own matching rather than a count of pips, so the strip
//! and the pay button cannot disagree about what the pool already covers: a
//! hybrid is paid by either half, `{2/G}` either way, a permission to spend
//! mana as another colour (CR 609.4b) counts, and restricted mana does not —
//! [`super::plan`]'s second rule, for the same reason.

use baylee_core::mana::{ManaCost, ManaSymbol};
use baylee_view::ManaPoolView;

use super::{ColorMask, augment, permitted, push_needs, units};

/// What `cost` still asks for with `pool` floating: the symbols the pool
/// cannot pay, the generic ones gathered into one `{n}` in front and every
/// other one as it was printed, in printed order. Empty once the pool pays it.
///
/// A symbol this side of the wire does not read (`{S}`, `{X}`) is never paid
/// from the pool here and always stays, which over-states the remainder
/// rather than hiding a debt.
#[must_use]
pub fn remainder(cost: &ManaCost, pool: &ManaPoolView) -> Vec<ManaSymbol> {
    let symbols: Vec<ManaSymbol> = cost.symbols().collect();
    // `{2/C}` reads two ways, as in `plan`: whichever leaves less to add is
    // what the player still owes, and on a tie the generic reading, which
    // says "one more mana of any kind" where the other would say "a green
    // or two more" about the same pool.
    let coloured = owed(&symbols, pool, false);
    let generic = owed(&symbols, pool, true);
    if still_to_add(&generic) <= still_to_add(&coloured) {
        generic
    } else {
        coloured
    }
}

/// How many mana a remainder still needs added, the cheapest way.
fn still_to_add(symbols: &[ManaSymbol]) -> u32 {
    symbols
        .iter()
        .map(|symbol| match symbol {
            ManaSymbol::Generic(n) => *n,
            _ => 1,
        })
        .sum()
}

/// The remainder under one reading of `{2/C}`.
fn owed(symbols: &[ManaSymbol], pool: &ManaPoolView, generic_twobrid: bool) -> Vec<ManaSymbol> {
    // One entry per mana demanded, and which symbol each was printed as.
    let mut needs: Vec<ColorMask> = Vec::new();
    let mut printed_as: Vec<usize> = Vec::new();
    let mut read = vec![true; symbols.len()];
    for (at, &symbol) in symbols.iter().enumerate() {
        if push_needs(symbol, generic_twobrid, &mut needs) {
            printed_as.resize(needs.len(), at);
        } else {
            read[at] = false;
        }
    }
    let needs: Vec<ColorMask> = needs.into_iter().map(|n| permitted(n, pool)).collect();

    // Kuhn's matching against what floats, every demand tried. Unlike
    // `assign` an unmatched demand is not a failure but the answer, and Kuhn
    // never unmatches a demand it has matched, so taking the narrowest first
    // spends a floating green on `{G}` before `{1}` can claim it.
    let floating = units(pool, &[], needs.len(), false);
    let order: Vec<usize> = (0..floating.len()).collect();
    let mut demands: Vec<usize> = (0..needs.len()).collect();
    demands.sort_by_key(|&n| needs[n].count());
    let mut taken: Vec<Option<usize>> = vec![None; floating.len()];
    for &demand in &demands {
        let mut seen = vec![false; floating.len()];
        augment(demand, &needs, &floating, &order, &mut taken, &mut seen);
    }
    let mut paid = vec![false; needs.len()];
    for &demand in taken.iter().flatten() {
        paid[demand] = true;
    }

    let mut generic = 0;
    let mut rest = Vec::new();
    for (at, &symbol) in symbols.iter().enumerate() {
        if !read[at] {
            rest.push(symbol);
            continue;
        }
        let (mine, unpaid) = printed_as
            .iter()
            .zip(&paid)
            .filter(|(of, _)| **of == at)
            .fold((0u32, 0u32), |(mine, unpaid), (_, paid)| {
                (mine + 1, unpaid + u32::from(!paid))
            });
        match symbol {
            ManaSymbol::Generic(_) => generic += unpaid,
            // Read as two generic: paid by half is one generic still owed;
            // paid by none is the symbol itself, which a green still pays.
            ManaSymbol::TwoOrColor(_) if generic_twobrid && unpaid < mine => generic += unpaid,
            _ if unpaid > 0 => rest.push(symbol),
            _ => {}
        }
    }
    let mut out = Vec::with_capacity(rest.len() + 1);
    if generic > 0 {
        out.push(ManaSymbol::Generic(generic));
    }
    out.extend(rest);
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use baylee_core::mana::ManaColor;

    fn cost(src: &str) -> ManaCost {
        ManaCost::try_parse(src).expect("a valid cost")
    }

    /// The spellings this test reads a remainder back in.
    const SPELLED: [&str; 12] = [
        "{C}", "{W}", "{U}", "{B}", "{R}", "{G}", "{W/U}", "{U/B}", "{2/G}", "{G/P}", "{S}", "{X}",
    ];

    /// The remainder of `src` over `pool`, spelled as a cost. Each symbol is
    /// spelled through the parser, so the spelling is the parser's own.
    fn owed_text(src: &str, pool: &ManaPoolView) -> String {
        remainder(&cost(src), pool)
            .into_iter()
            .map(|symbol| match symbol {
                ManaSymbol::Generic(n) => format!("{{{n}}}"),
                other => SPELLED
                    .into_iter()
                    .find(|s| cost(s).symbols().next() == Some(other))
                    .unwrap_or_else(|| panic!("{other:?} is not spelled by this test"))
                    .to_string(),
            })
            .collect()
    }

    fn pool(colours: &[(ManaColor, u32)]) -> ManaPoolView {
        let mut pool = ManaPoolView::default();
        for &(colour, n) in colours {
            *slot(&mut pool, colour) += n;
        }
        pool
    }

    fn slot(pool: &mut ManaPoolView, colour: ManaColor) -> &mut u32 {
        match colour {
            ManaColor::White => &mut pool.white,
            ManaColor::Blue => &mut pool.blue,
            ManaColor::Black => &mut pool.black,
            ManaColor::Red => &mut pool.red,
            ManaColor::Green => &mut pool.green,
            ManaColor::Colorless => &mut pool.colorless,
        }
    }

    /// Nothing floating: the cost, as printed.
    #[test]
    fn an_empty_pool_leaves_the_whole_cost() {
        let empty = ManaPoolView::default();
        assert_eq!(owed_text("{2}{G}", &empty), "{2}{G}");
        assert_eq!(owed_text("{1}{W/U}", &empty), "{1}{W/U}");
        assert_eq!(owed_text("{2/G}", &empty), "{2/G}");
    }

    /// A hand-tapped Forest pays the green and not the generic: the narrow
    /// demand is matched first, so `{2}` is what is left, never `{1}{G}`.
    #[test]
    fn a_floating_colour_pays_its_own_pip_before_the_generic() {
        assert_eq!(owed_text("{2}{G}", &pool(&[(ManaColor::Green, 1)])), "{2}");
        assert_eq!(owed_text("{2}{G}", &pool(&[(ManaColor::Red, 1)])), "{1}{G}");
        assert_eq!(owed_text("{2}{G}", &pool(&[(ManaColor::Green, 3)])), "");
    }

    /// A hybrid is paid by either half and stays a hybrid while it is owed.
    #[test]
    fn a_hybrid_is_paid_by_either_half_and_owed_as_itself() {
        for half in [ManaColor::White, ManaColor::Blue] {
            assert_eq!(owed_text("{1}{W/U}", &pool(&[(half, 1)])), "{1}");
        }
        assert_eq!(
            owed_text("{1}{W/U}", &pool(&[(ManaColor::Red, 1)])),
            "{W/U}"
        );
        assert_eq!(
            owed_text("{W/U}{U}", &pool(&[(ManaColor::Blue, 1)])),
            "{W/U}",
            "the blue goes where only it fits, and the hybrid is still owed"
        );
    }

    /// `{2/G}` either way, and the cheaper reading is what is said.
    #[test]
    fn a_twobrid_is_owed_the_cheaper_way() {
        assert_eq!(owed_text("{2/G}", &pool(&[(ManaColor::Green, 1)])), "");
        assert_eq!(owed_text("{2/G}", &pool(&[(ManaColor::Red, 1)])), "{1}");
        assert_eq!(owed_text("{2/G}", &pool(&[(ManaColor::Red, 2)])), "");
    }

    /// Restricted mana is not counted (the planner's rule 2), and a
    /// permission to spend one colour as another is.
    #[test]
    fn restricted_mana_is_not_subtracted_and_a_permission_is() {
        let mut restricted = ManaPoolView::default();
        restricted.restricted[ManaColor::Green.index()] = 1;
        assert_eq!(owed_text("{G}", &restricted), "{G}");

        let mut allowed = pool(&[(ManaColor::White, 1)]);
        assert_eq!(owed_text("{R}", &allowed), "{R}");
        allowed.spending.allow(ManaColor::White, ManaColor::Red);
        assert_eq!(owed_text("{R}", &allowed), "");
    }

    /// Phyrexian is paid by its colour here and owed as itself otherwise;
    /// a symbol this module does not read stays.
    #[test]
    fn phyrexian_and_unread_symbols_stay_as_printed() {
        assert_eq!(owed_text("{G/P}", &pool(&[(ManaColor::Green, 1)])), "");
        assert_eq!(owed_text("{G/P}", &ManaPoolView::default()), "{G/P}");
        assert_eq!(owed_text("{1}{S}", &pool(&[(ManaColor::Green, 1)])), "{S}");
    }

    /// The remainder is empty exactly when the planner would pay the cost
    /// from the pool with no tap, over every pool of up to two of each
    /// colour and a spread of costs: the strip's "nothing left" and the pay
    /// button's "nothing to tap" are one answer.
    #[test]
    fn nothing_is_owed_exactly_when_the_pool_pays_without_a_tap() {
        let costs = [
            "{0}",
            "{1}",
            "{G}",
            "{2}{G}",
            "{1}{W/U}",
            "{W/U}{U/B}",
            "{2/G}",
            "{2/G}{R}",
            "{G/P}",
            "{C}{C}",
            "{1}{W}{U}",
            "{B}{B}",
        ];
        let colours = ManaColor::ALL;
        for code in 0..3u32.pow(6) {
            let mut floating = ManaPoolView::default();
            let mut rest = code;
            for colour in colours {
                *slot(&mut floating, colour) = rest % 3;
                rest /= 3;
            }
            for text in costs {
                let asked = cost(text);
                let left = remainder(&asked, &floating);
                let planned = super::super::plan(&asked, &floating, &[]);
                assert_eq!(
                    left.is_empty(),
                    planned.is_some_and(|p| p.is_empty()),
                    "{text} over {floating:?}: {left:?}"
                );
                assert!(
                    still_to_add(&left) <= asked.cmc(),
                    "{text}: a remainder never asks for more than the cost"
                );
            }
        }
    }
}
