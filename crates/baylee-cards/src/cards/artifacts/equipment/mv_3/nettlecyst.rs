//! Nettlecyst — {3} — Artifact — Equipment
//! Oracle: Living weapon (When this Equipment enters, create a 0/0 black Phyrexian Germ creature token, then attach this to it.)
//! Oracle: Equipped creature gets +1/+1 for each artifact and/or enchantment you control.
//! Oracle: Equip {2}
//! Set: MKC #233 — Murders at Karlov Manor Commander | Scryfall ID: 0a7cb0f8-2946-4b00-a192-0b31c8e1ec5c | Oracle ID: 04c7f4fe-2098-4311-866d-6733c08d5178
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::NETTLECYST,
    oracle_id = "04c7f4fe-2098-4311-866d-6733c08d5178",
    scryfall_id = "0a7cb0f8-2946-4b00-a192-0b31c8e1ec5c",
    faces = &[face!(
        name = "Nettlecyst",
        mana_cost = mana!("{3}"),
        types = TypeSet::ARTIFACT,
        subtypes = &[subtypes::artifact::EQUIPMENT],
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
