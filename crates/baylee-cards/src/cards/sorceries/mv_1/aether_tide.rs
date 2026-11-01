//! Aether Tide — {X}{U} — Sorcery
//! Oracle: As an additional cost to cast this spell, discard X creature cards.
//! Oracle: Return X target creatures to their owners' hands.
//! Set: EXO #27 — Exodus | Scryfall ID: 9aab7526-5825-4f31-92ff-be25ab5af2f5 | Oracle ID: 2fbf95b4-bcf4-4b5e-b5dc-0294f2b48d3e
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::AETHER_TIDE,
    oracle_id = "2fbf95b4-bcf4-4b5e-b5dc-0294f2b48d3e",
    scryfall_id = "9aab7526-5825-4f31-92ff-be25ab5af2f5",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    faces = &[face!(
        name = "Aether Tide",
        mana_cost = mana!("{X}{U}"),
        types = TypeSet::SORCERY,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
