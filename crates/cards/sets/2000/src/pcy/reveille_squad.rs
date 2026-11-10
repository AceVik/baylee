//! Reveille Squad — {2}{W}{W} — Creature — Human Rebel
//! Oracle: Whenever one or more creatures attack you, if this creature is untapped, you may untap all creatures you control.
//! Set: PCY #18 — Prophecy | Scryfall ID: 8f6385bb-18f9-461b-b541-3c2a5e59189b | Oracle ID: 2531eb1a-ae24-450a-b8e8-dbbd65eaad64
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::REVEILLE_SQUAD,
    oracle_id = "2531eb1a-ae24-450a-b8e8-dbbd65eaad64",
    scryfall_id = "8f6385bb-18f9-461b-b541-3c2a5e59189b",
    color_identity = ColorSet::from_slice(&[Color::White]),
    faces = &[face!(
        name = "Reveille Squad",
        mana_cost = mana!("{2}{W}{W}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::HUMAN, subtypes::creature::REBEL],
        power = Some(3),
        toughness = Some(3),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
