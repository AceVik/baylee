//! Phyrexian Tower — (no cost) — Legendary Land
//! Oracle: {T}: Add {C}.
//! Oracle: {T}, Sacrifice a creature: Add {B}{B}.
//! Set: MH3 #303 — Modern Horizons 3 | Scryfall ID: 0b47f6d2-9f65-47a4-bfc4-15619befe53d | Oracle ID: 1861e642-21d5-4232-89f3-b5557f2946c1
// IMPLEMENTED — two printed mana abilities sharing one {T}: tap for {C}, or
// tap and eat a creature for {B}{B}. Both are mana abilities under CR 605.1
// and neither uses the stack; which creature the second one eats is asked as
// it is activated, by `engine::cost_wizard`. The shared {T} is what makes the
// pair worth reading twice — activating either takes the other off the offer.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::PHYREXIAN_TOWER,
    oracle_id = "1861e642-21d5-4232-89f3-b5557f2946c1",
    scryfall_id = "0b47f6d2-9f65-47a4-bfc4-15619befe53d",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Phyrexian Tower",
        types = TypeSet::LAND,
        supertypes = SupertypeSet::LEGENDARY,
    ),],
    abilities = &[
        mana_ability!(&[Effect::mana(ManaColor::Colorless, 1)]),
        mana_ability!(
            cost!(TapSelf, Sacrifice(&Filter::YOUR_CREATURE)),
            &[Effect::mana(ManaColor::Black, 2)]
        ),
    ],
);
