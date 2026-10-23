//! Infernal Denizen — {7}{B} — Creature — Demon
//! Oracle: At the beginning of your upkeep, sacrifice two Swamps. If you can't, tap this creature, and an opponent may gain control of a creature you control of their choice for as long as this creature remains on the battlefield.
//! Oracle: {T}: Gain control of target creature for as long as this creature remains on the battlefield.
//! Set: ICE #136 — Ice Age | Scryfall ID: b63ac9a6-aaa5-4659-97d1-c5f6b0d5ccfe | Oracle ID: 2f33aced-c737-4b3c-ba25-aff7b340c2d7
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

// Held back: Unsafe rule approximation found in Argon PR 394: missing printed cost, restriction, drawback, choice or target/zone semantics; the concrete finding for this card is recorded in docs/history/2026-10-23-argon-review.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::INFERNAL_DENIZEN,
    oracle_id = "2f33aced-c737-4b3c-ba25-aff7b340c2d7",
    scryfall_id = "b63ac9a6-aaa5-4659-97d1-c5f6b0d5ccfe",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    faces = &[face!(
        name = "Infernal Denizen",
        mana_cost = mana!("{7}{B}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::DEMON],
        power = Some(5),
        toughness = Some(7),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
