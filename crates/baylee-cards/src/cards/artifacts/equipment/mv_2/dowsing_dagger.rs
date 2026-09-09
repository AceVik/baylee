//! Dowsing Dagger // Lost Vale — {2} — Artifact — Equipment // Land
//! Oracle: When this Equipment enters, target opponent creates two 0/2 green Plant creature tokens with defender.
//! Oracle: Equipped creature gets +2/+1.
//! Oracle: Whenever equipped creature deals combat damage to a player, you may transform this Equipment.
//! Oracle: Equip {2}
//! Oracle: (Transforms from Dowsing Dagger.)
//! Oracle: {T}: Add three mana of any one color.
//! Set: XLN #235 — Ixalan | Scryfall ID: 514d53be-6ade-4f73-a844-e9ae2dafd6ce | Oracle ID: df34a6ad-ae1c-4470-8c9e-49815bba1973
//! Face: Dowsing Dagger — {2} — Artifact — Equipment
//! Face: Lost Vale —  — Land
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card! {
    index: 433,
    oracle_id: "df34a6ad-ae1c-4470-8c9e-49815bba1973",
    scryfall_id: "514d53be-6ade-4f73-a844-e9ae2dafd6ce",
    faces: &[
    face! {
        name: "Dowsing Dagger",
        mana_cost: baylee_core::mana!("{2}"),
        types: TypeSet::ARTIFACT,
        subtypes: &[subtypes::artifact::EQUIPMENT],
    },
    face! {
        name: "Lost Vale",
        types: TypeSet::LAND,
    },
    ],
}

// TODO(card): implement abilities, see docs/card-dsl.md.
