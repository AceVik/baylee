//! Mad Dog — {1}{R} — Creature — Dog
//! Oracle: At the beginning of your end step, if this creature didn't attack or come under your control this turn, sacrifice it.
//! Set: ODY #202 — Odyssey | Scryfall ID: 7be2da24-a0cd-4e90-bc29-4da649fa56bf | Oracle ID: 097bce84-22b1-4d7b-b566-e7c23eb626b9
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::MAD_DOG,
    oracle_id = "097bce84-22b1-4d7b-b566-e7c23eb626b9",
    scryfall_id = "7be2da24-a0cd-4e90-bc29-4da649fa56bf",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    faces = &[face!(
        name = "Mad Dog",
        mana_cost = mana!("{1}{R}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::DOG],
        power = Some(2),
        toughness = Some(2),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
