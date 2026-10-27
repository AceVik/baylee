//! Imagecrafter — {U} — Creature — Human Wizard
//! Oracle: {T}: Choose a creature type other than Wall. Target creature becomes that type until end of turn.
//! Set: ONS #87 — Onslaught | Scryfall ID: 91be6441-8a45-43e4-8d12-a886dcaadbd3 | Oracle ID: 9a0c6b4d-b16a-4849-9106-047b009a99b3
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::IMAGECRAFTER,
    oracle_id = "9a0c6b4d-b16a-4849-9106-047b009a99b3",
    scryfall_id = "91be6441-8a45-43e4-8d12-a886dcaadbd3",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    faces = &[face!(
        name = "Imagecrafter",
        mana_cost = mana!("{U}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::HUMAN, subtypes::creature::WIZARD],
        power = Some(1),
        toughness = Some(1),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
