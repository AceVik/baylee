//! Sea Serpent — {5}{U} — Creature — Serpent
//! Oracle: This creature can't attack unless defending player controls an Island.
//! Oracle: When you control no Islands, sacrifice this creature.
//! Set: ME4 #60 — Masters Edition IV | Scryfall ID: 1621438d-717a-477c-93ff-ff7415da70c5 | Oracle ID: c16495fc-784d-4bac-9a68-ed437008df73
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::SEA_SERPENT,
    oracle_id = "c16495fc-784d-4bac-9a68-ed437008df73",
    scryfall_id = "1621438d-717a-477c-93ff-ff7415da70c5",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    faces = &[face!(
        name = "Sea Serpent",
        mana_cost = mana!("{5}{U}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::SERPENT],
        power = Some(5),
        toughness = Some(5),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
