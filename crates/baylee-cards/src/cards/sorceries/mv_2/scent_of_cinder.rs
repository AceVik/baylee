//! Scent of Cinder — {1}{R} — Sorcery
//! Oracle: Reveal any number of red cards in your hand. Scent of Cinder deals X damage to any target, where X is the number of cards revealed this way.
//! Set: UDS #96 — Urza's Destiny | Scryfall ID: c030eca0-bc5f-403b-8600-1f295fc85fee | Oracle ID: ff50d002-834e-4b5e-8db0-b0c8b4efb21f
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::SCENT_OF_CINDER,
    oracle_id = "ff50d002-834e-4b5e-8db0-b0c8b4efb21f",
    scryfall_id = "c030eca0-bc5f-403b-8600-1f295fc85fee",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    faces = &[face!(
        name = "Scent of Cinder",
        mana_cost = mana!("{1}{R}"),
        types = TypeSet::SORCERY,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
