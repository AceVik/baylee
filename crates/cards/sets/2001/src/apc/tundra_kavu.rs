//! Tundra Kavu — {2}{R} — Creature — Kavu
//! Oracle: {T}: Target land becomes a Plains or an Island until end of turn.
//! Set: APC #71 — Apocalypse | Scryfall ID: fc34e735-ac3c-4954-a4c8-3ed55d811715 | Oracle ID: b0b7d88b-694b-4bbe-888d-05245e7d1c3e
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::TUNDRA_KAVU,
    oracle_id = "b0b7d88b-694b-4bbe-888d-05245e7d1c3e",
    scryfall_id = "fc34e735-ac3c-4954-a4c8-3ed55d811715",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    faces = &[face!(
        name = "Tundra Kavu",
        mana_cost = mana!("{2}{R}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::KAVU],
        power = Some(2),
        toughness = Some(2),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
