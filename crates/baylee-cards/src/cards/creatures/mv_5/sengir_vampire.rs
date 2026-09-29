//! Sengir Vampire — {3}{B}{B} — Creature — Vampire
//! Oracle: Flying (This creature can't be blocked except by creatures with flying or reach.)
//! Oracle: Whenever a creature dealt damage by this creature this turn dies, put a +1/+1 counter on this creature.
//! Set: JMP #275 — Jumpstart | Scryfall ID: de652420-eacf-4f9d-9f13-c6bc02b0fa72 | Oracle ID: 749141aa-f6c4-4ad8-b146-406e68ae9b0b
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::SENGIR_VAMPIRE,
    oracle_id = "749141aa-f6c4-4ad8-b146-406e68ae9b0b",
    scryfall_id = "de652420-eacf-4f9d-9f13-c6bc02b0fa72",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    faces = &[face!(
        name = "Sengir Vampire",
        mana_cost = mana!("{3}{B}{B}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::VAMPIRE],
        power = Some(4),
        toughness = Some(4),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
