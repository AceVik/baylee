//! Hidden Volcano — (no cost) — Land — Cave
//! Oracle: This land enters tapped.
//! Oracle: {T}: Add {R}.
//! Oracle: {4}{R}, {T}, Sacrifice this land: Discover 4. Activate only as a sorcery. (Exile cards from the top of your library until you exile a nonland card with mana value 4 or less. Cast it without paying its mana cost or put it into your hand. Put the rest on the bottom in a random order.)
//! Set: LCI #277 — The Lost Caverns of Ixalan | Scryfall ID: 9fa06aed-52c1-48f1-9906-362db12a3cf7 | Oracle ID: a1c7cd7a-0795-4135-b787-effeb981d95b
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card! {
    index: 625,
    oracle_id: "a1c7cd7a-0795-4135-b787-effeb981d95b",
    scryfall_id: "9fa06aed-52c1-48f1-9906-362db12a3cf7",
    color_identity: ColorSet::from_slice(&[Color::Red]),
    faces: &[
    face! {
        name: "Hidden Volcano",
        types: TypeSet::LAND,
        subtypes: &[subtypes::land::CAVE],
    },
    ],
}

// TODO(card): implement abilities, see docs/card-dsl.md.
