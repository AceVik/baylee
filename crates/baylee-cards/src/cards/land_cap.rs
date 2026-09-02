//! Land Cap — (no cost) — Land
//! Oracle: This land doesn't untap during your untap step if it has a depletion counter on it.
//! Oracle: At the beginning of your upkeep, remove a depletion counter from this land.
//! Oracle: {T}: Add {W} or {U}. Put a depletion counter on this land.
//! Set: ICE #357 — Ice Age | Scryfall ID: c4806c02-7a4d-42e3-affd-0338084bd3ab | Oracle ID: bfec4d0a-3792-4bc3-bae1-e639da5bb9a6
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card! {
    index: 703,
    oracle_id: "bfec4d0a-3792-4bc3-bae1-e639da5bb9a6",
    scryfall_id: "c4806c02-7a4d-42e3-affd-0338084bd3ab",
    color_identity: ColorSet::from_slice(&[Color::Blue, Color::White]),
    faces: &[
    face! {
        name: "Land Cap",
        types: TypeSet::LAND,
    },
    ],
}

// TODO(card): implement abilities, see docs/card-dsl.md.
