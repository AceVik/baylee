//! The mana cost left after using untapped creatures for convoke.

use baylee_core::mana::ManaCost;

pub(super) fn has_convoke(card: baylee_view::CardIdentity) -> bool {
    baylee_cards::by_index(card.index)
        .and_then(|def| def.faces.get(usize::from(card.face)).or(def.faces.first()))
        .is_some_and(|face| face.convoke)
}

pub(super) fn reduce_for_convoke(
    cost: ManaCost,
    creatures: &[baylee_core::color::ColorSet],
) -> ManaCost {
    let mut available: Vec<baylee_core::color::ColorSet> = creatures.to_vec();
    let mut out = ManaCost::ZERO;
    for symbol in cost.symbols() {
        let sym_colors = symbol.colors();
        if !sym_colors.is_empty()
            && let Some(idx) = available
                .iter()
                .position(|c| !c.intersection(sym_colors).is_empty())
        {
            available.remove(idx);
            continue;
        }
        out = out.with_symbol(symbol, 1);
    }
    let remaining = u32::try_from(available.len()).unwrap_or(u32::MAX);
    out.with_less_generic(remaining)
}
