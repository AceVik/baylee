//! Hidden Courtyard — (no cost) — Land — Cave
//! Oracle: This land enters tapped.
//! Oracle: {T}: Add {W}.
//! Oracle: {4}{W}, {T}, Sacrifice this land: Discover 4. Activate only as a sorcery. (Exile cards from the top of your library until you exile a nonland card with mana value 4 or less. Cast it without paying its mana cost or put it into your hand. Put the rest on the bottom in a random order.)
//! Set: LCI #274 — The Lost Caverns of Ixalan | Scryfall ID: b8685d46-99fc-44b3-be95-707a4b7b8327 | Oracle ID: e19d5071-4ea1-4883-b067-a21e553f96e0
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card! {
    index: 619,
    oracle_id: "e19d5071-4ea1-4883-b067-a21e553f96e0",
    scryfall_id: "b8685d46-99fc-44b3-be95-707a4b7b8327",
    color_identity: ColorSet::from_slice(&[Color::White]),
    faces: &[
    face! {
        name: "Hidden Courtyard",
        types: TypeSet::LAND,
        subtypes: &[subtypes::land::CAVE],
    },
    ],
}

// TODO(card): implement abilities, see docs/card-dsl.md.
