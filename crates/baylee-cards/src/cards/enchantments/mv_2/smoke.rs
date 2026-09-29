//! Smoke — {R}{R} — Enchantment
//! Oracle: Players can't untap more than one creature during their untap steps.
//! Set: ME4 #137 — Masters Edition IV | Scryfall ID: 109bea85-a991-46be-b502-0984160090a8 | Oracle ID: 8aa97d25-cd51-4ceb-b7eb-af64f0914a8c
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::SMOKE,
    oracle_id = "8aa97d25-cd51-4ceb-b7eb-af64f0914a8c",
    scryfall_id = "109bea85-a991-46be-b502-0984160090a8",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    faces = &[face!(
        name = "Smoke",
        mana_cost = mana!("{R}{R}"),
        types = TypeSet::ENCHANTMENT,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
