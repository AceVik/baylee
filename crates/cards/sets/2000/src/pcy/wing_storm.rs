//! Wing Storm — {2}{G} — Sorcery
//! Oracle: Wing Storm deals damage to each player equal to twice the number of creatures that player controls with flying.
//! Set: PCY #135 — Prophecy | Scryfall ID: 59e66be1-f18a-433f-8504-aa1e85e22023 | Oracle ID: c593f66d-0e9a-4d46-a9c6-ac7a70bf17ea
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::WING_STORM,
    oracle_id = "c593f66d-0e9a-4d46-a9c6-ac7a70bf17ea",
    scryfall_id = "59e66be1-f18a-433f-8504-aa1e85e22023",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    faces = &[face!(
        name = "Wing Storm",
        mana_cost = mana!("{2}{G}"),
        types = TypeSet::SORCERY,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
