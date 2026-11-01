//! Kaervek's Torch — {X}{R} — Sorcery
//! Oracle: As long as Kaervek's Torch is on the stack, spells that target it cost {2} more to cast.
//! Oracle: Kaervek's Torch deals X damage to any target.
//! Set: VMA #175 — Vintage Masters | Scryfall ID: 83ff4446-4afd-42db-9879-0295e95764f7 | Oracle ID: c629feae-084c-452f-9c09-4b9716331498
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::KAERVEK_S_TORCH,
    oracle_id = "c629feae-084c-452f-9c09-4b9716331498",
    scryfall_id = "83ff4446-4afd-42db-9879-0295e95764f7",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    faces = &[face!(
        name = "Kaervek's Torch",
        mana_cost = mana!("{X}{R}"),
        types = TypeSet::SORCERY,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
