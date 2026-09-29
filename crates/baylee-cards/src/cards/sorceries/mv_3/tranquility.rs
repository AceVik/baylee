//! Tranquility — {2}{G} — Sorcery
//! Oracle: Destroy all enchantments.
//! Set: TPR #201 — Tempest Remastered | Scryfall ID: cdf5c7f6-1b7f-49e2-ac01-040e529383a3 | Oracle ID: f671e3c3-cd59-4d06-a1af-5d04892cf74d
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::TRANQUILITY,
    oracle_id = "f671e3c3-cd59-4d06-a1af-5d04892cf74d",
    scryfall_id = "cdf5c7f6-1b7f-49e2-ac01-040e529383a3",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    faces = &[face!(
        name = "Tranquility",
        mana_cost = mana!("{2}{G}"),
        types = TypeSet::SORCERY,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
