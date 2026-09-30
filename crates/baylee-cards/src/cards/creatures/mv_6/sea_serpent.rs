//! Sea Serpent — {5}{U} — Creature — Serpent
//! Oracle: This creature can't attack unless defending player controls an Island.
//! Oracle: When you control no Islands, sacrifice this creature.
//! Set: ME4 #60 — Masters Edition IV | Scryfall ID: 1621438d-717a-477c-93ff-ff7415da70c5 | Oracle ID: c16495fc-784d-4bac-9a68-ed437008df73
// PARTIAL — the attack restriction and the sacrifice when you control no Islands
// are not in the engine; it is a 5/5 without them.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::SEA_SERPENT,
    oracle_id = "c16495fc-784d-4bac-9a68-ed437008df73",
    scryfall_id = "1621438d-717a-477c-93ff-ff7415da70c5",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    coverage = Coverage::Partial(
        "the attack restriction and the sacrifice when you control no Islands are not in the engine; it is a 5/5 without them"
    ),
    faces = &[face!(
        name = "Sea Serpent",
        mana_cost = mana!("{5}{U}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::SERPENT],
        power = Some(5),
        toughness = Some(5),
    ),],
    abilities = &[
        // NOT SUPPORTED: This creature can't attack unless defending player controls an
        // Island.
        // NOT SUPPORTED: When you control no Islands, sacrifice this creature.
    ],
);
