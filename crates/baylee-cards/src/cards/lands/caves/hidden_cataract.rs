//! Hidden Cataract — (no cost) — Land — Cave
//! Oracle: This land enters tapped.
//! Oracle: {T}: Add {U}.
//! Oracle: {4}{U}, {T}, Sacrifice this land: Discover 4. Activate only as a sorcery. (Exile cards from the top of your library until you exile a nonland card with mana value 4 or less. Cast it without paying its mana cost or put it into your hand. Put the rest on the bottom in a random order.)
//! Set: LCI #273 — The Lost Caverns of Ixalan | Scryfall ID: 69f317fc-f603-45b5-9208-545be4dcbf36 | Oracle ID: 927979d7-9b5c-4448-aef0-baf2907a89f1
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card! {
    index: 618,
    oracle_id: "927979d7-9b5c-4448-aef0-baf2907a89f1",
    scryfall_id: "69f317fc-f603-45b5-9208-545be4dcbf36",
    color_identity: ColorSet::from_slice(&[Color::Blue]),
    faces: &[
    face! {
        name: "Hidden Cataract",
        types: TypeSet::LAND,
        subtypes: &[subtypes::land::CAVE],
    },
    ],
}

// TODO(card): implement abilities, see docs/card-dsl.md.
