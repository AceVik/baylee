//! Hidden Nursery — (no cost) — Land — Cave
//! Oracle: This land enters tapped.
//! Oracle: {T}: Add {G}.
//! Oracle: {4}{G}, {T}, Sacrifice this land: Discover 4. Activate only as a sorcery. (Exile cards from the top of your library until you exile a nonland card with mana value 4 or less. Cast it without paying its mana cost or put it into your hand. Put the rest on the bottom in a random order.)
//! Set: LCI #276 — The Lost Caverns of Ixalan | Scryfall ID: a942939a-c06e-4b90-a404-ae5acfffcff9 | Oracle ID: 1a26e2d6-6bfc-4cdc-9bd6-8b37a9be2961
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card! {
    index: 624,
    oracle_id: "1a26e2d6-6bfc-4cdc-9bd6-8b37a9be2961",
    scryfall_id: "a942939a-c06e-4b90-a404-ae5acfffcff9",
    color_identity: ColorSet::from_slice(&[Color::Green]),
    faces: &[
    face! {
        name: "Hidden Nursery",
        types: TypeSet::LAND,
        subtypes: &[subtypes::land::CAVE],
    },
    ],
}

// TODO(card): implement abilities, see docs/card-dsl.md.
