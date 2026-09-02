//! Command Tower — (no cost) — Land
//! Oracle: {T}: Add one mana of any color in your commander's color identity.
//! Set: MSC #233 — Marvel Super Heroes Commander | Scryfall ID: 0548fb60-c843-4f8f-a029-6f10efc63a41 | Oracle ID: 0895c9b7-ae7d-4bb3-af17-3b75deb50a25
// IMPLEMENTED — color choice from the union of your command-zone cards'
// color identities at resolution; falls back to {C} without a commander.

use baylee_cards_dsl::prelude::*;

card! {
    index: 23,
    oracle_id: "0895c9b7-ae7d-4bb3-af17-3b75deb50a25",
    scryfall_id: "0548fb60-c843-4f8f-a029-6f10efc63a41",
    faces: &[face! {
        name: "Command Tower",
        types: TypeSet::LAND,
    }],
    coverage: Coverage::Implemented,
    abilities: &[mana_ability!(&[Effect::mana_commander_identity()])],
}
