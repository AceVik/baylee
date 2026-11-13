//! Dracoplasm — {U}{R} — Creature — Shapeshifter
//! Oracle: Flying
//! Oracle: As this creature enters, sacrifice any number of creatures. This creature's power becomes the total power of those creatures and its toughness becomes their total toughness.
//! Oracle: {R}: This creature gets +1/+0 until end of turn.
//! Set: TPR #208 — Tempest Remastered | Scryfall ID: 5dc9a21b-0935-4526-a0b0-9a85d0613e44 | Oracle ID: 3155c833-8170-4166-8f47-c3c1fad1575a
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

// Held back: Unsafe rule approximation found in Argon PR 394: missing printed cost, restriction, drawback, choice or target/zone semantics; the concrete finding for this card is recorded in docs/history/2026-10-23-argon-review.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::DRACOPLASM,
    oracle_id = "3155c833-8170-4166-8f47-c3c1fad1575a",
    scryfall_id = "5dc9a21b-0935-4526-a0b0-9a85d0613e44",
    color_identity = ColorSet::from_slice(&[Color::Red, Color::Blue]),
    faces = &[face!(
        name = "Dracoplasm",
        mana_cost = mana!("{U}{R}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::SHAPESHIFTER],
        power = Some(0),
        toughness = Some(0),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
