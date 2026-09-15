//! Search for Azcanta // Azcanta, the Sunken Ruin — {1}{U} — Legendary Enchantment // Legendary Land
//! Oracle: At the beginning of your upkeep, surveil 1. Then if you have seven or more cards in your graveyard, you may transform Search for Azcanta. (Look at the top card of your library. You may put that card into your graveyard.)
//! Oracle: (Transforms from Search for Azcanta.)
//! Oracle: {T}: Add {U}.
//! Oracle: {2}{U}, {T}: Look at the top four cards of your library. You may reveal a noncreature, nonland card from among them and put it into your hand. Put the rest on the bottom of your library in any order.
//! Set: XLN #74 — Ixalan | Scryfall ID: 1a7e242e-bb48-4134-a1c2-6033713d658f | Oracle ID: f74c4d96-bc4a-4d32-9519-a753d192144e
//! Face: Search for Azcanta — {1}{U} — Legendary Enchantment
//! Face: Azcanta, the Sunken Ruin —  — Legendary Land
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card! {
    index: 985,
    oracle_id: "f74c4d96-bc4a-4d32-9519-a753d192144e",
    scryfall_id: "1a7e242e-bb48-4134-a1c2-6033713d658f",
    color_identity: ColorSet::from_slice(&[Color::Blue]),
    faces: &[
    face! {
        name: "Search for Azcanta",
        mana_cost: mana!("{1}{U}"),
        types: TypeSet::ENCHANTMENT,
        supertypes: SupertypeSet::LEGENDARY,
    },
    face! {
        name: "Azcanta, the Sunken Ruin",
        types: TypeSet::LAND,
        supertypes: SupertypeSet::LEGENDARY,
    },
    ],
}

// TODO(card): implement abilities, see docs/card-dsl.md.
