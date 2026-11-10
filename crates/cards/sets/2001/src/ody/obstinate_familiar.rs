//! Obstinate Familiar — {R} — Creature — Lizard
//! Oracle: If you would draw a card, you may skip that draw instead.
//! Set: ODY #210 — Odyssey | Scryfall ID: 88468a76-1f64-4189-bbb8-7c333181d57c | Oracle ID: fbfa8b3f-58a7-44ca-8c22-6747b56fdcb9
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::OBSTINATE_FAMILIAR,
    oracle_id = "fbfa8b3f-58a7-44ca-8c22-6747b56fdcb9",
    scryfall_id = "88468a76-1f64-4189-bbb8-7c333181d57c",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    faces = &[face!(
        name = "Obstinate Familiar",
        mana_cost = mana!("{R}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::LIZARD],
        power = Some(1),
        toughness = Some(1),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
