//! Starting Town — (no cost) — Land — Town
//! Oracle: This land enters tapped unless it's your first, second, or third turn of the game.
//! Oracle: {T}: Add {C}.
//! Oracle: {T}, Pay 1 life: Add one mana of any color.
//! Set: FIN #289 — Final Fantasy | Scryfall ID: fc7d1912-7e27-49ef-bd98-375d975a42b0 | Oracle ID: d04e0975-f401-41b8-a9db-9bcf9cbbce66
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card! {
    index: 1085,
    oracle_id: "d04e0975-f401-41b8-a9db-9bcf9cbbce66",
    scryfall_id: "fc7d1912-7e27-49ef-bd98-375d975a42b0",
    faces: &[
    face! {
        name: "Starting Town",
        types: TypeSet::LAND,
        subtypes: &[subtypes::land::TOWN],
    },
    ],
}

// TODO(card): implement abilities, see docs/card-dsl.md.
