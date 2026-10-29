//! Harsh Mercy — {2}{W} — Sorcery
//! Oracle: Each player chooses a creature type. Destroy all creatures that aren't of a type chosen this way. They can't be regenerated.
//! Set: CMM #825 — Commander Masters | Scryfall ID: b1448ffd-42af-475c-8883-1933508a8a6a | Oracle ID: dcd65c9b-4aaa-42af-869f-de179ae57c9f
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::HARSH_MERCY,
    oracle_id = "dcd65c9b-4aaa-42af-869f-de179ae57c9f",
    scryfall_id = "b1448ffd-42af-475c-8883-1933508a8a6a",
    color_identity = ColorSet::from_slice(&[Color::White]),
    faces = &[face!(
        name = "Harsh Mercy",
        mana_cost = mana!("{2}{W}"),
        types = TypeSet::SORCERY,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
