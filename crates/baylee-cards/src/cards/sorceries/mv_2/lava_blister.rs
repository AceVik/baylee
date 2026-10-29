//! Lava Blister — {1}{R} — Sorcery
//! Oracle: Destroy target nonbasic land unless its controller has Lava Blister deal 6 damage to them.
//! Set: ODY #200 — Odyssey | Scryfall ID: cd0e9e53-2710-4c2a-a8e4-48f25375ebc7 | Oracle ID: 3260bede-d78d-41a6-8fe1-95ce14a2ba8c
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::LAVA_BLISTER,
    oracle_id = "3260bede-d78d-41a6-8fe1-95ce14a2ba8c",
    scryfall_id = "cd0e9e53-2710-4c2a-a8e4-48f25375ebc7",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    faces = &[face!(
        name = "Lava Blister",
        mana_cost = mana!("{1}{R}"),
        types = TypeSet::SORCERY,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
