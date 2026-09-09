//! Reflecting Pool — (no cost) — Land
//! Oracle: {T}: Add one mana of any type that a land you control could produce.
//! Set: CLB #358 — Commander Legends: Battle for Baldur's Gate | Scryfall ID: 18a1b3f5-473d-45ca-be0d-e67e77ba30ce | Oracle ID: 67f43ac6-2a58-4b53-b5d7-0330e2a252e2
// IMPLEMENTED — color choice from your lands' producible mana
// (colorless included when a land can produce it). The Pool contributes
// nothing to that union itself, so a lone Pool taps for nothing.

use baylee_cards_dsl::prelude::*;

card! {
    index: 128,
    oracle_id: "67f43ac6-2a58-4b53-b5d7-0330e2a252e2",
    scryfall_id: "18a1b3f5-473d-45ca-be0d-e67e77ba30ce",
    faces: &[face! {
        name: "Reflecting Pool",
        types: TypeSet::LAND,
    }],
    coverage: Coverage::Implemented,
    abilities: &[mana_ability!(&[Effect::mana_land_color(true)])],
}
